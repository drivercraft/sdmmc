#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

#[bare_test::tests]
mod tests {
    use alloc::{boxed::Box, vec::Vec};
    use bare_test::{
        globals::{PlatformInfoKind, global_val},
        mem::{iomap, page_size},
        println,
        time::since_boot,
    };
    use dma_api::{DVec, Direction};
    use log::{info, warn};
    use rk3588_clk::{constant::*, Rk3588Cru};
    use sdmmc::emmc::EMmcHost;
    use sdmmc::emmc::constant::*;
    use sdmmc::{
        Kernel,
        emmc::clock::{Clk, ClkError, init_global_clk},
        set_impl,
    };

    use core::ptr::NonNull;

    struct SKernel;

    impl Kernel for SKernel {
        fn sleep(us: u64) {
            let start = since_boot();
            let duration = core::time::Duration::from_micros(us);

            while since_boot() - start < duration {
                core::hint::spin_loop();
            }
        }
    }

    set_impl!(SKernel);

    #[test]
    fn test_platform() {
        let emmc_addr_ptr = get_device_addr("rockchip,dwcmshc-sdhci");
        let clk_add_ptr = get_device_addr("rockchip,rk3588-cru");
        
        info!("emmc ptr: {:p}", emmc_addr_ptr);
        info!("clk ptr: {:p}", clk_add_ptr);

        let emmc_addr = emmc_addr_ptr.as_ptr() as usize;
        let clk_addr = clk_add_ptr.as_ptr() as usize;

        info!("emmc addr: {:#x}", emmc_addr);
        info!("clk addr: {:#x}", clk_addr);

        test_emmc(emmc_addr, clk_addr);

        info!("test uboot");
    }

    pub struct ClkUnit(Rk3588Cru);

    impl ClkUnit {
        pub fn new(cru: Rk3588Cru) -> Self {
            ClkUnit(cru)
        }
    }

    impl Clk for ClkUnit {
        fn emmc_get_clk(&self) -> Result<u64, ClkError> {
            if let Ok(rate) = self.0.mmc_get_clk(CCLK_EMMC) {
                Ok(rate as u64)
            } else {
                Err(ClkError::InvalidClockRate)
            }
        }

        fn emmc_set_clk(&self, rate: u64) -> Result<u64, ClkError> {
            if let Ok(rate) = self.0.mmc_set_clk(CCLK_EMMC, rate as usize) {
                Ok(rate as u64)
            } else {
                Err(ClkError::InvalidClockRate)
            }
        }
    }

    fn init_clk(clk_addr: usize) -> Result<(), ClkError> {
        let cru = ClkUnit::new(Rk3588Cru::new(
            core::ptr::NonNull::new(clk_addr as *mut u8).unwrap(),
        ));

        let static_clk: &'static dyn Clk = Box::leak(Box::new(cru));
        init_global_clk(static_clk);
        Ok(())
    }

    fn test_emmc(emmc_addr: usize, clock: usize) {
        // Initialize custom SDHCI controller
        let mut emmc = EMmcHost::new(emmc_addr);
        let _ = init_clk(clock);

        // Try to initialize the SD card
        match emmc.init() {
            Ok(_) => {
                println!("SD card initialization successful!");

                // Get card information
                match emmc.get_card_info() {
                    Ok(card_info) => {
                        println!("Card type: {:?}", card_info.card_type);
                        println!("Manufacturer ID: 0x{:02X}", card_info.manufacturer_id);
                        println!("Capacity: {} MB", card_info.capacity_bytes / (1024 * 1024));
                        println!("Block size: {} bytes", card_info.block_size);
                    }
                    Err(e) => {
                        warn!("Failed to get card info: {:?}", e);
                    }
                }

                // Test reading the first block
                println!("Attempting to read first block...");
                let mut buffer: [u8; 512] = [0; 512];

                match emmc.read_blocks(5034498, 1, &mut buffer) {
                    Ok(_) => {
                        println!("Successfully read first block!");
                        let block_bytes: Vec<u8> = (0..512).map(|i| buffer[i]).collect();
                        println!("First 16 bytes of first block: {:02X?}", block_bytes);
                    }
                    Err(e) => {
                        warn!("Block read failed: {:?}", e);
                    }
                }

                // Test writing and reading back a block
                println!("Testing write and read back...");
                let test_block_id = 0x3; // Use a safe block address for testing

                let mut write_buffer: [u8; 512] = [0; 512];
                for i in 0..512 {
                    // write_buffer[i] = (i % 256) as u8; // Fill with test pattern data
                    write_buffer[i] = 0 as u8;
                }

                // Write data
                match emmc.write_blocks(test_block_id, 1, &write_buffer) {
                    Ok(_) => {
                        println!("Successfully wrote to block {}!", test_block_id);

                        // Read back data
                        let mut read_buffer: [u8; 512] = [0; 512];

                        match emmc.read_blocks(test_block_id, 1, &mut read_buffer) {
                            Ok(_) => {
                                println!("Successfully read back block {}!", test_block_id);

                                // Verify data consistency
                                let mut data_match = true;
                                for i in 0..512 {
                                    if write_buffer[i] != read_buffer[i] {
                                        data_match = false;
                                        println!(
                                            "Data mismatch: offset {}, wrote {:02X}, read {:02X}",
                                            i, write_buffer[i], read_buffer[i]
                                        );
                                        break;
                                    }
                                }

                                println!(
                                    "First 16 bytes of read block: {:?}",
                                    read_buffer.to_vec()
                                );

                                if data_match {
                                    println!(
                                        "Data verification successful: written and read data match perfectly!"
                                    );
                                } else {
                                    println!(
                                        "Data verification failed: written and read data do not match!"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read back block: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Block write failed: {:?}", e);
                    }
                }

                // Test multi-block read
                println!("Testing multi-block read...");
                let multi_block_addr = 200;
                let block_count = 4; // Read 4 blocks

                // Using a fixed size of 2048 (which is 512 * 4) instead of computing it at runtime
                let mut multi_buffer: [u8; 2048] = [0; 2048];

                match emmc.read_blocks(multi_block_addr, block_count, &mut multi_buffer) {
                    Ok(_) => {
                        println!(
                            "Successfully read {} blocks starting at block address {}!",
                            block_count, multi_block_addr
                        );

                        let first_block_bytes: Vec<u8> = (0..16).map(|i| multi_buffer[i]).collect();
                        println!("First 16 bytes of first block: {:02X?}", first_block_bytes);

                        let last_block_offset = (block_count as usize - 1) * 512;
                        let last_block_bytes: Vec<u8> = (0..16)
                            .map(|i| multi_buffer[last_block_offset + i])
                            .collect();
                        println!("First 16 bytes of last block: {:02X?}", last_block_bytes);
                    }
                    Err(e) => {
                        warn!("Multi-block read failed: {:?}", e);
                    }
                }
            }
            Err(e) => {
                warn!("SD card initialization failed: {:?}", e);
            }
        }

        // Test complete
        println!("SD card test complete");
    }

    fn get_device_addr(dtc_str: &str) -> NonNull<u8> {
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let fdt = fdt.get();

        let binding = [dtc_str];
        let node = fdt
            .find_compatible(&binding)
            .next()
            .expect("Failed to find syscon node");

        info!("Found node: {}", node.name());

        let regs = node.reg().unwrap().collect::<Vec<_>>();
        let start = regs[0].address as usize;
        let end = start + regs[0].size.unwrap_or(0);
        info!("Syscon address range: 0x{:x} - 0x{:x}", start, end);
        let start = start & !(page_size() - 1);
        let end = (end + page_size() - 1) & !(page_size() - 1);
        info!("Aligned Syscon address range: 0x{:x} - 0x{:x}", start, end);
        iomap(start.into(), end - start)
    }
}