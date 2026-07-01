/* Link address matches where QEMU's `virt` machine maps RAM (0x4000_0000),
   offset by 0x8_0000 to mirror the Linux Image boot convention that most
   AArch64 bare-metal tutorials and bootloaders expect. */
ENTRY(_start)

SECTIONS
{
    . = 0x40080000;

    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    . = ALIGN(8);
    __bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    __bss_end = .;

    . = ALIGN(16);
    . += 0x4000; /* 16 KiB boot stack */
    _stack_top = .;

    /DISCARD/ : {
        *(.comment)
        *(.note .note.*)
        *(.eh_frame .eh_frame_hdr)
    }
}
