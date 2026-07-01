//! Draws the desktop: gradient wallpaper, top bar (system icon, Terminal
//! icon, dropdown panel), a draggable/resizable Terminal window, a
//! selection rectangle, and a mouse cursor.
//!
//! Plain pixel math on `framebuffer::put_pixel`. One rule everywhere:
//! **the cursor is always drawn last** -- every mutating function repaints
//! its region then redraws the cursor on top, so it never gets visually
//! cut by something drawn underneath it.

use crate::font;
use crate::framebuffer::{self, HEIGHT, WIDTH};

/// Top bar height. 44 sits in DESIGN.md's 40-48px toolbar range.
const TOP_BAR_HEIGHT: i32 = 44;
const TOP_BAR_COLOR: u32 = 0x2C2C2E; // dark gray, macOS-style menu bar

/// Wallpaper gradient: `GRADIENT_CENTER` at screen center, `GRADIENT_EDGE`
/// at the corners, interpolated by distance (soft vignette).
const GRADIENT_CENTER: (u8, u8, u8) = (0xE8, 0xE8, 0xEA); // near-white
const GRADIENT_EDGE: (u8, u8, u8) = (0x18, 0x18, 0x1A); // near-black

// --- System icon (top bar, left side) --------------------------------

const ICON_CENTER_X: i32 = TOP_BAR_HEIGHT / 2;
const ICON_CENTER_Y: i32 = TOP_BAR_HEIGHT / 2;
const ICON_RADIUS: f32 = 10.0;
/// Ring half-thickness: "on the icon" = within this of `ICON_RADIUS`.
const ICON_RING_THICKNESS: f32 = 1.2;
const ICON_COLOR: u32 = 0xFF_FF_FF;
/// Bigger than the visible ring, per DESIGN.md's 36-44px touch target min.
const ICON_HIT_RADIUS: f32 = 20.0;

fn icon_ring_at(x: i32, y: i32) -> bool {
    let dist = sqrtf((x - ICON_CENTER_X) as f32 * (x - ICON_CENTER_X) as f32 + (y - ICON_CENTER_Y) as f32 * (y - ICON_CENTER_Y) as f32);
    (dist - ICON_RADIUS).abs() <= ICON_RING_THICKNESS
}

/// True if `(x, y)` counts as a click on the system icon.
pub fn icon_contains(x: i32, y: i32) -> bool {
    let dx = (x - ICON_CENTER_X) as f32;
    let dy = (y - ICON_CENTER_Y) as f32;
    sqrtf(dx * dx + dy * dy) <= ICON_HIT_RADIUS
}

// --- Top bar label -------------------------------------------------------

const TOPBAR_LABEL: &str = "vantageOS";
const TOPBAR_LABEL_COLOR: u32 = 0xFF_FF_FF; // matches the icon
const TOPBAR_LABEL_SCALE: i32 = 2; // smaller than boot console's 3x -- has to fit the 44px bar
const TOPBAR_LABEL_X: i32 = ICON_CENTER_X + ICON_RADIUS as i32 + 10;
const TOPBAR_LABEL_Y: i32 = (TOP_BAR_HEIGHT - font::GLYPH_HEIGHT as i32 * TOPBAR_LABEL_SCALE) / 2;

fn topbar_label_at(x: i32, y: i32) -> bool {
    font::text_pixel_at(TOPBAR_LABEL, TOPBAR_LABEL_X, TOPBAR_LABEL_Y, TOPBAR_LABEL_SCALE, x, y)
}

// --- Terminal icon (top bar, center zone) ---------------------------------
//
// DESIGN.md reserves the center zone for running/pinned apps. Only one app
// exists so far, so this is one hardcoded icon, not a real list.

const TERMINAL_ICON_CENTER_X: i32 = WIDTH as i32 / 2;
const TERMINAL_ICON_CENTER_Y: i32 = TOP_BAR_HEIGHT / 2;
const TERMINAL_ICON_SIZE: i32 = 20;
const TERMINAL_ICON_HIT_RADIUS: f32 = 20.0;
const TERMINAL_ICON_COLOR: u32 = 0xFF_FF_FF;
/// "This app is running" pill, shown while the window exists (open or
/// minimized) -- stands in for a dock/taskbar, which this kernel has none of.
const TERMINAL_ICON_ACTIVE_COLOR: u32 = 0x48_48_4A;
const TERMINAL_ICON_ACTIVE_RADIUS: i32 = 18;

fn terminal_icon_rect() -> Rect {
    Rect {
        x: TERMINAL_ICON_CENTER_X - TERMINAL_ICON_SIZE / 2,
        y: TERMINAL_ICON_CENTER_Y - TERMINAL_ICON_SIZE / 2,
        w: TERMINAL_ICON_SIZE,
        h: TERMINAL_ICON_SIZE,
    }
}

/// Tiny window-shaped glyph (outline + title-bar divider) -- not the
/// system icon's ring, so it reads as "an app" at a glance.
fn terminal_icon_glyph_at(x: i32, y: i32) -> bool {
    let r = terminal_icon_rect();
    if !r.contains(x, y) {
        return false;
    }
    let rel_y = y - r.y;
    let on_border = x == r.x || x == r.x + r.w - 1 || rel_y == 0 || rel_y == r.h - 1;
    on_border || rel_y == 4
}

fn terminal_icon_active_pill_at(x: i32, y: i32) -> bool {
    if !window_exists() {
        return false;
    }
    let dx = (x - TERMINAL_ICON_CENTER_X) as f32;
    let dy = (y - TERMINAL_ICON_CENTER_Y) as f32;
    sqrtf(dx * dx + dy * dy) <= TERMINAL_ICON_ACTIVE_RADIUS as f32
}

/// True if `(x, y)` counts as a click on the Terminal icon.
pub fn terminal_icon_contains(x: i32, y: i32) -> bool {
    let dx = (x - TERMINAL_ICON_CENTER_X) as f32;
    let dy = (y - TERMINAL_ICON_CENTER_Y) as f32;
    sqrtf(dx * dx + dy * dy) <= TERMINAL_ICON_HIT_RADIUS
}

/// Redraw area for the "running" pill appearing/disappearing -- only
/// needed on true open/close, not minimize/restore (window still exists).
fn terminal_icon_area_rect() -> Rect {
    Rect {
        x: TERMINAL_ICON_CENTER_X - TERMINAL_ICON_ACTIVE_RADIUS,
        y: 0,
        w: TERMINAL_ICON_ACTIVE_RADIUS * 2,
        h: TOP_BAR_HEIGHT,
    }
}

// --- Panel (opens below the icon when it's clicked) --------------------
//
// Volume slider + Shut Down button, DESIGN.md's control-center order.
// Brightness/Wi-Fi/Bluetooth/Settings aren't here -- no driver behind them.

const PANEL_X: i32 = 8;
const PANEL_Y: i32 = TOP_BAR_HEIGHT;
const PANEL_WIDTH: i32 = 180;
const PANEL_HEIGHT: i32 = 120; // fits volume row + button, DESIGN.md spacing
const PANEL_COLOR: u32 = 0x3A_3A_3C;
const PANEL_BORDER_COLOR: u32 = 0x54_54_56;
const PANEL_TEXT_COLOR: u32 = 0xF5_F5_F7; // off-white, not pure white

const PANEL_MARGIN: i32 = 12; // shared inner margin, keeps rows aligned

/// Panel visibility. Starts closed; only `toggle_panel` changes it.
static mut PANEL_OPEN: bool = false;

fn panel_rect() -> Rect {
    Rect { x: PANEL_X, y: PANEL_Y, w: PANEL_WIDTH, h: PANEL_HEIGHT }
}

// --- Volume row ----------------------------------------------------------

const VOLUME_LABEL: &str = "Volume";
const VOLUME_LABEL_SCALE: i32 = 2;
const VOLUME_LABEL_X: i32 = PANEL_X + PANEL_MARGIN;
const VOLUME_LABEL_Y: i32 = PANEL_Y + PANEL_MARGIN;

const VOLUME_TRACK_X: i32 = PANEL_X + PANEL_MARGIN;
const VOLUME_TRACK_Y: i32 = VOLUME_LABEL_Y + font::GLYPH_HEIGHT as i32 * VOLUME_LABEL_SCALE + 8;
const VOLUME_TRACK_WIDTH: i32 = PANEL_WIDTH - 2 * PANEL_MARGIN;
const VOLUME_TRACK_HEIGHT: i32 = 10;
const VOLUME_TRACK_COLOR: u32 = 0x54_54_56; // same as the panel border
const VOLUME_FILL_COLOR: u32 = 0x5E_8A_C4; // DESIGN.md's muted blue
/// Track is thin, but DESIGN.md wants easy-to-grab handles -- extend the
/// *clickable* region above/below rather than draw a fatter track.
const VOLUME_HIT_PADDING: i32 = 10;

fn volume_track_rect() -> Rect {
    Rect { x: VOLUME_TRACK_X, y: VOLUME_TRACK_Y, w: VOLUME_TRACK_WIDTH, h: VOLUME_TRACK_HEIGHT }
}

fn volume_hit_rect() -> Rect {
    Rect {
        x: VOLUME_TRACK_X,
        y: VOLUME_TRACK_Y - VOLUME_HIT_PADDING,
        w: VOLUME_TRACK_WIDTH,
        h: VOLUME_TRACK_HEIGHT + 2 * VOLUME_HIT_PADDING,
    }
}

/// Volume, 0-100. UI-only stub -- no audio driver to actually change.
static mut VOLUME_PERCENT: i32 = 50;

/// True if the panel is open and `(x, y)` hits the slider's click region.
pub fn volume_track_contains(x: i32, y: i32) -> bool {
    is_panel_open() && volume_hit_rect().contains(x, y)
}

/// Maps `x` to 0-100 along the track, stores it, repaints the track.
/// Clamped, so dragging past either end just pins to 0 or 100.
#[allow(static_mut_refs)]
pub fn set_volume_from_x(x: i32) {
    let relative = (x - VOLUME_TRACK_X).clamp(0, VOLUME_TRACK_WIDTH);
    let percent = relative * 100 / VOLUME_TRACK_WIDTH;
    unsafe {
        VOLUME_PERCENT = percent;
    }
    redraw_rect(volume_track_rect());
    redraw_cursor_on_top();
}

fn volume_label_at(x: i32, y: i32) -> bool {
    font::text_pixel_at(VOLUME_LABEL, VOLUME_LABEL_X, VOLUME_LABEL_Y, VOLUME_LABEL_SCALE, x, y)
}

fn volume_fill_width() -> i32 {
    unsafe { VOLUME_TRACK_WIDTH * VOLUME_PERCENT / 100 }
}

// --- Shut Down button ------------------------------------------------------

const BUTTON_LABEL: &str = "Shut Down";
const BUTTON_LABEL_SCALE: i32 = 2;
const BUTTON_HEIGHT: i32 = 40; // DESIGN.md's 36-44px control minimum
const BUTTON_COLOR: u32 = 0xB3_3B_3B; // muted red

fn button_rect() -> Rect {
    Rect {
        x: PANEL_X + PANEL_MARGIN,
        y: PANEL_Y + PANEL_HEIGHT - PANEL_MARGIN - BUTTON_HEIGHT,
        w: PANEL_WIDTH - 2 * PANEL_MARGIN,
        h: BUTTON_HEIGHT,
    }
}

fn button_label_at(x: i32, y: i32) -> bool {
    let rect = button_rect();
    let origin_x = centered_text_x(&rect, BUTTON_LABEL, BUTTON_LABEL_SCALE);
    let origin_y = centered_text_y(&rect, BUTTON_LABEL_SCALE);
    font::text_pixel_at(BUTTON_LABEL, origin_x, origin_y, BUTTON_LABEL_SCALE, x, y)
}

/// Horizontal origin that centers `text` (at the given scale) within `rect`.
fn centered_text_x(rect: &Rect, text: &str, scale: i32) -> i32 {
    let char_advance = (font::GLYPH_WIDTH as i32 + 1) * scale;
    let text_width = text.chars().count() as i32 * char_advance;
    rect.x + (rect.w - text_width) / 2
}

/// Vertical origin that centers one line of text within `rect`.
fn centered_text_y(rect: &Rect, scale: i32) -> i32 {
    let text_height = font::GLYPH_HEIGHT as i32 * scale;
    rect.y + (rect.h - text_height) / 2
}

/// True if the panel is open and `(x, y)` is the Shut Down button.
pub fn shutdown_button_contains(x: i32, y: i32) -> bool {
    is_panel_open() && button_rect().contains(x, y)
}

/// True if the panel is open and `(x, y)` is anywhere inside it -- stops
/// a panel click from also starting a desktop selection drag.
pub fn panel_contains(x: i32, y: i32) -> bool {
    is_panel_open() && panel_rect().contains(x, y)
}

pub fn is_panel_open() -> bool {
    unsafe { PANEL_OPEN }
}

/// Flips the panel open/closed, repaints the rectangle it occupies, and
/// puts the cursor back on top.
#[allow(static_mut_refs)]
pub fn toggle_panel() {
    unsafe {
        PANEL_OPEN = !PANEL_OPEN;
    }
    redraw_rect(panel_rect());
    redraw_cursor_on_top();
}

// --- Selection rectangle (drag on the desktop) --------------------------

const SELECTION_FILL: u32 = 0xD8_D8_D8; // light gray, as requested
const SELECTION_BORDER: u32 = 0xA0_A0_A0;

/// Current drag-selection box. `None` = no drag in progress.
static mut SELECTION: Option<Rect> = None;

fn selection_at(x: i32, y: i32) -> Option<u32> {
    let rect = unsafe { SELECTION }?;
    if !rect.contains(x, y) {
        return None;
    }
    let on_border = x == rect.x || x == rect.x + rect.w - 1 || y == rect.y || y == rect.y + rect.h - 1;
    Some(if on_border { SELECTION_BORDER } else { SELECTION_FILL })
}

/// Replaces (or clears, with `None`) the selection box, repainting only
/// the old + new areas, cursor back on top.
#[allow(static_mut_refs)]
pub fn set_selection(new_rect: Option<Rect>) {
    unsafe {
        let old_rect = SELECTION;
        SELECTION = new_rect;
        if let Some(r) = old_rect {
            redraw_rect(r);
        }
        if let Some(r) = new_rect {
            redraw_rect(r);
        }
    }
    redraw_cursor_on_top();
}

// --- Window (Terminal app) -------------------------------------------------
//
// One static window slot -- one app planned, no allocator for a real list.
// `visible` separates closed (`None`, next open resets geometry) from
// minimized (`Some` but hidden, geometry kept, restored via the Terminal
// icon -- this kernel's only stand-in for a taskbar).

#[derive(Clone, Copy)]
struct Window {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    visible: bool,
    /// Set while maximized; holds the geometry to restore on un-maximize.
    restore_rect: Option<(i32, i32, i32, i32)>,
}

static mut WINDOW: Option<Window> = None;

const WINDOW_MIN_WIDTH: i32 = 280;
const WINDOW_MIN_HEIGHT: i32 = 180;
const WINDOW_DEFAULT_WIDTH: i32 = 640;
const WINDOW_DEFAULT_HEIGHT: i32 = 420;

const WINDOW_TITLEBAR_HEIGHT: i32 = 36;
const WINDOW_CORNER_RADIUS: f32 = 10.0; // DESIGN.md's 8-12px range
const WINDOW_BORDER_COLOR: u32 = 0x54_54_56; // same as panel border
const WINDOW_TITLEBAR_COLOR: u32 = 0x3A_3A_3C; // same as panel body
const WINDOW_CONTENT_COLOR: u32 = 0x24_24_26; // a shade darker: "the page"
const WINDOW_TITLE_COLOR: u32 = 0xF5_F5_F7;
const WINDOW_TITLE_SCALE: i32 = 2;
/// Chrome only for now -- no keyboard/shell wired in yet.
const WINDOW_TITLE: &str = "Terminal";

fn default_window_rect() -> Window {
    Window {
        x: (WIDTH as i32 - WINDOW_DEFAULT_WIDTH) / 2,
        y: TOP_BAR_HEIGHT + 60,
        width: WINDOW_DEFAULT_WIDTH,
        height: WINDOW_DEFAULT_HEIGHT,
        visible: true,
        restore_rect: None,
    }
}

/// The window, only if visible (not minimized) -- everything that draws
/// or hit-tests it goes through this.
fn visible_window() -> Option<Window> {
    unsafe { WINDOW }.filter(|w| w.visible)
}

fn window_rect() -> Option<Rect> {
    visible_window().map(|w| Rect { x: w.x, y: w.y, w: w.width, h: w.height })
}

/// True if the window exists, open or minimized -- drives the Terminal
/// icon's "running" pill, which stays lit while minimized.
pub fn window_exists() -> bool {
    unsafe { WINDOW }.is_some()
}

/// True if `(x, y)` is inside the visible window -- stops a window click
/// from also starting a desktop selection drag.
pub fn window_contains(x: i32, y: i32) -> bool {
    window_rect().is_some_and(|r| r.contains(x, y))
}

fn titlebar_rect(win: &Window) -> Rect {
    Rect { x: win.x, y: win.y, w: win.width, h: WINDOW_TITLEBAR_HEIGHT }
}

// --- Traffic-light window controls (title bar, left side) -----------------

const TRAFFIC_LIGHT_RADIUS: f32 = 6.0;
/// Bigger than the visible dot, but capped under DESIGN.md's 36-44px norm
/// -- three that large in one title bar would overlap. Traffic lights are
/// a universal convention at this smaller scale, so it stays usable.
const TRAFFIC_LIGHT_HIT_RADIUS: f32 = 9.0;
const TRAFFIC_LIGHT_MARGIN_X: i32 = 16;
const TRAFFIC_LIGHT_SPACING: i32 = 22;
const TRAFFIC_LIGHT_CLOSE_COLOR: u32 = 0xB3_3B_3B; // same muted red as Shut Down
const TRAFFIC_LIGHT_MINIMIZE_COLOR: u32 = 0xC2_8A_35; // muted amber
const TRAFFIC_LIGHT_MAXIMIZE_COLOR: u32 = 0x4C_8A_57; // muted green

#[derive(Clone, Copy, PartialEq)]
pub enum TrafficLight {
    Close,
    Minimize,
    Maximize,
}

const TRAFFIC_LIGHTS: [TrafficLight; 3] = [TrafficLight::Close, TrafficLight::Minimize, TrafficLight::Maximize];

fn traffic_light_center(win: &Window, light: TrafficLight) -> (i32, i32) {
    let index = match light {
        TrafficLight::Close => 0,
        TrafficLight::Minimize => 1,
        TrafficLight::Maximize => 2,
    };
    (win.x + TRAFFIC_LIGHT_MARGIN_X + index * TRAFFIC_LIGHT_SPACING, win.y + WINDOW_TITLEBAR_HEIGHT / 2)
}

fn traffic_light_color(light: TrafficLight) -> u32 {
    match light {
        TrafficLight::Close => TRAFFIC_LIGHT_CLOSE_COLOR,
        TrafficLight::Minimize => TRAFFIC_LIGHT_MINIMIZE_COLOR,
        TrafficLight::Maximize => TRAFFIC_LIGHT_MAXIMIZE_COLOR,
    }
}

fn traffic_light_at(win: &Window, x: i32, y: i32) -> Option<u32> {
    TRAFFIC_LIGHTS.iter().find_map(|&light| {
        let (cx, cy) = traffic_light_center(win, light);
        let dx = (x - cx) as f32;
        let dy = (y - cy) as f32;
        (sqrtf(dx * dx + dy * dy) <= TRAFFIC_LIGHT_RADIUS).then(|| traffic_light_color(light))
    })
}

fn traffic_light_hit(win: &Window, x: i32, y: i32) -> Option<TrafficLight> {
    TRAFFIC_LIGHTS.iter().copied().find(|&light| {
        let (cx, cy) = traffic_light_center(win, light);
        let dx = (x - cx) as f32;
        let dy = (y - cy) as f32;
        sqrtf(dx * dx + dy * dy) <= TRAFFIC_LIGHT_HIT_RADIUS
    })
}

/// True if the visible window's title bar (outside any traffic light) is
/// hit -- callers should start a window drag on a press here.
pub fn window_titlebar_contains(x: i32, y: i32) -> bool {
    let Some(win) = visible_window() else { return false };
    titlebar_rect(&win).contains(x, y) && traffic_light_hit(&win, x, y).is_none()
}

/// Which traffic light (if any) `(x, y)` hits on the visible window.
pub fn window_traffic_light_at(x: i32, y: i32) -> Option<TrafficLight> {
    let win = visible_window()?;
    traffic_light_hit(&win, x, y)
}

fn window_title_at(win: &Window, x: i32, y: i32) -> bool {
    let rect = titlebar_rect(win);
    let origin_x = centered_text_x(&rect, WINDOW_TITLE, WINDOW_TITLE_SCALE);
    let origin_y = win.y + (WINDOW_TITLEBAR_HEIGHT - font::GLYPH_HEIGHT as i32 * WINDOW_TITLE_SCALE) / 2;
    font::text_pixel_at(WINDOW_TITLE, origin_x, origin_y, WINDOW_TITLE_SCALE, x, y)
}

// --- Resize grip (bottom-right corner) -------------------------------------

const RESIZE_GRIP_HIT_SIZE: i32 = 18;

fn resize_grip_hit_rect(win: &Window) -> Rect {
    Rect {
        x: win.x + win.width - RESIZE_GRIP_HIT_SIZE,
        y: win.y + win.height - RESIZE_GRIP_HIT_SIZE,
        w: RESIZE_GRIP_HIT_SIZE,
        h: RESIZE_GRIP_HIT_SIZE,
    }
}

/// True if the bottom-right resize grip is hit -- start a resize on press.
/// Only this one corner for now; more would follow the same pattern.
pub fn window_resize_grip_contains(x: i32, y: i32) -> bool {
    visible_window().is_some_and(|win| resize_grip_hit_rect(&win).contains(x, y))
}

/// Three short diagonal ticks near the corner (standard "resize here"
/// shorthand) -- each is the pixels an equal distance (4/8/12px) from the
/// corner along both axes.
fn resize_grip_at(win: &Window, x: i32, y: i32) -> bool {
    let rel_x = win.x + win.width - 1 - x;
    let rel_y = win.y + win.height - 1 - y;
    if !(0..=12).contains(&rel_x) || !(0..=12).contains(&rel_y) {
        return false;
    }
    matches!(rel_x + rel_y, 4 | 8 | 12)
}

// --- Drawing + state changes -----------------------------------------------

/// Point-in-rounded-rect: clamp `(x, y)` to the rect shrunk by `radius`,
/// check distance to that point against `radius`. Away from a corner the
/// clamp lands on `(x, y)` itself (distance 0) -- only the corners curve.
fn in_rounded_rect(rect: &Rect, radius: f32, x: i32, y: i32) -> bool {
    if !rect.contains(x, y) {
        return false;
    }
    let inner_x0 = rect.x as f32 + radius;
    let inner_y0 = rect.y as f32 + radius;
    let inner_x1 = (rect.x + rect.w) as f32 - radius;
    let inner_y1 = (rect.y + rect.h) as f32 - radius;
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    let cx = px.clamp(inner_x0, inner_x1);
    let cy = py.clamp(inner_y0, inner_y1);
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= radius * radius
}

/// What color `(x, y)` should show if it's within the window, checking
/// (in order) the rounded border, the traffic lights, the title bar, the
/// resize grip, then plain content -- `None` if `(x, y)` isn't part of the
/// window at all (including the four corners the rounding cuts off), so
/// callers fall through to whatever's underneath.
fn window_pixel_at(x: i32, y: i32) -> Option<u32> {
    let win = visible_window()?;
    let rect = Rect { x: win.x, y: win.y, w: win.width, h: win.height };
    if !in_rounded_rect(&rect, WINDOW_CORNER_RADIUS, x, y) {
        return None;
    }

    let inset = Rect { x: rect.x + 1, y: rect.y + 1, w: rect.w - 2, h: rect.h - 2 };
    let inset_radius = (WINDOW_CORNER_RADIUS - 1.0).max(0.0);
    if !in_rounded_rect(&inset, inset_radius, x, y) {
        return Some(WINDOW_BORDER_COLOR);
    }

    // Gated behind the title-bar check first: running the 3 traffic-light
    // sqrtf tests against every pixel in the whole window was real
    // per-pixel cost paid by the entire area, not just the 36px title bar.
    if titlebar_rect(&win).contains(x, y) {
        if let Some(color) = traffic_light_at(&win, x, y) {
            return Some(color);
        }
        return Some(if window_title_at(&win, x, y) { WINDOW_TITLE_COLOR } else { WINDOW_TITLEBAR_COLOR });
    }
    if resize_grip_at(&win, x, y) {
        return Some(WINDOW_BORDER_COLOR);
    }
    Some(WINDOW_CONTENT_COLOR)
}

/// Terminal icon click: opens a fresh window, restores a minimized one, or
/// no-ops if already open/visible -- not a close toggle.
#[allow(static_mut_refs)]
pub fn open_or_restore_window() {
    unsafe {
        match WINDOW {
            None => {
                WINDOW = Some(default_window_rect());
                redraw_rect(terminal_icon_area_rect());
            }
            Some(ref mut w) if !w.visible => w.visible = true,
            _ => return,
        }
    }
    redraw_rect(window_rect().unwrap());
    redraw_cursor_on_top();
}

/// Discards the window -- next open starts fresh at the default geometry.
#[allow(static_mut_refs)]
pub fn close_window() {
    let Some(old) = window_rect() else { return };
    unsafe {
        WINDOW = None;
    }
    redraw_rect(old);
    redraw_rect(terminal_icon_area_rect());
    redraw_cursor_on_top();
}

/// Hides the window, keeps position/size -- Terminal icon restores it.
#[allow(static_mut_refs)]
pub fn minimize_window() {
    let Some(old) = window_rect() else { return };
    unsafe {
        if let Some(ref mut w) = WINDOW {
            w.visible = false;
        }
    }
    redraw_rect(old);
    redraw_cursor_on_top();
}

/// Toggles between current geometry and full-height/width below the top
/// bar, remembering the prior geometry to restore on toggle-back.
#[allow(static_mut_refs)]
pub fn toggle_maximize_window() {
    let Some(old) = window_rect() else { return };
    unsafe {
        if let Some(ref mut w) = WINDOW {
            if let Some((rx, ry, rw, rh)) = w.restore_rect.take() {
                w.x = rx;
                w.y = ry;
                w.width = rw;
                w.height = rh;
            } else {
                w.restore_rect = Some((w.x, w.y, w.width, w.height));
                w.x = 0;
                w.y = TOP_BAR_HEIGHT;
                w.width = WIDTH as i32;
                w.height = HEIGHT as i32 - TOP_BAR_HEIGHT;
            }
        }
    }
    redraw_rect(old.union(window_rect().unwrap()));
    redraw_cursor_on_top();
}

/// Moves the window by `(dx, dy)`, fed straight from mouse motion each
/// drag tick. Clamped to stay fully on screen, below the top bar.
#[allow(static_mut_refs)]
pub fn move_window(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let Some(old) = window_rect() else { return };
    unsafe {
        if let Some(ref mut w) = WINDOW {
            w.x = (w.x + dx).clamp(0, (WIDTH as i32 - w.width).max(0));
            w.y = (w.y + dy).clamp(TOP_BAR_HEIGHT, (HEIGHT as i32 - w.height).max(TOP_BAR_HEIGHT));
        }
    }
    redraw_rect(old.union(window_rect().unwrap()));
    redraw_cursor_on_top();
}

/// Resizes the window by `(dx, dy)` each grip-drag tick. Clamped to a
/// sane minimum and to not grow past the screen edge.
#[allow(static_mut_refs)]
pub fn resize_window(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let Some(old) = window_rect() else { return };
    unsafe {
        if let Some(ref mut w) = WINDOW {
            let max_w = WIDTH as i32 - w.x;
            let max_h = HEIGHT as i32 - w.y;
            w.width = (w.width + dx).clamp(WINDOW_MIN_WIDTH, max_w);
            w.height = (w.height + dy).clamp(WINDOW_MIN_HEIGHT, max_h);
        }
    }
    redraw_rect(old.union(window_rect().unwrap()));
    redraw_cursor_on_top();
}

// --- Shared geometry + compositing --------------------------------------

/// An axis-aligned rectangle in screen pixels.
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// Rect spanning two corners, whichever order they come in -- what a
    /// drag needs, since the mouse can move in any direction.
    pub fn from_corners(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
        Rect { x: x0.min(x1), y: y0.min(y1), w: (x1 - x0).abs(), h: (y1 - y0).abs() }
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    /// Smallest rect containing both `self` and `other` -- repaints
    /// exactly the pixels a move/resize/maximize touched, in one pass.
    fn union(&self, other: Rect) -> Rect {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = (self.x + self.w).max(other.x + other.w);
        let y1 = (self.y + self.h).max(other.y + other.h);
        Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }
}

/// What color `(x, y)` should show right now, checking every layer except
/// the cursor, back to front: top bar > panel > window > selection >
/// wallpaper. Single source of truth for both `draw_desktop` and
/// `redraw_rect`, so they can't disagree about what's under the cursor.
fn desktop_pixel_at(x: i32, y: i32) -> u32 {
    if y < TOP_BAR_HEIGHT {
        if icon_ring_at(x, y) {
            return ICON_COLOR;
        }
        if topbar_label_at(x, y) {
            return TOPBAR_LABEL_COLOR;
        }
        if terminal_icon_glyph_at(x, y) {
            return TERMINAL_ICON_COLOR;
        }
        if terminal_icon_active_pill_at(x, y) {
            return TERMINAL_ICON_ACTIVE_COLOR;
        }
        return TOP_BAR_COLOR;
    }

    if is_panel_open() && panel_rect().contains(x, y) {
        if button_rect().contains(x, y) {
            return if button_label_at(x, y) { PANEL_TEXT_COLOR } else { BUTTON_COLOR };
        }
        if volume_label_at(x, y) {
            return PANEL_TEXT_COLOR;
        }
        if volume_track_rect().contains(x, y) {
            return if x < VOLUME_TRACK_X + volume_fill_width() { VOLUME_FILL_COLOR } else { VOLUME_TRACK_COLOR };
        }
        let r = panel_rect();
        let on_border = x == r.x || x == r.x + r.w - 1 || y == r.y || y == r.y + r.h - 1;
        return if on_border { PANEL_BORDER_COLOR } else { PANEL_COLOR };
    }

    if let Some(color) = window_pixel_at(x, y) {
        return color;
    }

    if let Some(color) = selection_at(x, y) {
        return color;
    }

    // t: 0 at dead-center, 1 at the corners, regardless of resolution.
    let center_x = WIDTH as f32 / 2.0;
    let center_y = HEIGHT as f32 / 2.0;
    let dx = x as f32 - center_x;
    let dy = y as f32 - center_y;
    let distance = sqrtf(dx * dx + dy * dy);
    let max_distance = sqrtf(center_x * center_x + center_y * center_y);
    let t = (distance / max_distance).clamp(0.0, 1.0);

    let r = lerp_channel(GRADIENT_CENTER.0, GRADIENT_EDGE.0, t);
    let g = lerp_channel(GRADIENT_CENTER.1, GRADIENT_EDGE.1, t);
    let b = lerp_channel(GRADIENT_CENTER.2, GRADIENT_EDGE.2, t);
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn lerp_channel(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t) as u8
}

/// `core` has no `f32::sqrt` (that's `std`/libm) -- AArch64 has a native
/// `fsqrt` instruction, so call it directly instead of adding a dependency.
/// Safe: `boot.s` enables FP/SIMD before any Rust code runs.
fn sqrtf(x: f32) -> f32 {
    let result: f32;
    unsafe {
        core::arch::asm!("fsqrt {result:s}, {x:s}", result = out(vreg) result, x = in(vreg) x);
    }
    result
}

/// Repaints every pixel in `rect` from `desktop_pixel_at`. Doesn't touch
/// the cursor -- callers follow up with `redraw_cursor_on_top`.
fn redraw_rect(rect: Rect) {
    let y0 = rect.y.max(0);
    let y1 = (rect.y + rect.h).min(HEIGHT as i32);
    let x0 = rect.x.max(0);
    let x1 = (rect.x + rect.w).min(WIDTH as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            framebuffer::put_pixel(x as usize, y as usize, desktop_pixel_at(x, y));
        }
    }
}

// --- Cursor --------------------------------------------------------------

/// Cursor position -- starting point, and where it lives between calls.
static mut CURSOR_X: i32 = (WIDTH / 2) as i32;
static mut CURSOR_Y: i32 = (HEIGHT / 2) as i32;

/// Current cursor position, for hit-testing clicks.
#[allow(static_mut_refs)]
pub fn cursor_pos() -> (i32, i32) {
    unsafe { (CURSOR_X, CURSOR_Y) }
}

fn redraw_cursor_on_top() {
    let (x, y) = cursor_pos();
    draw_cursor(x, y);
}

/// Paints the whole desktop, cursor on top. Call once at boot; after that
/// the targeted functions above repaint only what changed.
#[allow(static_mut_refs)]
pub fn draw_desktop() {
    for y in 0..HEIGHT as i32 {
        for x in 0..WIDTH as i32 {
            framebuffer::put_pixel(x as usize, y as usize, desktop_pixel_at(x, y));
        }
    }
    redraw_cursor_on_top();
}

/// Moves the cursor by `(dx, dy)`, clamped to screen edges, and repaints
/// the union of old/new cursor boxes in one pass (`redraw_cursor_move`).
#[allow(static_mut_refs)]
pub fn move_cursor(dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }

    unsafe {
        let old_x = CURSOR_X;
        let old_y = CURSOR_Y;

        CURSOR_X = (CURSOR_X + dx).clamp(0, WIDTH as i32 - 1);
        CURSOR_Y = (CURSOR_Y + dy).clamp(0, HEIGHT as i32 - 1);

        redraw_cursor_move(old_x, old_y, CURSOR_X, CURSOR_Y);
    }
}

/// Repaints the union of old/new cursor boxes in one pass: new-cursor
/// pixels get cursor color, everything else gets `desktop_pixel_at`. Must
/// be one pass, not erase-then-draw -- ramfb has no double buffering, so a
/// separate erase pass left a real (if brief) window with zero cursor
/// pixels on screen, visible as flicker on every move.
fn redraw_cursor_move(old_x: i32, old_y: i32, new_x: i32, new_y: i32) {
    let x0 = old_x.min(new_x).max(0);
    let y0 = old_y.min(new_y).max(0);
    let x1 = (old_x.max(new_x) + CURSOR_WIDTH as i32).min(WIDTH as i32);
    let y1 = (old_y.max(new_y) + CURSOR_HEIGHT as i32).min(HEIGHT as i32);

    for y in y0..y1 {
        for x in x0..x1 {
            let color = cursor_color_at(new_x, new_y, x, y).unwrap_or_else(|| desktop_pixel_at(x, y));
            framebuffer::put_pixel(x as usize, y as usize, color);
        }
    }
}

const CURSOR_WIDTH: usize = 16;
const CURSOR_HEIGHT: usize = 24;
const CURSOR_BLACK: u32 = 0x00_00_00;
const CURSOR_WHITE: u32 = 0xFF_FF_FF;

/// Arrow outline as a closed polygon, relative to the cursor's top-left
/// corner, in path order from the tip. From `cursor.svg`'s path (`M5,2
/// L5,19.5 L9,15.6 L12.3,22.5 L15.3,21.1 L12,14.3 L17.8,14.3 Z`), shifted
/// off column/row 0 (room for the white outline) and scaled to match the
/// top bar's ~20px icon.
const CURSOR_POLY: &[(f32, f32)] = &[
    (1.0, 1.0),   // tip
    (1.0, 18.5),  // spine bottom (heel), straight down from the tip
    (5.0, 14.6),  // first notch, cutting back up from the heel
    (8.3, 21.5),  // tail's outer point
    (11.3, 20.1), // tail's inner point
    (8.0, 13.3),  // second notch, cutting back toward the body
    (13.8, 13.3), // head's outer corner -- closes back to the tip from here
];

/// Point-in-polygon via ray casting (odd edge-crossings = inside). Tests
/// pixel centers to avoid a ray passing exactly through a vertex.
// Needs both the current vertex and the previous (`j`), so plain
// `.enumerate()` doesn't fit -- index by hand.
#[allow(clippy::needless_range_loop)]
fn point_in_polygon(px: f32, py: f32) -> bool {
    let mut inside = false;
    let mut j = CURSOR_POLY.len() - 1;
    for i in 0..CURSOR_POLY.len() {
        let (xi, yi) = CURSOR_POLY[i];
        let (xj, yj) = CURSOR_POLY[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// True if `(row, col)` is part of the arrow's solid black silhouette.
fn shape_filled_at(row: isize, col: isize) -> bool {
    if row < 0 || col < 0 {
        return false;
    }
    point_in_polygon(col as f32 + 0.5, row as f32 + 0.5)
}

/// Cursor color (drawn at `(origin_x, origin_y)`) at pixel `(x, y)`, if
/// any. Black = inside the shape; white = touches a black pixel (crisp
/// one-pixel outline); `None` = transparent, leave the background alone.
fn cursor_color_at(origin_x: i32, origin_y: i32, x: i32, y: i32) -> Option<u32> {
    let col = x - origin_x;
    let row = y - origin_y;
    if col < 0 || row < 0 {
        return None;
    }
    if shape_filled_at(row as isize, col as isize) {
        return Some(CURSOR_BLACK);
    }
    let touches_black = (-1..=1)
        .any(|dy| (-1..=1).any(|dx| (dy != 0 || dx != 0) && shape_filled_at(row as isize + dy, col as isize + dx)));
    touches_black.then_some(CURSOR_WHITE)
}

/// Draws the arrow at `(origin_x, origin_y)` -- used when there's no old
/// position to combine with. `move_cursor` uses `redraw_cursor_move` instead.
fn draw_cursor(origin_x: i32, origin_y: i32) {
    for row in 0..CURSOR_HEIGHT {
        for col in 0..CURSOR_WIDTH {
            let px = origin_x + col as i32;
            let py = origin_y + row as i32;
            if px >= 0 && py >= 0 {
                if let Some(color) = cursor_color_at(origin_x, origin_y, px, py) {
                    framebuffer::put_pixel(px as usize, py as usize, color);
                }
            }
        }
    }
}
