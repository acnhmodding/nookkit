use core::ffi::c_void;

use nookkit_sys::{KeyEvent as RawKeyEvent, KeyEventKind, KeyFlags, Player, Str};

use crate::{Page, PaneRef, Result, UiError, callback, check};

/// Discriminants are the game's ButtonId
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    A = 4,
    B = 5,
    X = 6,
    Y = 7,
    L = 8,
    R = 9,
    Zl = 10,
    Zr = 11,
    LeftStick = 12,
    RightStick = 13,
    Plus = 14,
    Minus = 15,
}

impl Key {
    pub(crate) fn id(self) -> i32 {
        self as i32
    }

    fn from_id(id: i32) -> Option<Self> {
        Some(match id {
            0 => Key::Up,
            1 => Key::Down,
            2 => Key::Left,
            3 => Key::Right,
            4 => Key::A,
            5 => Key::B,
            6 => Key::X,
            7 => Key::Y,
            8 => Key::L,
            9 => Key::R,
            10 => Key::Zl,
            11 => Key::Zr,
            12 => Key::LeftStick,
            13 => Key::RightStick,
            14 => Key::Plus,
            15 => Key::Minus,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

        let configured = match &self.guide {
            Some(Guide::Text(text)) => binding.set_guide(text),
            Some(Guide::Label(label)) => binding.set_guide_label(label),
            None => Ok(()),
        }
        .and_then(|()| match &self.hint_pane {
            Some(path) => check(unsafe { (ui.table.key_set_hint_pane)(raw, Str::new(path)) }),
            None => Ok(()),
        });

        // Bound but half configured, so it stays off rather than firing without its guide
        if let Err(e) = configured {
            binding.set_enabled(false)?;
            return Err(e);
        }

        Ok(binding)
    }
}

/// Lives as long as its page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
