mod pages;
mod table;

use core::sync::atomic::{AtomicBool, Ordering};

use acnh_ui_sys::{API_NAME, API_VERSION, AcnhUiV1, sl_api_publish};

#[unsafe(no_mangle)]
#[used]
pub static __sl_framework: u8 = 0;

static STARTED: AtomicBool = AtomicBool::new(false);

fn start() -> Result<(), acnh_sym::SymError> {
    let symbols = acnh_sym::init(include_str!("../../../offsets/3.0.3.toml"))?;
    println!(
        "[acnh-ui] {} symbols for {}",
        symbols.len(),
        symbols.game_version
    );

    pages::view_page_initialize::install()
}

#[skyline::main(name = "acnh-ui")]
pub fn main() {
    match start() {
        Ok(()) => STARTED.store(true, Ordering::Release),
        Err(e) => println!("[acnh-ui] not started: {e}"),
    }

    // Published even when not started, so clients get UnsupportedBuild instead of a missing framework
    unsafe {
        sl_api_publish(
            API_NAME.as_ptr(),
            API_VERSION,
            (&table::TABLE as *const AcnhUiV1).cast(),
        )
    };
}
