mod game;
mod mouths;
mod native;
mod offsets;

use acnh_ui_api::game::sead;
use game::ui::chara_make::*;
use native::report;
use sead::ObjArray;

acnh_ui_api::hook! {
    resolve = crate::offsets::resolve;
    seq_fill_mouth_list = "fill_mouth_list";
    fn(values: *mut ObjArray<u32>) {
        report("fill", unsafe { mouths::fill(values, || original(values)) });
    }
}

acnh_ui_api::hook! {
    resolve = crate::offsets::resolve;
    seq_refresh_enter = "refresh_enter";
    fn(seq: *mut CharaMakeSeq) {
        unsafe { original(seq) };

        ui_report("refresh", unsafe { mouths::refresh(seq) });
    }
}

acnh_ui_api::hook! {
    resolve = crate::offsets::resolve;
    seq_process_request = "process_request";
    fn(seq: *mut CharaMakeSeq) {
        let anchor = unsafe { (*seq).request.is(RequestState::UpdateAnchor) };

        unsafe { original(seq) };

        if anchor {
            ui_report("relink", unsafe { mouths::after_request(seq) });
        }
    }
}

acnh_ui_api::hook! {
    resolve = crate::offsets::resolve;
    seq_set_group_enabled = "set_group_enabled";
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

acnh_ui_api::hook! {
    resolve = crate::offsets::resolve;
    seq_enter_idle = "enter_idle";
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

fn install() -> acnh_ui_api::Result {
    let ui = acnh_ui_api::connect("acnh-mouth")?;

    seq_fill_mouth_list::install()?;
    seq_refresh_enter::install()?;
    seq_process_request::install()?;
    seq_set_group_enabled::install()?;
    seq_enter_idle::install()?;

    mouths::register(&ui)
}

#[skyline::main(name = "acnh-mouth")]
pub fn main() {
    if let Err(e) = install() {
        println!("[acnh-mouth] not installed: {e}");
    }
}
