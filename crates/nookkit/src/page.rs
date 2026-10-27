use core::ffi::c_void;

use nookkit_sys::{
    ControlEvent, ControlEventKind, Handle, PaneHandle, Player, SeqEvent, StatePhase, Str,
};

use nookkit_types::game::ui::SequenceBase;
use nookkit_types::game::ui_sys::VftSequence;

use crate::{
    Direction, Key, KeyBuilder, List, ListRef, ListTarget, PaneRef, Result, Ui, callback, check,
};

/// One build of a game page. Calls fail with [`UiError::Stale`](crate::UiError::Stale) once the
/// game destroys it
#[derive(Clone, Copy)]
pub struct Page {
    ui: Ui,
    handle: Handle,
}

impl Page {
    pub(crate) fn new(ui: Ui, handle: Handle) -> Self {
        Self { ui, handle }
    }

    pub fn raw(&self) -> *mut c_void {
        self.handle.raw
    }

    pub(crate) fn handle(&self) -> Handle {
        self.handle
    }

    /// The game hasn't destroyed the page
    pub fn is_alive(&self) -> bool {
        self.check().is_ok()
    }

    fn check(&self) -> Result {
        check(unsafe { (self.ui.table.page_check)(self.handle) })
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
            (self.ui.table.find_pane)(self.handle, Str::new(path), Str::new(class), &mut raw)
        })?;

        Ok(Pane { page: *self, raw })
    }

    pub fn on_focus(&self, f: impl FnMut(Event) -> Result + 'static) -> Result<Page> {
        self.on_event(ControlEventKind::FOCUS, f)
    }

    pub fn on_decide(&self, f: impl FnMut(Event) -> Result + 'static) -> Result<Page> {
        self.on_event(ControlEventKind::DECIDE, f)
    }

    fn on_event(
        &self,
        kind: ControlEventKind,
        mut f: impl FnMut(Event) -> Result + 'static,
    ) -> Result<Page> {
        let page = *self;
        let cb = callback::raw(self.ui, move |arg| {
            match unsafe { arg.cast::<ControlEvent>().as_ref() } {
                Some(event) => f(Event::new(page, event)),
                None => Ok(()),
            }
        });

        check(unsafe { (self.ui.table.on_control_event)(self.handle, kind, cb) })?;

        Ok(*self)
    }

    /// The scroll list at `path`; lists a layout mod added are bound by the framework while the page builds
    pub fn list(&self, path: impl ListRef) -> Result<List> {
        let mut raw = core::ptr::null_mut();

        check(unsafe { (self.ui.table.list_get)(self.handle, Str::new(path.path()), &mut raw) })?;

        Ok(List::new(*self, raw))
    }

    /// The button the page's player has focused, `None` before the page's seq is ready
    pub fn focused(&self) -> Result<Option<Pane>> {
        let Some(seq) = self.seq()? else {
            return Ok(None);
        };

        let mut raw = core::ptr::null_mut();

        check(unsafe { (self.ui.table.page_focused)(self.handle, seq.player()?, &mut raw) })?;

        Ok((!raw.is_null()).then(|| Pane::new(*self, raw)))
    }

    /// The seq that built this page, `None` before it is ready
    pub fn seq(&self) -> Result<Option<Seq>> {
        let mut raw = core::ptr::null_mut();

        check(unsafe { (self.ui.table.page_seq)(self.handle, &mut raw) })?;

        Ok((!raw.is_null()).then(|| Seq::new(*self, raw)))
    }

    /// Callbacks around one state of the page's seq, see [`StateHooks`]
    pub fn on_state(&self, state: SeqState) -> StateHooks {
        StateHooks { page: *self, state }
    }

    /// Binds `key` on this page, see [`KeyBuilder`]
    pub fn bind_key(&self, key: Key) -> KeyBuilder {
        KeyBuilder::new(*self, key)
    }

    /// After the page's seq has set up its controls
    pub fn on_ready(&self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Page> {
        self.on_seq(SeqEvent::READY, f)
    }

    pub fn on_activate(&self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Page> {
        self.on_seq(SeqEvent::ACTIVATE, f)
    }

    pub fn on_deactivate(&self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Page> {
        self.on_seq(SeqEvent::DEACTIVATE, f)
    }

    /// Every frame the page's seq updates
    pub fn on_update(&self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Page> {
        self.on_seq(SeqEvent::UPDATE, f)
    }

    /// Drops the game's own key-guide entries for `key` while this page is up, e.g. to replace them
    /// with a binding's guide
    pub fn set_guide_hidden(&self, key: Key, hidden: bool) -> Result {
        check(unsafe { (self.ui.table.page_hide_guide)(self.handle, key.id(), hidden) })
    }

    /// A disabled page fires none of its input, the game's handlers and bindings alike, e.g. while
    /// another menu is open over it
    pub fn set_input_enabled(&self, enable: bool) -> Result {
        check(unsafe { (self.ui.table.page_set_input_enabled)(self.handle, enable) })
    }

    fn on_seq(&self, kind: SeqEvent, mut f: impl FnMut(Seq) -> Result + 'static) -> Result<Page> {
        let page = *self;
        let cb = callback::raw(self.ui, move |arg| f(Seq::new(page, arg.cast_mut())));

        check(unsafe { (self.ui.table.on_seq_event)(self.handle, kind, cb) })?;

        Ok(*self)
    }

    pub(crate) fn ui(&self) -> Ui {
        self.ui
    }
}

/// A state of a seq's state machine. The numbers are each seq class's own, e.g. from its
/// `initStateMachine`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SeqState(pub i32);

/// Runs callbacks before or after the game's own code for one state of a page's seq. Exec runs
/// every step the state is current, [`Seq::state_frame`] is 0 on its first. Dropped with the page
#[derive(Debug, Clone, Copy)]
pub struct StateHooks {
    page: Page,
    state: SeqState,
}

impl StateHooks {
    pub fn before_enter(self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Self> {
        self.on(StatePhase::ENTER, false, f)
    }

    pub fn after_enter(self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Self> {
        self.on(StatePhase::ENTER, true, f)
    }

    pub fn before_exec(self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Self> {
        self.on(StatePhase::EXEC, false, f)
    }

    pub fn after_exec(self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Self> {
        self.on(StatePhase::EXEC, true, f)
    }

    pub fn before_exit(self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Self> {
        self.on(StatePhase::EXIT, false, f)
    }

    pub fn after_exit(self, f: impl FnMut(Seq) -> Result + 'static) -> Result<Self> {
        self.on(StatePhase::EXIT, true, f)
    }

    fn on(
        self,
        phase: StatePhase,
        after: bool,
        mut f: impl FnMut(Seq) -> Result + 'static,
    ) -> Result<Self> {
        let page = self.page;
        let ui = page.ui();
        let cb = callback::raw(ui, move |arg| f(Seq::new(page, arg.cast_mut())));

        check(unsafe { (ui.table.on_state)(page.handle(), self.state.0, phase, after, cb) })?;

        Ok(self)
    }
}

#[derive(Clone, Copy)]
pub struct Pane {
    page: Page,
    raw: *mut c_void,
}

/// The seq that built a page, valid while that page is
#[derive(Clone, Copy)]
pub struct Seq {
    page: Page,
    raw: *mut c_void,
}

impl Seq {
    pub(crate) fn new(page: Page, raw: *mut c_void) -> Self {
        Self { page, raw }
    }

    // Checked against its page, which the seq owns
    fn base(&self) -> Result<&SequenceBase> {
        self.page.check()?;

        unsafe { self.raw.cast::<SequenceBase>().as_ref() }.ok_or(crate::UiError::InvalidArg)
    }

    /// For reading the seq's own struct
    pub fn raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn page(&self) -> Page {
        self.page
    }

    /// `Stale` once its page is destroyed
    pub fn check(&self) -> Result {
        self.page.check()
    }

    /// The state machine's current state, -1 before the first
    pub fn state(&self) -> Result<SeqState> {
        Ok(SeqState(self.base()?.state_machine.current_state))
    }

    /// How many times the current state has run, 0 on its first exec
    pub fn state_frame(&self) -> Result<i32> {
        Ok(self.base()?.state_machine.frame)
    }

    /// The local player the seq is open for
    pub fn player(&self) -> Result<Player> {
        Ok(Player::new(self.base()?.player).unwrap_or_default())
    }

    /// `None` when the parent has no page the framework saw built
    pub fn parent(&self) -> Result<Option<Seq>> {
        let parent = self.base()?.base.parent;

        self.page.ui.seq_at(parent.cast())
    }

    /// A state request is queued for the state machine's next step
    pub fn has_request(&self) -> Result<bool> {
        Ok(self.base()?.has_request())
    }

    /// Settled with nothing pending: no open request, no queued state, and the seq's own idle
    /// check, e.g. the pocket menu once it has closed
    pub fn is_idle(&self) -> Result<bool> {
        let seq = self.base()?;

        Ok(!seq.base.pending_focus_restore && unsafe { seq.base.is_idle() })
    }

    /// Queues a state for the seq's state machine the way the game does, `false` when a request is
    /// already queued
    pub fn request_state(&self, state: SeqState) -> Result<bool> {
        self.page.check()?;

        let seq = unsafe { self.raw.cast::<SequenceBase>().as_mut() };

        Ok(seq.is_some_and(|seq| seq.request_state(state.0)))
    }
}

impl PartialEq for Pane {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.page == other.page
    }
}

impl Pane {
    pub fn raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn page(&self) -> Page {
        self.page
    }

    pub(crate) fn handle(&self) -> PaneHandle {
        PaneHandle {
            page: self.page.handle,
            raw: self.raw,
        }
    }

    /// Both panes must be buttons
    pub fn link(&self, direction: Direction, to: Pane) -> Result {
        check(unsafe { (self.page.ui.table.link)(self.handle(), direction.into(), to.handle()) })
    }

    pub fn unlink(&self, direction: Direction) -> Result {
        let none = PaneHandle {
            page: self.page.handle,
            raw: core::ptr::null_mut(),
        };

        check(unsafe { (self.page.ui.table.link)(self.handle(), direction.into(), none) })
    }

    pub fn set_focusable(&self, player: Player, enable: bool) -> Result {
        check(unsafe { (self.page.ui.table.set_focusable)(self.handle(), player, enable) })
    }

    pub(crate) fn new(page: Page, raw: *mut c_void) -> Self {
        Self { page, raw }
    }

    /// Keeps the link pointed at `target` of the list's visible window as it scrolls
    pub fn link_into(&self, direction: Direction, list: &List, target: ListTarget) -> Result {
        check(unsafe {
            (self.page.ui.table.list_link_into)(
                self.handle(),
                direction.into(),
                list.raw(),
                target.into(),
            )
        })
    }

    pub fn set_visible(&self, visible: bool) -> Result {
        check(unsafe { (self.page.ui.table.set_visible)(self.handle(), visible) })
    }

    /// Presses the button as if decided: its Decide animation plays and the page's decide event
    /// fires, so the game reacts as to A. `false` when it was already deciding
    pub fn decide(&self) -> Result<bool> {
        let mut decided = false;

        check(unsafe { (self.page.ui.table.pane_decide)(self.handle(), &mut decided) })?;

        Ok(decided)
    }

    /// The button's valid look, the one the game greys out when an entry can't be used
    pub fn set_valid(&self, valid: bool) -> Result {
        check(unsafe { (self.page.ui.table.pane_set_valid)(self.handle(), valid) })
    }

    pub fn is_valid(&self) -> Result<bool> {
        let mut valid = false;

        check(unsafe { (self.page.ui.table.pane_is_valid)(self.handle(), &mut valid) })?;

        Ok(valid)
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

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub page: Page,
    pub control: Option<Pane>,
}

impl Event {
    pub(crate) fn new(page: Page, event: &ControlEvent) -> Self {
        let control = (!event.control.is_null()).then_some(Pane {
            page,
            raw: event.control,
        });

        Self { page, control }
    }
}

impl PartialEq for Page {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl Eq for Page {}

impl core::hash::Hash for Page {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl core::fmt::Debug for Page {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Page")
            .field("raw", &self.handle.raw)
            .field("build", &self.handle.build)
            .finish()
    }
}

impl PartialEq for Seq {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.page == other.page
    }
}

impl Eq for Seq {}

impl core::hash::Hash for Seq {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
        self.page.hash(state);
    }
}

impl core::fmt::Debug for Seq {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Seq")
            .field("raw", &self.raw)
            .field("page", &self.page)
            .finish()
    }
}

impl Eq for Pane {}

impl core::hash::Hash for Pane {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
        self.page.hash(state);
    }
}

impl core::fmt::Debug for Pane {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Pane")
            .field("raw", &self.raw)
            .field("page", &self.page)
            .finish()
    }
}
