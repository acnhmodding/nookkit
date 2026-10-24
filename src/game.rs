use skyline::hooks::{Region, getRegionAddress};

// Functions (offsets from .text)
pub const PAGE_BIND_LISTS: usize = 0x2b9656c;
pub const SEQ_SETUP_CONTROLS: usize = 0x2b9d7d0;
pub const SEQ_ON_CONTROL_FOCUS: usize = 0x2b99ff0;
pub const SEQ_ON_CONTROL_DECIDE: usize = 0x2b9a8d0;
pub const SEQ_TICK_ICONS: usize = 0x2ba79b0;
pub const SEQ_ON_EXIT: usize = 0x2ba10e0;
pub const SEQ_MOUTH_HIGHLIGHT_SELECT: usize = 0x2ba3978;
pub const SEQ_MOUTH_NAV_GRID_DONE: usize = 0x2ba4034;
pub const SEQ_FILL_MOUTH_LIST: usize = 0x2b8fe60;
pub const SEQ_PROCESS_REQUEST: usize = 0x2ba4a00;
// (seq, group, enable, mode): enables a tab's control group; group 6 = the four old mouth buttons.
pub const SEQ_SET_GROUP_ENABLED: usize = 0x2ba7fe0;
pub const GROUP_MOUTH: u32 = 6;
// ScrollBtnListCtrl State::cPlay exec (list): applies the pending D-pad move, focused or not.
pub const LIST_PLAY_EXEC: usize = 0x375fd8c;
pub const REQUEST_UPDATE_ANCHOR: i32 = 5;

const FIND_BY_PATH: usize = 0x376e6b0;
const LIST_BIND_BUTTONS: usize = 0x2ab1110;
const LIST_SET_CONTENT: usize = 0x3764290;
const LIST_ITEM_INDEX_OF: usize = 0x3765200;
const LIST_REFRESH_GROUP: usize = 0x2ac30b0;
const ITEM_UPDATER_UPDATE: usize = 0x335c380;
const FIND_PLAYER_ICON: usize = 0x2baaf20;
const ICON_RESET_PART: usize = 0x2fc3694;
const ICON_RENDER: usize = 0x2fc37ac;
const ICON_MOVE_FROM: usize = 0x2fc3ff0;
const ICON_RELEASE: usize = 0x2fc4398;
const ICON_TICK: usize = 0x2fc47d8;
const LIST_ENABLE: usize = 0x3764fe0;
const LIST_ENABLE_FOCUS: usize = 0x3dbb160;
const LIST_DISABLE: usize = 0x335b550;
// ScrollBtnListCtrl attach: player, the page's focus manager, nav nodes, arrow buttons.
const LIST_ATTACH: usize = 0x3763fe0;

// Data (offsets from .text)
pub const LIST_SCROLL_DELEGATE_VTABLE: usize = 0x49b5088;
pub const LIST_LEAVE_DELEGATE_VTABLE: usize = 0x49b50d8;
pub const ITEM_DELEGATE_VTABLE: usize = 0x49b5268;
const SET_CONTENT_PARAMS_VTABLE: usize = 0x4a2b020;
const GOT_SAFE_STRING_VTABLE: usize = 0x4ab2020;
const GOT_CHARAMAKE_PARAM_MGR: usize = 0x4ab4868;
const GOT_SCROLL_LIST_TYPE_INFO: usize = 0x4ac4d68;
const GOT_SCROLL_LIST_TYPE_INFO_GUARD: usize = 0x4ac4d70;
const GOT_LIST_GROUP_VTABLE: usize = 0x4ac5170;
const GOT_BTN_GROUP_DELEGATE_VTABLE: usize = 0x4ac9b80;
const GOT_BTN_GROUP_DELEGATE_FN: usize = 0x4ac9b78;
const GOT_LIST_ENABLE_PARAMS_VTABLE: usize = 0x4ac9c80;
const GOT_PAGE_ROOT_TYPE_INFO: usize = 0x4ac4e58;
const GOT_PAGE_ROOT_TYPE_INFO_GUARD: usize = 0x4ac4e60;

// game::UI::CharaMakePage
const PAGE_ROOT: usize = 0x3f8;
pub const PAGE_BTN_NOSE: usize = 0x598;
pub const PAGE_BTN_MOUTH: usize = 0x5b0;
pub const PAGE_BTN_OK: usize = 0x608;
pub const NOSE_BUTTONS: usize = 3;
pub const VANILLA_MOUTH_BUTTONS: usize = 4;

// ScrollBtnListCtrl
pub const LIST_SCROLL_DELEGATE: usize = 0x330;
pub const LIST_LEAVE_DELEGATE: usize = 0x350;
const LIST_BUTTON_COUNT: usize = 0x208;
const LIST_BUTTONS: usize = 0x210;
// Items per line; CharaMake lists scroll horizontally, so a line is one column.
pub const LIST_LINE_SIZE: usize = 0x2e8;
const LIST_VISIBLE_SLOTS: usize = 0x2f8;
// Slot the list believes is focused.
const LIST_FOCUSED_SLOT: usize = 0x43c;
// Pending move kind (0 none, 1 within a line, 2 across lines, 3 scroll) for State::cPlay.
const LIST_PENDING_MOVE: usize = 0x3ec;

// game::UI::AppUniteBtnCtrl
pub const BTN_NAV_NODE: usize = 0x218;
const BTN_MODE_FLAGS: usize = 0x2b0;
const BTN_MODE_BITS: usize = 0x2c0;
const BTN_GROUP_DELEGATE: usize = 0x3a0;
const BTN_CHILDREN: usize = 0x38;

const NODE_NAV_LINKS: usize = 0x1460;
pub const NAV_LINK_UP: usize = 0;
pub const NAV_LINK_DOWN: usize = 1;

// PlayerIcon
const ICON_TEXTURE_POOL: usize = 0x208;
const ICON_PARTS: usize = 0x228;
const ICON_PART_STRIDE: usize = 0x50;
const ICON_PART_COUNT: usize = 7;

// game::UI::CharaMakeSeq
pub const SEQ_PAGE: usize = 0x210;
pub const SEQ_PLAYER: usize = 0x1f8;
pub const SEQ_FACE_PARAMS: usize = 0x314;
pub const SEQ_FACE_PARAM_MOUTH: usize = 5;
pub const SEQ_MOUTH_LIST: usize = 0xbc0;
const SEQ_TEXTURE_POOL: usize = 0xff0;
pub const SEQ_REQUEST: usize = 0x1cfcc;
pub const REQUEST_FOCUS: i32 = 2;
pub const REQUEST_DECIDE: i32 = 3;
pub const CATEGORY_MOUTH: i32 = 6;

// ACNH_Singleton_7104BF00A0 -> CharaMakeMouthTypeParam (KeyedTable)
const PARAM_MGR_MOUTH_TABLE: usize = 0x38;
const TABLE_ORDERED_COUNT: usize = 0x1c0;
const TABLE_RESOURCE: usize = 0xb0;

pub const FACE_PARAMS_SIZE: usize = 12;

#[repr(C)]
struct SafeString {
    vtable: *const u8,
    string: *const u8,
}

// sead-style member delegate: vtable, owner, member function pointer {ptr, adj}.
#[repr(C)]
pub struct Delegate {
    pub vtable: *const u8,
    pub owner: *mut u8,
    pub func: usize,
    pub adj: usize,
}

// Visible-slot event source for a scroll list (layout of seq+0x1cf70).
#[repr(C)]
pub struct ItemUpdater {
    pub count: u32,
    pub pad: u32,
    pub delegate: Delegate,
}

// sead-style fixed object array of u32 values (seq+0xBC0 for mouths). Nodes are 8 bytes: the
// value, or the next free node while on the free list.
#[repr(C)]
pub struct ValueList {
    pub count: i32,
    pub capacity: i32,
    pub ptrs: *mut *mut u64,
    pub free: *mut u64,
}

// Selection group for a scroll list (vtable 0x71049aaf10).
#[repr(C)]
pub struct ListGroup {
    pub vtable: *const u8,
    pub enabled: u8,
    pub pad0: [u8; 3],
    pub keep: i32,
    pub locked: u8,
    pub toggle: u8,
    pub pad1: u16,
    pub selected: i32,
}

pub unsafe fn text() -> *mut u8 {
    unsafe { getRegionAddress(Region::Text) as *mut u8 }
}

pub unsafe fn at<T>(base: *mut u8, offset: usize) -> *mut T {
    unsafe { base.add(offset) as *mut T }
}

pub unsafe fn got(offset: usize) -> usize {
    unsafe { *at::<usize>(text(), offset) }
}

unsafe fn func<F: Copy>(offset: usize) -> F {
    unsafe { core::mem::transmute_copy(&(text().add(offset) as usize)) }
}

pub unsafe fn vcall<F: Copy>(obj: *mut u8, slot: usize) -> F {
    unsafe {
        let vtable = *(obj as *const *const u8);

        core::mem::transmute_copy(&*(vtable.add(slot) as *const usize))
    }
}

pub unsafe fn delegate_is_bound(delegate: *mut u8) -> bool {
    unsafe {
        let is_bound: extern "C" fn(*mut u8) -> bool = vcall(delegate, 0x10);

        is_bound(delegate)
    }
}

pub unsafe fn find_by_path(parent: *mut u8, path: &[u8]) -> *mut u8 {
    unsafe {
        let find: extern "C" fn(*mut u8, *const SafeString) -> *mut u8 = func(FIND_BY_PATH);
        let name = SafeString {
            vtable: (got(GOT_SAFE_STRING_VTABLE) + 0x10) as *const u8,
            string: path.as_ptr(),
        };

        find(parent, &name)
    }
}

pub unsafe fn is_scroll_list(control: *mut u8) -> bool {
    unsafe {
        // The type info is a guarded local static; the game's own list binding initializes it.
        if *(got(GOT_SCROLL_LIST_TYPE_INFO_GUARD) as *const u8) & 1 == 0 {
            return false;
        }

        let is_derived: extern "C" fn(*mut u8, usize) -> bool = vcall(control, 0);

        is_derived(control, got(GOT_SCROLL_LIST_TYPE_INFO))
    }
}

pub unsafe fn list_bind_buttons(list: *mut u8, builder: *mut u8) {
    unsafe {
        let bind: extern "C" fn(*mut u8, *mut u8) = func(LIST_BIND_BUTTONS);

        bind(list, builder)
    }
}

pub unsafe fn list_visible_slots(list: *mut u8) -> usize {
    unsafe { (*at::<i32>(list, LIST_VISIBLE_SLOTS)).max(0) as usize }
}

pub unsafe fn list_button(list: *mut u8, slot: usize) -> *mut u8 {
    unsafe {
        if slot >= list_button_count(list) {
            return core::ptr::null_mut();
        }

        *(*at::<*mut *mut u8>(list, LIST_BUTTONS)).add(slot)
    }
}

pub unsafe fn list_item_index_of(list: *mut u8, button: *mut u8) -> i32 {
    unsafe {
        let index_of: extern "C" fn(*mut u8, *mut u8) -> i32 = func(LIST_ITEM_INDEX_OF);

        index_of(list, button)
    }
}

pub unsafe fn list_set_content(list: *mut u8, count: i32, selected: i32, player: u8) {
    unsafe {
        // Same values the game passes for the eye list on page refresh.
        let mut params = [0u8; 0x30];

        *(params.as_mut_ptr() as *mut usize) = text().add(SET_CONTENT_PARAMS_VTABLE) as usize;
        *(params.as_mut_ptr().add(0x8) as *mut i32) = count;
        *(params.as_mut_ptr().add(0xc) as *mut i32) = selected;
        *(params.as_mut_ptr().add(0x10) as *mut i32) = selected;
        params[0x16] = 1;
        params[0x19] = 1;
        *(params.as_mut_ptr().add(0x24) as *mut i32) = 0xc;
        *(params.as_mut_ptr().add(0x28) as *mut i32) = 6;
        params[0x2c] = if player < 8 { player } else { 0xff };

        let set_content: extern "C" fn(*mut u8, *const u8) = func(LIST_SET_CONTENT);

        set_content(list, params.as_ptr())
    }
}

pub unsafe fn list_refresh_group(group: *mut ListGroup, list: *mut u8) {
    unsafe {
        let refresh: extern "C" fn(*mut ListGroup, *mut u8) = func(LIST_REFRESH_GROUP);

        refresh(group, list)
    }
}

pub unsafe fn item_updater_update(updater: *mut ItemUpdater, event: *const *mut u8) {
    unsafe {
        let update: extern "C" fn(*mut ItemUpdater, *const *mut u8) = func(ITEM_UPDATER_UPDATE);

        update(updater, event)
    }
}

pub unsafe fn list_group_vtable() -> *const u8 {
    unsafe { (got(GOT_LIST_GROUP_VTABLE) + 0x10) as *const u8 }
}

// The per-button "is selectable" delegate the game gives every list button (owner = the group).
pub unsafe fn bind_button_to_group(button: *mut u8, group: *mut ListGroup) {
    unsafe {
        let delegate = at::<u8>(button, BTN_GROUP_DELEGATE);

        if delegate_is_bound(delegate) {
            return;
        }

        *at::<Delegate>(button, BTN_GROUP_DELEGATE) = Delegate {
            vtable: (got(GOT_BTN_GROUP_DELEGATE_VTABLE) + 0x10) as *const u8,
            owner: group as *mut u8,
            func: got(GOT_BTN_GROUP_DELEGATE_FN),
            adj: 0,
        };
    }
}

pub unsafe fn set_button_mode(button: *mut u8) {
    unsafe {
        let bits = at::<u8>(button, BTN_MODE_BITS);

        *bits = (*bits & 0xf8) | 1;
        *at::<i32>(button, BTN_MODE_FLAGS) = 2;
    }
}

// Attaches a list to the page the way the page's list attach routine (0x7102b99900) does for
// its own six lists on every CharaMake entry.
pub unsafe fn list_attach(list: *mut u8, page: *mut u8) {
    unsafe {
        let mut params = [0u64; 4];
        let root = **at::<*mut *mut u8>(page, PAGE_ROOT);

        params[0] = page as u64;
        params[3] = u64::MAX;

        // The type info is a guarded local static; the game's own attach initializes it.
        if !root.is_null() && *(got(GOT_PAGE_ROOT_TYPE_INFO_GUARD) as *const u8) & 1 != 0 {
            let is_derived: extern "C" fn(*mut u8, usize) -> bool = vcall(root, 0);

            if is_derived(root, got(GOT_PAGE_ROOT_TYPE_INFO)) {
                params[2] = root as u64;
            }
        }

        let attach: extern "C" fn(*mut u8, *const u64) = func(LIST_ATTACH);

        attach(list, params.as_ptr());
    }
}

// Same as the game's group switch does for the eye list (group 3).
pub unsafe fn list_set_enabled(list: *mut u8, enable: bool, mode: u32) {
    unsafe {
        if !enable {
            let disable: extern "C" fn(*mut u8, u32) = func(LIST_DISABLE);

            disable(list, mode);
            return;
        }

        // Both paths enter State::cPlay, whose enter focuses the list's cursor button unless a slot is
        // already marked focused. The mouth tab focuses the nose row right after, which the list never
        // sees: it would keep the D-pad. Mark a slot for the call so the list comes up unfocused.
        *at::<i32>(list, LIST_FOCUSED_SLOT) = 0;

        if *at::<u32>(list, 0x298) > 1 {
            let mut params = [0u64; 2];

            params[0] = (got(GOT_LIST_ENABLE_PARAMS_VTABLE) + 0x10) as u64;
            params[1] = 0xffffffff01000100;

            let enable: extern "C" fn(*mut u8, *const u64) = func(LIST_ENABLE);

            enable(list, params.as_ptr());
            list_clear_focus(list);
            return;
        }

        let slots = *at::<*mut u8>(list, 0x270);
        let scroll = *at::<*mut u8>(list, 0x480);
        let ready = *at::<i32>(list, 0x2f8) != 0
            && *at::<i32>(list, LIST_BUTTON_COUNT) != 0
            && !slots.is_null()
            && !(*at::<*mut u8>(slots, 8)).is_null()
            && (scroll.is_null() || *at::<i32>(scroll, 0x14) != -1)
            && !(*at::<*mut u8>(list, 0x2d8)).is_null()
            && *at::<i32>(list, 0x3d8) > 0
            && *at::<i32>(list, 0x3d4) != 0;

        if ready {
            let enable: extern "C" fn(*mut u8, u32) = func(LIST_ENABLE_FOCUS);

            enable(list.add(0x290), 2);
        }

        list_clear_focus(list);
    }
}

// What the list does itself when focus leaves it (State_710375F2AC::cPlay_Exec).
pub unsafe fn list_clear_focus(list: *mut u8) {
    unsafe { *at::<i32>(list, LIST_FOCUSED_SLOT) = -1 };
}

pub unsafe fn list_drop_pending_move(list: *mut u8) {
    unsafe { *at::<i32>(list, LIST_PENDING_MOVE) = 0 };
}

pub unsafe fn list_button_count(list: *mut u8) -> usize {
    unsafe { (*at::<i32>(list, LIST_BUTTON_COUNT)).max(0) as usize }
}

pub unsafe fn set_nav_link(button: *mut u8, link: usize, target: *mut u8) {
    unsafe {
        if button.is_null() || target.is_null() {
            return;
        }

        let node = *at::<*mut u8>(button, BTN_NAV_NODE);
        let target_node = *at::<*mut u8>(target, BTN_NAV_NODE);

        if !node.is_null() && !target_node.is_null() {
            *at::<*mut u8>(node, NODE_NAV_LINKS + link * 8) = target_node;
        }
    }
}

pub unsafe fn nav_link(button: *mut u8, link: usize) -> *mut u8 {
    unsafe {
        let node = *at::<*mut u8>(button, BTN_NAV_NODE);

        if node.is_null() {
            node
        } else {
            *at::<*mut u8>(node, NODE_NAV_LINKS + link * 8)
        }
    }
}

pub unsafe fn create_player_icon(seq: *mut u8, button: *mut u8) -> *mut u8 {
    unsafe {
        let find: extern "C" fn(*mut u8) -> *mut u8 = func(FIND_PLAYER_ICON);
        let reset: extern "C" fn(*mut u8) = func(ICON_RESET_PART);
        let icon = find(button.add(BTN_CHILDREN));

        if icon.is_null() {
            return icon;
        }

        *at::<*mut u8>(icon, ICON_TEXTURE_POOL) = seq.add(SEQ_TEXTURE_POOL);

        for part in 0..ICON_PART_COUNT {
            reset(icon.add(ICON_PARTS + part * ICON_PART_STRIDE));
        }

        icon
    }
}

pub unsafe fn icon_render(icon: *mut u8, face_params: &[u8; FACE_PARAMS_SIZE]) {
    unsafe {
        let render: extern "C" fn(*mut u8, *const u8) = func(ICON_RENDER);

        render(icon, face_params.as_ptr())
    }
}

pub unsafe fn icon_move_from(icon: *mut u8, source: *mut u8, face_params: &[u8; FACE_PARAMS_SIZE]) {
    unsafe {
        let move_from: extern "C" fn(*mut u8, *mut u8, *const u8) = func(ICON_MOVE_FROM);

        move_from(icon, source, face_params.as_ptr())
    }
}

pub unsafe fn icon_release(icon: *mut u8) {
    unsafe {
        let release: extern "C" fn(*mut u8) = func(ICON_RELEASE);

        release(icon)
    }
}

pub unsafe fn icon_tick(icon: *mut u8) {
    unsafe {
        let tick: extern "C" fn(*mut u8) = func(ICON_TICK);

        tick(icon)
    }
}

// Row count of CharaMakeMouthTypeParam.bcsv, read the same way the game's mouth accessor does.
pub unsafe fn mouth_row_count() -> usize {
    unsafe {
        let mgr = *(got(GOT_CHARAMAKE_PARAM_MGR) as *const *mut u8);

        if mgr.is_null() {
            return 0;
        }

        let table = *at::<*mut u8>(mgr, PARAM_MGR_MOUTH_TABLE);

        if table.is_null() {
            return 0;
        }

        let ordered = *at::<i32>(table, TABLE_ORDERED_COUNT);

        if ordered > 0 {
            return ordered as usize;
        }

        let resource = *at::<*mut u8>(table, TABLE_RESOURCE);

        if resource.is_null() {
            return 0;
        }

        let header = *at::<*const u32>(resource, 8);

        if header.is_null() { 0 } else { *header as usize }
    }
}
