use skyline::hooks::{Region, getRegionAddress};

pub fn text() -> *mut u8 {
    unsafe { getRegionAddress(Region::Text) as *mut u8 }
}

pub unsafe fn got<T>(offset: usize) -> *mut T {
    unsafe { *(text().add(offset) as *const *mut T) }
}

/// A vtable through a GOT entry pointing at its typeinfo header (offset-to-top, typeinfo).
pub unsafe fn got_vtable<T>(offset: usize) -> *const T {
    unsafe { got::<u8>(offset).add(0x10) as *const T }
}

pub fn text_vtable<T>(offset: usize) -> *const T {
    unsafe { text().add(offset) as *const T }
}

/// A guarded function-local static `sead::RuntimeTypeInfo`. The game initializes it the first time
/// its own code runs the check, so `None` means it hasn't yet.
pub unsafe fn guarded_type_info(info: usize, guard: usize) -> Option<*const core::ffi::c_void> {
    unsafe {
        if *got::<u8>(guard) & 1 == 0 {
            None
        } else {
            Some(got::<core::ffi::c_void>(info))
        }
    }
}

pub type NativeResult<T = ()> = Result<T, native_vtable::Unresolved>;

const SYMBOLS: &[(&str, usize)] = &[
    ("Lp::UI::Control::findByPath", 0x376e6b0),
    ("Lp::UI::Control::setFocusable", 0x3c5d0e0),
    ("Lp::Utl::StateMachine::changeState", 0x3dbb160),
    ("CharaMake_findPlayerIcon", 0x2baaf20),
    ("PlayerIcon_resetPart", 0x2fc3694),
    ("PlayerIcon_render", 0x2fc37ac),
    ("PlayerIcon_moveFrom", 0x2fc3ff0),
    ("PlayerIcon_release", 0x2fc4398),
    ("PlayerIcon_tick", 0x2fc47d8),
    ("game::UI::CharaMakeSeq::refreshListGroup", 0x2ac30b0),
    ("ListItemUpdater::update", 0x335c380),
    ("ScrollBtnListCtrl::bindButtons", 0x2ab1110),
    ("ScrollBtnListCtrl::attach", 0x3763fe0),
    ("ScrollBtnListCtrl::setContent", 0x3764290),
    ("ScrollBtnListCtrl::enable", 0x3764fe0),
    ("ScrollBtnListCtrl::disable", 0x335b550),
    ("ScrollBtnListCtrl::itemIndexOf", 0x3765200),
];

pub fn resolve(symbol: &'static str) -> Option<usize> {
    SYMBOLS
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|&(_, offset)| text() as usize + offset)
}

/// Hook and callback boundaries return to the game, so a failed native call is logged there.
pub fn report(context: &str, result: NativeResult) {
    if let Err(e) = result {
        println!("[acnh-mouth] {context}: {e}");
    }
}
