use core::cell::UnsafeCell;
use core::ptr::null_mut;

use acnh_ui_api::{Direction, ItemUpdate, List, ListTarget, PageFilter, Ui};

use crate::game::param::mouth_type_count;
use crate::game::ui::chara_make::{CATEGORY_MOUTH, CharaMakeSeq, FACE_PARAM_MOUTH, RequestState};
use crate::game::ui::{AppUniteBtnCtrl, PlayerIcon};
use crate::native::{NativeResult, report};
use crate::sead::ObjArray;

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

struct Mouths {
    seq: *mut CharaMakeSeq,
    list: Option<List>,
    icons: [*mut PlayerIcon; MAX_SLOTS],
    slots: usize,
    values: MouthValues,

    // The mouth tab is showing the list
    enabled: bool,
}

struct State(UnsafeCell<Mouths>);

// CharaMake UI is single-threaded
unsafe impl Sync for State {}

static STATE: State = State(UnsafeCell::new(Mouths {
    seq: null_mut(),
    list: None,
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
unsafe fn mouths() -> &'static mut Mouths {
    unsafe { &mut *STATE.0.get() }
}

unsafe fn active(seq: *mut CharaMakeSeq) -> Option<&'static mut Mouths> {
    unsafe {
        let mouths = mouths();

        (!seq.is_null() && mouths.seq == seq && mouths.list.is_some()).then_some(mouths)
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

impl Mouths {
    unsafe fn seq(&mut self) -> &mut CharaMakeSeq {
        unsafe { &mut *self.seq }
    }

    unsafe fn post(&mut self, state: RequestState, item: usize) {
        unsafe { self.seq().request.post(state, item as i32, CATEGORY_MOUTH) };
    }

    unsafe fn ready(&mut self, seq: *mut CharaMakeSeq, list: List) -> NativeResult {
        self.seq = seq;
        self.list = Some(list);
        self.slots = list.slots().unwrap_or(0).min(MAX_SLOTS);
        self.icons = [null_mut(); MAX_SLOTS];

        unsafe {
            let seq = &mut *seq;

            for slot in 0..self.slots {
                let Some(button) = list
                    .button(slot)
                    .ok()
                    .and_then(|b| b.raw().cast::<AppUniteBtnCtrl>().as_mut())
                else {
                    continue;
                };

                self.icons[slot] = PlayerIcon::create(button, &mut seq.icon_texture_pool)?;
            }

            self.values.own(&mut seq.mouth_values)
        }
    }

    unsafe fn render(&mut self, update: ItemUpdate) -> NativeResult {
        let icon_at = |icons: &[*mut PlayerIcon; MAX_SLOTS], slot: usize| {
            icons.get(slot).copied().unwrap_or(null_mut())
        };

        unsafe {
            let mut params = self.seq().face_params;
            let mouth_of =
                |item: usize| (item < self.values.count).then(|| self.values.value(item) as u8);

            match update {
                ItemUpdate::Clear { slot } => {
                    if let Some(icon) = icon_at(&self.icons, slot).as_mut() {
                        icon.release()?;
                    }
                }
                ItemUpdate::Move { slot, from, item } => {
                    params[FACE_PARAM_MOUTH] = mouth_of(item).unwrap_or(0);

                    if let Some(icon) = icon_at(&self.icons, slot).as_mut() {
                        icon.move_from(icon_at(&self.icons, from), &params)?;
                    }
                }
                ItemUpdate::Render { slot, item } => {
                    let (Some(mouth), Some(icon)) =
                        (mouth_of(item), icon_at(&self.icons, slot).as_mut())
                    else {
                        return Ok(());
                    };

                    params[FACE_PARAM_MOUTH] = mouth;
                    icon.render(&params)?;
                }
            }
        }

        Ok(())
    }

    unsafe fn tick(&mut self) -> NativeResult {
        for &icon in &self.icons[..self.slots] {
            if let Some(icon) = unsafe { icon.as_mut() } {
                unsafe { icon.tick()? };
            }
        }

        Ok(())
    }
}

pub fn register(ui: &Ui) -> acnh_ui_api::Result {
    ui.on_page(PageFilter::Layout("CharaMake"), |page| {
        let list = page.list("/L_BtnListMouth_00")?;
        let nose = [
            page.pane("/L_BtnNose_00")?,
            page.pane("/L_BtnNose_01")?,
            page.pane("/L_BtnNose_02")?,
        ];
        let ok = page.pane("/L_BtnOk_00")?;

        list.on_item(|update| {
            report("item", unsafe { mouths().render(update) });
            Ok(())
        })?
        .on_focus(|item| {
            unsafe { mouths().post(RequestState::Focus, item) };
            Ok(())
        })?
        .on_decide(|item| {
            unsafe { mouths().post(RequestState::Decide, item) };
            Ok(())
        })?
        .on_leave(move |direction, line| {
            Ok(match direction {
                Direction::Up => Some(nose[line.unwrap_or(1).min(nose.len() - 1)]),
                _ => Some(ok),
            })
        })?;

        for (line, pane) in nose.iter().enumerate() {
            pane.link_into(Direction::Down, &list, ListTarget::line(line))?;
        }

        ok.link_into(
            Direction::Up,
            &list,
            ListTarget::line(nose.len() / 2).last(),
        )?;

        page.on_ready(move |seq| {
            report("ready", unsafe { mouths().ready(seq.raw().cast(), list) });
            Ok(())
        })?;

        page.on_update(|| {
            let mouths = unsafe { mouths() };

            if mouths.list.is_some() {
                report("tick", unsafe { mouths.tick() });
            }

            Ok(())
        })
    })
}

pub unsafe fn is_active(seq: *mut CharaMakeSeq) -> bool {
    unsafe { active(seq).is_some() }
}

pub unsafe fn set_enabled(seq: *mut CharaMakeSeq, enable: bool, mode: u32) -> acnh_ui_api::Result {
    let Some(mouths) = (unsafe { active(seq) }) else {
        return Ok(());
    };

    mouths.enabled = enable;
    mouths
        .list
        .map_or(Ok(()), |list| list.set_enabled(enable, mode))
}

pub unsafe fn fill(values: *mut ObjArray<u32>, game_fill: impl FnOnce()) -> NativeResult {
    unsafe {
        let mouths = mouths();
        let ours = !mouths.seq.is_null() && core::ptr::eq(&(*mouths.seq).mouth_values, values);

        if ours {
            mouths.values.release(&mut *values);
        }

        game_fill();

        if ours {
            mouths.values.own(&mut *values)?;
        }

        Ok(())
    }
}

pub unsafe fn refresh(seq: *mut CharaMakeSeq) -> acnh_ui_api::Result {
    let Some(mouths) = (unsafe { active(seq) }).filter(|m| m.values.count != 0) else {
        return Ok(());
    };

    let (mouth, player) = {
        let seq = unsafe { mouths.seq() };
        (seq.face_params[FACE_PARAM_MOUTH] as u32, seq.player)
    };
    let selected = mouths.values.item_of(mouth).unwrap_or(0);

    mouths.list.map_or(Ok(()), |list| {
        list.set_items(mouths.values.count, selected, player)
    })
}

// UpdateAnchor links OK to the old mouth buttons
pub unsafe fn after_request(seq: *mut CharaMakeSeq) -> acnh_ui_api::Result {
    match unsafe { active(seq) }.filter(|m| m.enabled) {
        Some(mouths) => mouths.list.map_or(Ok(()), |list| list.relink()),
        None => Ok(()),
    }
}

pub unsafe fn enter_idle(seq: *mut CharaMakeSeq) -> NativeResult {
    let Some(mouths) = (unsafe { active(seq) }) else {
        return Ok(());
    };

    mouths.enabled = false;

    for &icon in &mouths.icons[..mouths.slots] {
        if let Some(icon) = unsafe { icon.as_mut() } {
            unsafe { icon.release()? };
        }
    }

    unsafe { mouths.values.release(&mut (*seq).mouth_values) };

    Ok(())
}
