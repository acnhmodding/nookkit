mod game;
mod mouths;
mod native;

use acnh_game::sead;
use game::ui::chara_make::*;
use native::report;
use sead::ObjArray;

acnh_sym::hook! {
    seq_fill_mouth_list = "game::UI::CharaMakeSeq::fillMouthList";
    fn(values: *mut ObjArray<u32>) {
        report("fill", unsafe { mouths::fill(values, || original(values)) });
    }
}

acnh_sym::hook! {
    seq_refresh_enter = "game::UI::CharaMakeSeq::refreshEnter";
    fn(seq: *mut CharaMakeSeq) {
        unsafe { original(seq) };

        ui_report("refresh", unsafe { mouths::refresh(seq) });
    }
}

acnh_sym::hook! {
    seq_process_request = "game::UI::CharaMakeSeq::processRequest";
    fn(seq: *mut CharaMakeSeq) {
        let anchor = unsafe { (*seq).request.is(RequestState::UpdateAnchor) };

        unsafe { original(seq) };

        if anchor {
            ui_report("relink", unsafe { mouths::after_request(seq) });
        }
    }
}

acnh_sym::hook! {
    seq_set_group_enabled = "game::UI::CharaMakeSeq::setGroupEnabled";
    fn(seq: *mut CharaMakeSeq, group: u32, enable: u32, mode: u32) {
        if group != GROUP_MOUTH || unsafe { !mouths::is_active(seq) } {
            unsafe { original(seq, group, enable, mode) };
            return;
        }

        // Disables the old mouth buttons
        unsafe { original(seq, group, 0, mode) };

        ui_report("set_enabled", unsafe { mouths::set_enabled(seq, enable & 1 != 0, mode) });
    }
}

acnh_sym::hook! {
    seq_enter_idle = "game::UI::CharaMakeSeq::enterIdle";
    fn(seq: *mut CharaMakeSeq) {
        report("exit", unsafe { mouths::enter_idle(seq) });

        unsafe { original(seq) };
    }
}

fn ui_report(context: &str, result: acnh_ui_api::Result) {
    if let Err(e) = result {
        println!("[acnh-mouth] {context}: {e}");
    }
}

fn install() -> Result<(), acnh_sym::SymError> {
    acnh_sym::init(include_str!("../../../offsets/3.0.3.toml"))?;

    seq_fill_mouth_list::install()?;
    seq_refresh_enter::install()?;
    seq_process_request::install()?;
    seq_set_group_enabled::install()?;
    seq_enter_idle::install()
}

#[skyline::main(name = "acnh-mouth")]
pub fn main() {
    if let Err(e) = install() {
        println!("[acnh-mouth] not installed: {e}");
        return;
    }

    if let Err(e) = acnh_ui_api::connect("acnh-mouth").and_then(|ui| mouths::register(&ui)) {
        println!("[acnh-mouth] acnh-ui: {e}");
    }
}
