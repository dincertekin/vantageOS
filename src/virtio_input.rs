//! Virtio "input" device driver -- how the mouse gets to us. `virt` has no
//! PS/2 controller, so mice sit on one of 32 identical virtio-mmio slots.
//!
//! Expects `-device virtio-tablet-device` (absolute), not
//! `virtio-mouse-device` (relative, which grabs the host mouse on click --
//! a hidden gesture this OS avoids, and unreliable for drag-release).
//! `poll` converts absolute samples back to relative `(dx, dy)` so nothing
//! above this module needs to know the difference.
//!
//! Only implements enough to receive events: no interrupts (poll instead),
//! no writes back, no config-space reads. Find the device, set up one
//! receive queue, drain it.
//!
//! **Uses the legacy (pre-1.0) virtio-mmio transport** (`Version` reads 1,
//! not 2) -- meaningfully different wire format from modern virtio 1.1: no
//! `FEATURES_OK` step, and desc/avail/used rings share one memory block
//! addressed by a single page-frame-number register instead of three
//! separate addresses. See the virtio spec's "Legacy Interfaces" appendix.

use crate::framebuffer;

/// 32 identical virtio-mmio slots; scan all of them rather than assume one.
const MMIO_BASE: usize = 0x0a00_0000;
const MMIO_STRIDE: usize = 0x200;
const MMIO_SLOTS: usize = 32;

/// `DeviceID` value meaning "input device" (mouse, keyboard, ...).
const VIRTIO_DEVICE_ID_INPUT: u32 = 18;

// Legacy virtio-mmio register offsets -- GuestPageSize/QueueAlign/QueuePFN
// only exist in legacy; no QueueReady or per-region address registers.
const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_HOST_FEATURES: usize = 0x010;
const REG_HOST_FEATURES_SEL: usize = 0x014;
const REG_GUEST_FEATURES: usize = 0x020;
const REG_GUEST_FEATURES_SEL: usize = 0x024;
const REG_GUEST_PAGE_SIZE: usize = 0x028;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_ALIGN: usize = 0x03c;
const REG_QUEUE_PFN: usize = 0x040;
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_STATUS: usize = 0x070;

const MAGIC_VALUE: u32 = 0x7472_6976; // ASCII "virt", little-endian u32
const LEGACY_VERSION: u32 = 1;

// Legacy init handshake: reset -> ACKNOWLEDGE -> DRIVER -> negotiate
// features -> set up queue -> DRIVER_OK. No FEATURES_OK step in legacy.
const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;

/// Receive-queue descriptor count. 8 is plenty -- no mouse produces that
/// many unconsumed events between two `poll()` calls.
const QUEUE_SIZE: usize = 8;

/// Unit `QueuePFN` divides the queue memory's address by. Just needs the
/// address to be an exact multiple of it; 4096 is conventional.
const GUEST_PAGE_SIZE: u32 = 4096;

/// Padding before the used ring's start. The spec formula's exact byte
/// offset is ambiguous across implementations for smaller values; using
/// the traditional page-sized alignment (like every real driver) makes
/// any such disagreement round up to the same boundary regardless.
const QUEUE_ALIGN: u32 = 4096;

/// One device-written event: type, code, value. No timestamp (virtio
/// strips it, unlike Linux's `input_event`). See virtio spec §5.8.6.
#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    kind: u16,
    code: u16,
    value: u32,
}

const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const EV_ABS: u16 = 0x03;
// ABS_X/Y share REL_X/Y's numeric codes -- harmless, `event.kind` is what
// the match below branches on.
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
/// QEMU's fixed range for absolute devices, regardless of screen res --
/// `poll` scales this against `framebuffer::WIDTH`/`HEIGHT`.
const ABS_MAX: i32 = 32767;
/// Linux input-event code for the left mouse button (borrowed namespace).
const BTN_LEFT: u16 = 0x110;

/// One descriptor-table entry: a buffer (address + length) and how the
/// device may use it.
#[repr(C)]
#[derive(Clone, Copy)]
struct Desc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

/// The device may write into this buffer (as opposed to reading from it).
const DESC_F_WRITE: u16 = 2;

/// The ring the *driver* writes to, telling the device which descriptors
/// are ready for it to use.
#[repr(C)]
struct AvailRing {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE],
}

/// One entry in the ring the *device* writes to, reporting a descriptor it
/// has finished with.
#[repr(C)]
#[derive(Clone, Copy)]
struct UsedElem {
    id: u32,
    len: u32,
}

/// The ring the device writes to.
#[repr(C)]
struct UsedRing {
    flags: u16,
    idx: u16,
    ring: [UsedElem; QUEUE_SIZE],
}

// Legacy requires desc/avail/used to live in *one* contiguous block
// (addressed by a single `QueuePFN`), with the device recomputing each
// region's offset via this exact spec formula -- must match precisely.
const DESC_BYTES: usize = 16 * QUEUE_SIZE; // sizeof(Desc) * QUEUE_SIZE
const AVAIL_OFFSET: usize = DESC_BYTES;
// +2: legacy always reserves a "used_event" u16 after the avail ring.
const AVAIL_BYTES_WITH_RESERVED: usize = 4 + 2 * QUEUE_SIZE + 2;
const USED_OFFSET: usize = align_up(AVAIL_OFFSET + AVAIL_BYTES_WITH_RESERVED, QUEUE_ALIGN as usize);
const USED_BYTES_WITH_RESERVED: usize = 4 + 8 * QUEUE_SIZE + 2;
const QUEUE_MEMORY_BYTES: usize = USED_OFFSET + USED_BYTES_WITH_RESERVED;

const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Combined desc/avail/used region. Page-aligned so `QueuePFN` (this
/// address / `GUEST_PAGE_SIZE`) is losslessly reversible by the device.
#[repr(C, align(4096))]
struct QueueMemory([u8; QUEUE_MEMORY_BYTES]);

static mut QUEUE_MEMORY: QueueMemory = QueueMemory([0; QUEUE_MEMORY_BYTES]);

/// Event payloads, one per descriptor -- don't need to live inside
/// `QUEUE_MEMORY`, only the ring structures have that requirement.
static mut EVENT_BUFS: [InputEvent; QUEUE_SIZE] = [InputEvent {
    kind: 0,
    code: 0,
    value: 0,
}; QUEUE_SIZE];

/// Our device's mmio slot, and how far we've read its used ring. `None`
/// until `init()` finds a device.
static mut SLOT_BASE: Option<usize> = None;
static mut LAST_SEEN_USED_IDX: u16 = 0;

fn reg_read(base: usize, offset: usize) -> u32 {
    unsafe { core::ptr::read_volatile((base + offset) as *const u32) }
}

fn reg_write(base: usize, offset: usize, value: u32) {
    unsafe { core::ptr::write_volatile((base + offset) as *mut u32, value) }
}

/// Raw pointers into `QUEUE_MEMORY`'s three regions.
#[allow(static_mut_refs)]
fn desc_ptr() -> *mut Desc {
    unsafe { QUEUE_MEMORY.0.as_mut_ptr() as *mut Desc }
}
#[allow(static_mut_refs)]
fn avail_ptr() -> *mut AvailRing {
    unsafe { QUEUE_MEMORY.0.as_mut_ptr().add(AVAIL_OFFSET) as *mut AvailRing }
}
#[allow(static_mut_refs)]
fn used_ptr() -> *mut UsedRing {
    unsafe { QUEUE_MEMORY.0.as_mut_ptr().add(USED_OFFSET) as *mut UsedRing }
}

/// Scans every virtio-mmio slot for one reporting `DeviceID == 18` (input).
fn find_input_slot() -> Option<usize> {
    for i in 0..MMIO_SLOTS {
        let base = MMIO_BASE + i * MMIO_STRIDE;
        if reg_read(base, REG_MAGIC) == MAGIC_VALUE && reg_read(base, REG_DEVICE_ID) == VIRTIO_DEVICE_ID_INPUT {
            return Some(base);
        }
    }
    None
}

/// Finds the input device and gets it running. `false` if none is attached
/// or it speaks a transport version this driver doesn't handle.
#[allow(static_mut_refs)]
pub fn init() -> bool {
    let Some(base) = find_input_slot() else {
        return false;
    };
    if reg_read(base, REG_VERSION) != LEGACY_VERSION {
        return false; // modern transport -- not handled here
    }

    reg_write(base, REG_STATUS, 0);
    reg_write(base, REG_STATUS, STATUS_ACKNOWLEDGE);
    reg_write(base, REG_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

    // Legacy features are one 32-bit word, no FEATURES_OK step -- accept
    // none of the optional ones (virtio-input has no mandatory bits).
    reg_write(base, REG_HOST_FEATURES_SEL, 0);
    let _host_features = reg_read(base, REG_HOST_FEATURES);
    reg_write(base, REG_GUEST_FEATURES_SEL, 0);
    reg_write(base, REG_GUEST_FEATURES, 0);

    reg_write(base, REG_GUEST_PAGE_SIZE, GUEST_PAGE_SIZE);

    // Queue 0 = eventq (device -> driver), the only one we need.
    reg_write(base, REG_QUEUE_SEL, 0);
    if reg_read(base, REG_QUEUE_NUM_MAX) < QUEUE_SIZE as u32 {
        return false; // smaller than assumed elsewhere -- don't overrun it
    }
    reg_write(base, REG_QUEUE_NUM, QUEUE_SIZE as u32);
    reg_write(base, REG_QUEUE_ALIGN, QUEUE_ALIGN);

    unsafe {
        // Hand the whole ring to the device up front, all "available".
        let desc = desc_ptr();
        let avail = avail_ptr();
        for i in 0..QUEUE_SIZE {
            *desc.add(i) = Desc {
                addr: &EVENT_BUFS[i] as *const InputEvent as u64,
                len: core::mem::size_of::<InputEvent>() as u32,
                flags: DESC_F_WRITE,
                next: 0,
            };
            (*avail).ring[i] = i as u16;
        }
        (*avail).flags = 0;
        (*avail).idx = QUEUE_SIZE as u16;

        (*used_ptr()).flags = 0;
        (*used_ptr()).idx = 0;

        // QueuePFN is what actually attaches this memory to the device.
        let queue_phys_addr = QUEUE_MEMORY.0.as_ptr() as u32;
        reg_write(base, REG_QUEUE_PFN, queue_phys_addr / GUEST_PAGE_SIZE);
    }

    reg_write(base, REG_QUEUE_NOTIFY, 0); // buffers ready in queue 0

    reg_write(base, REG_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_DRIVER_OK);

    unsafe {
        SLOT_BASE = Some(base);
        LAST_SEEN_USED_IDX = 0;
    }
    true
}

/// One tick's mouse activity: motion since last time, plus button state.
/// `left_pressed`/`left_released` are edge-triggered (true for one tick);
/// `left_down` is the level, true the whole time it's held.
#[derive(Default)]
pub struct MouseTick {
    pub dx: i32,
    pub dy: i32,
    pub left_pressed: bool,
    pub left_released: bool,
    pub left_down: bool,
}

/// Left button's level, kept across calls to detect press/release edges.
static mut LEFT_DOWN: bool = false;

/// Last absolute position seen, scaled to screen pixels. `None` until the
/// first sample, so it isn't reported as a jump from `(0, 0)`.
static mut LAST_ABS_X: Option<i32> = None;
static mut LAST_ABS_Y: Option<i32> = None;

/// Maps a raw `0..=ABS_MAX` sample from the device onto `0..screen_size`.
fn scale_abs(raw: i32, screen_size: i32) -> i32 {
    (raw as i64 * screen_size as i64 / (ABS_MAX as i64 + 1)) as i32
}

/// Drains events since the last call into one `MouseTick`. All zeros if
/// nothing happened or no device was found. No interrupts -- call this
/// periodically (the idle loop does, once per iteration).
#[allow(static_mut_refs)]
pub fn poll() -> MouseTick {
    let Some(base) = (unsafe { SLOT_BASE }) else {
        return MouseTick::default();
    };

    let mut tick = MouseTick::default();
    let mut drained_any = false;

    unsafe {
        let avail = avail_ptr();
        let used = used_ptr();

        while LAST_SEEN_USED_IDX != (*used).idx {
            drained_any = true;
            let slot = (LAST_SEEN_USED_IDX as usize) % QUEUE_SIZE;
            let descriptor_id = (*used).ring[slot].id as usize;
            let event = EVENT_BUFS[descriptor_id];

            match event.kind {
                // Kept in case a relative device shows up; the tablet
                // device we actually expect reports EV_ABS instead.
                EV_REL if event.code == REL_X => tick.dx += event.value as i32,
                EV_REL if event.code == REL_Y => tick.dy += event.value as i32,
                // Diff against the last sample to get relative motion.
                // First-ever sample has no "last" -- seed from the
                // cursor's real position, not skip it, or the pointer
                // wouldn't jump to its first reported spot.
                EV_ABS if event.code == ABS_X => {
                    let scaled = scale_abs(event.value as i32, framebuffer::WIDTH as i32);
                    let prev = LAST_ABS_X.unwrap_or_else(|| crate::gui::cursor_pos().0);
                    tick.dx += scaled - prev;
                    LAST_ABS_X = Some(scaled);
                }
                EV_ABS if event.code == ABS_Y => {
                    let scaled = scale_abs(event.value as i32, framebuffer::HEIGHT as i32);
                    let prev = LAST_ABS_Y.unwrap_or_else(|| crate::gui::cursor_pos().1);
                    tick.dy += scaled - prev;
                    LAST_ABS_Y = Some(scaled);
                }
                EV_KEY if event.code == BTN_LEFT => {
                    let now_down = event.value != 0;
                    if now_down && !LEFT_DOWN {
                        tick.left_pressed = true;
                    } else if !now_down && LEFT_DOWN {
                        tick.left_released = true;
                    }
                    LEFT_DOWN = now_down;
                }
                _ => {} // EV_SYN and other buttons/axes -- unused for now
            }

            // Hand the descriptor back so the device can reuse the buffer.
            let next_avail = (*avail).idx as usize % QUEUE_SIZE;
            (*avail).ring[next_avail] = descriptor_id as u16;
            (*avail).idx = (*avail).idx.wrapping_add(1);

            LAST_SEEN_USED_IDX = LAST_SEEN_USED_IDX.wrapping_add(1);
        }

        if drained_any {
            reg_write(base, REG_QUEUE_NOTIFY, 0);
        }

        tick.left_down = LEFT_DOWN;
    }

    tick
}
