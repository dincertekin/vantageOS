//! Turns mouse ticks into desktop behavior (icon/panel/window/selection
//! clicks and drags). `gui.rs` only draws + hit-tests; this decides *when*.

use crate::virtio_input::MouseTick;
use crate::{gui, power};

/// Selection drag's start point, screen coords. `None` = no drag.
static mut DRAG_START: Option<(i32, i32)> = None;

/// Volume slider drag -- unanchored, just tracks cursor x while held.
static mut VOLUME_DRAGGING: bool = false;

/// Window titlebar/resize-grip drag -- unanchored, fed the mouse's own
/// relative motion each tick.
static mut WINDOW_DRAGGING: bool = false;
static mut WINDOW_RESIZING: bool = false;

/// Call once per idle-loop iteration. Cursor must move before we hit-test
/// against its position, or a press+move in the same tick checks a stale spot.
#[allow(static_mut_refs)]
pub fn handle_tick(tick: MouseTick) {
    gui::move_cursor(tick.dx, tick.dy);
    let (x, y) = gui::cursor_pos();

    if tick.left_pressed {
        if let Some(light) = gui::window_traffic_light_at(x, y) {
            match light {
                gui::TrafficLight::Close => gui::close_window(),
                gui::TrafficLight::Minimize => gui::minimize_window(),
                gui::TrafficLight::Maximize => gui::toggle_maximize_window(),
            }
        } else if gui::window_resize_grip_contains(x, y) {
            unsafe {
                WINDOW_RESIZING = true;
            }
        } else if gui::window_titlebar_contains(x, y) {
            unsafe {
                WINDOW_DRAGGING = true;
            }
        } else if gui::icon_contains(x, y) {
            gui::toggle_panel();
        } else if gui::shutdown_button_contains(x, y) {
            power::shutdown();
        } else if gui::volume_track_contains(x, y) {
            unsafe {
                VOLUME_DRAGGING = true;
            }
            gui::set_volume_from_x(x);
        } else if gui::terminal_icon_contains(x, y) {
            gui::open_or_restore_window();
        } else if !gui::panel_contains(x, y) && !gui::window_contains(x, y) {
            // Click on open desktop -- start a selection drag.
            unsafe {
                DRAG_START = Some((x, y));
            }
        }
    }

    // React to left_down's level, not just the edge -- a missed press/
    // release event self-corrects on the next tick instead of sticking.
    unsafe {
        if tick.left_down {
            if VOLUME_DRAGGING {
                gui::set_volume_from_x(x);
            } else if WINDOW_DRAGGING {
                gui::move_window(tick.dx, tick.dy);
            } else if WINDOW_RESIZING {
                gui::resize_window(tick.dx, tick.dy);
            } else if let Some((start_x, start_y)) = DRAG_START {
                gui::set_selection(Some(gui::Rect::from_corners(start_x, start_y, x, y)));
            }
        } else {
            VOLUME_DRAGGING = false;
            WINDOW_DRAGGING = false;
            WINDOW_RESIZING = false;
            if DRAG_START.is_some() {
                DRAG_START = None;
                gui::set_selection(None);
            }
        }
    }
}
