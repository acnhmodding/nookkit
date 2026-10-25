#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod callback;
mod list;
mod offsets;
mod page;

use core::sync::atomic::{AtomicPtr, Ordering};

use acnh_game::native::SymbolKind;
use acnh_ui_sys::{
    API_NAME, API_VERSION, AcnhUiV1, SYMBOL_FN, SYMBOL_VTABLE, Status, Str, sl_api_get,
};

pub use acnh_game as game;
pub use acnh_ui_sys::Direction;
pub use list::{ItemUpdate, List, ListTarget};
pub use offsets::{__hook_function, module_base};
pub use page::{Event, Page, Pane, PaneRef, Seq};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UiError {
    #[error("acnh-ui is not loaded")]
    FrameworkMissing,
    #[error("acnh-ui is older than this mod")]
    ApiTooOld,
    #[error("invalid argument")]
    InvalidArg,
    #[error("pane not found")]
    PaneNotFound,
    #[error("pane is not of the expected class")]
    TypeMismatch,
    #[error("page has no control event table")]
    NoEventTable,
    #[error("page's control event table is full")]
    EventTableFull,
    #[error("player lock failed")]
    LockFailed,
    #[error("symbol missing from the symbol file")]
    MissingSymbol,
    #[error("unsupported game build")]
    UnsupportedBuild,
}

pub type Result<T = (), E = UiError> = core::result::Result<T, E>;

fn check(status: Status) -> Result {
    Err(match status {
        Status::Ok => return Ok(()),
        Status::InvalidArg => UiError::InvalidArg,
        Status::PaneNotFound => UiError::PaneNotFound,
        Status::TypeMismatch => UiError::TypeMismatch,
        Status::NoEventTable => UiError::NoEventTable,
        Status::EventTableFull => UiError::EventTableFull,
        Status::LockFailed => UiError::LockFailed,
        Status::MissingSymbol => UiError::MissingSymbol,
        Status::UnsupportedBuild => UiError::UnsupportedBuild,
    })
}

pub enum PageFilter<'a> {
    Layout(&'a str),
    Class(&'a str),
}

#[derive(Clone, Copy)]
pub struct Ui {
    table: &'static AcnhUiV1,
    name: &'static str,
}

static TABLE: AtomicPtr<AcnhUiV1> = AtomicPtr::new(core::ptr::null_mut());

fn resolve(name: &str, kind: SymbolKind) -> Option<usize> {
    let table = unsafe { TABLE.load(Ordering::Acquire).as_ref() }?;
    let kind = match kind {
        SymbolKind::Fn => SYMBOL_FN,
        SymbolKind::Vtable => SYMBOL_VTABLE,
    };

    let address = unsafe { (table.resolve)(Str::new(name), kind) };

    (address != 0).then_some(address)
}

/// Fails with [`UiError::UnsupportedBuild`] unless the game is the build this crate version supports.
/// `name` prefixes the errors printed for failed callbacks
pub fn connect(name: &'static str) -> Result<Ui> {
    let table = unsafe { sl_api_get(API_NAME.as_ptr(), API_VERSION) }.cast::<AcnhUiV1>();
    let Some(table_ref) = (unsafe { table.as_ref() }) else {
        return Err(UiError::FrameworkMissing);
    };

    if table_ref.size < core::mem::size_of::<AcnhUiV1>() {
        return Err(UiError::ApiTooOld);
    }

    if !unsafe { (table_ref.started)() } {
        return Err(UiError::UnsupportedBuild);
    }

    TABLE.store(table.cast_mut(), Ordering::Release);
    game::native::set_resolver(resolve);

    Ok(Ui {
        table: table_ref,
        name,
    })
}

impl Ui {
    pub fn on_page(
        &self,
        filter: PageFilter<'_>,
        mut f: impl FnMut(Page) -> Result + 'static,
    ) -> Result {
        let (layout, class) = match filter {
            PageFilter::Layout(layout) => (layout, ""),
            PageFilter::Class(class) => ("", class),
        };

        let ui = *self;
        let cb = callback::raw(ui, move |arg| f(Page::new(ui, arg.cast_mut())));

        check(unsafe { (self.table.on_page_built)(Str::new(layout), Str::new(class), cb) })
    }

    pub fn lock_player(&self, player: u8) -> Result {
        check(unsafe { (self.table.lock_player)(player) })
    }

    pub fn release_player(&self, player: u8) -> Result {
        check(unsafe { (self.table.release_player)(player) })
    }

    fn report(&self, result: Result) {
        if let Err(e) = result {
            println!("[{}] {e}", self.name);
        }
    }
}
