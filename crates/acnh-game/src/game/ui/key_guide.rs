use core::mem::offset_of;

use native_vtable::{native, vtable_impl};

use crate::game::ui::{AppLocatorCtrl, PageBase, SequenceBase, VftSequenceBase};
use crate::lp::ui::ButtonId;
use crate::native::{Class, NativeResult};
use crate::sead::{FixedSafeString, PtrArray};

pub const KEY_GUIDE_PARTS: usize = 5;

#[repr(C)]
pub struct KeyGuideEntry {
    pub key: i32,

    /// Takes precedence over `key` when set, -1 for none
    pub key_alt: i32,

    /// Shown after a slash
    pub key2: i32,

    pub key2_alt: i32,

    /// In LayoutMsg/KeyGuide.msbt
    pub label: FixedSafeString<64>,

    pub player: u8,
    _pad_69: [u8; 0x3],
    pub kind: i32,
    pub use_override_keys: bool,
    _pad_71: [u8; 0x3],
    pub override_key: i32,
    pub override_key_alt: i32,
    _unk_7c: [u8; 0x4],

    /// Set by `add_guide` from the key, the bar is sorted by it
    pub icon_index: i32,

    _unk_84: [u8; 0x4],
}

const _: () = {
    assert!(offset_of!(KeyGuideEntry, label) == 0x10);
    assert!(offset_of!(KeyGuideEntry, player) == 0x68);
    assert!(offset_of!(KeyGuideEntry, kind) == 0x6c);
    assert!(offset_of!(KeyGuideEntry, override_key) == 0x74);
    assert!(offset_of!(KeyGuideEntry, icon_index) == 0x80);
    assert!(core::mem::size_of::<KeyGuideEntry>() == 0x88);
};

impl KeyGuideEntry {
    pub fn new(key: ButtonId, label: &str, player: u8) -> NativeResult<Self> {
        Ok(Self {
            key: key as i32,
            key_alt: -1,
            key2: -1,
            key2_alt: -1,
            label: FixedSafeString::new(label)?,
            player,
            _pad_69: [0; 3],
            kind: 0,
            use_override_keys: false,
            _pad_71: [0; 3],
            override_key: -1,
            override_key_alt: -1,
            _unk_7c: [0; 4],
            icon_index: -1,
            _unk_84: [0; 4],
        })
    }
}

#[repr(C)]
pub struct KeyGuidePage {
    pub base: PageBase,
    pub parts: [*mut AppLocatorCtrl; KEY_GUIDE_PARTS],
}

const _: () = assert!(offset_of!(KeyGuidePage, parts) == 0x570);

crate::derive_base!(KeyGuidePage => PageBase);

pub const KEY_GUIDE_REQUEST_COMMIT: u8 = 1 << 0;
pub const KEY_GUIDE_REQUEST_BEGIN: u8 = 1 << 2;

#[repr(C)]
pub struct KeyGuideSeq {
    pub base: SequenceBase,
    pub page: *mut KeyGuidePage,
    pub pending: PtrArray<KeyGuideEntry>,
    pub free_entries: *mut KeyGuideEntry,
    _unk_230: [u8; 0x8],
    pub batch_player: u8,

    /// `KEY_GUIDE_REQUEST_*`, plus 0x10 / 0x20 set by `add_guide` when a batch starts
    pub request: u8,

    _unk_23a: [u8; 0x6],
    pub display_flags: u8,
    _unk_241: [u8; 0x7],

    /// The entries on screen, `display[i]` in part `i` once `fill_guides` sorted them
    pub display: PtrArray<KeyGuideEntry>,

    pub free_display: *mut KeyGuideEntry,
    _unk_260: [u8; 0x10],
}

const _: () = {
    assert!(offset_of!(KeyGuideSeq, page) == 0x210);
    assert!(offset_of!(KeyGuideSeq, pending) == 0x218);
    assert!(offset_of!(KeyGuideSeq, free_entries) == 0x228);
    assert!(offset_of!(KeyGuideSeq, batch_player) == 0x238);
    assert!(offset_of!(KeyGuideSeq, request) == 0x239);
    assert!(offset_of!(KeyGuideSeq, display_flags) == 0x240);
    assert!(offset_of!(KeyGuideSeq, display) == 0x248);
    assert!(core::mem::size_of::<KeyGuideSeq>() == 0x270);
};

crate::derive_base!(KeyGuideSeq => SequenceBase);

#[vtable_impl]
unsafe impl VftSequenceBase for KeyGuideSeq {}

unsafe impl Class for KeyGuideSeq {
    const RUNTIME_TYPE_INFO: &'static str = "game::UI::KeyGuideSeq::getRuntimeTypeInfo";
}

#[native(resolve = crate::native::resolve)]
impl KeyGuideSeq {
    #[native(symbol = "game::UI::KeyGuideSeq::addGuide")]
    unsafe fn add_guide_native(&mut self, entry: *const KeyGuideEntry, flag: bool);
}

impl KeyGuideSeq {
    /// Ignored while the bar takes no entries, or is full
    pub unsafe fn add_guide(&mut self, entry: &mut KeyGuideEntry) -> NativeResult {
        entry.label.pin();

        unsafe { self.add_guide_native(entry, true) }.map_err(Into::into)
    }

    /// The part showing display entry `index`
    pub unsafe fn part(&mut self, index: usize) -> Option<&mut AppLocatorCtrl> {
        unsafe { self.page.as_ref()?.parts.get(index)?.as_mut() }
    }
}
