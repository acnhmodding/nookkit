use acnh_ui_api::{PageFilter, Result, connect};

fn start() -> Result {
    let ui = connect("acnh-ui-smoke")?;
    println!("[acnh-ui-smoke] connected");

    ui.on_page(PageFilter::Layout("CharaMake"), |page| {
        println!("[acnh-ui-smoke] CharaMake built {:p}", page.raw());

        match page.pane("L_BtnOk") {
            Ok(pane) => println!("[acnh-ui-smoke] L_BtnOk {:p}", pane.raw()),
            Err(e) => println!("[acnh-ui-smoke] L_BtnOk: {e}"),
        }

        page.on_focus(|e| {
            println!("[acnh-ui-smoke] focus {:?}", e.control.map(|c| c.raw()));
            Ok(())
        })?;

        page.on_decide(|e| {
            println!("[acnh-ui-smoke] decide {:?}", e.control.map(|c| c.raw()));
            Ok(())
        })
    })
}

#[skyline::main(name = "acnh-ui-smoke")]
pub fn main() {
    if let Err(e) = start() {
        println!("[acnh-ui-smoke] {e}");
    }
}
