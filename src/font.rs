//! Tiny embedded 5x7 bitmap font -- only the characters currently used
//! anywhere in the kernel, not full ASCII (e.g. no j/q/y/z). Unmapped
//! chars render as a blank cell; extend `GLYPHS` when a new one's needed.
//!
//! Each glyph is 7 bytes, low 5 bits per byte = one pixel row (bit 4 =
//! leftmost). Binary literals so the 1s/0s roughly trace the letter shape.

pub const GLYPH_WIDTH: usize = 5;
pub const GLYPH_HEIGHT: usize = 7;

type Glyph = [u8; GLYPH_HEIGHT];

/// (character, bitmap) pairs. Unsorted -- `glyph_for` linear-scans, fine
/// given how rarely text actually gets drawn.
const GLYPHS: &[(char, Glyph)] = &[
    // Digits
    ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('2', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
    ('3', [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110]),
    ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    // Uppercase
    ('A', [0b00100, 0b01010, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001]),
    ('D', [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
    ('O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
    ('S', [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
    ('V', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    ('T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    // Lowercase
    ('a', [0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b10001, 0b01111]),
    ('b', [0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110]),
    ('c', [0b00000, 0b01110, 0b10000, 0b10000, 0b10000, 0b10000, 0b01110]),
    ('d', [0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111]),
    ('e', [0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b10000, 0b01110]),
    ('f', [0b00110, 0b01000, 0b11110, 0b01000, 0b01000, 0b01000, 0b01000]),
    ('g', [0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110]),
    ('h', [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001]),
    ('i', [0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('k', [0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010]),
    ('l', [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('m', [0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001]),
    ('n', [0b00000, 0b00000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001]),
    ('o', [0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('p', [0b00000, 0b00000, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000]),
    ('r', [0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000]),
    ('s', [0b00000, 0b00000, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110]),
    ('t', [0b00100, 0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00011]),
    ('u', [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10001, 0b01111]),
    ('v', [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    ('w', [0b00000, 0b00000, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010]),
    ('x', [0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001]),
    // Punctuation
    (' ', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
    ('.', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100]),
    (':', [0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000]),
    ('(', [0b00110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b00110]),
    (')', [0b01100, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01100]),
    ('[', [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110]),
    (']', [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110]),
    ('`', [0b01100, 0b00100, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
    ('-', [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
];

/// Looks up a glyph. `None` for anything not in `GLYPHS` -- callers should
/// leave a blank cell rather than guess.
pub fn glyph_for(c: char) -> Option<&'static Glyph> {
    GLYPHS.iter().find(|(ch, _)| *ch == c).map(|(_, bits)| bits)
}

/// Is `(x, y)` a lit pixel of `text` rendered at `(origin_x, origin_y)`,
/// scaled `scale`x? A query, not a draw -- needed by anything that has to
/// participate in `gui.rs`'s z-order compositor (asked "what's under you").
pub fn text_pixel_at(text: &str, origin_x: i32, origin_y: i32, scale: i32, x: i32, y: i32) -> bool {
    let glyph_w = GLYPH_WIDTH as i32 * scale;
    let glyph_h = GLYPH_HEIGHT as i32 * scale;
    let char_advance = (GLYPH_WIDTH as i32 + 1) * scale;

    let mut cursor_x = origin_x;
    for ch in text.chars() {
        let in_cell = x >= cursor_x && x < cursor_x + glyph_w && y >= origin_y && y < origin_y + glyph_h;
        if in_cell {
            if let Some(glyph) = glyph_for(ch) {
                let col = ((x - cursor_x) / scale) as usize;
                let row = ((y - origin_y) / scale) as usize;
                let bit = GLYPH_WIDTH - 1 - col;
                if (glyph[row] >> bit) & 1 != 0 {
                    return true;
                }
            }
        }
        cursor_x += char_advance;
    }
    false
}
