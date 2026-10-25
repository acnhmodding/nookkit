mod lists;
mod pages;
mod seqs;
mod table;

use core::sync::atomic::{AtomicBool, Ordering};

use acnh_game::native::NativeResult;
use acnh_ui_sys::{API_NAME, API_VERSION, AcnhUiV1, sl_api_publish};

#[unsafe(no_mangle)]
#[used]
pub static __sl_framework: u8 = 0;

static STARTED: AtomicBool = AtomicBool::new(false);

fn nav(direction: acnh_ui_sys::Direction) -> acnh_game::lp::ui::NavDirection {
    use acnh_game::lp::ui::NavDirection;
    use acnh_ui_sys::Direction;

    match direction {
        Direction::Up => NavDirection::Up,
        Direction::Down => NavDirection::Down,
        Direction::Left => NavDirection::Left,
        Direction::Right => NavDirection::Right,
    }
}

fn report(context: &str, result: NativeResult) {
    if let Err(e) = result {
        println!("[acnh-ui] {context}: {e}");
    }
}

fn start() -> Result<(), acnh_sym::SymError> {
    let symbols = acnh_sym::init(include_str!("../../../offsets/3.0.3.toml"))?;
    println!(
        "[acnh-ui] {} symbols for {}",
        symbols.len(),
        symbols.game_version
    );

    pages::view_page_initialize::install()?;
    pages::view_page_layout_built::install()?;
    seqs::sequence_create::install()?;
    seqs::sequence_on_activate::install()?;
    seqs::sequence_on_deactivated::install()?;
    seqs::sequence_update::install()
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
