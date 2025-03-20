//! PL011 UART.
use arm_pl011::Pl011Uart;
use kspin::SpinNoIrq;

const UART_BASE: usize = 0x0900_0000;

static UART: SpinNoIrq<Pl011Uart> =
    SpinNoIrq::new(Pl011Uart::new(UART_BASE as *mut u8));

/// Writes a byte to the console.
pub fn putchar(c: u8) {
    let mut uart = UART.lock();
    match c {
        b'\n' => {
            uart.putchar(b'\r');
            uart.putchar(b'\n');
        }
        c => uart.putchar(c),
    }
}

/// Reads a byte from the console, or returns [`None`] if no input is available.
fn getchar() -> Option<u8> {
    UART.lock().getchar()
}

/// Write a slice of bytes to the console.
pub fn write_bytes(bytes: &[u8]) {
    for c in bytes {
        putchar(*c);
    }
}

/// Reads bytes from the console into the given mutable slice.
/// Returns the number of bytes read.
pub fn read_bytes(bytes: &mut [u8]) -> usize {
    let mut read_len = 0;
    while read_len < bytes.len() {
        if let Some(c) = getchar() {
            bytes[read_len] = c;
        } else {
            break;
        }
        read_len += 1;
    }
    read_len
}

/// Initialize the UART
pub fn init_early() {
    UART.lock().init();
}
