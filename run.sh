APP=sdmmc
TARGET=aarch64-unknown-none
KERNEL=kernel.bin

cargo build --release

rust-objcopy --binary-architecture=aarch64 ./target/$TARGET/release/$APP --strip-all -O binary $KERNEL

echo "Running kernel..."
qemu-system-aarch64 -m 128M -smp 1 -cpu cortex-a72 -machine virt -kernel ./$KERNEL -nographic