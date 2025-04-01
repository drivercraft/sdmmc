#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

#[bare_test::tests]
mod tests {
    use bare_test::{globals::{global_val, PlatformInfoKind}, mem::iomap, platform::page_size, println};
    use fdt_parser::PciSpace;
    use log::{debug, info, warn};
    use pcie::{CommandRegister, DeviceType, Header, RootComplexGeneric, SimpleBarAllocator};
    use sdmmc::sdhci::SdHost;
    use sdmmc::emmc::EMmcHost;

    #[test]
    fn test_platform() {
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let fdt_parser = fdt.get();
        
        // Detect platform type by searching for compatible strings
        if fdt_parser.find_compatible(&["rockchip,rk3568-dwcmshc"]).next().is_some() {
            // Rockchip platform detected, run uboot test
            info!("Rockchip platform detected, running uboot test");
            test_uboot(&fdt_parser);
        } else if fdt_parser.find_compatible(&["pci-host-ecam-generic"]).next().is_some() {
            // QEMU platform detected, run qemu test
            info!("QEMU platform detected, running qemu test");
            test_qemu(&fdt_parser);
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

        info!("EMMC: {}", emmc.name);
    
        let reg = emmc.reg().unwrap().next().unwrap();
        println!("EMMC reg {:#x}, {:#x}", reg.address, reg.size.unwrap());
    
        let addr_ptr = iomap((reg.address as usize).into(), reg.size.unwrap());
    
        let addr = addr_ptr.as_ptr() as usize;

        test_emmc(addr);

        info!("test uboot");
    }

    fn test_qemu(fdt: &fdt_parser::Fdt) {
        let pcie = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .next()
            .unwrap()
            .into_pci()
            .unwrap();
    
        let mut pcie_regs = alloc::vec![];
    
        println!("test sdmmc");
    
        println!("pcie: {}", pcie.node.name);
    
        for reg in pcie.node.reg().unwrap() {
            println!("pcie reg: {:#x}", reg.address);
            pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
        }
    
        let mut bar_alloc = SimpleBarAllocator::default();
    
        for range in pcie.ranges().unwrap() {
            info!("pcie range: {:?}", range);
    
            match range.space {
                PciSpace::Memory32 => bar_alloc.set_mem32(range.cpu_address as _, range.size as _),
                PciSpace::Memory64 => bar_alloc.set_mem64(range.cpu_address, range.size),
                _ => {}
            }
        }
    
        let base_vaddr = pcie_regs[0];
    
        info!("Init PCIE @{:?}", base_vaddr);
    
        info!("Page size: {}", page_size());
    
        let mut root = RootComplexGeneric::new(base_vaddr);
    
        for elem in root.enumerate(None, Some(bar_alloc)) {
            debug!("PCI {}", elem);
            if let Header::Endpoint(ep) = elem.header {
                ep.update_command(elem.root, |cmd| {
                    cmd | CommandRegister::IO_ENABLE
                        | CommandRegister::MEMORY_ENABLE
                        | CommandRegister::BUS_MASTER_ENABLE
                });
                
                if ep.device_type() == DeviceType::SdHostController {
                    let bar_addr;
                    let bar_size;
                    match ep.bar {
                        pcie::BarVec::Memory32(bar_vec_t) => {
                            let bar0 = bar_vec_t[0].as_ref().unwrap();
                            bar_addr = bar0.address as usize;
                            bar_size = bar0.size as usize;
                        }
                        pcie::BarVec::Memory64(bar_vec_t) => {
                            let bar0 = bar_vec_t[0].as_ref().unwrap();
                            bar_addr = bar0.address as usize;
                            bar_size = bar0.size as usize;
                        }
                        pcie::BarVec::Io(_bar_vec_t) => todo!(),
                    };
    
                    println!("sdmmc bar_addr: {:#x}, bar_size {:#x}", bar_addr, bar_size);
    
                    let addr_ptr = iomap(bar_addr.into(), bar_size);
                    let addr = addr_ptr.as_ptr() as usize;
    
                    test_sdhci(addr);
                    return;
                }
            }
        }
    
        println!("No SD host controller found");
    }

    fn test_sdhci(addr: usize) {
        // Initialize custom SDHCI controller
        let mut sdhci = SdHost::new(addr);

        // Try to initialize the SD card
        match sdhci.init() {
            Ok(_) => {
                println!("SD card initialization successful!");
                
                // Get card information
                match sdhci.get_card_info() {
                    Ok(card_info) => {
                        println!("Card type: {:?}", card_info.card_type);
                        println!("Manufacturer ID: 0x{:02X}", card_info.manufacturer_id);
                        println!("Capacity: {} MB", card_info.capacity_bytes / (1024 * 1024));
                        println!("Block size: {} bytes", card_info.block_size);
                    },
                    Err(e) => {
                        warn!("Failed to get card info: {:?}", e);
                    }
                }
                
                // // Test reading the first block
                // println!("Attempting to read first block...");
                // let mut buffer = [0u8; 512];
                // match sdhci.read_block(0, &mut buffer) {
                //     Ok(_) => {
                //         println!("Successfully read first block!");
                //         println!("First 16 bytes: {:02X?}", &buffer[0..16]);
                //     },
                //     Err(e) => {
                //         warn!("Block read failed: {:?}", e);
                //     }
                // }
                
                // // Test writing and reading back a block
                // println!("Testing write and read back...");
                // let test_block_addr = 100; // Use a safe block address for testing
                
                // // Prepare test pattern data
                // let mut write_buffer = [0u8; 512];
                // for i in 0..512 {
                //     write_buffer[i] = (i % 256) as u8;
                // }
                
                // // Write data
                // match sdhci.write_block(test_block_addr, &write_buffer) {
                //     Ok(_) => {
                //         println!("Successfully wrote to block {}!", test_block_addr);
                        
                //         // Read back data
                //         let mut read_buffer = [0u8; 512];
                //         match sdhci.read_block(test_block_addr, &mut read_buffer) {
                //             Ok(_) => {
                //                 println!("Successfully read back block {}!", test_block_addr);
                                
                //                 // Verify data consistency
                //                 let mut data_match = true;
                //                 for i in 0..512 {
                //                     if write_buffer[i] != read_buffer[i] {
                //                         data_match = false;
                //                         println!("Data mismatch: offset {}, wrote {:02X}, read {:02X}",
                //                                 i, write_buffer[i], read_buffer[i]);
                //                         break;
                //                     }
                //                 }
                                
                //                 if data_match {
                //                     println!("Data verification successful: written and read data match perfectly!");
                //                 } else {
                //                     println!("Data verification failed: written and read data do not match!");
                //                 }
                //             },
                //             Err(e) => {
                //                 warn!("Failed to read back block: {:?}", e);
                //             }
                //         }
                //     },
                //     Err(e) => {
                //         warn!("Block write failed: {:?}", e);
                //     }
                // }
                
                // // Test multi-block read
                // println!("Testing multi-block read...");
                // let multi_block_addr = 200;
                // let block_count = 4; // Read 4 blocks
                // let mut multi_buffer = vec![0u8; 512 * block_count as usize];
                
                // match sdhci.read_blocks(multi_block_addr, block_count, &mut multi_buffer) {
                //     Ok(_) => {
                //         println!("Successfully read {} blocks starting at block address {}!", block_count, multi_block_addr);
                //         println!("First 16 bytes of first block: {:02X?}", &multi_buffer[0..16]);
                //         println!("First 16 bytes of last block: {:02X?}", 
                //                 &multi_buffer[(block_count as usize - 1) * 512..(block_count as usize - 1) * 512 + 16]);
                //     },
                //     Err(e) => {
                //         warn!("Multi-block read failed: {:?}", e);
                //     }
                // }
            },
            Err(e) => {
                warn!("SD card initialization failed: {:?}", e);
            }
        }

        // Test complete
        println!("SD card test complete");
    }

    fn test_emmc(addr: usize) {
        // Initialize custom SDHCI controller
        let mut sdhci = EMmcHost::new(addr);

        // Try to initialize the SD card
        match sdhci.init() {
            Ok(_) => {
                println!("SD card initialization successful!");
                
                // Get card information
                match sdhci.get_card_info() {
                    Ok(card_info) => {
                        println!("Card type: {:?}", card_info.card_type);
                        println!("Manufacturer ID: 0x{:02X}", card_info.manufacturer_id);
                        println!("Capacity: {} MB", card_info.capacity_bytes / (1024 * 1024));
                        println!("Block size: {} bytes", card_info.block_size);
                    },
                    Err(e) => {
                        warn!("Failed to get card info: {:?}", e);
                    }
                }
                
                // // Test reading the first block
                // println!("Attempting to read first block...");
                // let mut buffer = [0u8; 512];
                // match sdhci.read_block(0, &mut buffer) {
                //     Ok(_) => {
                //         println!("Successfully read first block!");
                //         println!("First 16 bytes: {:02X?}", &buffer[0..16]);
                //     },
                //     Err(e) => {
                //         warn!("Block read failed: {:?}", e);
                //     }
                // }
                
                // // Test writing and reading back a block
                // println!("Testing write and read back...");
                // let test_block_addr = 100; // Use a safe block address for testing
                
                // // Prepare test pattern data
                // let mut write_buffer = [0u8; 512];
                // for i in 0..512 {
                //     write_buffer[i] = (i % 256) as u8;
                // }
                
                // // Write data
                // match sdhci.write_block(test_block_addr, &write_buffer) {
                //     Ok(_) => {
                //         println!("Successfully wrote to block {}!", test_block_addr);
                        
                //         // Read back data
                //         let mut read_buffer = [0u8; 512];
                //         match sdhci.read_block(test_block_addr, &mut read_buffer) {
                //             Ok(_) => {
                //                 println!("Successfully read back block {}!", test_block_addr);
                                
                //                 // Verify data consistency
                //                 let mut data_match = true;
                //                 for i in 0..512 {
                //                     if write_buffer[i] != read_buffer[i] {
                //                         data_match = false;
                //                         println!("Data mismatch: offset {}, wrote {:02X}, read {:02X}",
                //                                 i, write_buffer[i], read_buffer[i]);
                //                         break;
                //                     }
                //                 }
                                
                //                 if data_match {
                //                     println!("Data verification successful: written and read data match perfectly!");
                //                 } else {
                //                     println!("Data verification failed: written and read data do not match!");
                //                 }
                //             },
                //             Err(e) => {
                //                 warn!("Failed to read back block: {:?}", e);
                //             }
                //         }
                //     },
                //     Err(e) => {
                //         warn!("Block write failed: {:?}", e);
                //     }
                // }
                
                // // Test multi-block read
                // println!("Testing multi-block read...");
                // let multi_block_addr = 200;
                // let block_count = 4; // Read 4 blocks
                // let mut multi_buffer = vec![0u8; 512 * block_count as usize];
                
                // match sdhci.read_blocks(multi_block_addr, block_count, &mut multi_buffer) {
                //     Ok(_) => {
                //         println!("Successfully read {} blocks starting at block address {}!", block_count, multi_block_addr);
                //         println!("First 16 bytes of first block: {:02X?}", &multi_buffer[0..16]);
                //         println!("First 16 bytes of last block: {:02X?}", 
                //                 &multi_buffer[(block_count as usize - 1) * 512..(block_count as usize - 1) * 512 + 16]);
                //     },
                //     Err(e) => {
                //         warn!("Multi-block read failed: {:?}", e);
                //     }
                // }
            },
            Err(e) => {
                warn!("SD card initialization failed: {:?}", e);
            }
        }

        // Test complete
        println!("SD card test complete");
    }
}