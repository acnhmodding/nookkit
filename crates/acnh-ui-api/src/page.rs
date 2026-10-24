use core::ffi::c_void;

use acnh_ui_sys::{ControlEvent, Direction, EVENT_DECIDE, EVENT_FOCUS, Str};

use crate::{Result, Ui, callback, check};

pub trait PaneRef {
    fn path(&self) -> &str;
}

impl PaneRef for &str {
    fn path(&self) -> &str {
        self
    }
}

impl PaneRef for String {
    fn path(&self) -> &str {
        self
    }
}

/// Valid while the game keeps the page built, which covers its callbacks
#[derive(Clone, Copy)]
pub struct Page {
    ui: Ui,
    raw: *mut c_void,
}

unsafe impl Send for Page {}

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

    /// `class` must be a `[type_info]` entry of the symbol file
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

    pub fn on_focus(&self, f: impl FnMut(Event) -> Result + Send + 'static) -> Result {
        self.on_event(EVENT_FOCUS, f)
    }

    pub fn on_decide(&self, f: impl FnMut(Event) -> Result + Send + 'static) -> Result {
        self.on_event(EVENT_DECIDE, f)
    }

    fn on_event(&self, kind: u32, f: impl FnMut(Event) -> Result + Send + 'static) -> Result {
        let cb = callback::event(self.ui, f);

        check(unsafe { (self.ui.table.on_control_event)(self.raw, kind, cb) })
    }
}

#[derive(Clone, Copy)]
pub struct Pane {
    page: Page,
    raw: *mut c_void,
}

unsafe impl Send for Pane {}

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

    pub fn on_focus(&self, mut f: impl FnMut(Event) -> Result + Send + 'static) -> Result<Pane> {
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

    pub fn on_decide(&self, mut f: impl FnMut(Event) -> Result + Send + 'static) -> Result<Pane> {
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
