TARGET  := aarch64-unknown-none
BIN     := vantageos
QEMU    := qemu-system-aarch64

.PHONY: build release run run-release run-headless clean

build:
	cargo build

release:
	cargo build --release

# ramfb = virtual display; virtio-tablet-device = absolute pointer (not
# virtio-mouse-device, which grabs the host mouse on click). Ctrl-A C for
# the QEMU monitor, Ctrl-A X to exit.
run: build
	$(QEMU) -M virt -cpu cortex-a72 -m 256 -device ramfb -device virtio-tablet-device -serial mon:stdio -kernel target/$(TARGET)/debug/$(BIN)

run-release: release
	$(QEMU) -M virt -cpu cortex-a72 -m 256 -device ramfb -device virtio-tablet-device -serial mon:stdio -kernel target/$(TARGET)/release/$(BIN)

# Serial-only, no display window.
run-headless: build
	$(QEMU) -M virt -cpu cortex-a72 -nographic -kernel target/$(TARGET)/debug/$(BIN)

clean:
	cargo clean
