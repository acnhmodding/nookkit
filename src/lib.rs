use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use skyline::hooks::{getRegionAddress, Region};

const VANILLA_MOUTHS: usize = 4;
const MAX_EXTRA_MOUTHS: usize = SEQ_MOUTH_LIST_CAPACITY - VANILLA_MOUTHS;

const PAGE_LOOKUP_BUTTONS: usize = 0x2b95580;
const SEARCH_AS_APP_UNITE_BTN: usize = 0x2ab2688;
const SEQ_SETUP_CONTROLS: usize = 0x2b9d7d0;
const SEQ_ON_CONTROL_FOCUS: usize = 0x2b99ff0;
const SEQ_ON_CONTROL_DECIDE: usize = 0x2b9a8d0;
const SEQ_UPDATE_FOCUS_REQUEST: usize = 0x2ba4630;
const SEQ_PROCESS_REQUEST: usize = 0x2ba4a00;
const SEQ_MOUTH_HIGHLIGHT_SELECT: usize = 0x2ba3978;
const SEQ_MOUTH_NAV_GRID_DONE: usize = 0x2ba4034;
const NAV_GRID_BUILD: usize = 0x376f420;
const MOUTH_ACCESSOR_VTABLE: usize = 0x49b4f28;
const GOT_SAFE_STRING_VTABLE: usize = 0x4ab2020;

const PAGE_HELPER: usize = 0x210;
const PAGE_ROOT_CONTROL: usize = 0x3f8;
const PAGE_BTN_NOSE: usize = 0x598;
const PAGE_BTN_MOUTH: usize = 0x5b0;
const NOSE_BUTTONS: usize = 3;

const BTN_ICON_NODE: usize = 0x218;
const NODE_NAV_LINKS: usize = 0x1460;
const NAV_LINK_DOWN: usize = 1;
const NAV_GRID_MAX_CELLS: usize = 0x80;
const BTN_MODE_FLAGS: usize = 0x2b0;
const BTN_MODE_BITS: usize = 0x2c0;
const BTN_GROUP_DELEGATE: usize = 0x3a0;

const SEQ_PAGE: usize = 0x210;
const SEQ_FACE_PARAM_MOUTH: usize = 0x319;
const SEQ_MOUTH_LIST_COUNT: usize = 0xbc0;
const SEQ_MOUTH_LIST_PTRS: usize = 0xbc8;
const SEQ_MOUTH_LIST_CAPACITY: usize = 12;
const SEQ_MOUTH_GROUP: usize = 0xf68;
const SEQ_MOUTH_GROUP_LOCKED: usize = 0xf78;
const SEQ_MOUTH_GROUP_TOGGLE: usize = 0xf79;
const SEQ_MOUTH_GROUP_KEEP: usize = 0xf74;
const SEQ_REQUEST: usize = 0x1cfcc;
const REQUEST_FOCUS: i32 = 2;
const REQUEST_DECIDE: i32 = 3;
const CATEGORY_MOUTH: i32 = 6;

#[repr(C)]
struct SafeString {
    vtable: *const u8,
    string: *const u8,
}

#[repr(C)]
struct ButtonAccessor {
    vtable: *const u8,
    data: *mut *mut u8,
}

const NO_BUTTON: AtomicPtr<u8> = AtomicPtr::new(null_mut());

static MOUTH_PAGE: AtomicPtr<u8> = AtomicPtr::new(null_mut());
static EXTRA_BUTTONS: [AtomicPtr<u8>; MAX_EXTRA_MOUTHS] = [NO_BUTTON; MAX_EXTRA_MOUTHS];
static EXTRA_COUNT: AtomicUsize = AtomicUsize::new(0);

// Mouth value of button index i is i; the game's mouth list stores pointers to these.
static MOUTH_VALUES: [u32; SEQ_MOUTH_LIST_CAPACITY] = {
    let mut values = [0; SEQ_MOUTH_LIST_CAPACITY];
    let mut i = 0;

    while i < SEQ_MOUTH_LIST_CAPACITY {
        values[i] = i as u32;
        i += 1;
    }

    values
};

unsafe fn text() -> *mut u8 {
    getRegionAddress(Region::Text) as *mut u8
}

unsafe fn at<T>(base: *mut u8, offset: usize) -> *mut T {
    base.add(offset) as *mut T
}

unsafe fn vcall<F: Copy>(obj: *mut u8, slot: usize) -> F {
    let vtable = *(obj as *const *const u8);

    core::mem::transmute_copy(&*(vtable.add(slot) as *const usize))
}

fn extra_count(page: *mut u8) -> usize {
    if !page.is_null() && MOUTH_PAGE.load(Ordering::Relaxed) == page {
        EXTRA_COUNT.load(Ordering::Relaxed)
    } else {
        0
    }
}

fn extra_button(page: *mut u8, extra: usize) -> *mut u8 {
    if extra < extra_count(page) {
        EXTRA_BUTTONS[extra].load(Ordering::Relaxed)
    } else {
        null_mut()
    }
}

fn extra_index_of(page: *mut u8, button: *mut u8) -> Option<usize> {
    if button.is_null() {
        return None;
    }

    (0..extra_count(page)).find(|&extra| EXTRA_BUTTONS[extra].load(Ordering::Relaxed) == button)
}

unsafe fn find_mouth_button(root: *mut u8, index: usize) -> *mut u8 {
    let search: extern "C" fn(*mut u8, *const SafeString) -> *mut u8 =
        core::mem::transmute(text().add(SEARCH_AS_APP_UNITE_BTN));

    let mut pane = *b"/L_BtnMouth_00\0";
    pane[12] = b'0' + (index / 10) as u8;
    pane[13] = b'0' + (index % 10) as u8;

    let got = *at::<*const u8>(text(), GOT_SAFE_STRING_VTABLE);
    let name = SafeString {
        vtable: got.add(0x10),
        string: pane.as_ptr(),
    };

    search(root.add(0x38), &name)
}

unsafe fn link_icon_node(page: *mut u8, button: *mut u8) {
    let list = *at::<*mut u8>(*at::<*mut u8>(page, PAGE_HELPER), 0x48);
    let icon = *at::<*mut u8>(button, BTN_ICON_NODE);
    *at::<*mut u8>(icon, 0x18) = list;

    let node = icon.offset(*at::<i32>(list, 0x14) as isize);
    let old_head = *at::<*mut u8>(list, 0);

    *at::<*mut u8>(list, 0) = node;
    *at::<*mut u8>(node, 0) = old_head;
    *at::<*mut u8>(node, 8) = list;

    if !old_head.is_null() {
        *at::<*mut u8>(old_head, 8) = node;
    }

    *at::<i32>(list, 0x10) += 1;
}

#[skyline::hook(offset = PAGE_LOOKUP_BUTTONS)]
unsafe fn page_lookup_buttons(page: *mut u8) {
    call_original!(page);

    EXTRA_COUNT.store(0, Ordering::Relaxed);
    MOUTH_PAGE.store(page, Ordering::Relaxed);

    let root = **at::<*mut *mut u8>(page, PAGE_ROOT_CONTROL);

    if root.is_null() {
        return;
    }

    let mut count = 0;

    while count < MAX_EXTRA_MOUTHS {
        let button = find_mouth_button(root, VANILLA_MOUTHS + count);

        if button.is_null() {
            break;
        }

        link_icon_node(page, button);
        EXTRA_BUTTONS[count].store(button, Ordering::Relaxed);
        count += 1;
    }

    EXTRA_COUNT.store(count, Ordering::Relaxed);
}

#[skyline::hook(offset = SEQ_SETUP_CONTROLS)]
unsafe fn seq_setup_controls(seq: *mut u8) {
    call_original!(seq);

    let page = *at::<*mut u8>(seq, SEQ_PAGE);
    let first = *at::<*mut u8>(page, PAGE_BTN_MOUTH);

    if first.is_null() {
        return;
    }

    for extra in 0..extra_count(page) {
        let button = extra_button(page, extra);

        let bits = at::<u8>(button, BTN_MODE_BITS);
        *bits = (*bits & 0xf8) | 1;
        *at::<i32>(button, BTN_MODE_FLAGS) = 2;

        core::ptr::copy_nonoverlapping(
            at::<u64>(first, BTN_GROUP_DELEGATE),
            at::<u64>(button, BTN_GROUP_DELEGATE),
            4,
        );
    }
}

unsafe fn accessor_page(acc: *const ButtonAccessor) -> *mut u8 {
    ((*acc).data as *mut u8).sub(PAGE_BTN_MOUTH)
}

extern "C" fn accessor_index_of(acc: *const ButtonAccessor, button: *mut u8) -> i32 {
    unsafe {
        let data = (*acc).data;

        if data.is_null() {
            return -1;
        }

        for i in 0..VANILLA_MOUTHS {
            if *data.add(i) == button {
                return i as i32;
            }
        }

        match extra_index_of(accessor_page(acc), button) {
            Some(extra) => (VANILLA_MOUTHS + extra) as i32,
            None => -1,
        }
    }
}

extern "C" fn accessor_at(acc: *const ButtonAccessor, index: u32) -> *mut u8 {
    unsafe {
        let data = (*acc).data;
        let index = index as usize;

        if data.is_null() {
            null_mut()
        } else if index < VANILLA_MOUTHS {
            *data.add(index)
        } else {
            extra_button(accessor_page(acc), index - VANILLA_MOUTHS)
        }
    }
}

extern "C" fn accessor_size(acc: *const ButtonAccessor) -> i32 {
    unsafe {
        if (*acc).data.is_null() {
            0
        } else {
            (VANILLA_MOUTHS + extra_count(accessor_page(acc))) as i32
        }
    }
}

unsafe fn patch_accessor_vtable() {
    let slots: [(usize, usize); 3] = [
        (0x08, accessor_index_of as *const () as usize),
        (0x18, accessor_at as *const () as usize),
        (0x20, accessor_size as *const () as usize),
    ];

    for (slot, func) in slots {
        skyline::patching::Patch::in_text(MOUTH_ACCESSOR_VTABLE + slot)
            .data(func)
            .expect("patch mouth accessor vtable");
    }
}

unsafe fn post_request(seq: *mut u8, state: i32, index: usize) {
    let request = at::<i32>(seq, SEQ_REQUEST);

    if *request == -1 {
        *request = state;
        *request.add(1) = index as i32;
        *request.add(2) = CATEGORY_MOUTH;
    }
}

unsafe fn event_button_index(seq: *mut u8, event: *mut u8) -> Option<usize> {
    let page = *at::<*mut u8>(seq, SEQ_PAGE);

    extra_index_of(page, *at::<*mut u8>(event, 8)).map(|extra| VANILLA_MOUTHS + extra)
}

#[skyline::hook(offset = SEQ_ON_CONTROL_FOCUS)]
unsafe fn seq_on_control_focus(seq: *mut u8, event: *mut u8) {
    call_original!(seq, event);

    if let Some(index) = event_button_index(seq, event) {
        post_request(seq, REQUEST_FOCUS, index);
    }
}

#[skyline::hook(offset = SEQ_ON_CONTROL_DECIDE)]
unsafe fn seq_on_control_decide(seq: *mut u8, event: *mut u8) {
    call_original!(seq, event);

    let Some(index) = event_button_index(seq, event) else {
        return;
    };

    if *at::<u8>(seq, SEQ_MOUTH_GROUP_LOCKED) != 0 {
        return;
    }

    let page = *at::<*mut u8>(seq, SEQ_PAGE);

    let accessor = ButtonAccessor {
        vtable: text().add(MOUTH_ACCESSOR_VTABLE),
        data: at::<*mut u8>(page, PAGE_BTN_MOUTH),
    };

    let group = at::<u8>(seq, SEQ_MOUTH_GROUP);
    let is_new_selection: extern "C" fn(*mut u8, u32) -> bool = vcall(group, 0x20);
    let select: extern "C" fn(*mut u8, i32) = vcall(group, 0x18);
    let refresh: extern "C" fn(*mut u8, *const ButtonAccessor) = vcall(group, 0x28);

    if is_new_selection(group, index as u32) {
        refresh(group, &accessor);
        select(group, index as i32);
    } else if *at::<u8>(seq, SEQ_MOUTH_GROUP_TOGGLE) == 0 {
        refresh(group, &accessor);
        select(group, -1);

        return;
    } else if *at::<i32>(seq, SEQ_MOUTH_GROUP_KEEP) == 0 {
        return;
    }

    post_request(seq, REQUEST_DECIDE, index);
}

unsafe fn with_extended_mouth_list(seq: *mut u8, f: impl FnOnce()) {
    let request = at::<i32>(seq, SEQ_REQUEST);
    let count = at::<u32>(seq, SEQ_MOUTH_LIST_COUNT);
    let ptrs = *at::<*mut *const u32>(seq, SEQ_MOUTH_LIST_PTRS);
    let index = *request.add(1);

    let pending = (*request == REQUEST_FOCUS || *request == REQUEST_DECIDE)
        && *request.add(2) == CATEGORY_MOUTH;

    let original_count = *count as usize;

    if !pending
        || ptrs.is_null()
        || index < original_count as i32
        || index as usize >= SEQ_MOUTH_LIST_CAPACITY
    {
        f();
        return;
    }

    let index = index as usize;
    let mut saved = [core::ptr::null::<u32>(); SEQ_MOUTH_LIST_CAPACITY];

    for i in original_count..=index {
        saved[i] = *ptrs.add(i);
        *ptrs.add(i) = &MOUTH_VALUES[i];
    }

    *count = (index + 1) as u32;
    f();
    *count = original_count as u32;

    for i in original_count..=index {
        *ptrs.add(i) = saved[i];
    }
}

#[skyline::hook(offset = SEQ_UPDATE_FOCUS_REQUEST)]
unsafe fn seq_update_focus_request(seq: *mut u8) {
    with_extended_mouth_list(seq, || call_original!(seq));
}

#[skyline::hook(offset = SEQ_PROCESS_REQUEST)]
unsafe fn seq_process_request(seq: *mut u8) {
    with_extended_mouth_list(seq, || call_original!(seq));
}

#[skyline::hook(offset = SEQ_MOUTH_HIGHLIGHT_SELECT, inline)]
unsafe fn seq_mouth_highlight_select(ctx: &mut skyline::hooks::InlineCtx) {
    let seq = ctx.registers[19].x() as *mut u8;
    let page = *at::<*mut u8>(seq, SEQ_PAGE);
    let mouth = *at::<u8>(seq, SEQ_FACE_PARAM_MOUTH) as usize;

    if ctx.registers[1].w() as i32 == -1
        && mouth >= VANILLA_MOUTHS
        && mouth < VANILLA_MOUTHS + extra_count(page)
    {
        ctx.registers[1].set_w(mouth as u32);
    }
}

#[skyline::hook(offset = SEQ_MOUTH_NAV_GRID_DONE, inline)]
unsafe fn seq_mouth_nav_grid(ctx: &mut skyline::hooks::InlineCtx) {
    let seq = ctx.registers[19].x() as *mut u8;
    let page = *at::<*mut u8>(seq, SEQ_PAGE);
    let extras = extra_count(page);

    if extras == 0 {
        return;
    }

    let build: extern "C" fn(*mut *mut u8, u32, u32) =
        core::mem::transmute(text().add(NAV_GRID_BUILD));

    let cols = VANILLA_MOUTHS + extras;
    let mut grid = [null_mut::<u8>(); NAV_GRID_MAX_CELLS];

    for i in 0..NOSE_BUTTONS {
        grid[i] = *at::<*mut u8>(page, PAGE_BTN_NOSE + i * 8);
    }

    for i in 0..VANILLA_MOUTHS {
        grid[cols + i] = *at::<*mut u8>(page, PAGE_BTN_MOUTH + i * 8);
    }

    for extra in 0..extras {
        grid[cols + VANILLA_MOUTHS + extra] = extra_button(page, extra);
    }

    build(grid.as_mut_ptr(), cols as u32, 2);

    for &button in &grid[cols..cols * 2] {
        if button.is_null() {
            continue;
        }

        let node = *at::<*mut u8>(button, BTN_ICON_NODE);

        if !node.is_null() {
            *at::<*mut u8>(node, NODE_NAV_LINKS + NAV_LINK_DOWN * 8) = null_mut();
        }
    }
}

#[skyline::main(name = "acnh-mouth")]
pub fn main() {
    unsafe { patch_accessor_vtable() };
    skyline::install_hooks!(
        page_lookup_buttons,
        seq_setup_controls,
        seq_on_control_focus,
        seq_on_control_decide,
        seq_update_focus_request,
        seq_process_request,
        seq_mouth_highlight_select,
        seq_mouth_nav_grid,
    );
}
