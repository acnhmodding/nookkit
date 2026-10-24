pub use acnh_game::native::*;

pub fn report(context: &str, result: NativeResult) {
    if let Err(e) = result {
        println!("[acnh-mouth] {context}: {e}");
    }
}
