extern crate alloc;

mod cmd;
mod block;
mod constant;

use core::{fmt::Display, sync::atomic::Ordering};
use block::EMmcCard;
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
pub struct EMmcHost {
    base_addr: usize,
    card: Option<EMmcCard>,
    caps: u32,
    clock_base: u32,
}

impl Display for EMmcHost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "EMMC Controller {{ base_addr: 0x{:#x}, card: {:?}, caps: {:#x}, clock_base: {} }}", self.base_addr, self.card, self.caps, self.clock_base)
    }
}

impl EMmcHost {
    pub fn new(base_addr: usize) -> Self {
        let mut host = Self {
            base_addr,
            card: None,
            caps: 0,
            clock_base: 0,
        };

        // Read capabilities
        host.caps = host.read_reg(EMMC_CAPABILITIES1);
        // host.max_current = host.read_reg(EMMC_MAX_CURRENT);

        // Calculate base clock from capabilities
        host.clock_base = (host.caps >> 8) & 0xFF;
        host.clock_base *= 1000000; // convert to Hz

        info!("EMMC Controller created: {}", host);

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
        info!("Init EMMC Controller");

        let version = self.read_reg16(EMMC_HOST_CNTRL_VER);
        info!("EMMC Version: 0x{:x}", version);

        let caps1 = self.read_reg(EMMC_CAPABILITIES1);
        info!("EMMC Capabilities 1: 0b{:b}", caps1);
        let caps2 = self.read_reg(EMMC_CAPABILITIES2);
        info!("EMMC Capabilities 2: 0x{:x}", caps2);

        // Reset the controller
        self.reset_all()?;

        // Add extra delay after reset
        for _ in 0..50000 {
            let _ = self.read_reg8(EMMC_POWER_CTRL);
        }

        // Enable interrupts
        self.write_reg(EMMC_NORMAL_INT_STAT_EN, EMMC_INT_ALL_MASK);
        self.write_reg(EMMC_SIGNAL_ENABLE, EMMC_INT_ALL_MASK);

        // Perform full power cycle
        self.set_power(0)?; // Power off
        self.set_power(1)?; // Power on

        // Set initial clock and wait for it to stabilize
        self.set_clock(400000)?; // Start with 400 KHz for initialization
        
        // Additional delay after setting clock
        for _ in 0..50000 {
            let _ = self.read_reg8(EMMC_POWER_CTRL);
        }

        // Clear any pending interrupts
        self.write_reg(EMMC_NORMAL_INT_STAT, EMMC_INT_ALL_MASK);

        // Set initial bus width to 1-bit
        let ctrl = self.read_reg8(EMMC_HOST_CTRL1);
        self.write_reg8(EMMC_HOST_CTRL1, ctrl & !EMMC_CTRL_4BITBUS & !EMMC_CTRL_8BITBUS);

        // Check if card is present
        if !self.is_card_present() {
            return Err(SdError::NoCard);
        }

        // Initialize the card
        self.init_card()?;
        
        info!("EMMC initialization completed successfully");
        Ok(())
    }

    // Reset the controller
    fn reset_all(&self) -> Result<(), SdError> {
        // Request reset
        self.write_reg8(EMMC_SOFTWARE_RESET, EMMC_RESET_ALL);

        // Wait for reset to complete with timeout
        let mut timeout = 200000; // Increased timeout
        while (self.read_reg8(EMMC_SOFTWARE_RESET) & EMMC_RESET_ALL) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        // Additional delay after reset completes
        for _ in 0..10000 {
            let _ = self.read_reg8(EMMC_SOFTWARE_RESET);
        }

        Ok(())
    }

    // Reset data line
    fn reset_data(&self) -> Result<(), SdError> {
        self.write_reg8(EMMC_SOFTWARE_RESET, EMMC_RESET_DATA);

        // Wait for reset to complete
        let mut timeout = 100000;
        while (self.read_reg8(EMMC_SOFTWARE_RESET) & EMMC_RESET_DATA) != 0 {
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
        self.write_reg16(EMMC_CLOCK_CONTROL, 0);

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
        let mut clk = ((div as u16) << (EMMC_CLOCK_DIV_SHIFT as u16)) | EMMC_CLOCK_INT_EN;
        self.write_reg16(EMMC_CLOCK_CONTROL, clk);

        // Wait for clock stability
        let mut timeout = 100000;
        loop {
            clk = self.read_reg16(EMMC_CLOCK_CONTROL);
            if (clk & EMMC_CLOCK_INT_STABLE) != 0 {
                break;
            }
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        // Enable card clock
        clk |= EMMC_CLOCK_CARD_EN;
        self.write_reg16(EMMC_CLOCK_CONTROL, clk);

        Ok(())
    }

    // Set power to the card
    fn set_power(&self, on: u8) -> Result<(), SdError> {
        // Set voltage level to 3.3V (0x0E)
        let pwr = if on != 0 { 0x0E | 0x01 } else { 0 };
        self.write_reg8(EMMC_POWER_CTRL, pwr);

        // Small delay for power to stabilize
        for _ in 0..20000 {
            let _ = self.read_reg8(EMMC_POWER_CTRL);
        }

        Ok(())
    }

    // Check if card is present
    fn is_card_present(&self) -> bool {
        let state = self.read_reg(EMMC_PRESENT_STATE);
        (state & EMMC_CARD_INSERTED) != 0
    }

    // Check if card is write protected
    fn is_write_protected(&self) -> bool {
        let state = self.read_reg(EMMC_PRESENT_STATE);
        (state & EMMC_WRITE_PROTECT) != 0
    }

    // Initialize the eMMC card
    fn init_card(&mut self) -> Result<(), SdError> {
        info!("eMMC initialization started");

        // Send CMD0 to reset the card
        let cmd = EMmcCommand::new(MMC_GO_IDLE_STATE, 0, MMC_RSP_NONE);
        self.send_command(&cmd)?;

        info!("eMMC reset complete");

        // Add sufficient delay after reset
        for _ in 0..100000 {
            let _ = self.read_reg8(EMMC_POWER_CTRL);
        }

        // For eMMC, we use CMD1 instead of ACMD41
        // HCS=1, voltage window for eMMC as per specs
        let mut ocr = 0x40FF8080; // Adjusted for eMMC fixed pattern
        let mut retry = 100;
        let mut ready = false;

        // Create card structure
        let mut card = EMmcCard::init(self.base_addr, CardType::Mmc);

        while retry > 0 && !ready {
            // Send CMD1 for eMMC
            let cmd = EMmcCommand::new(MMC_SEND_OP_COND, ocr, MMC_RSP_R3);
            self.send_command(&cmd)?;
            let response = self.get_response();
            ocr = response.as_r3();

            // Check if card is ready (bit 31 set)
            if (ocr & (1 << 31)) != 0 {
                ready = true;
                card.ocr = ocr;
                if (ocr & (1 << 30)) != 0 {
                    card.card_type = CardType::MmcHc;
                    card.state |= MMC_STATE_HIGHCAPACITY;
                }
            } else {
                retry -= 1;
                // Delay between retries
                for _ in 0..10000 {
                    let _ = self.read_reg8(EMMC_POWER_CTRL);
                }
            }
        }

        info!("eMMC initialization status: {}", ready);

        if !ready {
            return Err(SdError::UnsupportedCard);
        }

        // Send CMD2 to get CID
        let cmd = EMmcCommand::new(MMC_ALL_SEND_CID, 0, MMC_RSP_R2);
        self.send_command(&cmd)?;
        let response = self.get_response();
        card.cid = response.as_r2();

        info!("eMMC Card CID: {:b} {:b} {:b} {:b}", 
            response.as_r2()[0], response.as_r2()[1], 
            response.as_r2()[2], response.as_r2()[3]);

        // For eMMC, host assigns the RCA value (unlike SD where card provides it)
        let mmc_rca = 0x0001 << 16; // Typical RCA value for eMMC is 1
        card.rca = mmc_rca;

        // Send CMD3 to set RCA for eMMC
        let cmd = EMmcCommand::new(MMC_SET_RELATIVE_ADDR, card.rca, MMC_RSP_R1);
        self.send_command(&cmd)?;

        // Send CMD9 to get CSD
        let cmd = EMmcCommand::new(MMC_SEND_CSD, card.rca, MMC_RSP_R2);
        self.send_command(&cmd)?;
        let response = self.get_response();
        card.csd = response.as_r2();

        info!("eMMC Card info: CSD: {:b} {:b} {:b} {:b}", 
            response.as_r2()[0], response.as_r2()[1], 
            response.as_r2()[2], response.as_r2()[3]);

        // Calculate card capacity from CSD
        let csd_version = (card.csd[3] >> 22) & 0x3;
        debug!("eMMC CSD version: {}", csd_version);

        if csd_version <= 2 {
            // Standard capacity calculation for older eMMC
            let c_size = ((card.csd[2] & 0x3) << 10) | ((card.csd[1] >> 22) & 0x3FF);
            let c_size_mult = (card.csd[1] >> 7) & 0x7;
            let read_bl_len = (card.csd[2] >> 8) & 0xF;
            
            debug!("c_size: {}, c_size_mult: {}, read_bl_len: {}", c_size, c_size_mult, read_bl_len);
            
            let block_size = 1 << read_bl_len;
            let mult = 1 << (c_size_mult + 2);
            let capacity = (c_size + 1) as u64 * mult as u64 * block_size as u64;
            card.capacity_blocks = capacity / 512;
            
            debug!("eMMC capacity: {} blocks ({} bytes)", 
                card.capacity_blocks, card.capacity_blocks * 512);
        } else {
            // For newer eMMC rev4.4+, may need to read Extended CSD
            let c_size = ((card.csd[2] & 0x3) << 10) | ((card.csd[1] >> 22) & 0x3FF);
            let c_size_mult = (card.csd[1] >> 7) & 0x7;
            let read_bl_len = (card.csd[2] >> 8) & 0xF;
            
            let block_size = 1 << read_bl_len;
            let mult = 1 << (c_size_mult + 2);
            let capacity = (c_size + 1) as u64 * mult as u64 * block_size as u64;
            card.capacity_blocks = capacity / 512;
            
            debug!("eMMC capacity (from CSD): {} blocks", card.capacity_blocks);
        }

        // Set block size to 512 bytes (standard for eMMC)
        card.block_size = 512;

        // Send CMD7 to select the card
        let cmd = EMmcCommand::new(MMC_SELECT_CARD, card.rca, MMC_RSP_R1B);
        self.send_command(&cmd)?;

        // Set block size to 512 bytes
        let cmd = EMmcCommand::new(MMC_SET_BLOCKLEN, 512, MMC_RSP_R1);
        self.send_command(&cmd)?;
        
        // Switch to wider bus width if supported
        if !self.is_write_protected() {
            // For eMMC, use CMD6 to switch to wider bus width
            // 8-bit bus width if hardware supports it
            if self.supports_8bit_bus() {
                // Add delay before bus width switch
                for _ in 0..10000 {
                    let _ = self.read_reg8(EMMC_POWER_CTRL);
                }
                
                let cmd = EMmcCommand::new(MMC_SWITCH, 
                                        (3 << 24) | (183 << 16) | (2 << 8) | 1, 
                                        MMC_RSP_R1B);
                if self.send_command(&cmd).is_ok() {
                    // Set controller to 8-bit mode
                    let ctrl = self.read_reg8(EMMC_HOST_CTRL1);
                    self.write_reg8(EMMC_HOST_CTRL1, ctrl | EMMC_CTRL_8BITBUS);
                    info!("eMMC: Switched to 8-bit bus width");
                    
                    // Add delay after bus width switch
                    for _ in 0..10000 {
                        let _ = self.read_reg8(EMMC_POWER_CTRL);
                    }
                }
            } else {
                // Try 4-bit bus width
                let cmd = EMmcCommand::new(MMC_SWITCH, 
                                        (3 << 24) | (183 << 16) | (1 << 8) | 1, 
                                        MMC_RSP_R1B);
                if self.send_command(&cmd).is_ok() {
                    // Set controller to 4-bit mode
                    let ctrl = self.read_reg8(EMMC_HOST_CTRL1);
                    self.write_reg8(EMMC_HOST_CTRL1, ctrl | EMMC_CTRL_4BITBUS);
                    info!("eMMC: Switched to 4-bit bus width");
                    
                    // Add delay after bus width switch
                    for _ in 0..10000 {
                        let _ = self.read_reg8(EMMC_POWER_CTRL);
                    }
                }
            }
        }

        // Set higher clock speed for data transfer
        // Start with a safe speed, can be increased based on ext_csd capabilities
        self.set_clock(26000000)?; // 26 MHz for standard eMMC
        
        // Card is initialized
        card.initialized.store(true, Ordering::SeqCst);
        card.state |= MMC_STATE_PRESENT;

        // Store the card in the host
        self.card = Some(card);

        info!("eMMC initialization complete");
        Ok(())
    }

    // Helper function to check if controller supports 8-bit bus
    fn supports_8bit_bus(&self) -> bool {
        // Read controller capabilities register
        // This is a placeholder - actual implementation depends on your EMMC controller
        let caps = self.read_reg(EMMC_CAPABILITIES1);
        (caps & EMMC_CAN_DO_8BIT) != 0
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
        let cmd = EMmcCommand::new(MMC_SEND_STATUS, card.rca, MMC_RSP_R1);
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

