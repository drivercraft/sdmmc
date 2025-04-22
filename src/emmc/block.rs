// ===== Block Device Interface =====

use core::sync::atomic::{AtomicBool, Ordering};
use aux::MMC_VERSION_UNKNOWN;

use crate::err::SdError;

use super::{aux, cmd::EMmcCommand, constant::*, CardType, EMmcHost};

// Simple block device trait that could be used by a filesystem\
#[allow(unused)]
pub trait BlockDevice {
    fn read_block(&self, block_addr: u32, buffer: &mut [u8]) -> Result<(), SdError>;
    fn write_block(&self, block_addr: u32, buffer: &[u8]) -> Result<(), SdError>;
    fn read_blocks(&self, block_addr: u32, blocks: u16, buffer: &mut [u8]) -> Result<(), SdError>;
    fn write_blocks(&self, block_addr: u32, blocks: u16, buffer: &[u8]) -> Result<(), SdError>;
    fn get_capacity(&self) -> Result<u64, SdError>;
}

// EMmc Card structure
#[derive(Debug)]
pub struct EMmcCard {
    pub card_type: CardType,
    pub rca: u32,
    pub ocr: u32,
    pub cid: [u32; 4],
    pub csd: [u32; 4],
    pub state: u32,
    pub block_size: u32,
    pub capacity_blocks: u64,
    pub initialized: AtomicBool,

    pub version: u32,
    pub dsr: u32,
    pub timing: u32,
    pub clock: u32,
    pub bus_width: u8,

    // 扩展CSD相关字段
    pub ext_csd_rev: u8,
    pub ext_csd_sectors: u64,
    pub hs_max_dtr: u32,
}

impl EMmcCard {
    pub fn init(card_type: CardType) -> Self {
        Self {
            card_type,
            rca: 0,
            ocr: 0,
            cid: [0; 4],
            csd: [0; 4],
            state: 0,
            block_size: 0,
            capacity_blocks: 0,
            initialized: AtomicBool::new(false),

            version: MMC_VERSION_UNKNOWN,
            dsr: 0xffffffff,
            timing: MMC_TIMING_LEGACY,
            clock: 0,
            bus_width: 0,

            ext_csd_rev: 0,
            ext_csd_sectors: 0,
            hs_max_dtr: 0,
        }
    }
}

impl EMmcHost {
    pub fn add_card(&mut self, card: EMmcCard) {
        self.card = Some(card);
    }

    // Read a block from the card
    pub fn read_block(&self, block_addr: u32, buffer: &mut [u8]) -> Result<(), SdError> {
        if buffer.len() != 512 {
            return Err(SdError::IoError);
        }
        
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Convert to byte address for standard capacity cards
        let addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_addr
        } else {
            block_addr * 512
        };

        // Send READ_SINGLE_BLOCK command
        let cmd = EMmcCommand::new(MMC_READ_SINGLE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, 1, true);
        self.send_command(&cmd)?;

        // Read data from buffer register
        self.read_buffer(buffer)?;

        Ok(())
    }

    // Read data from the buffer register
    fn read_buffer(&self, buffer: &mut [u8]) -> Result<(), SdError> {
        // Wait for data available
        let mut timeout = 100000;
        while timeout > 0 {
            let int_status = self.read_reg(EMMC_NORMAL_INT_STAT);
            if int_status & EMMC_INT_DATA_AVAIL != 0 {
                // Data available
                break;
            }
            
            if int_status & EMMC_INT_ERROR_MASK != 0 {
                // Error
                self.reset_data()?;
                return Err(SdError::DataCrc);
            }
            
            timeout -= 1;
        }
        
        if timeout == 0 {
            return Err(SdError::DataTimeout);
        }

        // Read data from buffer
        let len = buffer.len();
        for i in (0..len).step_by(4) {
            let val = self.read_reg(EMMC_BUF_DATA);
            
            // Convert u32 to bytes (little endian)
            buffer[i] = (val & 0xFF) as u8;
            
            if i + 1 < len {
                buffer[i + 1] = ((val >> 8) & 0xFF) as u8;
            }
            
            if i + 2 < len {
                buffer[i + 2] = ((val >> 16) & 0xFF) as u8;
            }
            
            if i + 3 < len {
                buffer[i + 3] = ((val >> 24) & 0xFF) as u8;
            }
        }

        Ok(())
    }

    // Write data to the buffer register
    fn write_buffer(&self, buffer: &[u8]) -> Result<(), SdError> {
        // Wait for space available
        let mut timeout = 100000;
        while timeout > 0 {
            let int_status = self.read_reg(EMMC_NORMAL_INT_STAT);
            if int_status & EMMC_INT_SPACE_AVAIL != 0 {
                // Space available
                break;
            }
            
            if int_status & EMMC_INT_ERROR_MASK != 0 {
                // Error
                self.reset_data()?;
                return Err(SdError::DataCrc);
            }
            
            timeout -= 1;
        }
        
        if timeout == 0 {
            return Err(SdError::DataTimeout);
        }

        // Write data to buffer
        let len = buffer.len();
        for i in (0..len).step_by(4) {
            let mut val: u32 = buffer[i] as u32;
            
            if i + 1 < len {
                val |= (buffer[i + 1] as u32) << 8;
            }
            
            if i + 2 < len {
                val |= (buffer[i + 2] as u32) << 16;
            }
            
            if i + 3 < len {
                val |= (buffer[i + 3] as u32) << 24;
            }
            
            self.write_reg(EMMC_BUF_DATA, val);
        }

        Ok(())
    }

    // Read multiple blocks from the card
    pub fn read_blocks(&self, block_addr: u32, blocks: u16, buffer: &mut [u8]) -> Result<(), SdError> {
        if buffer.len() != (blocks as usize * 512) {
            return Err(SdError::IoError);
        }
        
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Convert to byte address for standard capacity cards
        let addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_addr
        } else {
            block_addr * 512
        };

        // Send READ_MULTIPLE_BLOCK command
        let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, blocks, true);
        self.send_command(&cmd)?;

        // Read data from buffer register
        for i in 0..blocks {
            let offset = i as usize * 512;
            self.read_buffer(&mut buffer[offset..offset + 512])?;
        }

        // Send STOP_TRANSMISSION command
        let cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
        self.send_command(&cmd)?;

        Ok(())
    }

    // Write a block to the card
    pub fn write_block(&self, block_addr: u32, buffer: &[u8]) -> Result<(), SdError> {
        if buffer.len() != 512 {
            return Err(SdError::IoError);
        }
        
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Check if card is write protected
        if self.is_write_protected() {
            return Err(SdError::IoError);
        }

        // Convert to byte address for standard capacity cards
        let addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_addr
        } else {
            block_addr * 512
        };

        // Send WRITE_BLOCK command
        let cmd = EMmcCommand::new(MMC_WRITE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, 1, false);
        self.send_command(&cmd)?;

        // Write data to buffer register
        self.write_buffer(buffer)?;

        Ok(())
    }

    // Write multiple blocks to the card
    pub fn write_blocks(&self, block_addr: u32, blocks: u16, buffer: &[u8]) -> Result<(), SdError> {
        if buffer.len() != (blocks as usize * 512) {
            return Err(SdError::IoError);
        }
        
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Check if card is write protected
        if self.is_write_protected() {
            return Err(SdError::IoError);
        }

        // Convert to byte address for standard capacity cards
        let addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_addr
        } else {
            block_addr * 512
        };

        // Send WRITE_MULTIPLE_BLOCK command
        let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, blocks, false);
        self.send_command(&cmd)?;

        // Write data to buffer register
        for i in 0..blocks {
            let offset = i as usize * 512;
            self.write_buffer(&buffer[offset..offset + 512])?;
        }

        // Send STOP_TRANSMISSION command
        let cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
        self.send_command(&cmd)?;

        Ok(())
    }
}