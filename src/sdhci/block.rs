// ===== Block Device Interface =====

use core::sync::atomic::{AtomicBool, Ordering};
use crate::err::SdError;
use log::{debug, info, warn};

use super::{cmd::SdCommand, constant::*, CardType, SdHost};

// Simple block device trait that could be used by a filesystem\
#[allow(unused)]
pub trait BlockDevice {
    fn read_block(&self, block_addr: u32, buffer: &mut [u8]) -> Result<(), SdError>;
    fn write_block(&self, block_addr: u32, buffer: &[u8]) -> Result<(), SdError>;
    fn read_blocks(&self, block_addr: u32, blocks: u16, buffer: &mut [u8]) -> Result<(), SdError>;
    fn write_blocks(&self, block_addr: u32, blocks: u16, buffer: &[u8]) -> Result<(), SdError>;
    fn get_capacity(&self) -> Result<u64, SdError>;
}

// SD Card structure
#[derive(Debug)]
pub struct SdCard {
    pub base_addr: usize,
    pub rca: u32, // Relative Card Address
    pub cid: [u32; 4],
    pub csd: [u32; 4],
    pub ocr: u32,
    pub card_type: CardType,
    pub state: u32,
    pub initialized: AtomicBool,
    pub block_size: u32,
    pub capacity_blocks: u64,
}

impl SdCard {
    #[allow(unused)]
    pub fn init(ase_addr: usize, card_type: CardType) -> Self {
        Self {
            base_addr: ase_addr,
            rca: 0,
            cid: [0; 4],
            csd: [0; 4],
            ocr: 0,
            card_type,
            state: 0,
            initialized: AtomicBool::new(false),
            block_size: 512,
            capacity_blocks: 0,
        }
    }
}

impl SdHost {
    // Read a block from the card
    pub fn read_block(&self, block_addr: u32, buffer: &mut [u8]) -> Result<(), SdError> {
        debug!("read block start");
        debug!("block_addr is 0x{:x}", block_addr);
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
        let cmd = SdCommand::new(MMC_READ_SINGLE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, 1, true);
        self.send_command(&cmd)?;

        // Read data from buffer register
        self.read_buffer(buffer)?;
        debug!("read block completed");
        Ok(())
    }

    // Read data from the buffer register
    fn read_buffer(&self, buffer: &mut [u8]) -> Result<(), SdError> {
        // Wait for data available
        debug!("read buffer start");
        let mut timeout = 100000;
        while timeout > 0 {
            let int_status = self.read_reg(SDHCI_INT_STATUS);
            if int_status & SDHCI_INT_DATA_AVAIL != 0 {
                // Data available
                break;
            }
            
            if int_status & SDHCI_INT_ERROR_MASK != 0 {
                // Error
                self.reset_data()?;
                return Err(SdError::DataCrc);
            }
            
            timeout -= 1;
        }
        
        if timeout == 0 {
            warn!("read buffer timeout");
            return Err(SdError::DataTimeout);
        }

        // Read data from buffer
        let len = buffer.len();
        for i in (0..len).step_by(4) {
            let val = self.read_reg(SDHCI_BUFFER);
            
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
        debug!("read buffer completed");
        Ok(())
    }

    // Write data to the buffer register
    fn write_buffer(&self, buffer: &[u8]) -> Result<(), SdError> {
        // Wait for space available
        let mut timeout = 100000;
        while timeout > 0 {
            let int_status = self.read_reg(SDHCI_INT_STATUS);
            if int_status & SDHCI_INT_SPACE_AVAIL != 0 {
                // Space available
                break;
            }
            
            if int_status & SDHCI_INT_ERROR_MASK != 0 {
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
            
            self.write_reg(SDHCI_BUFFER, val);
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
        let cmd = SdCommand::new(MMC_READ_MULTIPLE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, blocks, true);
        self.send_command(&cmd)?;

        // Read data from buffer register
        for i in 0..blocks {
            let offset = i as usize * 512;
            self.read_buffer(&mut buffer[offset..offset + 512])?;
        }

        // Send STOP_TRANSMISSION command
        let cmd = SdCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
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
        let cmd = SdCommand::new(MMC_WRITE_BLOCK, addr, MMC_RSP_R1)
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
        let cmd = SdCommand::new(MMC_WRITE_MULTIPLE_BLOCK, addr, MMC_RSP_R1)
            .with_data(512, blocks, false);
        self.send_command(&cmd)?;

        // Write data to buffer register
        for i in 0..blocks {
            let offset = i as usize * 512;
            self.write_buffer(&buffer[offset..offset + 512])?;
        }

        // Send STOP_TRANSMISSION command
        let cmd = SdCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
        self.send_command(&cmd)?;

        Ok(())
    }
}