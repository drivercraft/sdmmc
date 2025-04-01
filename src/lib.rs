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