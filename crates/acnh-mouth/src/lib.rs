mod game;
mod lp;
mod mouth_list;
mod native;
mod sead;

use core::ffi::c_void;

use game::ui::chara_make::*;
use lp::ui::Control;
use native::report;
use sead::ObjArray;

acnh_sym::hook! {
    page_bind_lists = "game::UI::CharaMakePage::bindLists";
    fn(page: *mut CharaMakePage, parent: *mut Control, builder: *mut c_void) {
        unsafe { original(page, parent, builder) };

        report("bind", unsafe { mouth_list::bind(page, parent, builder) });
    }
}

acnh_sym::hook! {
    seq_setup_controls = "game::UI::CharaMakeSeq::setupControls";
    fn(seq: *mut CharaMakeSeq) {
        unsafe { original(seq) };

        report("setup", unsafe { mouth_list::setup(seq) });
    }
}

acnh_sym::hook! {
    seq_on_control_focus = "game::UI::CharaMakeSeq::onControlFocus";
    fn(seq: *mut CharaMakeSeq, event: *mut ControlEvent) {
        unsafe { original(seq, event) };

        report("focus", unsafe { mouth_list::on_focus(seq, (*event).control) });
    }
}

acnh_sym::hook! {
    seq_on_control_decide = "game::UI::CharaMakeSeq::onControlDecide";
    fn(seq: *mut CharaMakeSeq, event: *mut ControlEvent) {
        unsafe { original(seq, event) };

        report("decide", unsafe { mouth_list::on_decide(seq, (*event).control) });
    }
}

acnh_sym::hook! {
    seq_tick_icons = "game::UI::CharaMakeSeq::tickIcons";
    fn(seq: *mut CharaMakeSeq) {
        unsafe { original(seq) };

        report("tick", unsafe { mouth_list::tick(seq) });
    }
}

acnh_sym::hook! {
    seq_on_exit = "game::UI::CharaMakeSeq::onExit";
    fn(seq: *mut CharaMakeSeq) {
        report("exit", unsafe { mouth_list::exit(seq) });

        unsafe { original(seq) };
    }
}

acnh_sym::hook! {
    seq_process_request = "game::UI::CharaMakeSeq::processRequest";
    fn(seq: *mut CharaMakeSeq) {
        let anchor = unsafe { (*seq).request.is(RequestState::UpdateAnchor) };

        unsafe { original(seq) };

        if anchor {
            unsafe { mouth_list::after_request(seq) };
        }
    }
}

acnh_sym::hook! {
    seq_set_group_enabled = "game::UI::CharaMakeSeq::setGroupEnabled";
    fn(seq: *mut CharaMakeSeq, group: u32, enable: u32, mode: u32) {
        if group != GROUP_MOUTH || unsafe { !mouth_list::is_active(seq) } {
            unsafe { original(seq, group, enable, mode) };
            return;
        }

        // Disables the old mouth buttons
        unsafe { original(seq, group, 0, mode) };

        report("set_enabled", unsafe { mouth_list::set_enabled(seq, enable & 1 != 0, mode) });
    }
}

acnh_sym::hook! {
    seq_fill_mouth_list = "game::UI::CharaMakeSeq::fillMouthList";
    fn(values: *mut ObjArray<u32>) {
        report("fill", unsafe { mouth_list::fill(values, || original(values)) });
    }
}

acnh_sym::hook! {
    seq_refresh_enter = "game::UI::CharaMakeSeq::refreshEnter";
    fn(seq: *mut CharaMakeSeq) {
        unsafe { original(seq) };

        report("refresh", unsafe { mouth_list::refresh(seq) });
    }
}

fn install() -> Result<(), acnh_sym::SymError> {
    acnh_sym::init(include_str!("../../../offsets/3.0.3.toml"))?;

    page_bind_lists::install()?;
    seq_setup_controls::install()?;
    seq_on_control_focus::install()?;
    seq_on_control_decide::install()?;
    seq_tick_icons::install()?;
    seq_on_exit::install()?;
    seq_fill_mouth_list::install()?;
    seq_process_request::install()?;
    seq_set_group_enabled::install()?;
    seq_refresh_enter::install()
}

#[skyline::main(name = "acnh-mouth")]
pub fn main() {
    if let Err(e) = install() {
        println!("[acnh-mouth] not installed: {e}");
    }
}
