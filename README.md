# vantageOS
A personal operating system built from scratch.

## Description
vantageOS is a personal operating system project built from scratch, combining the privacy and open-source philosophy of Linux, the smoothness and animations of macOS, and the user-friendly interface of both Mac and Windows. Targeting ARM64 architecture for long-lasting battery life and solid performance, this project is a work in progress that grows a little better every day.

## Getting Started

### Dependencies
* NASM (assembler)
* GCC (C compiler)
* LD (GNU Linker)

### Installing
* Clone the repository:
```bash
git clone https://github.com/dincertekin/vantageOS.git
cd vantageOS/
```

### Executing program
* Assemble the bootsector:
```bash
nasm -f bin bootsector.asm -o bootsector.bin
```

* Build the kernel:
```bash
gcc -ffreestanding -c kernel.c -o kernel.o
```

* Link the files:
```bash
ld -T linker.ld -o hobbyOS.bin kernel.o
```

## Help
Refer to the [OSDev Wiki](https://wiki.osdev.org) for guidance on OS development concepts and troubleshooting.

## Contributing
Contributions are welcome! To get started:
1. Fork the repository
2. Create a new branch (`git checkout -b feature/your-feature`)
3. Commit your changes (`git commit -m 'Add your feature'`)
4. Push to the branch (`git push origin feature/your-feature`)
5. Open a Pull Request

Please open an issue first for major changes to discuss what you'd like to change.

## License
This project is licensed under the [Apache-2.0](LICENSE) License - see the LICENSE.md file for details

## Acknowledgments
* [OSDev Wiki](https://wiki.osdev.org) — the backbone of this project
