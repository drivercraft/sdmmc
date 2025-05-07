// ===== Block Device Interface =====

use core::sync::atomic::{AtomicBool, Ordering};
use aux::MMC_VERSION_UNKNOWN;
use dma_api::DVec;
use log::debug;
use log::info;

use crate::{delay_us, err::SdError};

use super::{aux, cmd::EMmcCommand, constant::*, CardType, EMmcHost};

pub const EMMC_DEFAULT_BOUNDARY_SIZE: u32 = 512 * 1024;

#[cfg(feature = "pio")]
pub enum DataBuffer<'a> {
    Read(&'a mut [u8]),
    Write(&'a [u8]),
}

#[cfg(feature = "dma")]
pub enum DataBuffer<'a> {
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

    #[cfg(feature = "pio")]
    pub fn read_blocks(&self, block_id: u32, blocks: u16, buffer: &mut [u8]) -> Result<(), SdError> {
        info!("pio read_blocks: block_id = {}, blocks = {}", block_id, blocks);
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // 根据卡的类型调整块地址（高容量卡使用块地址，标准容量卡使用字节地址）
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id  // 高容量卡：直接使用块地址
        } else {
            block_id * 512  // 标准容量卡：转换为字节地址
        };

        debug!("Reading {} blocks starting at address: {:#x}", blocks, card_addr);

        if blocks == 1 {
            // 单块读取
            let cmd = EMmcCommand::new(MMC_READ_SINGLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, 1, true);
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
        } else {
            // 多块读取
            let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, true);
            
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
            
            // 多块读取后必须发送停止传输命令
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    #[cfg(feature = "pio")]
    pub fn write_blocks(&self, block_id: u32, blocks: u16, buffer: &[u8]) -> Result<(), SdError> {
        info!("pio write_blocks: block_id = {}, blocks = {}", block_id, blocks);
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // Check if card is write protected
        if self.is_write_protected() {
            return Err(SdError::IoError);
        }

        // Convert to byte address for standard capacity cards
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id  // 高容量卡：直接使用块地址
        } else {
            block_id * 512  // 标准容量卡：转换为字节地址
        };

        debug!("Writing {} blocks starting at address: {:#x}", blocks, card_addr);

        // 根据块数选择合适的命令
        if blocks == 1 {
            // 单块写入
            let cmd = EMmcCommand::new(MMC_WRITE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, 1, false);
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
        } else {
            // 多块写入
            let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, false);
            
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
            
            // 多块写入后必须发送停止传输命令
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    /// 从卡中读取一个或多个数据块
    #[cfg(feature = "dma")]
    pub fn read_blocks(&self, block_id: u32, blocks: u16, buffer: &mut DVec<u8>) -> Result<(), SdError> {
        let expected_size = blocks as usize * 512;
        if buffer.len() != expected_size {
            return Err(SdError::IoError);
        }
        
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // 根据卡的类型调整块地址（高容量卡使用块地址，标准容量卡使用字节地址）
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id  // 高容量卡：直接使用块地址
        } else {
            block_id * 512  // 标准容量卡：转换为字节地址
        };

        debug!("Reading {} blocks starting at address: {:#x}", blocks, card_addr);

        // 根据块数选择合适的命令
        if blocks == 1 {
            // 单块读取
            let cmd = EMmcCommand::new(MMC_READ_SINGLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, 1, true);
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
        } else {
            // 多块读取
            let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, true);
            
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
            
            // 多块读取后必须发送停止传输命令
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    // Write multiple blocks to the card
    #[cfg(feature = "dma")] 
    pub fn write_blocks(&self, block_id: u32, blocks: u16, buffer: &DVec<u8>) -> Result<(), SdError> {
        // 验证缓冲区大小是否匹配请求的块数
        let expected_size = blocks as usize * 512;
        if buffer.len() != expected_size {
            return Err(SdError::IoError);
        }

        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };
    
        // Check if card is initialized
        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }
    
        // Check if card is write protected
        if self.is_write_protected() {
            return Err(SdError::IoError);
        }
    
        // Convert to byte address for standard capacity cards
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id  // 高容量卡：直接使用块地址
        } else {
            block_id * 512  // 标准容量卡：转换为字节地址
        };
    
        debug!("Writing {} blocks starting at address: {:#x}", blocks, card_addr);
    
        // 根据块数选择合适的命令
        if blocks == 1 {
            // 单块写入
            let cmd = EMmcCommand::new(MMC_WRITE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, 1, false);
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
        } else {
            // 多块写入
            let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, false);
            
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
            
            // 多块写入后必须发送停止传输命令
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }
    
        Ok(())
    }
    
    #[cfg(feature = "dma")]
    pub fn transfer_data_by_dma(&self) -> Result<(), SdError> {
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

    pub fn transfer_data_by_pio(&self, data_dir_read: bool, buffer: &mut [u8]) -> Result<(), SdError> {
        for i in (0..buffer.len()).step_by(16) {
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

    pub fn write_buffer(&self, buffer: &[u8]) -> Result<(), SdError> {
        self.wait_for_interrupt(EMMC_INT_SPACE_AVAIL, 100000)?;

        let len = buffer.len();
        for i in (0..len).step_by(4) {
            let mut val: u32 = (buffer[i] as u32) << 0;
            
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
    
        // 等待传输完成
        self.wait_for_interrupt(EMMC_INT_DATA_END, 1000000)?;
    
        Ok(())
    }

    pub fn read_buffer(&self, buffer: &mut [u8]) -> Result<(), SdError> {
        // 等待数据可用
        self.wait_for_interrupt(EMMC_INT_DATA_AVAIL, 100000)?;

        // 读取数据到缓冲区
        let len = buffer.len();
        for i in (0..len).step_by(4) {
            let val = self.read_reg(EMMC_BUF_DATA);
            
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
        
        // 等待传输完成
        self.wait_for_interrupt(EMMC_INT_DATA_END, 100000)?;

        Ok(())
    }

    // 用于等待特定中断标志的辅助函数
    fn wait_for_interrupt(&self, flag: u32, timeout_count: u32) -> Result<(), SdError> {
        let mut timeout = timeout_count;
        while timeout > 0 {
            let int_status = self.read_reg(EMMC_NORMAL_INT_STAT);
            
            // 检查目标标志
            if int_status & flag != 0 {
                // 清除该标志
                self.write_reg16(EMMC_NORMAL_INT_STAT, flag as u16);
                return Ok(());
            }
            
            // 检查错误
            if int_status & EMMC_INT_ERROR_MASK != 0 {
                // 清除错误标志
                self.write_reg16(EMMC_NORMAL_INT_STAT, (int_status & EMMC_INT_ERROR_MASK) as u16);
                self.reset_data()?;
                return Err(SdError::DataError);
            }
            
            timeout -= 1;
        }
        
        if timeout == 0 {
            return Err(SdError::DataTimeout);
        }
        
        Ok(())
    }
}