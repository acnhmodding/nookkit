mod game;
mod lp;
mod mouth_list;
mod native;
mod sead;

use core::ffi::c_void;

use game::ui::chara_make::*;
use lp::ui::Control;
use sead::ObjArray;

#[skyline::hook(offset = PAGE_BIND_LISTS)]
unsafe fn page_bind_lists(page: *mut CharaMakePage, parent: *mut Control, builder: *mut c_void) {
    call_original!(page, parent, builder);

    unsafe { mouth_list::bind(page, parent, builder) };
}

#[skyline::hook(offset = SEQ_SETUP_CONTROLS)]
unsafe fn seq_setup_controls(seq: *mut CharaMakeSeq) {
    call_original!(seq);

    unsafe { mouth_list::setup(seq) };
}

#[skyline::hook(offset = SEQ_ON_CONTROL_FOCUS)]
unsafe fn seq_on_control_focus(seq: *mut CharaMakeSeq, event: *mut ControlEvent) {
    call_original!(seq, event);

    unsafe { mouth_list::on_focus(seq, (*event).control) };
}

#[skyline::hook(offset = SEQ_ON_CONTROL_DECIDE)]
unsafe fn seq_on_control_decide(seq: *mut CharaMakeSeq, event: *mut ControlEvent) {
    call_original!(seq, event);

    unsafe { mouth_list::on_decide(seq, (*event).control) };
}

#[skyline::hook(offset = SEQ_TICK_ICONS)]
unsafe fn seq_tick_icons(seq: *mut CharaMakeSeq) {
    call_original!(seq);

    unsafe { mouth_list::tick(seq) };
}

#[skyline::hook(offset = SEQ_ON_EXIT)]
unsafe fn seq_on_exit(seq: *mut CharaMakeSeq) {
    unsafe { mouth_list::exit(seq) };

    call_original!(seq);
}

#[skyline::hook(offset = SEQ_PROCESS_REQUEST)]
unsafe fn seq_process_request(seq: *mut CharaMakeSeq) {
    let anchor = unsafe { (*seq).request.is(RequestState::UpdateAnchor) };

    call_original!(seq);

    if anchor {
        unsafe { mouth_list::after_request(seq) };
    }
}

#[skyline::hook(offset = SEQ_SET_GROUP_ENABLED)]
unsafe fn seq_set_group_enabled(seq: *mut CharaMakeSeq, group: u32, enable: u32, mode: u32) {
    if group != GROUP_MOUTH || unsafe { !mouth_list::is_active(seq) } {
        call_original!(seq, group, enable, mode);
        return;
    }

    // The old mouth buttons stay hidden and out of navigation; the list stands in for them.
    call_original!(seq, group, 0, mode);

    unsafe { mouth_list::set_enabled(seq, enable & 1 != 0, mode) };
}

#[skyline::hook(offset = SEQ_FILL_MOUTH_LIST)]
unsafe fn seq_fill_mouth_list(values: *mut ObjArray<u32>) {
    unsafe { mouth_list::fill(values, || call_original!(values)) };
}

#[skyline::hook(offset = SEQ_REFRESH_ENTER)]
unsafe fn seq_refresh_enter(seq: *mut CharaMakeSeq) {
    call_original!(seq);

    unsafe { mouth_list::refresh(seq) };
}

#[skyline::main(name = "acnh-mouth")]
pub fn main() {
    skyline::install_hooks!(
        page_bind_lists,
        seq_setup_controls,
        seq_on_control_focus,
        seq_on_control_decide,
        seq_tick_icons,
        seq_on_exit,
        seq_fill_mouth_list,
        seq_process_request,
        seq_set_group_enabled,
        seq_refresh_enter,
    );
}
