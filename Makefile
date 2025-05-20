LOG ?= INFO
FLAGS = "--cfg LOG_$(LOG) -Clink-arg=-Tsrc/linker.ld -Cforce-frame-pointers=yes"

build:
	RUSTFLAGS=$(FLAGS) cargo build --release 
	rust-objcopy --strip-all \
		target/riscv64gc-unknown-none-elf/release/os -O binary \
		target/riscv64gc-unknown-none-elf/release/os.bin

.PHONY = build load load_gdb gdb_connect

load:

	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu.bin \
		-m 64\
		-device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000

load_gdb:

	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios ../bootloader/rustsbi-qemu.bin \
		-device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \
		-s -S

gdb_connect:

	riscv64-unknown-elf-gdb \
		-ex 'file target/riscv64gc-unknown-none-elf/release/os' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'
