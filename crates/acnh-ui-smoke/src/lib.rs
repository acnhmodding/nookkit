use acnh_ui_api::{Page, PageFilter, Result, connect};

fn log_seq(page: Page, name: &'static str) -> Result {
    page.on_ready(move |seq| {
        println!("[acnh-ui-smoke] {name} ready, seq {:p}", seq.raw());
        Ok(())
    })?;

    page.on_activate(move || {
        println!("[acnh-ui-smoke] {name} activate");
        Ok(())
    })?;

    page.on_deactivated(move || {
        println!("[acnh-ui-smoke] {name} deactivated");
        Ok(())
    })?;

    let mut frames = 0u32;

    page.on_update(move || {
        frames += 1;

        if frames % 300 == 1 {
            println!("[acnh-ui-smoke] {name} update #{frames}");
        }

        Ok(())
    })
}

fn start() -> Result {
    let ui = connect("acnh-ui-smoke")?;
    println!("[acnh-ui-smoke] connected");

    ui.on_page(PageFilter::Layout("MenuDevice"), |page| log_seq(page, "MenuDevice"))?;

    ui.on_page(PageFilter::Layout("CharaMake"), |page| {
        println!("[acnh-ui-smoke] CharaMake built {:p}", page.raw());
        log_seq(page, "CharaMake")?;

        let ok = page.pane("/L_BtnOk_00")?;
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
