use core::ffi::c_void;

use nookkit_sys::{KeyEvent as RawKeyEvent, KeyEventKind, KeyFlags, Player, Str};

use crate::{Page, PaneRef, Result, UiError, callback, check};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    L,
    R,
    Zl,
    Zr,
    LeftStick,
    RightStick,
    Plus,
    Minus,
}

impl Key {
    const ALL: [Key; 16] = [
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
        Key::A,
        Key::B,
        Key::X,
        Key::Y,
        Key::L,
        Key::R,
        Key::Zl,
        Key::Zr,
        Key::LeftStick,
        Key::RightStick,
        Key::Plus,
        Key::Minus,
    ];

    pub(crate) fn id(self) -> i32 {
        self as i32
    }

    fn from_id(id: i32) -> Option<Self> {
        Self::ALL.get(usize::try_from(id).ok()?).copied()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub player: Player,
}

enum Guide {
    Text(String),
    Label(String),
}

#[must_use = "a key is only bound by `build`"]
pub struct KeyBuilder {
    page: Page,
    key: Key,
    player: Option<Player>,
    consume: bool,
    guide: Option<Guide>,
    hint_pane: Option<String>,
}

impl KeyBuilder {
    pub(crate) fn new(page: Page, key: Key) -> Self {
        Self {
            page,
            key,
            player: None,
            consume: false,
            guide: None,
            hint_pane: None,
        }
    }

    /// Pins the binding to one player. Without it the binding follows the player the page's seq
    /// is opened for, each time it activates
    pub fn player(self, player: Player) -> Self {
        Self {
            player: Some(player),
            ..self
        }
    }

    /// Takes the key from the page's own bindings, e.g. CharaMake's B and X. D-pad navigation and
    /// A on buttons are handled before any key binding, so they can't be taken
    pub fn consume(self) -> Self {
        Self {
            consume: true,
            ..self
        }
    }

    /// Literal text in the key-guide bar, shown while the page's seq is active. The bar orders
    /// its entries by button
    pub fn guide(self, text: &str) -> Self {
        Self {
            guide: Some(Guide::Text(text.to_owned())),
            ..self
        }
    }

    /// A label from LayoutMsg/KeyGuide.msbt instead of literal text, e.g. "0102" for Back
    pub fn guide_label(self, label: &str) -> Self {
        Self {
            guide: Some(Guide::Label(label.to_owned())),
            ..self
        }
    }

    /// A pane in the page's layout shown while the binding is enabled
    pub fn hint_pane(self, path: impl PaneRef) -> Self {
        Self {
            hint_pane: Some(path.path().to_owned()),
            ..self
        }
    }

    pub fn build(self) -> Result<KeyBinding> {
        let ui = self.page.ui();
        let mut flags = if self.consume {
            KeyFlags::CONSUME
        } else {
            KeyFlags::NONE
        };
        if self.player.is_none() {
            flags = flags | KeyFlags::SEQ_PLAYER;
        }
        let mut raw = core::ptr::null_mut();

        check(unsafe {
            (ui.table.key_bind)(
                self.page.handle(),
                self.player.unwrap_or_default(),
                self.key.id(),
                flags,
                &mut raw,
            )
        })?;

        let binding = KeyBinding {
            page: self.page,
            raw,
        };

        match &self.guide {
            Some(Guide::Text(text)) => binding.set_guide(text)?,
            Some(Guide::Label(label)) => binding.set_guide_label(label)?,
            None => {}
        }

        if let Some(path) = &self.hint_pane {
            check(unsafe { (ui.table.key_set_hint_pane)(raw, Str::new(path)) })?;
        }

        Ok(binding)
    }
}

/// Lives as long as its page
#[derive(Clone, Copy)]
pub struct KeyBinding {
    page: Page,
    raw: *mut c_void,
}

impl KeyBinding {
    pub fn on_trigger(&self, f: impl FnMut(KeyEvent) -> Result + 'static) -> Result<KeyBinding> {
        self.on(KeyEventKind::TRIGGER, f)
    }

    pub fn on_release(&self, f: impl FnMut(KeyEvent) -> Result + 'static) -> Result<KeyBinding> {
        self.on(KeyEventKind::RELEASE, f)
    }

    /// Taken by an earlier handler, or the binding was disabled, while held
    pub fn on_cancel(&self, f: impl FnMut(KeyEvent) -> Result + 'static) -> Result<KeyBinding> {
        self.on(KeyEventKind::CANCEL, f)
    }

    /// At press, after 30 frames, then every 5
    pub fn on_repeat(&self, f: impl FnMut(KeyEvent) -> Result + 'static) -> Result<KeyBinding> {
        self.on(KeyEventKind::REPEAT, f)
    }

    /// Every held frame
    pub fn on_hold(&self, f: impl FnMut(KeyEvent) -> Result + 'static) -> Result<KeyBinding> {
        self.on(KeyEventKind::HOLD, f)
    }

    fn on(
        &self,
        kind: KeyEventKind,
        mut f: impl FnMut(KeyEvent) -> Result + 'static,
    ) -> Result<KeyBinding> {
        let ui = self.page.ui();
        let cb = callback::raw(ui, move |arg| {
            let Some(event) = (unsafe { arg.cast::<RawKeyEvent>().as_ref() }) else {
                return Ok(());
            };

            let key = Key::from_id(event.key).ok_or(UiError::InvalidArg)?;

            f(KeyEvent {
                key,
                player: event.player,
            })
        });

        check(unsafe { (ui.table.key_on)(self.raw, kind, cb) })?;

        Ok(*self)
    }

    /// A disabled binding gets no events and leaves the key-guide bar at its next refresh
    pub fn set_enabled(&self, enable: bool) -> Result {
        check(unsafe { (self.page.ui().table.key_set_enabled)(self.raw, enable) })
    }

    pub fn set_guide(&self, text: &str) -> Result {
        self.guide(text, "")
    }

    pub fn set_guide_label(&self, label: &str) -> Result {
        self.guide("", label)
    }

    pub fn clear_guide(&self) -> Result {
        self.guide("", "")
    }

    fn guide(&self, text: &str, label: &str) -> Result {
        check(unsafe {
            (self.page.ui().table.key_set_guide)(self.raw, Str::new(text), Str::new(label))
        })
    }
}
