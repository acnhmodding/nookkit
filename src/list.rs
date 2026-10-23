use core::cell::UnsafeCell;
use core::ptr::{null, null_mut};

use crate::game::*;

const LIST_PATH: &[u8] = b"/L_BtnListMouth_00\0";
const MAX_SLOTS: usize = 32;
const MAX_MOUTHS: usize = 256;
const LEAVE_UP: i32 = 0;
const LEAVE_DOWN: i32 = 1;

struct MouthList {
    page: *mut u8,
    seq: *mut u8,
    list: *mut u8,
    group: ListGroup,
    updater: ItemUpdater,
    icons: [*mut u8; MAX_SLOTS],
    slots: usize,
    // Writable replacement storage for the seq's mouth value list (item -> mouth value).
    nodes: [u64; MAX_MOUTHS],
    node_ptrs: [*mut u64; MAX_MOUTHS],
    count: usize,
    owns_values: bool,
    saved_count: i32,
    saved_capacity: i32,
    saved_ptrs: *mut *mut u64,
}

struct State(UnsafeCell<MouthList>);

// The CharaMake UI runs on a single thread.
unsafe impl Sync for State {}

static STATE: State = State(UnsafeCell::new(MouthList {
    page: null_mut(),
    seq: null_mut(),
    list: null_mut(),
    group: ListGroup {
        vtable: null(),
        enabled: 0,
        pad0: [0; 3],
        keep: 0,
        locked: 0,
        toggle: 0,
        pad1: 0,
        selected: 0,
    },
    updater: ItemUpdater {
        count: 0,
        pad: 0,
        delegate: Delegate {
            vtable: null(),
            owner: null_mut(),
            func: 0,
            adj: 0,
        },
    },
    icons: [null_mut(); MAX_SLOTS],
    slots: 0,
    nodes: [0; MAX_MOUTHS],
    node_ptrs: [null_mut(); MAX_MOUTHS],
    count: 0,
    owns_values: false,
    saved_count: 0,
    saved_capacity: 0,
    saved_ptrs: null_mut(),
}));

#[allow(clippy::mut_from_ref)]
unsafe fn state() -> &'static mut MouthList {
    unsafe { &mut *STATE.0.get() }
}

unsafe fn active(seq: *mut u8) -> Option<&'static mut MouthList> {
    unsafe {
        let state = state();

        if seq.is_null() || state.seq != seq || state.list.is_null() {
            None
        } else {
            Some(state)
        }
    }
}

unsafe fn face_params(seq: *mut u8) -> [u8; FACE_PARAMS_SIZE] {
    unsafe { *at::<[u8; FACE_PARAMS_SIZE]>(seq, SEQ_FACE_PARAMS) }
}

unsafe fn post_request(seq: *mut u8, state: i32, index: i32) {
    unsafe {
        let request = at::<i32>(seq, SEQ_REQUEST);

        if *request == -1 {
            *request = state;
            *request.add(1) = index;
            *request.add(2) = CATEGORY_MOUTH;
        }
    }
}

impl MouthList {
    fn value(&self, item: usize) -> u32 {
        self.nodes[item] as u32
    }

    fn item_of(&self, value: u32) -> Option<usize> {
        (0..self.count).find(|&item| self.value(item) == value)
    }
}

// CharaMakePage::vf_0d0 runs while parts are built; look the list up the same way the game does
// for its six lists until it exists.
pub unsafe fn bind(page: *mut u8, parent: *mut u8, builder: *mut u8) {
    unsafe {
        let state = state();

        if state.page != page {
            state.page = page;
            state.seq = null_mut();
            state.list = null_mut();
        }

        if !state.list.is_null() || parent.is_null() {
            return;
        }

        let list = find_by_path(parent, LIST_PATH);

        if list.is_null() || !is_scroll_list(list) {
            return;
        }

        list_bind_buttons(list, builder);
        state.list = list;
    }
}

pub unsafe fn setup(seq: *mut u8) {
    unsafe {
        let state = state();
        let page = *at::<*mut u8>(seq, SEQ_PAGE);

        if state.page != page || state.list.is_null() {
            return;
        }

        let list = state.list;
        state.seq = seq;

        state.group = ListGroup {
            vtable: list_group_vtable(),
            enabled: 1,
            pad0: [0; 3],
            keep: 0,
            locked: 0,
            toggle: 1,
            pad1: 0,
            selected: 0,
        };

        if !delegate_is_bound(list.add(LIST_SCROLL_DELEGATE)) {
            *at::<Delegate>(list, LIST_SCROLL_DELEGATE) = Delegate {
                vtable: text().add(LIST_SCROLL_DELEGATE_VTABLE),
                owner: seq,
                func: on_scroll as *const () as usize,
                adj: 0,
            };
        }

        if !delegate_is_bound(list.add(LIST_LEAVE_DELEGATE)) {
            *at::<Delegate>(list, LIST_LEAVE_DELEGATE) = Delegate {
                vtable: text().add(LIST_LEAVE_DELEGATE_VTABLE),
                owner: seq,
                func: on_leave as *const () as usize,
                adj: 0,
            };
        }

        state.slots = list_visible_slots(list).min(MAX_SLOTS);
        state.icons = [null_mut(); MAX_SLOTS];

        for slot in 0..state.slots {
            let button = list_button(list, slot);

            if button.is_null() {
                continue;
            }

            set_button_mode(button);
            bind_button_to_group(button, &mut state.group);
            state.icons[slot] = create_player_icon(seq, button);
        }

        state.updater = ItemUpdater {
            count: state.slots as u32,
            pad: 0,
            delegate: Delegate {
                vtable: text().add(ITEM_DELEGATE_VTABLE),
                owner: seq,
                func: on_item as *const () as usize,
                adj: 0,
            },
        };

        own_values(seq, state);
    }
}

unsafe fn mouth_value_list(seq: *mut u8) -> *mut ValueList {
    unsafe { at::<ValueList>(seq, SEQ_MOUTH_LIST) }
}

// Swap the seq's mouth value list over to our storage: the game's own entries first (in its
// order), then every other CharaMakeMouthTypeParam row.
unsafe fn own_values(seq: *mut u8, state: &mut MouthList) {
    unsafe {
        release_values(seq, state);

        let list = &mut *mouth_value_list(seq);
        let rows = mouth_row_count().min(MAX_MOUTHS);
        let mut count = 0;

        if rows == 0 {
            state.count = list.count.max(0) as usize;

            for i in 0..state.count.min(MAX_MOUTHS) {
                state.nodes[i] = **list.ptrs.add(i);
            }

            return;
        }

        for i in 0..list.count.max(0) as usize {
            let value = **list.ptrs.add(i) as u32;

            if (value as usize) < rows && !state.nodes[..count].contains(&(value as u64)) {
                state.nodes[count] = value as u64;
                count += 1;
            }
        }

        for row in 0..rows {
            if !state.nodes[..count].contains(&(row as u64)) {
                state.nodes[count] = row as u64;
                count += 1;
            }
        }

        if count == 0 {
            return;
        }

        for (ptr, node) in state.node_ptrs.iter_mut().zip(state.nodes.iter_mut()) {
            *ptr = node;
        }

        state.saved_count = list.count;
        state.saved_capacity = list.capacity;
        state.saved_ptrs = list.ptrs;
        state.owns_values = true;
        state.count = count;

        list.count = count as i32;
        list.capacity = count as i32;
        list.ptrs = state.node_ptrs.as_mut_ptr();
    }
}

unsafe fn release_values(seq: *mut u8, state: &mut MouthList) {
    unsafe {
        if !state.owns_values {
            return;
        }

        let list = &mut *mouth_value_list(seq);

        list.count = state.saved_count;
        list.capacity = state.saved_capacity;
        list.ptrs = state.saved_ptrs;
        state.owns_values = false;
    }
}

// The game refills the mouth value list on enter; let it work on its own storage, then take over.
pub unsafe fn before_fill(value_list: *mut u8) {
    unsafe {
        let state = state();

        if !state.seq.is_null() && state.seq.add(SEQ_MOUTH_LIST) == value_list {
            release_values(state.seq, state);
        }
    }
}

pub unsafe fn after_fill(value_list: *mut u8) {
    unsafe {
        let state = state();

        if !state.seq.is_null() && state.seq.add(SEQ_MOUTH_LIST) == value_list {
            own_values(state.seq, state);
        }
    }
}

extern "C" fn on_scroll(seq: *mut u8, event: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        item_updater_update(&mut state.updater, &event);
        list_refresh_group(&mut state.group, state.list);
    }
}

// Event: {i32 op, i32 slot, i32 from_slot, i32, scroll_info* @0x10 (+0xc = first visible item)}.
extern "C" fn on_item(seq: *mut u8, event: *const i32) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let slot_of = |slot: i32| -> usize {
            if slot >= 0 && (slot as usize) < state.slots {
                slot as usize
            } else {
                0
            }
        };

        let op = *event;
        let slot = *event.add(1);
        let icon = state.icons[slot_of(slot)];

        if icon.is_null() {
            return;
        }

        if op == 2 {
            icon_release(icon);
            return;
        }

        let scroll_info = *(event.add(4) as *const *mut u8);
        let item = slot + *at::<i32>(scroll_info, 0xc);
        let in_range = item >= 0 && (item as usize) < state.count;
        let mut params = face_params(seq);

        params[SEQ_FACE_PARAM_MOUTH] = if in_range { state.value(item as usize) as u8 } else { 0 };

        if op == 1 {
            let source = state.icons[slot_of(*event.add(2))];

            icon_move_from(icon, source, &params);
        } else if op == 0 && in_range {
            icon_render(icon, &params);
        }
    }
}

// Leaving the list vertically: up goes to the nose row, down to the OK button.
extern "C" fn on_leave(seq: *mut u8, target: *mut *mut u8, event: *mut u8) {
    unsafe {
        if active(seq).is_none() {
            return;
        }

        let page = *at::<*mut u8>(seq, SEQ_PAGE);

        let control = match *at::<i32>(event, 0x10) {
            LEAVE_UP => *at::<*mut u8>(page, PAGE_BTN_NOSE + 8 * (NOSE_BUTTONS / 2)),
            LEAVE_DOWN => *at::<*mut u8>(page, PAGE_BTN_OK),
            _ => null_mut(),
        };

        if !control.is_null() {
            *target = control;
        }
    }
}

// Page refresh: select the current mouth and (re)fill the list, which renders the visible items.
pub unsafe fn refresh(seq: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let mouth = face_params(seq)[SEQ_FACE_PARAM_MOUTH] as u32;
        let selected = state.item_of(mouth).unwrap_or(0) as i32;

        state.group.selected = selected;
        list_set_content(state.list, state.count as i32, selected, *at::<u8>(seq, SEQ_PLAYER));
    }
}

// The game links the nose buttons down to the old mouth buttons; link them to the list instead.
pub unsafe fn link_navigation(seq: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let page = *at::<*mut u8>(seq, SEQ_PAGE);
        let columns = (*at::<i32>(state.list, LIST_COLUMNS)).max(0) as usize;

        for column in 0..NOSE_BUTTONS.min(columns) {
            let nose = *at::<*mut u8>(page, PAGE_BTN_NOSE + column * 8);
            let button = list_button(state.list, column);

            if nose.is_null() || button.is_null() {
                continue;
            }

            let nose_node = *at::<*mut u8>(nose, BTN_NAV_NODE);
            let button_node = *at::<*mut u8>(button, BTN_NAV_NODE);

            if !nose_node.is_null() && !button_node.is_null() {
                *at::<*mut u8>(nose_node, NODE_NAV_LINKS + NAV_LINK_DOWN * 8) = button_node;
            }
        }
    }
}

pub unsafe fn on_focus(seq: *mut u8, control: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let index = list_item_index_of(state.list, control);

        if index >= 0 {
            post_request(seq, REQUEST_FOCUS, index);
        }
    }
}

// Mirrors the game's eye list branch of onControlDecide.
pub unsafe fn on_decide(seq: *mut u8, control: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        let index = list_item_index_of(state.list, control);

        if index < 0 || state.group.locked != 0 {
            return;
        }

        let group = &mut state.group as *mut ListGroup as *mut u8;
        let get_selected: extern "C" fn(*mut u8) -> i32 = vcall(group, 0x10);
        let set_selected: extern "C" fn(*mut u8, i32) = vcall(group, 0x18);
        let is_changed: extern "C" fn(*mut u8, i32) -> bool = vcall(group, 0x20);
        let clear_visual: extern "C" fn(*mut u8, *mut u8) = vcall(group, 0x28);

        if is_changed(group, index) {
            clear_visual(group, state.list);
            set_selected(group, index);
        } else if state.group.toggle == 0 {
            clear_visual(group, state.list);
            set_selected(group, -1);
            return;
        } else if state.group.keep == 0 {
            return;
        }

        post_request(seq, REQUEST_DECIDE, get_selected(group).max(0));
    }
}

pub unsafe fn tick(seq: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        for &icon in &state.icons[..state.slots] {
            if !icon.is_null() {
                icon_tick(icon);
            }
        }
    }
}

pub unsafe fn exit(seq: *mut u8) {
    unsafe {
        let Some(state) = active(seq) else {
            return;
        };

        for &icon in &state.icons[..state.slots] {
            if !icon.is_null() {
                icon_release(icon);
            }
        }

        release_values(seq, state);
        state.icons = [null_mut(); MAX_SLOTS];
        state.slots = 0;
        state.seq = null_mut();
    }
}
