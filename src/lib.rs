mod game;
mod list;

use game::*;

#[skyline::hook(offset = PAGE_BIND_LISTS)]
unsafe fn page_bind_lists(page: *mut u8, parent: *mut u8, builder: *mut u8) {
    call_original!(page, parent, builder);

    unsafe { list::bind(page, parent, builder) };
}

#[skyline::hook(offset = SEQ_SETUP_CONTROLS)]
unsafe fn seq_setup_controls(seq: *mut u8) {
    call_original!(seq);

    unsafe { list::setup(seq) };
}

#[skyline::hook(offset = SEQ_ON_CONTROL_FOCUS)]
unsafe fn seq_on_control_focus(seq: *mut u8, event: *mut u8) {
    call_original!(seq, event);

    unsafe { list::on_focus(seq, *at::<*mut u8>(event, 8)) };
}

#[skyline::hook(offset = SEQ_ON_CONTROL_DECIDE)]
unsafe fn seq_on_control_decide(seq: *mut u8, event: *mut u8) {
    call_original!(seq, event);

    unsafe { list::on_decide(seq, *at::<*mut u8>(event, 8)) };
}

#[skyline::hook(offset = SEQ_TICK_ICONS)]
unsafe fn seq_tick_icons(seq: *mut u8) {
    call_original!(seq);

    unsafe { list::tick(seq) };
}

#[skyline::hook(offset = SEQ_ON_EXIT)]
unsafe fn seq_on_exit(seq: *mut u8) {
    unsafe { list::exit(seq) };

    call_original!(seq);
}

#[skyline::hook(offset = SEQ_PROCESS_REQUEST)]
unsafe fn seq_process_request(seq: *mut u8) {
    let anchor = unsafe { *at::<i32>(seq, SEQ_REQUEST) == REQUEST_UPDATE_ANCHOR };

    call_original!(seq);

    if anchor {
        unsafe { list::after_request(seq) };
    }
}

#[skyline::hook(offset = SEQ_SET_GROUP_ENABLED)]
unsafe fn seq_set_group_enabled(seq: *mut u8, group: u32, enable: u32, mode: u32) {
    if group != GROUP_MOUTH || unsafe { !list::is_active(seq) } {
        call_original!(seq, group, enable, mode);
        return;
    }

    // The old mouth buttons stay hidden and out of navigation; the list stands in for them.
    call_original!(seq, group, 0, mode);

    unsafe { list::set_enabled(seq, enable & 1 != 0, mode) };
}

#[skyline::hook(offset = LIST_PLAY_EXEC)]
unsafe fn list_play_exec(list: *mut u8) {
    unsafe { list::before_play_exec(list) };

    call_original!(list);
}

#[skyline::hook(offset = SEQ_FILL_MOUTH_LIST)]
unsafe fn seq_fill_mouth_list(value_list: *mut u8) {
    unsafe { list::before_fill(value_list) };

    call_original!(value_list);

    unsafe { list::after_fill(value_list) };
}

// Page refresh, right where the game selects the current mouth button.
#[skyline::hook(offset = SEQ_MOUTH_HIGHLIGHT_SELECT, inline)]
unsafe fn seq_mouth_refresh(ctx: &mut skyline::hooks::InlineCtx) {
    unsafe { list::refresh(ctx.registers[19].x() as *mut u8) };
}

// Right after the game links the nose/mouth tab's navigation grid.
#[skyline::hook(offset = SEQ_MOUTH_NAV_GRID_DONE, inline)]
unsafe fn seq_mouth_navigation(ctx: &mut skyline::hooks::InlineCtx) {
    unsafe { list::link_navigation(ctx.registers[19].x() as *mut u8) };
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
        list_play_exec,
        seq_mouth_refresh,
        seq_mouth_navigation,
    );
}
