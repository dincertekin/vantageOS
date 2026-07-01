//! Boot console: off-white text on dark gray (DESIGN.md's dark-mode pair),
//! one line per `log` call, drawn with `font.rs`'s glyphs. `draw_desktop()`
//! overwrites it once boot finishes -- no scrollback.
//!
//! Lines draw top to bottom, never scroll -- if they run off the bottom
//! they just stop appearing on screen (still go to serial). Fine for the
//! half-dozen lines `main.rs` prints today.

use core::fmt::{self, Write};

use crate::{font, framebuffer};

const SCALE: i32 = 3; // each font pixel -> SCALE x SCALE block
const CHAR_SPACING: i32 = 1; // blank columns between chars, pre-scale
const LINE_SPACING: i32 = 3; // blank rows between lines, pre-scale
const MARGIN: i32 = 16;
const TEXT_COLOR: u32 = 0xF5_F5_F7; // off-white, not harsh pure white
const BACKGROUND_COLOR: u32 = 0x1A_1A_1A;

const CHAR_ADVANCE: i32 = (font::GLYPH_WIDTH as i32 + CHAR_SPACING) * SCALE;
const LINE_HEIGHT: i32 = (font::GLYPH_HEIGHT as i32 + LINE_SPACING) * SCALE;

/// Whether the boot console is on screen. `begin` flips it on; nothing
/// flips it back off (nobody calls `log` after the desktop takes over).
static mut ACTIVE: bool = false;
static mut CURRENT_ROW: i32 = 0;

/// Clears to background color and resets to the top row.
pub fn begin() {
    framebuffer::fill_rect(0, 0, framebuffer::WIDTH, framebuffer::HEIGHT, BACKGROUND_COLOR);
    unsafe {
        ACTIVE = true;
        CURRENT_ROW = 0;
    }
}

/// Callers like `main.rs`'s pacing pause should skip themselves when this
/// is `false` -- no point pausing for text that was never drawn.
pub fn is_active() -> bool {
    unsafe { ACTIVE }
}

/// `no_std` stand-in for `format!()`: `core::fmt::Write` over a fixed
/// byte array, since there's no allocator.
struct LineBuffer {
    bytes: [u8; 96],
    len: usize,
}

impl LineBuffer {
    fn new() -> Self {
        LineBuffer { bytes: [0; 96], len: 0 }
    }

    /// Bytes written so far. Falls back to "" rather than show corrupted
    /// text if a multi-byte char got truncated at the buffer's edge.
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl Write for LineBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            if self.len < self.bytes.len() {
                self.bytes[self.len] = b;
                self.len += 1;
            }
        }
        Ok(())
    }
}

/// Formats and draws one boot-log line on the next free row. No-op if
/// `begin` hasn't been called yet.
#[allow(static_mut_refs)]
pub fn log(level: &str, args: fmt::Arguments) {
    if unsafe { !ACTIVE } {
        return;
    }

    let mut line = LineBuffer::new();
    let _ = write!(line, "[vantageOS] [{level}] {args}");

    unsafe {
        let y = MARGIN + CURRENT_ROW * LINE_HEIGHT;
        if y + LINE_HEIGHT > framebuffer::HEIGHT as i32 {
            return; // ran off the bottom
        }
        draw_text(MARGIN, y, line.as_str());
        CURRENT_ROW += 1;
    }
}

/// Draws `text` with its top-left corner at `(x, y)`. Unmapped chars just
/// leave a blank advance.
fn draw_text(x: i32, y: i32, text: &str) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if let Some(glyph) = font::glyph_for(ch) {
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..font::GLYPH_WIDTH {
                    let bit = font::GLYPH_WIDTH - 1 - col;
                    if (bits >> bit) & 1 == 0 {
                        continue;
                    }
                    let px = cursor_x + col as i32 * SCALE;
                    let py = y + row as i32 * SCALE;
                    for sy in 0..SCALE {
                        for sx in 0..SCALE {
                            framebuffer::put_pixel((px + sx) as usize, (py + sy) as usize, TEXT_COLOR);
                        }
                    }
                }
            }
        }
        cursor_x += CHAR_ADVANCE;
    }
}
