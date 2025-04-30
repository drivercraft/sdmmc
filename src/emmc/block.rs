// ===== Block Device Interface =====

use core::sync::atomic::{AtomicBool, Ordering};
use aux::MMC_VERSION_UNKNOWN;
use dma_api::DVec;
use dma_api::Direction;
use log::debug;
use log::info;

use crate::{delay_us, err::SdError};

use super::{aux, cmd::EMmcCommand, constant::*, CardType, EMmcHost};

pub const EMMC_DEFAULT_BOUNDARY_SIZE: u32 = 512 * 1024;

// #[derive(Debug)]
pub enum DataBuffer<'a> {
    // Read(&'a mut [u8]),
    // Write(&'a [u8]),
    Read(&'a mut DVec<u8>),
    Write(&'a DVec<u8>),
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

    pub high_capacity: bool,
    pub version: u32,
    pub dsr: u32,
    pub timing: u32,
    pub clock: u32,
    pub bus_width: u8,
    pub part_support: u8,
    pub part_attr: u8,
    pub wr_rel_set: u8,
    pub part_config: u8,
    pub dsr_imp: u32,
    pub card_caps: u32,
    pub read_bl_len: u32,
    pub write_bl_len: u32,
    pub erase_grp_size: u32,
    pub hc_wp_grp_size: u64,
    pub capacity: u64,
	pub capacity_user: u64,
	pub capacity_boot: u64,
	pub capacity_rpmb: u64,
	pub capacity_gp: [u64; 4],
    pub enh_user_size: u64,
    pub enh_user_start: u64,
    pub raw_driver_strength: u8,

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

            high_capacity: false,
            card_caps: 0,
            dsr_imp: 0,
            part_support: 0,
            part_attr: 0,
            wr_rel_set: 0,
            part_config: 0,
            read_bl_len: 0,
            write_bl_len: 0,
            erase_grp_size: 0,
            hc_wp_grp_size: 0,
            capacity: 0,
            capacity_user: 0,
            capacity_boot: 0,
            capacity_rpmb: 0,
            capacity_gp: [0; 4],
            enh_user_size: 0,
            enh_user_start: 0,
            raw_driver_strength: 0,

            ext_csd_rev: 0,
            ext_csd_sectors: 0,
            hs_max_dtr: 0,
        }
    }
}

impl EMmcCard {
    // CID 数组
    pub fn cid(&self) -> [u32; 4] {
        self.cid
    }
    
    pub fn set_cid(&mut self, value: [u32; 4]) {
        self.cid = value;
    }
    
    // CSD 数组
    pub fn csd(&self) -> [u32; 4] {
        self.csd
    }
    
    pub fn set_csd(&mut self, value: [u32; 4]) {
        self.csd = value;
    }
    
    // capacity_gp 数组
    pub fn capacity_gp(&self) -> [u64; 4] {
        self.capacity_gp
    }
    
    pub fn set_capacity_gp(&mut self, value: [u64; 4]) {
        self.capacity_gp = value;
    }
    
    // 对于 AtomicBool 类型
    pub fn initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }
    
    pub fn set_initialized(&self, value: bool) {
        self.initialized.store(value, Ordering::Relaxed);
    }
    
    // 对于 enh_user_size 和 enh_user_start
    pub fn enh_user_size(&self) -> u64 {
        self.enh_user_size
    }
    
    pub fn set_enh_user_size(&mut self, value: u64) {
        self.enh_user_size = value;
    }
    
    pub fn enh_user_start(&self) -> u64 {
        self.enh_user_start
    }
    
    pub fn set_enh_user_start(&mut self, value: u64) {
        self.enh_user_start = value;
    }
}

impl EMmcHost {
    pub fn add_card(&mut self, card: EMmcCard) {
        self.card = Some(card);
    }

    // Read a block from the card
    pub fn read_block(&self, block_addr: u32, buffer: &mut DVec<u8>) -> Result<(), SdError> {
        if buffer.len() != 512 {
            return Err(SdError::IoError);
        }
        
        let addr = buffer.bus_addr() as u32;
        debug!("Buffer address: {:#x}", addr);

        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // if !card.initialized.load(Ordering::SeqCst) {
        //     return Err(SdError::UnsupportedCard);
        // }

        // // Convert to byte address for standard capacity cards
        // let addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
        //     block_addr
        // } else {
        //     block_addr * 512
        // };

        debug!("Reading block at address: {:#x}", block_addr);

        // Send READ_SINGLE_BLOCK command
        let cmd = EMmcCommand::new(MMC_READ_SINGLE_BLOCK, block_addr, MMC_RSP_R1)
            .with_data(512, 1, true);
        self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;

        Ok(())
    }

    // Read data from the buffer register
    fn read_buffer(&self, buffer: &mut DVec<u8>) -> Result<(), SdError> {
        // Wait for data available
        let mut timeout = 1000000;
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
        
        // 处理4字节对齐的数据
        for i in (0..len).step_by(4) {
            let val = self.read_reg(EMMC_BUF_DATA);
            
            buffer.set(i, (val & 0xFF) as u8);
            
            if i + 1 < len {
                buffer.set(i + 1, ((val >> 8) & 0xFF) as u8);
            }
            
            if i + 2 < len {
                buffer.set(i + 2, ((val >> 16) & 0xFF) as u8);
            }
            
            if i + 3 < len {
                buffer.set(i + 3, ((val >> 24) & 0xFF) as u8);
            }
        }
        
        self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_DATA_AVAIL as u16);

        Ok(())
    }

    // Write a block to the card
    pub fn write_block(&self, block_addr: u32, buffer: &DVec<u8>) -> Result<(), SdError> {
        if buffer.len() != 512 {
            return Err(SdError::IoError);
        }
        
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // if !card.initialized.load(Ordering::SeqCst) {
        //     return Err(SdError::UnsupportedCard);
        // }

        // // Check if card is write protected
        // if self.is_write_protected() {
        //     return Err(SdError::IoError);
        // }

        // Convert to byte address for standard capacity cards
        let addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_addr
        } else {
            block_addr * 512
        };

        // 选择使用DMA模式还是PIO模式
        let use_dma = true; // 这可以是配置选项

        if use_dma {
            let cmd = EMmcCommand::new(MMC_WRITE_BLOCK, addr, MMC_RSP_R1)
                .with_data(512, 1, false);
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
        } else {
            // PIO模式 - 使用缓冲寄存器直接写入
            let cmd = EMmcCommand::new(MMC_WRITE_BLOCK, addr, MMC_RSP_R1);
            self.send_command(&cmd, None)?;
            
            // 写入数据到缓冲寄存器
            self.write_buffer(buffer)?;
        }

        Ok(())
    }

    // Write multiple blocks to the card
    pub fn write_blocks(&self, block_addr: u32, blocks: u16, buffer: &DVec<u8>) -> Result<(), SdError> {
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

        let use_dma = true; // 这可以是配置选项

        if use_dma {
            let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, addr, MMC_RSP_R1)
                .with_data(512, blocks, false);
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
        } else {
            let cmd = EMmcCommand::new(MMC_SET_BLOCK_COUNT, blocks as u32, MMC_RSP_R1);
            self.send_command(&cmd, None)?;

            let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, addr, MMC_RSP_R1);
            self.send_command(&cmd, None)?;

            for i in 0..blocks {
                let offset = i as usize * 512;
                let _end = offset + 512;

                let mut temp_buffer = DVec::zeros(512, 4, Direction::ToDevice)
                    .ok_or(SdError::MemoryError)?;

                for j in 0..512 {
                    if offset + j < buffer.len() {
                        temp_buffer.set(j, buffer[offset + j]);
                    }
                }

                self.write_buffer(&temp_buffer)?;
            }

            let cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&cmd, None)?;
        }

        if use_dma {
            let cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&cmd, None)?;
        }

        Ok(())
    }

    fn write_buffer(&self, buffer: &DVec<u8>) -> Result<(), SdError> {
        let mut timeout = 100000;
        while timeout > 0 {
            let int_status = self.read_reg(EMMC_NORMAL_INT_STAT);
            if int_status & EMMC_INT_SPACE_AVAIL != 0 {
                // 缓冲区准备好接收数据
                break;
            }
            
            if int_status & EMMC_INT_ERROR_MASK != 0 {
                // 错误
                self.reset_data()?;
                return Err(SdError::DataError);
            }
            
            timeout -= 1;
        }
        
        if timeout == 0 {
            return Err(SdError::DataTimeout);
        }

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

        self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_SPACE_AVAIL as u16);

        Ok(())
    }

    // Read multiple blocks from the card
    pub fn read_blocks(&self, block_addr: u32, blocks: u16, buffer: &mut DVec<u8>) -> Result<(), SdError> {
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

        // 选择使用DMA模式还是PIO模式
        let use_dma = true; // 这可以是配置选项

        if use_dma {
            // DMA模式 - 使用send_command的DMA能力
            let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, addr, MMC_RSP_R1)
                .with_data(512, blocks, true);
            
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
        } else {
            let cmd = EMmcCommand::new(MMC_SET_BLOCK_COUNT, blocks as u32, MMC_RSP_R1);
            self.send_command(&cmd, None)?;

            let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, addr, MMC_RSP_R1);
            self.send_command(&cmd, None)?;

            let mut temp_buf = DVec::zeros(512, 512, Direction::FromDevice)
                .ok_or(SdError::MemoryError)?;
                
            for i in 0..blocks {
                self.read_buffer(&mut temp_buf)?;

                let offset = i as usize * 512;
                for j in 0..512 {
                    if offset + j < buffer.len() {
                        buffer.set(offset + j, temp_buf[j]);
                    }
                }
            }
        }

        let cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
        self.send_command(&cmd, None)?;

        Ok(())
    }

    pub fn read_data_buffer(&self, buffer: &mut [u8]) -> Result<(), SdError> {
        let len = buffer.len();

        for i in 0..len {
            buffer[i] = self.read_reg8(EMMC_BUF_DATA);
        }
        
        Ok(())
    }

    pub fn write_data_buffer(&self, buffer: &[u8]) -> Result<(), SdError> {
        let len = buffer.len();

        for i in 0..len {
            self.write_reg8(EMMC_BUF_DATA, buffer[i]);
        }
        
        Ok(())
    }

    pub fn transfer_data(
        &self, 
    ) -> Result<(), SdError> {
        let mut timeout = 100;

        loop {
            let stat = self.read_reg16(EMMC_NORMAL_INT_STAT);
            debug!("Transfer status: {:#b}", stat);
    
            // 检查错误
            if stat & EMMC_INT_ERROR as u16 != 0 {
                let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
                info!("Data transfer error: status={:#b}, err_status={:#b}", stat, err_status);
                
                self.reset_data()?;
                
                let err = if err_status & 0x10 != 0 {
                        SdError::DataTimeout
                    } else if err_status & 0x20 != 0 {
                        SdError::DataCrc
                    } else if err_status & 0x40 != 0 {
                        SdError::DataEndBit
                    } else {
                        SdError::DataError
                    };
                return Err(err);
            }

            if stat & EMMC_INT_DATA_END as u16 != 0 {
                self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_DATA_END as u16);
                break;
            }
    
            // 超时处理
            if timeout > 0 {
                timeout -= 1;
                delay_us(1000);
            } else {
                info!("Data transfer timeout");
                return Err(SdError::DataTimeout);
            }
        }
    
        Ok(())
    }

    pub fn transfer_pio(&self, data_dir_read: bool, buffer: &mut [u8]) -> Result<(), SdError> {
        for i in (0..buffer.len()).step_by(16) {  // 每行显示16字节，即4个u32
            if data_dir_read {
                let mut values = [0u32; 4];
                for j in 0..4 {
                    if i + j*4 < buffer.len() {
                        values[j] = self.read_reg(EMMC_BUF_DATA);

                        if i + j*4 + 3 < buffer.len() {
                            buffer[i + j*4] = (values[j] & 0xFF) as u8;
                            buffer[i + j*4 + 1] = ((values[j] >> 8) & 0xFF) as u8;
                            buffer[i + j*4 + 2] = ((values[j] >> 16) & 0xFF) as u8;
                            buffer[i + j*4 + 3] = ((values[j] >> 24) & 0xFF) as u8;
                        }
                    }
                }

                debug!("0x{:08x}: 0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x}", 
                       buffer.as_ptr() as usize + i,
                       values[0], values[1], values[2], values[3]);
            } else {
                let mut values = [0u32; 4];
                for j in 0..4 {
                    if i + j*4 + 3 < buffer.len() {
                        values[j] = (buffer[i + j*4] as u32) |
                                   ((buffer[i + j*4 + 1] as u32) << 8) |
                                   ((buffer[i + j*4 + 2] as u32) << 16) |
                                   ((buffer[i + j*4 + 3] as u32) << 24);

                        self.write_reg(EMMC_BUF_DATA, values[j]);
                    }
                }

                debug!("0x{:08x}: 0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x}", 
                       buffer.as_ptr() as usize + i,
                       values[0], values[1], values[2], values[3]);
            }
        }
        
        Ok(())
    }
}