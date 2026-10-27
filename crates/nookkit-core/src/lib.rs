mod keys;
mod lists;
mod pages;
mod seqs;
mod states;
mod table;

use core::sync::atomic::{AtomicBool, Ordering};

use nookkit_sys::{API_NAME, API_VERSION, NookKitV2, sl_api_publish};
use nookkit_types::native::{NativeResult, SymbolKind};

#[unsafe(no_mangle)]
#[used]
pub static __sl_framework: u8 = 0;

static STARTED: AtomicBool = AtomicBool::new(false);

fn nav(direction: nookkit_sys::Direction) -> Option<nookkit_types::lp::ui::NavDirection> {
    use nookkit_sys::Direction;
    use nookkit_types::lp::ui::NavDirection;

    Some(match direction {
        Direction::UP => NavDirection::Up,
        Direction::DOWN => NavDirection::Down,
        Direction::LEFT => NavDirection::Left,
        Direction::RIGHT => NavDirection::Right,
        _ => return None,
    })
}

fn report(context: &str, result: NativeResult) {
    if let Err(e) = result {
        println!("[NookKit] {context}: {e}");
    }
}

fn start() -> Result<(), nookkit_sym::SymError> {
    let symbols = nookkit_sym::init(include_str!("../../../offsets/3.0.3.toml"))?;
    println!(
        "[NookKit] {} symbols for {}",
        symbols.len(),
        symbols.game_version
    );

    nookkit_types::native::set_resolver(|name, kind| {
        let kind = match kind {
            SymbolKind::Fn => nookkit_sym::Kind::Fn,
            SymbolKind::Vtable => nookkit_sym::Kind::Vtable,
        };

        nookkit_sym::address(kind, name).ok()
    });

    pages::view_page_initialize::install()?;
    pages::view_page_destroy::install()?;
    pages::view_page_layout_built::install()?;
    seqs::sequence_create::install()?;
    seqs::sequence_on_activate::install()?;
    seqs::sequence_on_deactivated::install()?;
    seqs::sequence_update::install()?;
    keys::key_guide_add::install()?;
    keys::message_find::install()
}

#[skyline::main(name = "NookKit")]
pub fn main() {
    match start() {
        Ok(()) => STARTED.store(true, Ordering::Release),
        Err(e) => println!("[NookKit] not started: {e}"),
    }

    // Published even when not started, so clients get UnsupportedBuild instead of a missing framework
    unsafe {
        sl_api_publish(
            API_NAME.as_ptr(),
            API_VERSION,
            (&table::TABLE as *const NookKitV2).cast(),
        )
    };
}
