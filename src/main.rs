#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

// Shell isn't wired into boot right now -- kept for later, not deleted.
#[allow(dead_code)]
pub mod commands;
pub mod console;
pub mod desktop;
pub mod font;
pub mod framebuffer;
pub mod fw_cfg;
pub mod gui;
pub mod power;
#[allow(dead_code)]
pub mod shell;
pub mod uart;
pub mod virtio_input;

global_asm!(include_str!("boot.s"));

/// Spins for roughly `iterations` passes -- no timer driver, so "roughly"
/// is doing real work here. Calibrated by the constants below.
fn busy_wait(iterations: u32) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

/// Boot-log line pacing (~175ms) and final-line lingertime (~1.5s) before
/// the desktop takes over. Hand-calibrated against `make run`'s debug
/// build (~13,300 iters/ms under QEMU/TCG) -- release/real hardware races
/// through much faster.
const LINE_PAUSE_ITERS: u32 = 2_300_000;
const FINAL_PAUSE_ITERS: u32 = 20_000_000;

/// Prints one boot-log line to serial (always) and the screen (if up),
/// pausing after so lines appear one at a time instead of in a burst.
fn log_line(level: &str, args: core::fmt::Arguments) {
    print!("[vantageOS] [{level}] ");
    println!("{args}");
    console::log(level, args);
    if console::is_active() {
        busy_wait(LINE_PAUSE_ITERS);
    }
}

macro_rules! log_info {
    ($($arg:tt)*) => { log_line("info", format_args!($($arg)*)) };
}

/// Same shape, for a step that didn't fail but didn't fully succeed
/// (e.g. an optional device QEMU wasn't started with).
macro_rules! log_warn {
    ($($arg:tt)*) => { log_line("warn", format_args!($($arg)*)) };
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // Must run before the first log line -- nowhere to draw on screen
    // until it does, so "framebuffer loaded" is only ever visible over
    // serial, same as a monitor can't show itself turning on.
    let has_display = framebuffer::init();
    if has_display {
        console::begin();
    }

    log_info!("vantageOS booting...");
    log_info!("UART console loaded");

    if has_display {
        log_info!("framebuffer ({}x{}) loaded", framebuffer::WIDTH, framebuffer::HEIGHT);
    } else {
        log_warn!("framebuffer not found (run with `-device ramfb`)");
    }

    let has_mouse = virtio_input::init();
    if has_mouse {
        log_info!("virtio mouse loaded");
    } else {
        log_warn!("virtio mouse not found (run with `-device virtio-tablet-device`)");
    }

    if has_display {
        log_info!("desktop renderer loaded");
    }

    // Last line to ever appear on screen -- draw_desktop() below overwrites
    // the boot console entirely.
    log_info!("boot complete");

    if has_display {
        busy_wait(FINAL_PAUSE_ITERS); // let the last line stay readable
        gui::draw_desktop();
    }

    // Swap for `shell::Shell::new().run()` to get the serial shell back.
    loop {
        desktop::handle_tick(virtio_input::poll());
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("kernel panic: {info}");
    loop {
        core::hint::spin_loop();
    }
}
