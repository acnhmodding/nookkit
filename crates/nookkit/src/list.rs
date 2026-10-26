use core::ffi::c_void;

use nookkit_sys::{
    Direction, ITEM_CLEAR, ITEM_MOVE, ITEM_RENDER, ItemUpdate as RawItemUpdate, LIST_DECIDE,
    LIST_FOCUS, LIST_ITEM, LIST_LEAVE, LeaveQuery, ListTarget as RawListTarget, NO_LINE,
};

use crate::{Page, Pane, Result, callback, check};

pub enum ItemUpdate {
    Render {
        slot: usize,
        item: usize,
    },
    Move {
        slot: usize,
        from: usize,
        item: usize,
    },
    Clear {
        slot: usize,
    },
}

impl ItemUpdate {
    fn from_raw(raw: &RawItemUpdate) -> Option<Self> {
        let (slot, from, item) = (
            raw.slot as usize,
            raw.from_slot as usize,
            usize::try_from(raw.item),
        );

        match raw.op {
            ITEM_CLEAR => Some(Self::Clear { slot }),
            ITEM_MOVE => Some(Self::Move {
                slot,
                from,
                item: item.ok()?,
            }),
            ITEM_RENDER => Some(Self::Render {
                slot,
                item: item.ok()?,
            }),
            _ => None,
        }
    }
}

/// A line of the list's visible window
#[derive(Clone, Copy)]
pub struct ListTarget {
    line: usize,
    last: bool,
}

impl ListTarget {
    pub fn line(line: usize) -> Self {
        Self { line, last: false }
    }

    pub fn first(self) -> Self {
        Self {
            last: false,
            ..self
        }
    }

    pub fn last(self) -> Self {
        Self { last: true, ..self }
    }
}

impl From<ListTarget> for RawListTarget {
    fn from(target: ListTarget) -> Self {
        Self {
            line: target.line as u32,
            last: target.last,
        }
    }
}

#[derive(Clone, Copy)]
pub struct List {
    page: Page,
    raw: *mut c_void,
}

impl List {
    pub(crate) fn new(page: Page, raw: *mut c_void) -> Self {
        Self { page, raw }
    }

    pub(crate) fn raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn on_item(&self, mut f: impl FnMut(ItemUpdate) -> Result + 'static) -> Result<List> {
        self.on(LIST_ITEM, move |arg| {
            match unsafe { arg.cast::<RawItemUpdate>().as_ref() }.and_then(ItemUpdate::from_raw) {
                Some(update) => f(update),
                None => Ok(()),
            }
        })
    }

    pub fn on_focus(&self, mut f: impl FnMut(usize) -> Result + 'static) -> Result<List> {
        self.on(LIST_FOCUS, move |arg| {
            match unsafe { arg.cast::<u32>().as_ref() } {
                Some(&item) => f(item as usize),
                None => Ok(()),
            }
        })
    }

    /// After the list's selection group accepted the press
    pub fn on_decide(&self, mut f: impl FnMut(usize) -> Result + 'static) -> Result<List> {
        self.on(LIST_DECIDE, move |arg| {
            match unsafe { arg.cast::<u32>().as_ref() } {
                Some(&item) => f(item as usize),
                None => Ok(()),
            }
        })
    }

    /// Picks where focus goes when it leaves the list, given the visible line it leaves from
    pub fn on_leave(
        &self,
        mut f: impl FnMut(Direction, Option<usize>) -> Result<Option<Pane>> + 'static,
    ) -> Result<List> {
        self.on(LIST_LEAVE, move |arg| {
            let Some(query) = (unsafe { arg.cast::<LeaveQuery>().as_ref() }) else {
                return Ok(());
            };

            let line = (query.line != NO_LINE).then_some(query.line as usize);

            if let (Some(target), false) = (f(query.direction, line)?, query.target.is_null()) {
                unsafe { *query.target = target.raw() };
            }

            Ok(())
        })
    }

    fn on(&self, kind: u32, f: impl FnMut(*const c_void) -> Result + 'static) -> Result<List> {
        let cb = callback::raw(self.page.ui(), f);

        check(unsafe { (self.page.ui().table.list_on)(self.raw, kind, cb) })?;

        Ok(*self)
    }

    pub fn set_items(&self, count: usize, selected: usize, player: u8) -> Result {
        check(unsafe {
            (self.page.ui().table.list_set_items)(self.raw, count as u32, selected as u32, player)
        })
    }

    pub fn set_enabled(&self, enable: bool, mode: u32) -> Result {
        check(unsafe { (self.page.ui().table.list_set_enabled)(self.raw, enable, mode) })
    }

    /// Re-applies the `link_into` links, e.g. after the game rewrote them
    pub fn relink(&self) -> Result {
        check(unsafe { (self.page.ui().table.list_relink)(self.raw) })
    }

    pub fn slots(&self) -> Result<usize> {
        let mut count = 0;

        check(unsafe { (self.page.ui().table.list_slot_count)(self.raw, &mut count) })?;

        Ok(count as usize)
    }

    pub fn button(&self, slot: usize) -> Result<Pane> {
        let mut raw = core::ptr::null_mut();

        check(unsafe { (self.page.ui().table.list_button)(self.raw, slot as u32, &mut raw) })?;

        Ok(Pane::new(self.page, raw))
    }
}
