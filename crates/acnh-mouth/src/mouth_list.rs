use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ptr::null_mut;

use crate::game::param::mouth_type_count;
use crate::game::ui::chara_make::{
    CATEGORY_MOUTH, CharaMakePage, CharaMakeSeq, FACE_PARAM_MOUTH, RequestState,
};
use crate::game::ui::{ItemEvent, ItemOp, ItemUpdater, ListBtnGroup, PlayerIcon, VftListBtnGroup};
use crate::lp::ui::{Control, LeaveDirection, LeaveEvent, NavDirection, ScrollBtnListCtrl};
use crate::native::{NativeResult, report};
use crate::sead::{Delegate, ObjArray, VftDelegate};

const LIST_PATH: &core::ffi::CStr = c"/L_BtnListMouth_00";
const MAX_SLOTS: usize = 32;
const MAX_MOUTHS: usize = 256;

struct MouthValues {
    // The game reuses these slots on refill
    nodes: [u64; MAX_MOUTHS],

    node_ptrs: [*mut u32; MAX_MOUTHS],
    count: usize,

    // The game's storage while ours is installed
    saved: Option<(i32, i32, *mut *mut u32)>,
}

struct MouthList {
    page: *mut CharaMakePage,
    seq: *mut CharaMakeSeq,
    list: *mut ScrollBtnListCtrl,
    group: ListBtnGroup,
    updater: ItemUpdater,
    icons: [*mut PlayerIcon; MAX_SLOTS],
    slots: usize,
    values: MouthValues,

    // The focus delegates run on every tab
    enabled: bool,
}

struct State(UnsafeCell<MouthList>);

// CharaMake UI is single-threaded
unsafe impl Sync for State {}

static STATE: State = State(UnsafeCell::new(MouthList {
    page: null_mut(),
    seq: null_mut(),
    list: null_mut(),
    group: ListBtnGroup::empty(),
    updater: ItemUpdater::empty(),
    icons: [null_mut(); MAX_SLOTS],
    slots: 0,
    values: MouthValues {
        nodes: [0; MAX_MOUTHS],
        node_ptrs: [null_mut(); MAX_MOUTHS],
        count: 0,
        saved: None,
    },
    enabled: false,
}));

#[allow(clippy::mut_from_ref)]
unsafe fn state() -> &'static mut MouthList {
    unsafe { &mut *STATE.0.get() }
}

#[cfg(feature = "framework")]
pub unsafe fn seq() -> *mut CharaMakeSeq {
    unsafe { state().seq }
}

unsafe fn active(seq: *mut CharaMakeSeq) -> Option<&'static mut MouthList> {
    unsafe {
        let state = state();

        (!seq.is_null() && state.seq == seq && !state.list.is_null()).then_some(state)
    }
}

impl MouthValues {
    fn value(&self, item: usize) -> u32 {
        self.nodes[item] as u32
    }

    fn item_of(&self, value: u32) -> Option<usize> {
        (0..self.count).find(|&item| self.value(item) == value)
    }

    unsafe fn own(&mut self, list: &mut ObjArray<u32>) -> NativeResult {
        unsafe {
            self.release(list);
            self.count = 0;

            let game = &list.base;
            let rows = mouth_type_count()?.min(MAX_MOUTHS);
            let game_values = (0..game.size.max(0) as usize).map(|i| **game.ptrs.add(i));
            let mut count = 0;

            for value in game_values.chain(0..rows as u32) {
                if (value as usize) < rows && !self.nodes[..count].contains(&(value as u64)) {
                    self.nodes[count] = value as u64;
                    count += 1;
                }
            }

            if count == 0 {
                return Ok(());
            }

            for (ptr, node) in self.node_ptrs.iter_mut().zip(self.nodes.iter_mut()) {
                *ptr = (node as *mut u64).cast();
            }

            self.saved = Some((list.base.size, list.base.capacity, list.base.ptrs));
            self.count = count;

            list.base.size = count as i32;
            list.base.capacity = count as i32;
            list.base.ptrs = self.node_ptrs.as_mut_ptr();
        }

        Ok(())
    }

    fn release(&mut self, list: &mut ObjArray<u32>) {
        if let Some((size, capacity, ptrs)) = self.saved.take() {
            list.base.size = size;
            list.base.capacity = capacity;
            list.base.ptrs = ptrs;
        }
    }
}

impl MouthList {
    unsafe fn seq(&mut self) -> &mut CharaMakeSeq {
        unsafe { &mut *self.seq }
    }

    unsafe fn page(&mut self) -> &mut CharaMakePage {
        unsafe { &mut *self.seq().page }
    }

    unsafe fn list(&mut self) -> &mut ScrollBtnListCtrl {
        unsafe { &mut *self.list }
    }

    unsafe fn post_request(&mut self, state: RequestState, index: i32) {
        unsafe { self.seq().request.post(state, index, CATEGORY_MOUTH) };
    }

    unsafe fn relink(&mut self) {
        unsafe {
            let list = &*self.list;
            let page = self.page();
            let last = list.line_size.max(1) as usize - 1;

            // Nose button C links to visible line C, OK to the middle line
            for (line, &nose) in page.nose_buttons.iter().enumerate() {
                if let Some(nose) = nose.as_mut() {
                    nose.set_link(NavDirection::Down, list.visible_button(line, 0));
                }
            }

            if let Some(ok) = page.ok_button.as_mut() {
                ok.set_link(
                    NavDirection::Up,
                    list.visible_button(page.nose_buttons.len() / 2, last),
                );
            }
        }
    }

    unsafe fn release_icons(&mut self) -> NativeResult {
        for &icon in &self.icons[..self.slots] {
            if let Some(icon) = unsafe { icon.as_mut() } {
                unsafe { icon.release()? };
            }
        }

        Ok(())
    }
}

pub unsafe fn bind(
    page: *mut CharaMakePage,
    parent: *mut Control,
    builder: *mut c_void,
) -> NativeResult {
    unsafe {
        let state = state();

        if state.page != page {
            state.page = page;
            state.seq = null_mut();
            state.list = null_mut();
        }

        let Some(parent) = parent.as_mut() else {
            return Ok(());
        };

        let Some(list) = ScrollBtnListCtrl::cast(parent.find_by_path(LIST_PATH)?)? else {
            return Ok(());
        };

        if core::ptr::eq(list, state.list) && list.button_count() != 0 {
            return Ok(());
        }

        list.bind_buttons(builder)?;
        state.list = list;

        Ok(())
    }
}

pub unsafe fn setup(seq: *mut CharaMakeSeq) -> NativeResult {
    unsafe {
        let state = state();
        let seq_ref = &mut *seq;

        if state.page != seq_ref.page || state.list.is_null() {
            return Ok(());
        }

        state.seq = seq;

        let page = &mut *seq_ref.page;
        let list = &mut *state.list;

        list.attach(page as *mut CharaMakePage as *mut c_void, *page.root)?;
        state.group = ListBtnGroup::new()?;

        if !list.scroll_delegate.is_bound() {
            list.scroll_delegate = Delegate::bind(seq, on_scroll as *const () as usize);
        }

        if !list.leave_delegate.is_bound() {
            list.leave_delegate = Delegate::bind(seq, on_leave as *const () as usize);
        }

        state.slots = list.slot_count().min(MAX_SLOTS);
        state.icons = [null_mut(); MAX_SLOTS];

        for slot in 0..state.slots {
            let Some(button) = list.button(slot).as_mut() else {
                continue;
            };

            button.set_list_mode();
            button.bind_to_group(&mut state.group)?;
            state.icons[slot] = PlayerIcon::create(button, &mut seq_ref.icon_texture_pool)?;
        }

        state.updater = ItemUpdater::new(
            state.slots as u32,
            Delegate::bind(seq, on_item as *const () as usize),
        );
        state.values.own(&mut seq_ref.mouth_values)
    }
}

pub unsafe fn set_enabled(seq: *mut CharaMakeSeq, enable: bool, mode: u32) -> NativeResult {
    unsafe {
        if let Some(state) = active(seq) {
            state.enabled = enable;
            state.list().set_enabled(enable, mode)?;
        }

        Ok(())
    }
}

pub unsafe fn is_active(seq: *mut CharaMakeSeq) -> bool {
    unsafe { active(seq).is_some() }
}

pub unsafe fn fill(values: *mut ObjArray<u32>, game_fill: impl FnOnce()) -> NativeResult {
    unsafe {
        let state = state();
        let ours = !state.seq.is_null() && core::ptr::eq(&(*state.seq).mouth_values, values);

        if ours {
            state.values.release(&mut *values);
        }

        game_fill();

        if ours {
            state.values.own(&mut *values)?;
        }

        Ok(())
    }
}

extern "C" fn on_scroll(seq: *mut CharaMakeSeq, event: *mut c_void) {
    let result = unsafe {
        match active(seq) {
            Some(state) => state
                .updater
                .update(event)
                .and_then(|()| Ok(state.group.refresh(state.list)?)),
            None => Ok(()),
        }
    };

    report("scroll", result);
}

extern "C" fn on_item(seq: *mut CharaMakeSeq, event: *const ItemEvent) {
    report("item", unsafe { update_item(seq, event) });
}

unsafe fn update_item(seq: *mut CharaMakeSeq, event: *const ItemEvent) -> NativeResult {
    unsafe {
        let Some(state) = active(seq) else {
            return Ok(());
        };

        let event = &*event;
        let slot_of = |slot: i32| {
            if (0..state.slots as i32).contains(&slot) {
                slot as usize
            } else {
                0
            }
        };
        let Some(icon) = state.icons[slot_of(event.slot)].as_mut() else {
            return Ok(());
        };

        let Some(op) = event.op() else {
            return Ok(());
        };

        if op == ItemOp::Clear {
            return Ok(icon.release()?);
        }

        let item = event.item();
        let in_range = item >= 0 && (item as usize) < state.values.count;
        let mut params = (*seq).face_params;

        params[FACE_PARAM_MOUTH] = if in_range {
            state.values.value(item as usize) as u8
        } else {
            0
        };

        match op {
            ItemOp::Move => icon.move_from(state.icons[slot_of(event.from_slot)], &params)?,
            ItemOp::Render if in_range => icon.render(&params)?,
            _ => {}
        }

        Ok(())
    }
}

extern "C" fn on_leave(
    seq: *mut CharaMakeSeq,
    target: *mut *mut Control,
    event: *const LeaveEvent,
) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let line = state.list().focused_visible_line();
        let page = state.page();

        let nose = line.map_or(page.nose_buttons.len() / 2, |line| {
            line.min(page.nose_buttons.len() - 1)
        });

        let control = match (*event).direction() {
            Some(LeaveDirection::Up) => page.nose_buttons[nose],
            Some(LeaveDirection::Down) => page.ok_button,
            None => null_mut(),
        };

        if let Some(control) = control.as_mut() {
            *target = control.as_control();
        }
    }
}

// The refresh relinks every tab, so point the nose row and OK back at the list
pub unsafe fn refresh(seq: *mut CharaMakeSeq) -> NativeResult {
    unsafe {
        let Some(state) = active(seq).filter(|state| state.values.count != 0) else {
            return Ok(());
        };

        let mouth = state.seq().face_params[FACE_PARAM_MOUTH] as u32;
        let selected = state.values.item_of(mouth).unwrap_or(0) as i32;
        let (count, player) = (state.values.count as i32, state.seq().player);

        state.group.selected = selected;
        state.list().set_content(count, selected, player)?;

        if state.enabled {
            state.relink();
        }

        Ok(())
    }
}

// UpdateAnchor links OK to the old mouth buttons
pub unsafe fn after_request(seq: *mut CharaMakeSeq) {
    unsafe {
        if let Some(state) = active(seq).filter(|state| state.enabled) {
            state.relink();
        }
    }
}

pub unsafe fn on_focus(seq: *mut CharaMakeSeq, control: *mut Control) -> NativeResult {
    unsafe {
        let Some(state) = active(seq) else {
            return Ok(());
        };

        let index = state.list().item_index_of(control)?;

        if index >= 0 {
            state.post_request(RequestState::Focus, index);
        } else if state.enabled {
            // Outside focus (onTabEnter) bypasses the list, which would keep the D-pad
            state.list().pause()?;
            state.list().enable_visible_buttons()?;
            state.relink();
        }

        Ok(())
    }
}

pub unsafe fn on_decide(seq: *mut CharaMakeSeq, control: *mut Control) -> NativeResult {
    unsafe {
        let Some(state) = active(seq) else {
            return Ok(());
        };

        let index = state.list().item_index_of(control)?;

        if index < 0 || state.group.locked {
            return Ok(());
        }

        let list = state.list;
        let group = &mut state.group;

        if group.is_changed(index) {
            group.clear_selection(list);
            group.set_selected(index);
        } else if !group.toggle {
            group.clear_selection(list);
            group.set_selected(-1);

            return Ok(());
        } else if group.keep == 0 {
            return Ok(());
        }

        let selected = group.selected().max(0);

        state.post_request(RequestState::Decide, selected);

        Ok(())
    }
}

pub unsafe fn tick(seq: *mut CharaMakeSeq) -> NativeResult {
    unsafe {
        let Some(state) = active(seq) else {
            return Ok(());
        };

        for &icon in &state.icons[..state.slots] {
            if let Some(icon) = icon.as_mut() {
                icon.tick()?;
            }
        }

        Ok(())
    }
}

pub unsafe fn exit(seq: *mut CharaMakeSeq) -> NativeResult {
    unsafe {
        let Some(state) = active(seq) else {
            return Ok(());
        };

        state.enabled = false;
        let released = state.release_icons();
        state.values.release(&mut (*seq).mouth_values);

        released
    }
}
