pub use acnh_ui_api::game::native::*;

pub fn report(context: &str, result: NativeResult) {
    if let Err(e) = result {
        println!("[acnh-mouth] {context}: {e}");
    }
}
