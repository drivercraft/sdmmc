#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

pub mod emmc;
pub mod err;

use log::warn;
use core::{arch::asm, sync::atomic::{fence, Ordering}};

pub const BLOCK_SIZE: usize = 512;

pub unsafe fn dump_memory_region(addr: usize, size: usize) {
    let start_ptr = addr as *const u32;
    let word_count = size / 4; // 每个u32是4字节
    
    warn!("Memory dump from 0x{:08x} to 0x{:08x}:", addr, addr + size - 1);
    
    for i in 0..word_count {
        if i % 4 == 0 {
            warn!("\n0x{:08x}:", addr + i * 4);
        }
        
        let value = unsafe { *start_ptr.add(i) };
        warn!(" 0x{:08x}", value);
    }
    warn!("");
}

// pub trait Kernel {
//     fn sleep(us: u64);
// }

// pub(crate) fn delay_us(us: u64) {
//     unsafe extern "Rust" {
//         fn delay_us(us: u64);
//     }

//     unsafe {
//         delay_us(us);
//     }
// }

// #[macro_export]
// macro_rules! set_impl {
//     ($t: ty) => {
//         #[unsafe(no_mangle)]
//         unsafe fn delay_us(us: u64) {
//             <$t as $crate::Kernel>::sleep(us)
//         }
//     };
// }

/// 微秒延时函数
fn delay_us(us: u32) {
    let nop_count = 20; // 这个值需要根据特定的 ARM 处理器和时钟频率进行校准
    
    for _ in 0..us {
        for _ in 0..nop_count {
            unsafe { asm!("nop"); }
        }
    }
}