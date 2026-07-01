//! Virtual display. `virt` has no real GPU, so we use QEMU's "ramfb"
//! device: hand over a RAM address, QEMU redraws its window from those
//! bytes each frame. "The screen" is just an array we write pixels into.
//!
//! Pixel format XRGB8888 (`0x00RRGGBB` per `u32`), native on little-endian
//! AArch64 -- only the one-time fw_cfg setup message needs byte-swapping,
//! since that wire protocol is big-endian.

use crate::fw_cfg;

// 1920x1200 (16:10): matches a MacBook Air 13", and QEMU sizes its window
// directly off these dims (1 fb pixel = 1 window pixel, no scaling).
pub const WIDTH: usize = 1920;
pub const HEIGHT: usize = 1200;

/// DRM/pixman fourcc for XRGB8888 -- tells QEMU how to read our buffer.
const FORMAT_XRGB8888: u32 = 0x3432_5258;

/// Pixel data, in `.bss` (free, zero-init). MMU is off, so its address is
/// already physical -- what ramfb needs. `static mut` is fine: only the
/// single-threaded functions below ever touch it.
static mut FRAMEBUFFER: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

/// Config record QEMU's ramfb device expects, big-endian on the wire (see
/// `hw/display/ramfb.c`). Built as plain values, `to_be_bytes()` on write.
struct RamfbConfig {
    addr: u64,
    fourcc: u32,
    flags: u32,
    width: u32,
    height: u32,
    stride: u32,
}

impl RamfbConfig {
    fn to_bytes(&self) -> [u8; 28] {
        let mut buf = [0u8; 28];
        buf[0..8].copy_from_slice(&self.addr.to_be_bytes());
        buf[8..12].copy_from_slice(&self.fourcc.to_be_bytes());
        buf[12..16].copy_from_slice(&self.flags.to_be_bytes());
        buf[16..20].copy_from_slice(&self.width.to_be_bytes());
        buf[20..24].copy_from_slice(&self.height.to_be_bytes());
        buf[24..28].copy_from_slice(&self.stride.to_be_bytes());
        buf
    }
}

/// Sets up the display. `false` if QEMU wasn't started with `-device ramfb`
/// -- callers should fall back to serial-only output.
pub fn init() -> bool {
    let Some(selector) = fw_cfg::find_file("etc/ramfb") else {
        return false;
    };

    #[allow(static_mut_refs)]
    let addr = unsafe { FRAMEBUFFER.as_ptr() as u64 };

    let config = RamfbConfig {
        addr,
        fourcc: FORMAT_XRGB8888,
        flags: 0,
        width: WIDTH as u32,
        height: HEIGHT as u32,
        stride: (WIDTH * 4) as u32,
    };

    fw_cfg::write_file(selector, &config.to_bytes());
    true
}

/// Sets one pixel (`0x00RRGGBB`). Out-of-bounds is silently ignored so
/// callers don't need their own edge-clamping.
pub fn put_pixel(x: usize, y: usize, color: u32) {
    if x >= WIDTH || y >= HEIGHT {
        return;
    }
    #[allow(static_mut_refs)]
    unsafe {
        FRAMEBUFFER[y * WIDTH + x] = color;
    }
}

/// Fills the rectangle `[x, x+w) x [y, y+h)` with a solid color. Used for
/// simple flat-colored UI elements like the top bar.
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    for row in y..(y + h).min(HEIGHT) {
        for col in x..(x + w).min(WIDTH) {
            put_pixel(col, row, color);
        }
    }
}
