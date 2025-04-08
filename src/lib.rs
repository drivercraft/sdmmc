#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use core::sync::atomic::{fence, Ordering};

pub mod sdhci;
pub mod emmc;
mod err;

/// 微秒延时函数
fn delay_us(us: u32) {
    for _ in 0..us * 10 {
        // 防止编译器优化掉的内存屏障
        fence(Ordering::SeqCst);
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