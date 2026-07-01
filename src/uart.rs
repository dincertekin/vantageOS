//! PL011 UART driver, QEMU `virt`'s console at 0x0900_0000. Only I/O
//! device right now (no keyboard, no framebuffer input) -- QEMU's model
//! resets into a usable state on its own, so we only need two registers.

use core::fmt;

const UART0_BASE: usize = 0x0900_0000;

/// Data register: write queues a TX byte, read dequeues an RX byte.
const DR_OFFSET: usize = 0x00;

/// Flag register: is it safe to read/write `DR` right now.
const FR_OFFSET: usize = 0x18;

const FR_RXFE: u8 = 1 << 4; // RX FIFO empty
const FR_TXFF: u8 = 1 << 5; // TX FIFO full

pub struct Uart;

impl Uart {
    pub const fn new() -> Self {
        Uart
    }

    fn reg(offset: usize) -> *mut u8 {
        (UART0_BASE + offset) as *mut u8
    }

    /// Blocks until there's room in the TX FIFO, then sends one byte.
    pub fn putc(&mut self, c: u8) {
        unsafe {
            while core::ptr::read_volatile(Self::reg(FR_OFFSET)) & FR_TXFF != 0 {}
            core::ptr::write_volatile(Self::reg(DR_OFFSET), c);
        }
    }

    /// Blocks until a byte has arrived on RX, then returns it.
    pub fn getc(&mut self) -> u8 {
        unsafe {
            while core::ptr::read_volatile(Self::reg(FR_OFFSET)) & FR_RXFE != 0 {}
            core::ptr::read_volatile(Self::reg(DR_OFFSET))
        }
    }

    /// Non-blocking `getc` -- `None` if nothing's waiting. Lets the shell
    /// poll the mouse between keystrokes instead of blocking on input.
    pub fn try_getc(&mut self) -> Option<u8> {
        unsafe {
            if core::ptr::read_volatile(Self::reg(FR_OFFSET)) & FR_RXFE != 0 {
                None
            } else {
                Some(core::ptr::read_volatile(Self::reg(DR_OFFSET)))
            }
        }
    }

    pub fn puts(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                // terminals want CRLF, not bare LF
                self.putc(b'\r');
            }
            self.putc(byte);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.puts(s);
        Ok(())
    }
}

/// Like `std`'s `print!`/`println!`, but to the UART. Each call opens a
/// fresh `Uart` -- it's a zero-sized handle, nothing to share or lock.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::uart::Uart::new(), $($arg)*);
    }};
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {{
        $crate::print!($($arg)*);
        $crate::print!("\n");
    }};
}
