.section ".text.boot"
.global _start

_start:
    // Park every core except core 0; vantageOS is single-core for now.
    mrs x1, mpidr_el1
    and x1, x1, #3
    cbz x1, 2f
1:  wfe
    b 1b

2:  // Core 0: set up the stack pointer.
    ldr x1, =_stack_top
    mov sp, x1

    // Allow EL0/EL1 to use FP/SIMD registers. rustc happily emits NEON
    // instructions (e.g. `movi`/`str q0` to zero a buffer) even in code
    // that never touches a `f32`/`f64` -- without this, the first one
    // traps, and since we haven't installed an exception vector table
    // (VBAR_EL1 defaults to 0) that trap has nowhere sane to go.
    mrs x1, cpacr_el1
    orr x1, x1, #(0x3 << 20) // FPEN = 0b11
    msr cpacr_el1, x1
    isb

    // Zero the .bss section before any Rust code touches statics.
    ldr x1, =__bss_start
    ldr x2, =__bss_end
    sub x2, x2, x1
    cbz x2, 4f
3:  str xzr, [x1], #8
    subs x2, x2, #8
    b.gt 3b

4:  bl kernel_main

    // kernel_main never returns; park the core if it somehow does.
5:  wfe
    b 5b
