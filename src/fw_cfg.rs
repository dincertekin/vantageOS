//! Driver for QEMU's "fw_cfg" device: a firmware/guest config channel with
//! no real chip behind it. Used only to find + configure the "ramfb"
//! virtual display (see `framebuffer.rs`); real hardware has neither.
//!
//! Write a 16-bit selector naming the item, then read/write the data
//! register one byte at a time (auto-advancing cursor). Dead simple.

const FW_CFG_BASE: usize = 0x0902_0000;

const REG_DATA: usize = 0x00; // next byte of the selected item
const REG_SELECTOR: usize = 0x08; // picks what REG_DATA refers to
/// Write a physical address here to trigger a `DmaAccess` transfer.
const REG_DMA: usize = 0x10;

/// Selector for the file directory (every named blob QEMU offers), used to
/// find ramfb's own selector without hardcoding it.
const SELECTOR_FILE_DIR: u16 = 0x0019;

/// Fixed-size, NUL-padded file name length in directory entries.
const FILE_NAME_MAX: usize = 56;

fn select(selector: u16) {
    unsafe {
        // Wire format is big-endian regardless of host endianness.
        core::ptr::write_volatile((FW_CFG_BASE + REG_SELECTOR) as *mut u16, selector.to_be());
    }
}

fn read_u8() -> u8 {
    unsafe { core::ptr::read_volatile((FW_CFG_BASE + REG_DATA) as *const u8) }
}

fn read_u32_be() -> u32 {
    let mut bytes = [0u8; 4];
    for b in bytes.iter_mut() {
        *b = read_u8();
    }
    u32::from_be_bytes(bytes)
}

fn read_u16_be() -> u16 {
    let mut bytes = [0u8; 2];
    for b in bytes.iter_mut() {
        *b = read_u8();
    }
    u16::from_be_bytes(bytes)
}

/// Looks up a file's selector by name (e.g. "etc/ramfb"). `None` means
/// QEMU wasn't started with that device.
pub fn find_file(name: &str) -> Option<u16> {
    select(SELECTOR_FILE_DIR);

    let count = read_u32_be();
    for _ in 0..count {
        // entry: size(4) + selector(2) + reserved(2) + name(56)
        let _size = read_u32_be();
        let selector = read_u16_be();
        let _reserved = read_u16_be();

        let mut entry_name = [0u8; FILE_NAME_MAX];
        for b in entry_name.iter_mut() {
            *b = read_u8();
        }

        // NUL-padded -- trim before comparing.
        let nul_at = entry_name.iter().position(|&b| b == 0).unwrap_or(FILE_NAME_MAX);
        if &entry_name[..nul_at] == name.as_bytes() {
            return Some(selector);
        }
    }
    None
}

const DMA_CTL_SELECT: u32 = 0x08; // select the file named by control's top 16 bits
const DMA_CTL_WRITE: u32 = 0x10; // guest -> device, not the default read

/// Descriptor QEMU reads to run a DMA transfer: pulls `length` bytes from
/// `address` in one shot. Fields big-endian on the wire.
#[repr(C)]
struct DmaAccess {
    control: u32,
    length: u32,
    address: u64,
}

/// Writes `data` into file `selector` via DMA -- the only write mechanism
/// QEMU honors for files like "etc/ramfb".
pub fn write_file(selector: u16, data: &[u8]) {
    let control = ((selector as u32) << 16) | DMA_CTL_SELECT | DMA_CTL_WRITE;

    let access = DmaAccess {
        control: control.to_be(),
        length: (data.len() as u32).to_be(),
        address: (data.as_ptr() as u64).to_be(),
    };

    unsafe {
        let access_addr = &access as *const DmaAccess as u64;
        // Fence required: `access`'s fields are plain stores, and
        // `write_volatile` only orders against other volatile ops -- an
        // optimizing build (release uses LTO) can reorder them past the
        // trigger below, handing QEMU a stale descriptor. Compiler fence
        // is enough (not a hardware barrier): TCG runs the compiled
        // stream in order, only the compiler's own scheduling is at risk.
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        // Trigger: QEMU reads `access` back and runs the transfer.
        core::ptr::write_volatile((FW_CFG_BASE + REG_DMA) as *mut u64, access_addr.to_be());
    }
}
