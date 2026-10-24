use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ptr::null_mut;

use crate::game::param::mouth_type_count;
use crate::game::ui::chara_make::{
    CATEGORY_MOUTH, CharaMakePage, CharaMakeSeq, FACE_PARAM_MOUTH, RequestState,
};
use crate::game::ui::{ItemEvent, ItemOp, ItemUpdater, ListBtnGroup, PlayerIcon, VftListBtnGroup};
use crate::lp::ui::{Control, LeaveDirection, LeaveEvent, NavDirection, ScrollBtnListCtrl};
use crate::native::{NativeResult, report, text_vtable};
use crate::sead::{Delegate, ObjArray, VftDelegate};

const LIST_PATH: &core::ffi::CStr = c"/L_BtnListMouth_00";
const MAX_SLOTS: usize = 32;
const MAX_MOUTHS: usize = 256;

const SCROLL_DELEGATE_VTABLE: usize = 0x49b5088;
const LEAVE_DELEGATE_VTABLE: usize = 0x49b50d8;
const ITEM_DELEGATE_VTABLE: usize = 0x49b5268;

// Writable replacement storage for the seq's mouth value list (item -> mouth value). The game
// recycles the list's slots when it refills it, so it can't point at read-only data.
struct MouthValues {
    nodes: [u64; MAX_MOUTHS],
    node_ptrs: [*mut u32; MAX_MOUTHS],
    count: usize,
    // The game's own storage while ours is installed.
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
    // Whether the nose/mouth tab has the list switched on; the focus delegates run on every tab.
    enabled: bool,
}

struct State(UnsafeCell<MouthList>);

// The CharaMake UI runs on a single thread.
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

/// The mouth list, if it's bound to this seq.
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

    // Swap the seq's mouth value list over to our storage: the game's own entries first (in its
    // order), then every other CharaMakeMouthTypeParam row. Without the table, leave the game's list.
    unsafe fn own(&mut self, list: &mut ObjArray<u32>) {
        unsafe {
            self.release(list);
            self.count = 0;

            let game = &list.base;
            let rows = mouth_type_count().min(MAX_MOUTHS);
            let game_values = (0..game.size.max(0) as usize).map(|i| **game.ptrs.add(i));
            let mut count = 0;

            for value in game_values.chain(0..rows as u32) {
                if (value as usize) < rows && !self.nodes[..count].contains(&(value as u64)) {
                    self.nodes[count] = value as u64;
                    count += 1;
                }
            }

            if count == 0 {
                return;
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

    // Nose button c links down to the top of visible line c, OK links up to the bottom of the
    // middle visible line. The list scrolls through a ring of buttons, so this follows its window.
    unsafe fn relink(&mut self) {
        unsafe {
            let list = &*self.list;
            let page = self.page();
            let last = list.line_size.max(1) as usize - 1;

            for (line, &nose) in page.nose_buttons.iter().enumerate() {
                if let Some(nose) = nose.as_mut() {
                    nose.set_link(NavDirection::Down, list.visible_button(line, 0));
                }
            }

            if let Some(ok) = page.ok_button.as_mut() {
                ok.set_link(NavDirection::Up, list.visible_button(page.nose_buttons.len() / 2, last));
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

// CharaMakePage::vf_0d0 runs while parts are built, on every CharaMake entry; look the list up the
// same way the game does for its six lists and bind its buttons whenever they were rebuilt.
pub unsafe fn bind(page: *mut CharaMakePage, parent: *mut Control, builder: *mut c_void) -> NativeResult {
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

        let Some(list) = ScrollBtnListCtrl::cast(parent.find_by_path(LIST_PATH)?) else {
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
        state.group = ListBtnGroup::new();

        if !list.scroll_delegate.is_bound() {
            list.scroll_delegate =
                Delegate::new(text_vtable(SCROLL_DELEGATE_VTABLE), seq, on_scroll as *const () as usize);
        }

        if !list.leave_delegate.is_bound() {
            list.leave_delegate = Delegate::new(text_vtable(LEAVE_DELEGATE_VTABLE), seq, on_leave as *const () as usize);
        }

        state.slots = list.slot_count().min(MAX_SLOTS);
        state.icons = [null_mut(); MAX_SLOTS];

        for slot in 0..state.slots {
            let Some(button) = list.button(slot).as_mut() else {
                continue;
            };

            button.set_list_mode();
            button.bind_to_group(&mut state.group);
            state.icons[slot] = PlayerIcon::create(button, &mut seq_ref.icon_texture_pool)?;
        }

        state.updater = ItemUpdater::new(
            state.slots as u32,
            Delegate::new(text_vtable(ITEM_DELEGATE_VTABLE), seq, on_item as *const () as usize),
        );
        state.values.own(&mut seq_ref.mouth_values);

        Ok(())
    }
}

// The tab switches its mouth group on and off; the list takes the old buttons' place in it.
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

// The game refills the mouth value list on enter; let it work on its own storage, then take over.
pub unsafe fn fill(values: *mut ObjArray<u32>, game_fill: impl FnOnce()) {
    unsafe {
        let state = state();
        let ours = !state.seq.is_null() && core::ptr::eq(&(*state.seq).mouth_values, values);

        if ours {
            state.values.release(&mut *values);
        }

        game_fill();

        if ours {
            state.values.own(&mut *values);
        }
    }
}

extern "C" fn on_scroll(seq: *mut CharaMakeSeq, event: *mut c_void) {
    let result = unsafe {
        match active(seq) {
            Some(state) => state.updater.update(event).and_then(|()| state.group.refresh(state.list)),
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
        let slot_of = |slot: i32| if (0..state.slots as i32).contains(&slot) { slot as usize } else { 0 };
        let Some(icon) = state.icons[slot_of(event.slot)].as_mut() else {
            return Ok(());
        };

        let Some(op) = event.op() else {
            return Ok(());
        };

        if op == ItemOp::Clear {
            return icon.release();
        }

        let item = event.item();
        let in_range = item >= 0 && (item as usize) < state.values.count;
        let mut params = (*seq).face_params;

        params[FACE_PARAM_MOUTH] = if in_range { state.values.value(item as usize) as u8 } else { 0 };

        match op {
            ItemOp::Move => icon.move_from(state.icons[slot_of(event.from_slot)], &params),
            ItemOp::Render if in_range => icon.render(&params),
            _ => Ok(()),
        }
    }
}

// Leaving the list vertically: up goes to the nose button above the column (the reverse of its down
// link), down to the OK button. The list still has the slot it leaves from marked focused here.
extern "C" fn on_leave(seq: *mut CharaMakeSeq, target: *mut *mut Control, event: *const LeaveEvent) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let line = state.list().focused_visible_line();
        let page = state.page();
        let nose = line.map_or(page.nose_buttons.len() / 2, |line| line.min(page.nose_buttons.len() - 1));
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

// Page refresh: select the current mouth and (re)fill the list, which renders the visible items. The
// refresh also relinks every tab's grid, so on the mouth tab point the nose row and OK back at the list.
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

// The UpdateAnchor request points the OK button's up link at the old mouth button for the
// focused index.
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
            // Focus set from outside (onTabEnter focusing the nose row) bypasses the list, which would
            // stay in cPlay and keep handling the D-pad from its own cursor.
            state.list().pause()?;
            state.list().enable_visible_buttons()?;
            state.relink();
        }

        Ok(())
    }
}

// Mirrors the game's eye list branch of onControlDecide.
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

// Runs every time CharaMake is left; the seq, page and list stay alive and are reused on return,
// so only release the textures and hand the value list back (the fill hook takes it again).
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
