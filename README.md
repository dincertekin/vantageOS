# vantageOS

![Architecture](https://img.shields.io/badge/arch-ARM64-blue)
![License](https://img.shields.io/badge/license-Apache%202.0-green)
![Status](https://img.shields.io/badge/status-work%20in%20progress-orange)

vantageOS is a personal operating system built from scratch, targeting ARM64 for long battery life and solid performance. It takes inspiration from the privacy and open-source philosophy of Linux, the smooth animations of macOS, and the user-friendly interface of both. Built with [OSDev Wiki](https://wiki.osdev.org) as the primary reference.

## Tech Stack

| Layer | Tool |
|---|---|
| Assembler | NASM |
| Compiler | GCC |
| Linker | GNU LD |
| Architecture | ARM64 |

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/dincertekin/vantageOS.git
   cd vantageOS/
   ```

## Build

Assemble the bootsector, compile the kernel, then link:

```bash
nasm -f bin bootsector.asm -o bootsector.bin
gcc -ffreestanding -c kernel.c -o kernel.o
ld -T linker.ld -o hobbyOS.bin kernel.o
```

## Contributing

Contributions are welcome! To get started:

1. Fork the repository.
2. Create a new branch (`git checkout -b feature/your-feature`).
3. Commit your changes (`git commit -m 'Add your feature'`).
4. Push to the branch (`git push origin feature/your-feature`).
5. Open a Pull Request.

For major changes, please open an issue first to discuss what you'd like to change.

## License

Apache 2.0 License — see [LICENSE](./LICENSE) for details.
