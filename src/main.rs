#![no_std]
#![no_main]

mod uart;
mod dw_mmc;
mod mmc_write;
pub mod constant;

use core::{arch::global_asm, panic::PanicInfo};
use uart::pl011::write_bytes;

global_asm!(include_str!("entry.asm"));

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "C" fn rust_main() -> ! {
    // Create a byte slice containing "Hello World"
    let hello = b"Hello World";

    // Use the write_bytes function to print it to the console
    write_bytes(hello);

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}