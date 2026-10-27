#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod callback;
mod key;
mod list;
mod offsets;
mod page;
pub mod screens;

use core::sync::atomic::{AtomicPtr, Ordering};

use nookkit_sys::{API_NAME, API_VERSION, Handle, NookKitV2, Status, Str, SymbolKind, sl_api_get};
use nookkit_types::native::SymbolKind as NativeKind;

pub use key::{Key, KeyBinding, KeyBuilder, KeyEvent};
pub use list::{ItemUpdate, List, ListTarget};
pub use nookkit_layouts::{ListRef, PageClass, PanePath, PaneRef, RelPath, kind, layouts, pages};
pub use nookkit_sys::Player;
pub use nookkit_types as types;
pub use nookkit_types::native::sym;
pub use offsets::{__hook_function, module_base};
pub use page::{Event, Page, Pane, Seq, SeqState, StateHooks};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UiError {
    #[error("NookKit is not loaded")]
    FrameworkMissing,

    #[error("NookKit is older than this mod")]
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

    #[error("the page was destroyed")]
    Stale,

    #[error("installing a hook failed")]
    HookFailed,

    #[error("a callback fired its own event, the nested call was skipped")]
    Reentrant,

    #[error("unknown status {0} from a newer NookKit")]
    Unknown(i32),
}

pub type Result<T = (), E = UiError> = core::result::Result<T, E>;

fn check(status: Status) -> Result {
    Err(match status {
        Status::OK => return Ok(()),
        Status::INVALID_ARG => UiError::InvalidArg,
        Status::PANE_NOT_FOUND => UiError::PaneNotFound,
        Status::TYPE_MISMATCH => UiError::TypeMismatch,
        Status::NO_EVENT_TABLE => UiError::NoEventTable,
        Status::EVENT_TABLE_FULL => UiError::EventTableFull,
        Status::LOCK_FAILED => UiError::LockFailed,
        Status::MISSING_SYMBOL => UiError::MissingSymbol,
        Status::UNSUPPORTED_BUILD => UiError::UnsupportedBuild,
        Status::STALE => UiError::Stale,
        Status(code) => UiError::Unknown(code),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl From<Direction> for nookkit_sys::Direction {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Up => Self::UP,
            Direction::Down => Self::DOWN,
            Direction::Left => Self::LEFT,
            Direction::Right => Self::RIGHT,
        }
    }
}

impl TryFrom<nookkit_sys::Direction> for Direction {
    type Error = UiError;

    fn try_from(direction: nookkit_sys::Direction) -> Result<Self> {
        Ok(match direction {
            nookkit_sys::Direction::UP => Self::Up,
            nookkit_sys::Direction::DOWN => Self::Down,
            nookkit_sys::Direction::LEFT => Self::Left,
            nookkit_sys::Direction::RIGHT => Self::Right,
            nookkit_sys::Direction(code) => return Err(UiError::Unknown(code as i32)),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PageFilter<'a> {
    Layout(&'a str),
    Class(&'a str),
    Both { layout: &'a str, class: &'a str },
}

#[derive(Clone, Copy)]
pub struct Ui {
    table: &'static NookKitV2,
    name: &'static str,
}

impl core::fmt::Debug for Ui {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ui").field("name", &self.name).finish()
    }
}

#[must_use = "dropping the lock releases the player"]
#[derive(Debug)]
pub struct PlayerLock {
    ui: Ui,
    player: Player,
}

impl PlayerLock {
    pub fn player(&self) -> Player {
        self.player
    }

    pub fn release(self) -> Result {
        let result = check(unsafe { (self.ui.table.release_player)(self.player) });
        core::mem::forget(self);
        result
    }
}

impl Drop for PlayerLock {
    fn drop(&mut self) {
        self.ui
            .report(check(unsafe { (self.ui.table.release_player)(self.player) }));
    }
}

static TABLE: AtomicPtr<NookKitV2> = AtomicPtr::new(core::ptr::null_mut());

fn resolve(name: &str, kind: NativeKind) -> Option<usize> {
    let table = unsafe { TABLE.load(Ordering::Acquire).as_ref() }?;
    let kind = match kind {
        NativeKind::Fn => SymbolKind::FN,
        NativeKind::Vtable => SymbolKind::VTABLE,
        NativeKind::Data => SymbolKind::DATA,
    };

    let address = unsafe { (table.resolve)(Str::new(name), kind) };

    (address != 0).then_some(address)
}

/// Fails with [`UiError::UnsupportedBuild`] unless the game is the build this crate version supports.
/// `name` prefixes the errors printed for failed callbacks
pub fn connect(name: &'static str) -> Result<Ui> {
    let table = unsafe { sl_api_get(API_NAME.as_ptr(), API_VERSION) }.cast::<NookKitV2>();
    if table.is_null() {
        return Err(UiError::FrameworkMissing);
    }

    // An older framework's table is shorter, so only its size may be read before this check
    if unsafe { table.cast::<usize>().read() } < core::mem::size_of::<NookKitV2>() {
        return Err(UiError::ApiTooOld);
    }

    let table_ref = unsafe { &*table };

    if !unsafe { (table_ref.started)() } {
        return Err(UiError::UnsupportedBuild);
    }

    TABLE.store(table.cast_mut(), Ordering::Release);
    types::native::set_resolver(resolve);

    Ok(Ui {
        table: table_ref,
        name,
    })
}

impl Ui {
    /// Runs `f` each time the game builds a `P` page
    ///
    /// ```no_run
    /// use nookkit::pages::CharaMake;
    ///
    /// fn hook(ui: nookkit::Ui) -> nookkit::Result {
    ///     ui.on_page::<CharaMake>(|page| {
    ///         let eyes = page.list(CharaMake::L_BTN_LIST_EYE_00)?;
    ///         let ok = page.pane(CharaMake::L_BTN_OK_00)?;
    ///         Ok(())
    ///     })
    /// }
    /// ```
    pub fn on_page<P: PageClass>(&self, f: impl FnMut(Page) -> Result + 'static) -> Result {
        self.on_page_filter(PageFilter::Class(P::CLASS), f)
    }

    /// [`on_page`](Self::on_page) by name, for pages missing from [`pages`]
    pub fn on_page_filter(
        &self,
        filter: PageFilter<'_>,
        mut f: impl FnMut(Page) -> Result + 'static,
    ) -> Result {
        let (layout, class) = match filter {
            PageFilter::Layout(layout) => (layout, ""),
            PageFilter::Class(class) => ("", class),
            PageFilter::Both { layout, class } => (layout, class),
        };

        let ui = *self;
        let cb = callback::raw(ui, move |arg| match unsafe { arg.cast::<Handle>().as_ref() } {
            Some(&handle) => f(Page::new(ui, handle)),
            None => Ok(()),
        });

        check(unsafe { (self.table.on_page_built)(Str::new(layout), Str::new(class), cb) })
    }

    /// Freezes `player` in the field until the lock is released or dropped
    pub fn lock_player(&self, player: Player) -> Result<PlayerLock> {
        check(unsafe { (self.table.lock_player)(player) })?;

        Ok(PlayerLock { ui: *self, player })
    }

    /// The most recently built `P` page the game hasn't destroyed, e.g. the resident pocket menu
    pub fn find_page<P: PageClass>(&self) -> Result<Option<Page>> {
        let mut handle = Handle::NULL;

        check(unsafe { (self.table.find_page)(Str::new(P::CLASS), &mut handle) })?;

        Ok((!handle.raw.is_null()).then(|| Page::new(*self, handle)))
    }

    /// Opens the pocket menu for `player` over whatever is open, its page's `on_deactivate` fires
    /// when it closes
    pub fn open_pocket_menu(&self, player: Player, close_enabled: bool) -> Result {
        check(unsafe { (self.table.open_pocket_menu)(player, close_enabled) })
    }

    /// The seq at `raw` if it built a page that is still alive
    pub(crate) fn seq_at(&self, raw: *mut core::ffi::c_void) -> Result<Option<Seq>> {
        if raw.is_null() {
            return Ok(None);
        }

        let mut handle = Handle::NULL;

        check(unsafe { (self.table.seq_page)(raw, &mut handle) })?;

        Ok((!handle.raw.is_null()).then(|| Seq::new(Page::new(*self, handle), raw)))
    }

    pub fn framework_version(&self) -> u32 {
        self.table.framework_version
    }

    fn report(&self, result: Result) {
        if let Err(e) = result {
            println!("[{}] {e}", self.name);
        }
    }
}
