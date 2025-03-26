
extern crate alloc;

mod cmd;
mod block;
mod constant;

use core::{fmt::Display, sync::atomic::Ordering};
use block::SdCard;
use constant::*;
use cmd::*;
use crate::err::*;
use log::{debug, info};

#[derive(Debug, Clone, Copy)]
pub enum CardType {
    Unknown,
    Mmc,
    SdV1,
    SdV2,
    SdHc,
    MmcHc,
}

// SD Host Controller structure
#[derive(Debug)]
pub struct SdHost {
    base_addr: usize,
    card: Option<SdCard>,
    caps: u32,
    max_current: u32,
    clock_base: u32,
}

impl Display for SdHost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SDHCI Controller {{ base_addr: 0x{:#x}, card: {:?}, caps: {:#x}, max_current: 0x{:#x}, clock_base: {} }}", self.base_addr, self.card, self.caps, self.max_current, self.clock_base)
    }
}

impl SdHost {
    pub fn new(base_addr: usize) -> Self {
        let mut host = Self {
            base_addr,
            card: None,
            caps: 0,
            max_current: 0,
            clock_base: 0,
        };

        // Read capabilities
        host.caps = host.read_reg(SDHCI_CAPABILITIES);
        host.max_current = host.read_reg(SDHCI_MAX_CURRENT);

        // Calculate base clock from capabilities
        host.clock_base = (host.caps >> 8) & 0xFF;
        host.clock_base *= 1000000; // convert to Hz

        info!("SDHCI Controller created: {}", host);

        host
    }

    // Read a 32-bit register
    fn read_reg(&self, offset: u32) -> u32 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset as usize) as *const u32) }
    }

    // Read a 16-bit register
    fn read_reg16(&self, offset: u32) -> u16 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset as usize) as *const u16) }
    }

    // Read an 8-bit register
    fn read_reg8(&self, offset: u32) -> u8 {
        unsafe { core::ptr::read_volatile((self.base_addr + offset as usize) as *const u8) }
    }

    // Write a 32-bit register
    fn write_reg(&self, offset: u32, value: u32) {
        unsafe { core::ptr::write_volatile((self.base_addr + offset as usize) as *mut u32, value) }
    }

    // Write a 16-bit register
    fn write_reg16(&self, offset: u32, value: u16) {
        unsafe { core::ptr::write_volatile((self.base_addr + offset as usize) as *mut u16, value) }
    }

    // Write an 8-bit register
    fn write_reg8(&self, offset: u32, value: u8) {
        unsafe { core::ptr::write_volatile((self.base_addr + offset as usize) as *mut u8, value) }
    }

    // Initialize the host controller
    pub fn init(&mut self) -> Result<(), SdError> {
        info!("Init SDHCI Controller");

        let version = self.read_reg16(SDHCI_HOST_VERSION);
        info!("SDHCI Version: 0x{:x}", version);

        let caps = self.read_reg(SDHCI_CAPABILITIES);
        info!("SDHCI Capabilities: 0b{:b}", caps);
        let caps1 = self.read_reg(SDHCI_CAPABILITIES_1);
        info!("SDHCI Capabilities 1: 0x{:x}", caps1);

        // Reset the controller
        self.reset_all()?;

        info!("checkpoint 00");

        // Enable interrupts
        self.write_reg(SDHCI_INT_ENABLE, SDHCI_INT_ALL_MASK);
        self.write_reg(SDHCI_SIGNAL_ENABLE, SDHCI_INT_ALL_MASK);

        // Set initial clock and power
        self.set_clock(400000)?; // Start with 400 KHz for initialization

        info!("checkpoint 01");

        self.set_power(1)?; // Enable power to the card

        info!("checkpoint 02");

        // Set initial bus width to 1-bit
        let ctrl = self.read_reg8(SDHCI_HOST_CONTROL);
        self.write_reg8(SDHCI_HOST_CONTROL, ctrl & !SDHCI_CTRL_4BITBUS);

        info!("checkpoint 03");

        // Check if card is present
        if !self.is_card_present() {
            return Err(SdError::NoCard);
        }

        info!("checkpoint 04");

        // Initialize the card
        self.init_card()?;
        
        info!("checkpoint 05");

        Ok(())
    }

    // Reset the controller
    fn reset_all(&self) -> Result<(), SdError> {
        // Request reset
        self.write_reg8(SDHCI_SOFTWARE_RESET, SDHCI_RESET_ALL);

        // Wait for reset to complete
        let mut timeout = 100000;
        while (self.read_reg8(SDHCI_SOFTWARE_RESET) & SDHCI_RESET_ALL) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        Ok(())
    }

    // Reset command line
    fn reset_cmd(&self) -> Result<(), SdError> {
        self.write_reg8(SDHCI_SOFTWARE_RESET, SDHCI_RESET_CMD);

        // Wait for reset to complete
        let mut timeout = 100000;
        while (self.read_reg8(SDHCI_SOFTWARE_RESET) & SDHCI_RESET_CMD) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        Ok(())
    }

    // Reset data line
    fn reset_data(&self) -> Result<(), SdError> {
        self.write_reg8(SDHCI_SOFTWARE_RESET, SDHCI_RESET_DATA);

        // Wait for reset to complete
        let mut timeout = 100000;
        while (self.read_reg8(SDHCI_SOFTWARE_RESET) & SDHCI_RESET_DATA) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        Ok(())
    }

    // Set controller clock frequency
    fn set_clock(&self, freq: u32) -> Result<(), SdError> {
        // Disable clock first
        self.write_reg16(SDHCI_CLOCK_CONTROL, 0);

        // Calculate divider
        // Clock = base_clock / (2 * div)
        let mut div = if self.clock_base <= freq {
            0
        } else {
            ((self.clock_base + freq - 1) / freq) >> 1
        };

        // Check if divider is too large
        if div > 0xFF {
            div = 0xFF;
        }

        // Enable internal clock
        let mut clk = ((div as u16) << (SDHCI_CLOCK_DIV_SHIFT as u16)) | SDHCI_CLOCK_INT_EN;
        self.write_reg16(SDHCI_CLOCK_CONTROL, clk);

        // Wait for clock stability
        let mut timeout = 100000;
        loop {
            clk = self.read_reg16(SDHCI_CLOCK_CONTROL);
            if (clk & SDHCI_CLOCK_INT_STABLE) != 0 {
                break;
            }
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        // Enable card clock
        clk |= SDHCI_CLOCK_CARD_EN;
        self.write_reg16(SDHCI_CLOCK_CONTROL, clk);

        Ok(())
    }

    // Set power to the card
    fn set_power(&self, on: u8) -> Result<(), SdError> {
        // Set voltage level to 3.3V (0x0E)
        let pwr = if on != 0 { 0x0E | 0x01 } else { 0 };
        self.write_reg8(SDHCI_POWER_CONTROL, pwr);

        // Small delay for power to stabilize
        for _ in 0..10000 {
            let _ = self.read_reg8(SDHCI_POWER_CONTROL);
        }

        Ok(())
    }

    // Check if card is present
    fn is_card_present(&self) -> bool {
        let state = self.read_reg(SDHCI_PRESENT_STATE);
        (state & SDHCI_CARD_INSERTED) != 0
    }

    // Check if card is write protected
    fn is_write_protected(&self) -> bool {
        let state = self.read_reg(SDHCI_PRESENT_STATE);
        (state & SDHCI_WRITE_PROTECT) != 0
    }

    // Initialize the card
    fn init_card(&mut self) -> Result<(), SdError> {
        info!("checkpoint 000");

        // Send CMD0 to reset the card
        let cmd = SdCommand::new(MMC_GO_IDLE_STATE, 0, MMC_RSP_NONE);
        self.send_command(&cmd)?;

        info!("checkpoint 001");

        // Send CMD8 to check SD Card version (SD v2 specific)
        let check_pattern = 0xAA;
        let mut voltage = 0x100; // 2.7-3.6V
        let cmd = SdCommand::new(SD_SEND_IF_COND, voltage | check_pattern, MMC_RSP_R7);
        let res = self.send_command(&cmd);
        let card_type = if res.is_ok() {
            // SD v2 or later
            let response = self.get_response().as_r7();
            if (response & 0xFF) == check_pattern {
                voltage = (response >> 8) & 0xF;
                info!("SD v2 card detected, voltage: 0x{:x}", voltage);
                CardType::SdV2
            } else {
                return Err(SdError::UnsupportedCard);
            }
        } else {
            // SD v1 or MMC
            CardType::SdV1
        };

        info!("checkpoint 002 {:?}", card_type);

        // Initialize the card
        let mut card = SdCard::init(self.base_addr, card_type);

        // Send ACMD41 to initialize the card
        let mut ocr = 0x40FF8000; // HCS = 1, voltage window = 2.7-3.6V
        let mut retry = 100;
        let mut ready = false;

        info!("checkpoint 005");

        while retry > 0 && !ready {
            // Send CMD55 (APP_CMD) before ACMD
            let cmd = SdCommand::new(MMC_APP_CMD, card.rca, MMC_RSP_R1);
            self.send_command(&cmd)?;

            // Send ACMD41
            let cmd = SdCommand::new(SD_APP_OP_COND, ocr, MMC_RSP_R3);
            self.send_command(&cmd)?;
            let response = self.get_response();
            ocr = response.as_r3();

            // Check if card is ready
            if (ocr & (1 << 31)) != 0 {
                ready = true;
                card.ocr = ocr;
                if (ocr & (1 << 30)) != 0 {
                    card.card_type = CardType::SdHc;
                    card.state |= MMC_STATE_HIGHCAPACITY;
                }
            } else {
                retry -= 1;
                // Small delay
                for _ in 0..10000 {
                    let _ = self.read_reg8(SDHCI_POWER_CONTROL);
                }
            }
        }

        info!("checkpoint 003");

        if !ready {
            return Err(SdError::UnsupportedCard);
        }

        info!("checkpoint 004");

        // Send CMD2 to get CID
        let cmd = SdCommand::new(MMC_ALL_SEND_CID, 0, MMC_RSP_R2);
        self.send_command(&cmd)?;
        let response = self.get_response();
        card.cid = response.as_r2();

        // Send CMD3 to get RCA
        let cmd = SdCommand::new(SD_SEND_RELATIVE_ADDR, 0, MMC_RSP_R6);
        self.send_command(&cmd)?;
        let response = self.get_response();
        card.rca = response.as_r6() & 0xFFFF0000;

        // Send CMD9 to get CSD
        let cmd = SdCommand::new(MMC_SEND_CSD, card.rca, MMC_RSP_R2);
        self.send_command(&cmd)?;
        let response = self.get_response();
        card.csd = response.as_r2();

        info!("SD Card initialized: {:b} {:b} {:b} {:b}", response.as_r2()[0], response.as_r2()[1], response.as_r2()[2], response.as_r2()[3]);

        // Calculate card capacity from CSD
        // 长响应（136位）：R[127:8] → REP[119:0]（CID/CSD数据，忽略CRC和起始位）
        let csd_structure = card.csd[3] >> 22;
        debug!("csd_structure {:b}", csd_structure);

        // Add check for unknown CSD structure
        if csd_structure != 0 && csd_structure != 1 {
            panic!("Unknown CSD structure version: {}", csd_structure);
        }

        // Calculate card capacity
        if csd_structure == 0 {
            // CSD version 1.0 (Standard SD Card)

            // [0:31] [32:63] [64:95] [96:119]
            // [73:62] (-8) -> [54:65] -> [54:63] [64: 65] -> 1 [22:31] 2 [0:1]
            // [49:47] (-8) -> [41:39]
            // [69:48] (-8) -> [61:40]

            // 119 <- 01000000 -> 112
            // 111 <- 00001110 -> 104
            // 103 <- 00000000 -> 96

            //  95 <- 00110010 -> 88
            //  87 <- 01011011 -> 80
            //  79 <- 01011001 -> 72
            //  71 <- 00000000 -> 64
            
            //  63 <- 00000000 -> 56
            //  55 <- 00011111 -> 48
            //  47 <- 11111111 -> 40
            //  39 <- 01111111 -> 32

            //  31 <- 10000000 -> 24
            //  23 <- 00001010 -> 16
            //  15 <- 01000000 -> 8
            //   7 <- 00000000 -> 0

            let c_size = ((card.csd[2] & 0x3) << 10) | ((card.csd[1] >> 22) & 0x3FF);
            let c_size_mult = (card.csd[1] >> 7) & 0x7;
            let read_bl_len = (card.csd[2] >> 8) & 0xF;
            
            debug!("c_size: {}, c_size_mult: {}, read_bl_len: {}", c_size, c_size_mult, read_bl_len);
            
            // Ensure read_bl_len is within reasonable range
            if read_bl_len > 12 {
                panic!("Invalid READ_BL_LEN value: {}", read_bl_len);
            }
            
            let block_size = 1 << read_bl_len;
            let mult = 1 << (c_size_mult + 2);
            let capacity = (c_size + 1) as u64 * mult as u64 * block_size as u64;
            card.capacity_blocks = capacity / 512;
            
            debug!("SD v1 card capacity: {} blocks ({} bytes)", 
                card.capacity_blocks, card.capacity_blocks * 512);
        } else if csd_structure == 1 {
            // CSD version 2.0 (SDHC/SDXC)
            let c_size = (card.csd[1] >> 8) & 0x3FFFFF;
            debug!("c_size: {:b}", c_size);
            
            card.capacity_blocks = (c_size + 1) as u64 * 1024;
            debug!("SD v2 card capacity: {} blocks ({} MB)", 
                card.capacity_blocks, (card.capacity_blocks * 512) / (1024 * 1024));
        }

        // Set block size, SDHC cards are fixed at 512 bytes
        card.block_size = 512;

        // Send CMD7 to select the card
        let cmd = SdCommand::new(MMC_SELECT_CARD, card.rca, MMC_RSP_R1B);
        self.send_command(&cmd)?;

        // Set block size to 512 bytes
        let cmd = SdCommand::new(MMC_SET_BLOCKLEN, 512, MMC_RSP_R1);
        self.send_command(&cmd)?;

        // Switch to 4-bit bus width if supported
        if !self.is_write_protected() {
            // Send CMD55 (APP_CMD) before ACMD
            let cmd = SdCommand::new(MMC_APP_CMD, card.rca, MMC_RSP_R1);
            self.send_command(&cmd)?;

            // Send ACMD6 to set bus width
            let cmd = SdCommand::new(6, 2, MMC_RSP_R1); // ACMD6, arg=2 for 4-bit bus width
            self.send_command(&cmd)?;

            // Now set the controller's bus width
            let ctrl = self.read_reg8(SDHCI_HOST_CONTROL);
            self.write_reg8(SDHCI_HOST_CONTROL, ctrl | SDHCI_CTRL_4BITBUS);
        }

        // Set higher clock speed for data transfer
        self.set_clock(25000000)?; // 25 MHz for SD cards

        // Card is initialized
        card.initialized.store(true, Ordering::SeqCst);
        card.state |= MMC_STATE_PRESENT;

        // Store the card in the host
        self.card = Some(card);

        Ok(())
    }
    
    // Get card status
    pub fn get_status(&self) -> Result<u32, SdError> {
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Send SEND_STATUS command
        let cmd = SdCommand::new(MMC_SEND_STATUS, card.rca, MMC_RSP_R1);
        self.send_command(&cmd)?;
        let response = self.get_response();

        Ok(response.as_r1())
    }

    // Get card info
    pub fn get_card_info(&self) -> Result<CardInfo, SdError> {
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Extract information from CID
        let cid = card.cid;
        
        // SD card CID format
        let manufacturer_id = (cid[0] >> 24) as u8;
        let application_id = ((cid[0] >> 8) & 0xFFFF) as u16;
        let serial_number = ((cid[0] & 0xFF) << 24) | ((cid[1] >> 8) & 0xFFFFFF);

        // Extract manufacturing date
        let manufacturing_year = (((cid[1] & 0xF) << 4) | ((cid[2] >> 28) & 0xF)) as u16 + 2000;
        let manufacturing_month = ((cid[2] >> 24) & 0xF) as u8;

        let card_info = CardInfo {
            card_type: card.card_type,
            manufacturer_id,
            application_id,
            serial_number,
            manufacturing_month,
            manufacturing_year,
            capacity_bytes: card.capacity_blocks * 512,
            block_size: 512,
        };

        Ok(card_info)
    }

    // Get card capacity in bytes
    pub fn get_capacity(&self) -> Result<u64, SdError> {
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        Ok(card.capacity_blocks * 512)
    }
}

// Card information structure
#[derive(Debug)]
pub struct CardInfo {
    pub card_type: CardType,
    pub manufacturer_id: u8,
    pub application_id: u16,
    pub serial_number: u32,
    pub manufacturing_month: u8,
    pub manufacturing_year: u16,
    pub capacity_bytes: u64,
    pub block_size: u32,
}

