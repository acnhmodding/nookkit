use core::cell::{Cell, RefCell, UnsafeCell};
use core::ffi::c_void;
use core::ptr::null_mut;
use std::ffi::CString;

use nookkit_sys::{
    Callback, ControlEvent, Direction, EVENT_DECIDE, EVENT_FOCUS, ITEM_CLEAR, ITEM_MOVE,
    ITEM_RENDER, ItemUpdate, LIST_DECIDE, LIST_EVENT_KINDS, LIST_FOCUS, LIST_ITEM, LIST_LEAVE,
    LeaveQuery, ListTarget, NO_LINE, Status,
};
use nookkit_types::game::ui::{
    AppUniteBtnCtrl, ItemEvent, ItemOp, ItemUpdater, ListBtnGroup, VftListBtnGroup, ViewPage,
};
use nookkit_types::game::ui_sys::{Content, Enable, LeaveDirection, LeaveEvent, ScrollBtnListCtrl};
use nookkit_types::lp::ui::{BtnPartsCtrl, Control};
use nookkit_types::native::NativeResult;
use nookkit_types::sead::Delegate;

use crate::pages::{self, call, snapshot, with_page};
use crate::report;

// What CharaMake passes for its eye list, the only list setup observed so far
const DEFAULT_ENABLE: Enable = Enable {
    update_cursor_button: false,
    cursor_button_arg0: 1,
    cursor_button_arg1: 0,
    focus_cursor: true,
    cursor_item: -1,
};

fn default_content(item_count: i32, selected: i32) -> Content {
    let mut flags = [0; 0xc];

    flags[0x16 - 0x14] = 1;
    flags[0x19 - 0x14] = 1;

    Content {
        item_count,
        scroll_item: selected,
        focus_item: selected,
        flags,
        unk_20: 0,
        unk_24: 0xc,
        unk_28: 6,
    }
}

struct Link {
    from: *mut BtnPartsCtrl,
    direction: Direction,
    target: ListTarget,
}

/// Leaked on purpose: a list control can outlive a page rebuild and still call its delegates, so a
/// retired state only goes inert
pub struct ListState {
    page: *mut ViewPage,
    list: *mut ScrollBtnListCtrl,
    alive: Cell<bool>,

    /// Set by the tab switch; outside focus only pauses an active list
    active: Cell<bool>,

    group: UnsafeCell<ListBtnGroup>,
    updater: UnsafeCell<ItemUpdater>,
    links: RefCell<Vec<Link>>,
    cbs: [RefCell<Vec<Callback>>; LIST_EVENT_KINDS],
}

unsafe impl Send for ListState {}
unsafe impl Sync for ListState {}

impl ListState {
    pub fn retire(&self) {
        self.alive.set(false);
        self.links.borrow_mut().clear();

        let cbs: Vec<Vec<Callback>> = self
            .cbs
            .iter()
            .map(|cbs| core::mem::take(&mut *cbs.borrow_mut()))
            .collect();

        pages::bury(cbs);
    }

    pub fn is_alive(&self) -> bool {
        self.alive.get()
    }

    pub fn set_active(&self, active: bool) {
        self.active.set(active);
    }

    // Game memory, not borrowed from self
    #[allow(clippy::mut_from_ref)]
    fn list(&self) -> &mut ScrollBtnListCtrl {
        unsafe { &mut *self.list }
    }

    fn as_user(&self) -> *mut c_void {
        (self as *const Self).cast_mut().cast()
    }

    unsafe fn fire(&self, kind: u32, arg: *const c_void) {
        let Some(cbs) = self.cbs.get(kind as usize) else {
            return;
        };

        let cbs = snapshot(&cbs.borrow());

        unsafe { call(cbs, arg) };
    }

    pub unsafe fn setup(&self) -> NativeResult {
        if !self.alive.get() {
            return Ok(());
        }

        unsafe {
            let list = self.list();
            let page = &*self.page;

            list.attach(self.page.cast(), page.root(), 0)?;
            *self.group.get() = ListBtnGroup::new()?;

            list.scroll_delegate = Delegate::bind(self.as_user(), on_scroll as *const () as usize);
            list.leave_delegate = Delegate::bind(self.as_user(), on_leave as *const () as usize);

            for slot in 0..list.slot_count() {
                if let Some(button) = list.button(slot).as_mut() {
                    button.set_list_mode();
                    button.bind_to_group(self.group.get())?;
                }
            }

            *self.updater.get() = ItemUpdater::new(
                list.slot_count() as u32,
                Delegate::bind(self.as_user(), on_item as *const () as usize),
            );
        }

        self.active.set(false);

        Ok(())
    }

    pub unsafe fn set_items(&self, count: u32, selected: u32, player: u8) -> NativeResult {
        unsafe {
            (*self.group.get()).selected = selected as i32;
            self.list()
                .set_content(&default_content(count as i32, selected as i32), player)?;
        }

        if self.active.get() {
            unsafe { self.relink() };
        }

        Ok(())
    }

    pub unsafe fn set_enabled(&self, enable: bool, mode: u32) -> NativeResult {
        self.active.set(enable);

        let list = self.list();

        unsafe {
            if !enable {
                list.disable(mode)?;
            } else if list.state_machine.current_state > 1 {
                list.enable(&DEFAULT_ENABLE)?;
            } else if list.is_ready_to_play() {
                list.play()?;
            }
        }

        Ok(())
    }

    pub unsafe fn relink(&self) {
        let list = self.list();
        let last = list.line_size.max(1) as usize - 1;

        for link in self.links.borrow().iter() {
            let pos = if link.target.last { last } else { 0 };
            let target = unsafe { list.visible_button(link.target.line as usize, pos) };

            unsafe {
                let (Some(from), Some(target)) = (link.from.as_mut(), target.as_mut()) else {
                    continue;
                };

                if !target.focus_node.is_null() {
                    from.set_link(crate::nav(link.direction), Some(target));
                }
            }
        }
    }

    // Neighbours link into every visible line, so all of them stay focusable while paused
    unsafe fn enable_visible_buttons(&self) -> NativeResult {
        let list = self.list();
        let line_size = list.line_size.max(1);
        let visible = list.first_line..list.first_line + list.visible_lines;

        for slot in 0..list.button_count() {
            let Some(button) = (unsafe { list.button(slot).as_mut() }) else {
                continue;
            };

            let control = (button as *mut AppUniteBtnCtrl).cast::<Control>();
            let item = unsafe { list.item_index_of(control)? };

            if item >= 0 && item < list.padded_item_count && visible.contains(&(item / line_size)) {
                unsafe { (*control).set_focusable(list.player, true)? };
            }
        }

        Ok(())
    }

    unsafe fn on_focus(&self, control: *mut Control) -> NativeResult {
        let index = unsafe { self.list().item_index_of(control)? };

        if index >= 0 {
            let item = index as u32;
            unsafe { self.fire(LIST_FOCUS, (&item as *const u32).cast()) };
        } else if self.active.get() {
            // Outside focus (e.g. a tab entering on another row) bypasses the list, which would keep the D-pad
            unsafe {
                self.list().pause()?;
                self.enable_visible_buttons()?;
                self.relink();
            }
        }

        Ok(())
    }

    unsafe fn on_decide(&self, control: *mut Control) -> NativeResult {
        let index = unsafe { self.list().item_index_of(control)? };
        let group = unsafe { &mut *self.group.get() };

        if index < 0 || group.locked {
            return Ok(());
        }

        unsafe {
            if group.is_changed(index) {
                group.clear_selection(self.list);
                group.set_selected(index);
            } else if !group.toggle {
                group.clear_selection(self.list);
                group.set_selected(-1);

                return Ok(());
            } else if group.keep == 0 {
                return Ok(());
            }
        }

        let item = unsafe { group.selected() }.max(0) as u32;
        unsafe { self.fire(LIST_DECIDE, (&item as *const u32).cast()) };

        Ok(())
    }
}

unsafe fn state<'a>(user: *mut c_void) -> Option<&'a ListState> {
    unsafe { user.cast::<ListState>().as_ref() }.filter(|state| state.alive.get())
}

extern "C" fn on_scroll(user: *mut c_void, event: *mut c_void) {
    let Some(state) = (unsafe { state(user) }) else {
        return;
    };

    let result: NativeResult = unsafe {
        (*state.updater.get())
            .update(event)
            .and_then(|()| (*state.group.get()).refresh(state.list))
    };

    report("list scroll", result);
}

extern "C" fn on_item(user: *mut c_void, event: *const ItemEvent) {
    let (Some(state), Some(event)) = (unsafe { state(user) }, unsafe { event.as_ref() }) else {
        return;
    };

    let Some(op) = event.op() else {
        return;
    };

    let (op, item) = match op {
        ItemOp::Clear => (ITEM_CLEAR, -1),
        ItemOp::Move => (ITEM_MOVE, unsafe { event.item() }),
        ItemOp::Render => (ITEM_RENDER, unsafe { event.item() }),
    };

    let update = ItemUpdate {
        op,
        slot: event.slot.max(0) as u32,
        from_slot: event.from_slot.max(0) as u32,
        item,
    };

    unsafe { state.fire(LIST_ITEM, (&update as *const ItemUpdate).cast()) };
}

extern "C" fn on_leave(user: *mut c_void, target: *mut *mut Control, event: *const LeaveEvent) {
    let (Some(state), Some(event)) = (unsafe { state(user) }, unsafe { event.as_ref() }) else {
        return;
    };

    let direction = match event.direction() {
        Some(LeaveDirection::Up) => Direction::Up,
        Some(LeaveDirection::Down) => Direction::Down,
        None => return,
    };

    let mut out: *mut c_void = null_mut();
    let query = LeaveQuery {
        direction,
        line: state
            .list()
            .focused_visible_line()
            .map_or(NO_LINE, |line| line as u32),
        target: &mut out,
    };

    unsafe { state.fire(LIST_LEAVE, (&query as *const LeaveQuery).cast()) };

    if !out.is_null() && !target.is_null() {
        unsafe { *target = out.cast() };
    }
}

unsafe extern "C" fn page_focus(user: *mut c_void, arg: *const c_void) {
    let (Some(state), Some(event)) = (unsafe { state(user) }, unsafe {
        arg.cast::<ControlEvent>().as_ref()
    }) else {
        return;
    };

    report("list focus", unsafe {
        state.on_focus(event.control.cast())
    });
}

unsafe extern "C" fn page_decide(user: *mut c_void, arg: *const c_void) {
    let (Some(state), Some(event)) = (unsafe { state(user) }, unsafe {
        arg.cast::<ControlEvent>().as_ref()
    }) else {
        return;
    };

    report("list decide", unsafe {
        state.on_decide(event.control.cast())
    });
}

pub unsafe fn get(
    page: *mut ViewPage,
    path: &str,
) -> NativeResult<Result<&'static ListState, Status>> {
    let (Some(page_ref), Ok(path)) = (unsafe { page.as_ref() }, CString::new(path)) else {
        return Ok(Err(Status::InvalidArg));
    };

    let Some(root) = (unsafe { page_ref.root().as_mut() }) else {
        return Ok(Err(Status::PaneNotFound));
    };

    let control = unsafe { root.find_by_path(&path)? };

    if control.is_null() {
        return Ok(Err(Status::PaneNotFound));
    }

    let Some(list) = (unsafe { Control::cast_ptr::<ScrollBtnListCtrl>(control)? }) else {
        return Ok(Err(Status::TypeMismatch));
    };

    let list = list as *mut ScrollBtnListCtrl;
    let existing = with_page(page, |state| {
        state
            .lists
            .iter()
            .copied()
            .find(|l| core::ptr::eq(l.list, list))
    })
    .flatten();

    if let Some(existing) = existing {
        return Ok(Ok(existing));
    }

    let state: &'static ListState = Box::leak(Box::new(ListState {
        page,
        list,
        alive: Cell::new(true),
        active: Cell::new(false),
        group: UnsafeCell::new(ListBtnGroup::empty()),
        updater: UnsafeCell::new(ItemUpdater::empty()),
        links: RefCell::new(Vec::new()),
        cbs: Default::default(),
    }));

    // Published only once both events are in, so a failed get isn't found half registered later
    for (kind, func) in [
        (EVENT_FOCUS, page_focus as _),
        (EVENT_DECIDE, page_decide as _),
    ] {
        let cb = Callback {
            func,
            user: state.as_user(),
            drop: None,
        };

        let status = match unsafe { pages::on_control_event(page, kind, cb) } {
            Ok(status) => status,
            Err(e) => {
                state.retire();
                return Err(e);
            }
        };

        if status != Status::Ok {
            state.retire();
            return Ok(Err(status));
        }
    }

    with_page(page, |page_state| page_state.lists.push(state));

    // Lists are set up when the seq is ready, so one asked for later is set up now
    if pages::live_seq(page).is_some() {
        report("list setup", unsafe { state.setup() });
    }

    Ok(Ok(state))
}

pub unsafe fn on(list: &ListState, kind: u32, cb: Callback) -> Status {
    match list.cbs.get(kind as usize) {
        Some(cbs) => {
            cbs.borrow_mut().push(cb);
            Status::Ok
        }
        None => Status::InvalidArg,
    }
}

pub unsafe fn link_into(
    from: *mut c_void,
    direction: Direction,
    list: &ListState,
    target: ListTarget,
) -> NativeResult<Status> {
    let Some(from) = (unsafe { Control::cast_ptr::<BtnPartsCtrl>(from.cast())? }) else {
        return Ok(Status::TypeMismatch);
    };

    list.links.borrow_mut().push(Link {
        from,
        direction,
        target,
    });

    Ok(Status::Ok)
}

pub fn slot_count(list: &ListState) -> u32 {
    list.list().slot_count() as u32
}

pub unsafe fn button(list: &ListState, slot: u32) -> Option<*mut c_void> {
    let button = unsafe { list.list().button(slot as usize) };

    (!button.is_null()).then_some(button.cast())
}
