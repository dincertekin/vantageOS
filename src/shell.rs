//! Line-input shell: read a line, dispatch to `commands.rs`, repeat.
//! "Commands" are just Rust functions -- no process model yet.

use crate::commands;
use crate::uart::Uart;
use crate::virtio_input;
use crate::{print, println};

/// Longest line accepted. No heap, so this is a fixed-size buffer.
const LINE_MAX: usize = 128;

const BACKSPACE: u8 = 0x08;
const DELETE: u8 = 0x7F; // what most terminals actually send for backspace
const CARRIAGE_RETURN: u8 = b'\r';
const LINE_FEED: u8 = b'\n';

pub struct Shell {
    uart: Uart,
    buf: [u8; LINE_MAX],
    len: usize,
}

impl Shell {
    pub const fn new() -> Self {
        Shell {
            uart: Uart::new(),
            buf: [0; LINE_MAX],
            len: 0,
        }
    }

    /// Main loop, never returns -- this is the kernel's idle activity.
    pub fn run(&mut self) -> ! {
        println!("vantageOS shell -- type `help` for a list of commands");
        loop {
            print!("vantage> ");
            let line = self.read_line();
            commands::dispatch(line);
        }
    }

    /// Reads one line, echoing each keystroke ourselves (host terminal is
    /// raw mode). Returns it without the trailing newline, no allocation.
    fn read_line(&mut self) -> &str {
        self.len = 0;

        loop {
            // Poll the mouse every pass so it stays responsive while we
            // wait for a keystroke -- no interrupt controller, so this is
            // the only place mouse motion gets noticed.
            crate::desktop::handle_tick(virtio_input::poll());

            let Some(c) = self.uart.try_getc() else {
                core::hint::spin_loop();
                continue;
            };

            match c {
                CARRIAGE_RETURN | LINE_FEED => {
                    self.uart.puts("\n");
                    break;
                }
                BACKSPACE | DELETE => {
                    if self.len > 0 {
                        self.len -= 1;
                        self.uart.puts("\u{8} \u{8}"); // back, blank, back
                    }
                }
                printable if self.len < self.buf.len()
                    && (printable.is_ascii_graphic() || printable == b' ') =>
                {
                    self.buf[self.len] = printable;
                    self.len += 1;
                    self.uart.putc(printable);
                }
                _ => {} // unprintable, or buffer full -- drop it
            }
        }

        // Only ASCII ever got pushed into `buf`, so this can't fail.
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}
