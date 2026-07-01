//! Powers off via PSCI (ARM's standard firmware power interface) -- unlike
//! the rest of this kernel, not a QEMU-only trick, so it should work on
//! real hardware too. Uses `hvc`, not `smc`: `smc` hangs under this boot
//! setup (no vector table to catch the trap it expects).

/// PSCI `SYSTEM_OFF` function ID. No return on success.
const PSCI_SYSTEM_OFF: u32 = 0x8400_0008;

/// Powers off and never returns. Spins if firmware somehow declines --
/// nothing sensible left to do.
pub fn shutdown() -> ! {
    unsafe {
        core::arch::asm!(
            "hvc #0",
            in("w0") PSCI_SYSTEM_OFF,
            options(nomem, nostack),
        );
    }
    loop {
        core::hint::spin_loop();
    }
}
