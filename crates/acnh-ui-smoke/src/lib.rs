use acnh_ui_sys::{API_NAME, API_VERSION, AcnhUiV1, sl_api_get};

#[skyline::main(name = "acnh-ui-smoke")]
pub fn main() {
    let table = unsafe {
        sl_api_get(API_NAME.as_ptr(), API_VERSION)
            .cast::<AcnhUiV1>()
            .as_ref()
    };

    match table {
        Some(t) => println!(
            "[acnh-ui-smoke] got api: size {} version {}",
            t.size, t.framework_version
        ),
        None => println!("[acnh-ui-smoke] acnh-ui api not found"),
    }
}
