use acnh_ui_api::{PageFilter, Result, connect};

fn start() -> Result {
    let ui = connect("acnh-ui-smoke")?;
    println!("[acnh-ui-smoke] connected");

    ui.on_page(PageFilter::Layout("CharaMake"), |page| {
        println!("[acnh-ui-smoke] CharaMake built {:p}", page.raw());

        let ok = page.pane("/L_BtnOk_00")?;
        println!("[acnh-ui-smoke] /L_BtnOk_00 {:p}", ok.raw());

        page.on_focus(|e| {
            println!("[acnh-ui-smoke] focus {:?}", e.control.map(|c| c.raw()));
            Ok(())
        })?;

        let mut visible = true;

        page.on_decide(move |e| {
            println!("[acnh-ui-smoke] decide {:?}", e.control.map(|c| c.raw()));

            if e.control != Some(ok) {
                visible = !visible;
                ok.set_visible(visible)?;
                println!("[acnh-ui-smoke] OK visible: {visible}");
            }

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
