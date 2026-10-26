use core::ffi::c_void;

use nookkit_sys::{
    ControlEvent, Direction, EVENT_DECIDE, EVENT_FOCUS, SEQ_ACTIVATE, SEQ_DEACTIVATED, SEQ_READY,
    SEQ_UPDATE, Str,
};

use nookkit_types::game::ui::SequenceBase;

use crate::{Key, KeyBuilder, List, ListRef, ListTarget, PaneRef, Result, Ui, callback, check};

/// Valid while the game keeps the page built, which covers its callbacks
#[derive(Clone, Copy)]
pub struct Page {
    ui: Ui,
    raw: *mut c_void,
}

impl Page {
    pub(crate) fn new(ui: Ui, raw: *mut c_void) -> Self {
        Self { ui, raw }
    }

    pub fn raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn pane(&self, path: impl PaneRef) -> Result<Pane> {
        self.find(path.path(), "")
    }

    /// `class` needs its `<class>::getRuntimeTypeInfo` in the framework's symbol file
    pub fn pane_as(&self, path: impl PaneRef, class: &str) -> Result<Pane> {
        self.find(path.path(), class)
    }

    fn find(&self, path: &str, class: &str) -> Result<Pane> {
        let mut raw = core::ptr::null_mut();

        check(unsafe {
            (self.ui.table.find_pane)(self.raw, Str::new(path), Str::new(class), &mut raw)
        })?;

        Ok(Pane { page: *self, raw })
    }

    pub fn on_focus(&self, f: impl FnMut(Event) -> Result + 'static) -> Result {
        self.on_event(EVENT_FOCUS, f)
    }

    pub fn on_decide(&self, f: impl FnMut(Event) -> Result + 'static) -> Result {
        self.on_event(EVENT_DECIDE, f)
    }

    fn on_event(&self, kind: u32, mut f: impl FnMut(Event) -> Result + 'static) -> Result {
        let ui = self.ui;
        let cb = callback::raw(ui, move |arg| {
            match unsafe { arg.cast::<ControlEvent>().as_ref() } {
                Some(event) => f(Event::new(ui, event)),
                None => Ok(()),
            }
        });

        check(unsafe { (self.ui.table.on_control_event)(self.raw, kind, cb) })
    }

    /// The scroll list at `path`; lists a layout mod added are bound by the framework while the page builds
    pub fn list(&self, path: impl ListRef) -> Result<List> {
        let mut raw = core::ptr::null_mut();

        check(unsafe { (self.ui.table.list_get)(self.raw, Str::new(path.path()), &mut raw) })?;

        Ok(List::new(*self, raw))
    }

    /// Binds `key` on this page, see [`KeyBuilder`]
    pub fn bind_key(&self, key: Key) -> KeyBuilder {
        KeyBuilder::new(*self, key)
    }

    /// After the page's seq has set up its controls
    pub fn on_ready(&self, mut f: impl FnMut(Seq) -> Result + 'static) -> Result {
        self.on_seq(SEQ_READY, move |arg| {
            f(Seq {
                raw: arg.cast_mut(),
            })
        })
    }

    pub fn on_activate(&self, mut f: impl FnMut() -> Result + 'static) -> Result {
        self.on_seq(SEQ_ACTIVATE, move |_| f())
    }

    pub fn on_deactivated(&self, mut f: impl FnMut() -> Result + 'static) -> Result {
        self.on_seq(SEQ_DEACTIVATED, move |_| f())
    }

    /// Every frame the page's seq updates
    pub fn on_update(&self, mut f: impl FnMut() -> Result + 'static) -> Result {
        self.on_seq(SEQ_UPDATE, move |_| f())
    }

    /// Drops the game's own key-guide entries for `key` while this page is up, e.g. to replace them
    /// with a binding's guide. Reset when the page is built again
    pub fn set_guide_hidden(&self, key: Key, hidden: bool) -> Result {
        check(unsafe { (self.ui.table.page_hide_guide)(self.raw, key.id(), hidden) })
    }

    /// A disabled page fires none of its input, the game's handlers and bindings alike, e.g. while
    /// another menu is open over it
    pub fn set_input_enabled(&self, enable: bool) -> Result {
        check(unsafe { (self.ui.table.page_set_input_enabled)(self.raw, enable) })
    }

    fn on_seq(&self, kind: u32, f: impl FnMut(*const c_void) -> Result + 'static) -> Result {
        let cb = callback::raw(self.ui, f);

        check(unsafe { (self.ui.table.on_seq_event)(self.raw, kind, cb) })
    }

    pub(crate) fn ui(&self) -> Ui {
        self.ui
    }
}

#[derive(Clone, Copy)]
pub struct Pane {
    page: Page,
    raw: *mut c_void,
}

#[derive(Clone, Copy)]
pub struct Seq {
    raw: *mut c_void,
}

impl Seq {
    /// For reading the seq's own struct
    pub fn raw(&self) -> *mut c_void {
        self.raw
    }

    /// Queues a state for the seq's state machine the way the game does, `false` when a request is
    /// already queued
    pub fn request_state(&self, state: i32) -> bool {
        unsafe { self.raw.cast::<SequenceBase>().as_mut() }
            .is_some_and(|seq| seq.request_state(state))
    }
}

impl PartialEq for Pane {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Pane {
    pub fn raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn page(&self) -> Page {
        self.page
    }

    /// Both panes must be buttons
    pub fn link(&self, direction: Direction, to: Pane) -> Result<Pane> {
        check(unsafe { (self.page.ui.table.link)(self.raw, direction, to.raw) })?;

        Ok(*self)
    }

    pub fn unlink(&self, direction: Direction) -> Result<Pane> {
        check(unsafe { (self.page.ui.table.link)(self.raw, direction, core::ptr::null_mut()) })?;

        Ok(*self)
    }

    pub fn set_focusable(&self, player: u32, enable: bool) -> Result<Pane> {
        check(unsafe { (self.page.ui.table.set_focusable)(self.raw, player, enable) })?;

        Ok(*self)
    }

    pub(crate) fn new(page: Page, raw: *mut c_void) -> Self {
        Self { page, raw }
    }

    /// Keeps the link pointed at `target` of the list's visible window as it scrolls
    pub fn link_into(&self, direction: Direction, list: &List, target: ListTarget) -> Result<Pane> {
        check(unsafe {
            (self.page.ui.table.list_link_into)(self.raw, direction, list.raw(), target.into())
        })?;

        Ok(*self)
    }

    pub fn set_visible(&self, visible: bool) -> Result<Pane> {
        check(unsafe { (self.page.ui.table.set_visible)(self.raw, visible) })?;

        Ok(*self)
    }

    pub fn on_focus(&self, mut f: impl FnMut(Event) -> Result + 'static) -> Result<Pane> {
        let this = *self;
        self.page.on_focus(move |e| {
            if e.control == Some(this) {
                f(e)
            } else {
                Ok(())
            }
        })?;

        Ok(*self)
    }

    pub fn on_decide(&self, mut f: impl FnMut(Event) -> Result + 'static) -> Result<Pane> {
        let this = *self;
        self.page.on_decide(move |e| {
            if e.control == Some(this) {
                f(e)
            } else {
                Ok(())
            }
        })?;

        Ok(*self)
    }
}

pub struct Event {
    pub page: Page,
    pub control: Option<Pane>,
}

impl Event {
    pub(crate) fn new(ui: Ui, event: &ControlEvent) -> Self {
        let page = Page::new(ui, event.page);
        let control = (!event.control.is_null()).then_some(Pane {
            page,
            raw: event.control,
        });

        Self { page, control }
    }
}
