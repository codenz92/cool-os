.PHONY: run build clean

TARGET  := x86_64-unknown-none.json
KERNEL  := $(CURDIR)/target/x86_64-unknown-none/release/cool_os
BIOS    := $(CURDIR)/target/x86_64-unknown-none/release/bios.img
FSIMG   := $(CURDIR)/target/x86_64-unknown-none/release/fs.img

run: build
	@echo "Booting coolOS in QEMU..."
	qemu-system-x86_64 \
		-drive format=raw,file="$(BIOS)" \
		-drive file="$(FSIMG)",if=ide,format=raw,index=1 \
		-m 512M \
		-vga std \
		-display cocoa \
		-debugcon stdio

build:
	cargo build --release --target $(TARGET) \
		-Z build-std=core,compiler_builtins,alloc \
		-Z build-std-features=compiler-builtins-mem
	(cd disk-image && cargo run --bin disk-image -- "$(KERNEL)")
	(cd disk-image && cargo run --bin fs-image -- "$(FSIMG)")

clean:
	cargo clean
	rm -rf target
	rm -rf disk-image/target
