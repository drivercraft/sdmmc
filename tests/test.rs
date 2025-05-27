#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

#[bare_test::tests]
mod tests {
    use alloc::{boxed::Box, vec::Vec};
    use bare_test::{
        globals::{PlatformInfoKind, global_val},
        mem::iomap,
        println,
        time::since_boot,
    };
    use dma_api::{DVec, Direction};
    use log::{info, warn};
    use rk3568_clk::RK3568ClkPriv;
    use sdmmc::emmc::EMmcHost;
    use sdmmc::emmc::constant::*;
    use sdmmc::{
        Kernel,
        emmc::clock::{Clk, ClkError, init_global_clk},
        set_impl,
    };

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
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let fdt_parser = fdt.get();

        // Detect platform type by searching for compatible strings
        if fdt_parser
            .find_compatible(&["rockchip,rk3568-dwcmshc"])
            .next()
            .is_some()
        {
            // Rockchip platform detected, run uboot test
            info!("Rockchip platform detected, running uboot test");
            test_uboot(&fdt_parser);
        } else {
            // Unknown platform, output debug information
            println!("Unknown platform, no compatible devices found");
        }
    }

    fn test_uboot(fdt: &fdt_parser::Fdt) {
        let emmc = fdt
            .find_compatible(&["rockchip,dwcmshc-sdhci"])
            .next()
            .unwrap();
        let clock = fdt
            .find_compatible(&["rockchip,rk3568-cru"])
            .next()
            .unwrap();
        // let syscon = fdt.find_compatible(&["rockchip,rk3568-grf"]).next().unwrap();

        info!("EMMC: {} Clock: {}", emmc.name, clock.name);

        let emmc_reg = emmc.reg().unwrap().next().unwrap();
        let clk_reg = clock.reg().unwrap().next().unwrap();
        // let syscon_reg = syscon.reg().unwrap().next().unwrap();

        println!(
            "EMMC reg {:#x}, {:#x}",
            emmc_reg.address,
            emmc_reg.size.unwrap()
        );
        println!(
            "Clock reg {:#x}, {:#x}",
            clk_reg.address,
            clk_reg.size.unwrap()
        );
        // println!("Syscon reg {:#x}, {:#x}", syscon_reg.address, syscon_reg.size.unwrap());

        let emmc_addr_ptr = iomap((emmc_reg.address as usize).into(), emmc_reg.size.unwrap());
        let clk_add_ptr = iomap((clk_reg.address as usize).into(), clk_reg.size.unwrap());
        // let syscon_addr_ptr = iomap((syscon_reg.address as usize).into(), syscon_reg.size.unwrap());

        let emmc_addr = emmc_addr_ptr.as_ptr() as usize;
        let clk_addr = clk_add_ptr.as_ptr() as usize;

        test_emmc(emmc_addr, clk_addr);

        info!("test uboot");
    }

    pub struct ClkUnit(RK3568ClkPriv);

    impl ClkUnit {
        pub fn new(cru: RK3568ClkPriv) -> Self {
            ClkUnit(cru)
        }
    }

    impl Clk for ClkUnit {
        fn emmc_get_clk(&self) -> Result<u64, ClkError> {
            if let Ok(rate) = self.0.emmc_get_bclk() {
                Ok(rate)
            } else {
                Err(ClkError::InvalidClockRate)
            }
        }

        fn emmc_set_clk(&self, rate: u64) -> Result<u64, ClkError> {
            if let Ok(rate) = self.0.emmc_set_clk(rate) {
                Ok(rate)
            } else {
                Err(ClkError::InvalidClockRate)
            }
        }
    }

    fn init_clk(clk_addr: usize) -> Result<(), ClkError> {
        let cru = ClkUnit::new(unsafe { RK3568ClkPriv::new(clk_addr as *mut _) });

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

                cfg_if::cfg_if! {
                    if #[cfg(feature = "dma")] {
                        let mut buffer: DVec<u8> = DVec::zeros(MMC_MAX_BLOCK_LEN as usize, 0x1000, Direction::FromDevice).unwrap();
                    } else if #[cfg(feature = "pio")] {
                        let mut buffer: [u8; 512] = [0; 512];
                    }
                }

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

                cfg_if::cfg_if! {
                    if #[cfg(feature = "dma")] {
                        // Prepare test pattern data
                        let mut write_buffer = DVec::zeros(512, 0x1000, Direction::ToDevice).unwrap();
                        for i in 0..512 {
                            // write_buffer.set(i, (i % 256) as u8);
                            write_buffer.set(i, 0 as u8); // Fill with test pattern data
                        }
                    } else if #[cfg(feature = "pio")] {
                        let mut write_buffer: [u8; 512] = [0; 512];
                        for i in 0..512 {
                            // write_buffer[i] = (i % 256) as u8; // Fill with test pattern data
                            write_buffer[i] = 0 as u8;
                        }
                    }
                }

                // Write data
                match emmc.write_blocks(test_block_id, 1, &write_buffer) {
                    Ok(_) => {
                        println!("Successfully wrote to block {}!", test_block_id);

                        // Read back data
                        cfg_if::cfg_if! {
                            if #[cfg(feature = "dma")] {
                                let mut read_buffer: DVec<u8> = DVec::zeros(MMC_MAX_BLOCK_LEN as usize, 0x1000, Direction::FromDevice).unwrap();
                            } else if #[cfg(feature = "pio")] {
                                let mut read_buffer: [u8; 512] = [0; 512];
                            }
                        }

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
                cfg_if::cfg_if! {
                    if #[cfg(feature = "dma")] {
                        let mut multi_buffer: DVec<u8> = DVec::zeros(MMC_MAX_BLOCK_LEN as usize * block_count as usize, 0x1000, Direction::FromDevice).unwrap();
                    } else if #[cfg(feature = "pio")] {
                        // Using a fixed size of 2048 (which is 512 * 4) instead of computing it at runtime
                        let mut multi_buffer: [u8; 2048] = [0; 2048];
                    }
                }

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
}
