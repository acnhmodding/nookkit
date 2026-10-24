use acnh_ui_sys::{API_NAME, API_VERSION, AcnhUiV1, sl_api_publish};

#[unsafe(no_mangle)]
#[used]
pub static __sl_framework: u8 = 0;

static TABLE: AcnhUiV1 = AcnhUiV1 {
    size: core::mem::size_of::<AcnhUiV1>(),
    framework_version: 1,
};

#[skyline::main(name = "acnh-ui")]
pub fn main() {
    println!("[acnh-ui] main, publishing api v{API_VERSION}");

    match acnh_sym::init("") {
        Ok(table) => println!(
            "[acnh-ui] {} symbols for {}",
            table.len(),
            table.game_version
        ),
        Err(e) => println!("[acnh-ui] {e}"),
    }

    unsafe {
        sl_api_publish(
            API_NAME.as_ptr(),
            API_VERSION,
            (&TABLE as *const AcnhUiV1).cast(),
        )
    };
}
