#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use bare_test::{print, println, time::since_boot};

pub mod sdhci;
pub mod emmc;
mod err;

pub fn delay_us(us: u64) {
    let start = since_boot();
    let duration = core::time::Duration::from_micros(us);
    
    while since_boot() - start < duration {
        core::hint::spin_loop();
    }
}

pub fn generic_fls(x: u32) -> u32 {
    let mut r = 32;
    let mut val = x;

    if val == 0 {
        return 0;
    }
    
    if (val & 0xffff0000) == 0 {
        val <<= 16;
        r -= 16;
    }
    
    if (val & 0xff000000) == 0 {
        val <<= 8;
        r -= 8;
    }
    
    if (val & 0xf0000000) == 0 {
        val <<= 4;
        r -= 4;
    }
    
    if (val & 0xc0000000) == 0 {
        val <<= 2;
        r -= 2;
    }
    
    if (val & 0x80000000) == 0 {
        val <<= 1;
        r -= 1;
    }
    
    r
}

pub unsafe fn dump_memory_region(addr: usize, size: usize) {
    let start_ptr = addr as *const u32;
    let word_count = size / 4; // 每个u32是4字节
    
    println!("Memory dump from 0x{:08x} to 0x{:08x}:", addr, addr + size - 1);
    
    for i in 0..word_count {
        if i % 4 == 0 {
            print!("\n0x{:08x}:", addr + i * 4);
        }
        
        // 在unsafe块中读取内存
        let value = unsafe { *start_ptr.add(i) };
        print!(" 0x{:08x}", value);
    }
    println!("");
}
