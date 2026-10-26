use crate::native::{NativeResult, sym};

#[repr(C)]
pub struct PocketOpenOptions {
    /// 0 for the normal pockets
    pub mode: u32,
    pub close_enabled: bool,
}

const _: () = assert!(core::mem::size_of::<PocketOpenOptions>() == 8);

crate::native! {
    unsafe fn request_open(player: *const u8, options: *const PocketOpenOptions) = sym::game::UI::PocketMenuSeq::requestOpen;
}

/// Opens the resident pocket menu (UIRoot+0x88) for `player` in its normal mode, over whatever is open
pub unsafe fn open_pocket_menu(player: u8, close_enabled: bool) -> NativeResult<()> {
    let options = PocketOpenOptions {
        mode: 0,
        close_enabled,
    };

    unsafe { request_open(&player, &options) }
}
