use dma_api::DVec;
use log::{debug, info};

use crate::{delay_us, emmc::CardType, err::SdError};

use super::{block::DataBuffer, constant::*, EMmcHost};

const EMMC_DEFAULT_BOUNDARY_ARG: u16 = 7;
const CMD_DEFAULT_TIMEOUT: u32 = 100;
const CMD_MAX_TIMEOUT: u32 = 500;

#[derive(Debug)]
pub struct EMmcCommand {
    pub opcode: u8,
    pub arg: u32,
    pub resp_type: u32,
    pub data_present: bool,
    pub data_dir_read: bool,
    pub block_size: u16,
    pub block_count: u16,
}

impl EMmcCommand {
    pub fn new(opcode: u8, arg: u32, resp_type: u32) -> Self {
        Self {
            opcode,
            arg,
            resp_type,
            data_present: false,
            data_dir_read: true,
            block_size: 0,
            block_count: 0,
        }
    }

    pub fn with_data(mut self, block_size: u16, block_count: u16, is_read: bool) -> Self {
        self.data_present = true;
        self.data_dir_read = is_read;
        self.block_size = block_size;
        self.block_count = block_count;
        self
    }
}

pub struct SdResponse {
    pub raw: [u32; 4],
}

impl SdResponse {
    pub fn new() -> Self {
        Self { raw: [0; 4] }
    }

    pub fn as_r1(&self) -> u32 {
        self.raw[0]
    }

    pub fn as_r2(&self) -> [u32; 4] {
        let mut response = [0; 4];
        for i in 0..4 {
            response[i] = self.raw[3 - i] << 8;
            if i != 3 {
                response[i] |= self.raw[3-i-1] >> 24;
            }
        }
        info!("eMMC response: {:#x} {:#x} {:#x} {:#x}", 
            response[0], response[1], 
            response[2], response[3]);
        
        response
    }

    pub fn as_r3(&self) -> u32 {
        self.raw[0]
    }

    pub fn as_r6(&self) -> u32 {
        self.raw[0]
    }

    pub fn as_r7(&self) -> u32 {
        self.raw[0]
    }
}

impl EMmcHost {
    // 发送命令
    pub fn send_command(&self, cmd: &EMmcCommand, mut data_buffer: Option<DataBuffer>) -> Result<(), SdError> {
        let mut cmd_timeout = CMD_DEFAULT_TIMEOUT;
        
        // 检查命令或数据线是否忙碌
        let mut mask = EMMC_CMD_INHIBIT;
        if cmd.data_present {
            mask |= EMMC_DATA_INHIBIT;
        }
        
        // 对于STOP_TRANSMISSION命令，不需要等待数据抑制
        if cmd.opcode == MMC_STOP_TRANSMISSION {
            mask &= !EMMC_DATA_INHIBIT;
        }

        // 使用动态调整的超时时间进行等待
        let mut time: u32 = 0;
        while (self.read_reg(EMMC_PRESENT_STATE) & mask) != 0 {
            if time >= cmd_timeout {
                info!("MMC: busy timeout");
                
                // 如果超时时间还可以增加，则增加超时时间并继续
                if 2 * cmd_timeout <= CMD_MAX_TIMEOUT {
                    cmd_timeout += cmd_timeout;
                    info!("timeout increasing to: {} ms.", cmd_timeout);
                    self.write_reg16(EMMC_NORMAL_INT_STAT, 0xFFFF);
                } else {
                    info!("timeout.");
                    // 不返回错误，尝试继续发送命令
                    break;
                }
            }
            time += 1;
            delay_us(1000);
        }
        
        // 清除所有中断状态
        self.write_reg16(EMMC_NORMAL_INT_STAT, 0xFFFF);
        self.write_reg16(EMMC_ERROR_INT_STAT, 0xFFFF);

        info!(
            "Sending command: opcode={:#x}, arg={:#x}, resp_type={:#x}",
            cmd.opcode, cmd.arg, cmd.resp_type
        );

        let mut int_mask = EMMC_INT_RESPONSE as u16;
        
        // 如果有数据且响应类型包含BUSY标志，则也等待数据结束中断
        if cmd.data_present && (cmd.resp_type & MMC_RSP_BUSY != 0) {
            int_mask |= EMMC_INT_DATA_END as u16;
        }

        if cmd.opcode == MMC_SEND_TUNING_BLOCK || cmd.opcode == MMC_SEND_TUNING_BLOCK_HS200 {
            int_mask &= !EMMC_INT_RESPONSE as u16;
            int_mask |= EMMC_INT_DATA_AVAIL as u16;
        }

        // 设置数据传输相关寄存器
        if cmd.data_present {
            self.write_reg8(EMMC_TIMEOUT_CONTROL, 0xe);

            let mut mode = EMMC_TRNS_BLK_CNT_EN;

            if cmd.block_count > 1 {
                mode |= EMMC_TRNS_MULTI;
            }

            if cmd.data_dir_read {
                mode |= EMMC_TRNS_READ;
            }

            // 传输模式配置
            self.write_reg16(EMMC_XFER_MODE, mode);

            match data_buffer {
                Some(DataBuffer::Read(ref read_buf)) if cmd.data_dir_read => {
                    let ptr = read_buf.bus_addr() as usize;

                    debug!("Read buffer address: {:#x}", ptr);
                    self.write_reg(EMMC_SDMASA, ptr as u32);
                },
                Some(DataBuffer::Write(write_buf)) if !cmd.data_dir_read => {
                    let ptr = write_buf.as_ptr() as usize;
                    let start_addr = ptr as u32;
                    self.write_reg(EMMC_SDMASA, start_addr);
                },
                _ => return Err(SdError::InvalidArgument),
            }
            
            mode |= EMMC_TRNS_DMA;

            // 设置块大小和数量
            self.write_reg16(EMMC_BLOCK_SIZE, (((EMMC_DEFAULT_BOUNDARY_ARG & 0x7) << 12) | (cmd.block_size & 0xFFF)).try_into().unwrap());
            self.write_reg16(EMMC_BLOCK_COUNT, cmd.block_count);
            self.write_reg16(EMMC_XFER_MODE, mode);
        } else if cmd.resp_type & MMC_RSP_BUSY != 0 {
            // 对于带BUSY的命令，但没有数据的情况，仍然设置超时控制
            self.write_reg8(EMMC_TIMEOUT_CONTROL, 0xe);
        }

        // 设置参数
        self.write_reg(EMMC_ARGUMENT, cmd.arg);

        // 设置命令寄存器
        let mut command = (cmd.opcode as u16) << 8;

        // 映射响应类型
        if cmd.resp_type & MMC_RSP_PRESENT != 0 {
            if cmd.resp_type & MMC_RSP_136 != 0 {
                command |= EMMC_CMD_RESP_LONG;
            } else if cmd.resp_type & MMC_RSP_BUSY != 0 {
                command |= EMMC_CMD_RESP_SHORT_BUSY;
            } else {
                command |= EMMC_CMD_RESP_SHORT;
            }
        }

        if cmd.resp_type & MMC_RSP_CRC != 0 {
            command |= EMMC_CMD_CRC;
        }

        if cmd.resp_type & MMC_RSP_OPCODE != 0 {
            command |= EMMC_CMD_INDEX;
        }

        if cmd.data_present {
            command |= EMMC_CMD_DATA;
        }

        info!("Sending command: {:#x}", command);
        // 0x151a == 0001 0101 0001 1010
        // 0x153a == 0001 0101 0011 1010
 
        // 特殊命令特殊处理
        let mut timeout_val = if cmd.opcode == MMC_GO_IDLE_STATE || cmd.opcode == MMC_SEND_OP_COND {
            CMD_MAX_TIMEOUT // 初始化命令的更长超时
        } else {
            CMD_DEFAULT_TIMEOUT
        };

        // if cmd.opcode == 8 {
        //     unsafe {dump_memory_region(0xfffff000fe310000, 0x900);}
        // }

        // 发送命令
        self.write_reg16(EMMC_COMMAND, command);

        // 等待命令完成
        let mut status: u16;
        loop {
            status = self.read_reg16(EMMC_NORMAL_INT_STAT);
            info!("Response Status: {:#b}", status);
            
            // 检查错误
            if status & EMMC_INT_ERROR as u16 != 0 {
                break;
            }
            
            // 检查响应完成
            if (status & int_mask) == int_mask {
                break;
            }
            
            // 检查超时
            if timeout_val <= 0 {
                info!("Timeout for status update!");
                return Err(SdError::Timeout);
            }
            
            timeout_val -= 1;
            delay_us(100);
        }

        // if cmd.opcode == 8 {
        //     unsafe {dump_memory_region(0xfffff000fe310000, 0x900);}
        // }
        
        // 处理命令完成
        if (status & (EMMC_INT_ERROR as u16 | int_mask)) == int_mask {
            // 命令成功完成
            info!("Command completed: status={:#b}", status);
            self.write_reg16(EMMC_NORMAL_INT_STAT, int_mask);
        } else {
            // 发生错误
            debug!("EMMC Normal Int Status: 0x{:x}", self.read_reg16(EMMC_NORMAL_INT_STAT));
            debug!("EMMC Error Int Status: 0x{:x}", self.read_reg16(EMMC_ERROR_INT_STAT));
            
            let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
            info!("Command error: status={:#b}, err_status={:#b}", status, err_status);
            
            // 复位命令和数据线
            self.reset_cmd()?;
            if cmd.data_present {
                self.reset_data()?;
            }
            
            // 映射具体错误类型
            let err = if err_status & 0x1 != 0 {
                    SdError::Timeout
                } else if err_status & 0x2 != 0 {
                    SdError::Crc
                } else if err_status & 0x4 != 0 {
                    SdError::EndBit
                } else if err_status & 0x8 != 0 {
                    SdError::Index
                } else if err_status & 0x10 != 0 {
                    SdError::DataTimeout
                } else if err_status & 0x20 != 0 {
                    SdError::DataCrc
                } else if err_status & 0x40 != 0 {
                    SdError::DataEndBit
                } else if err_status & 0x80 != 0 {
                    SdError::CurrentLimit
                } else {
                    SdError::CommandError
                };
            
            return Err(err);
        }

        // 处理数据传输部分（如果有的话）
        if cmd.data_present {
            debug!("Data transfer: cmd.data_present={}", cmd.data_present);
            if let Some(_buffer) = &mut data_buffer {
                self.transfer_data()?;
            } else {
                return Err(SdError::InvalidArgument);
            }
        }

        // 清除所有中断状态
        self.write_reg16(EMMC_NORMAL_INT_STAT, 0xFFFF);
        self.write_reg16(EMMC_ERROR_INT_STAT, 0xFFFF);

        self.reset(EMMC_RESET_CMD)?;
        self.reset(EMMC_RESET_DATA)?;
        
        Ok(())
    }

    // Reset command line
    pub fn reset_cmd(&self) -> Result<(), SdError> {
        self.write_reg8(EMMC_SOFTWARE_RESET, EMMC_RESET_CMD);

        // Wait for reset to complete
        let mut timeout = 100;
        while (self.read_reg8(EMMC_SOFTWARE_RESET) & EMMC_RESET_CMD) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
            delay_us(1000);
        }

        Ok(())
    }

    // Reset data line
    pub fn reset_data(&self) -> Result<(), SdError> {
        self.write_reg8(EMMC_SOFTWARE_RESET, EMMC_RESET_DATA);

        // Wait for reset to complete
        let mut timeout = 100;
        while (self.read_reg8(EMMC_SOFTWARE_RESET) & EMMC_RESET_DATA) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
            delay_us(1000);
        }

        Ok(())
    }

    // Get response from the last command
    pub fn get_response(&self) -> SdResponse {
        let mut response = SdResponse::new();
        response.raw[0] = self.read_reg(EMMC_RESPONSE);
        response.raw[1] = self.read_reg(EMMC_RESPONSE + 4);
        response.raw[2] = self.read_reg(EMMC_RESPONSE + 8);
        response.raw[3] = self.read_reg(EMMC_RESPONSE + 12);

        response
    }

    // Send CMD0 to reset the card
    pub fn mmc_go_idle(&self)  -> Result<(), SdError>{

        let cmd = EMmcCommand::new(MMC_GO_IDLE_STATE, 0, MMC_RSP_NONE);
        self.send_command(&cmd, None)?;

        delay_us(10000);

        info!("eMMC reset complete");
        Ok(())
    }
    
    // Send CMD1 to set OCR and check if card is ready
    pub fn mmc_send_op_cond(&mut self, ocr: u32, mut retry: u32) -> Result<u32, SdError> {
        // First command to get capabilities
        
        let mut cmd = EMmcCommand::new(MMC_SEND_OP_COND, ocr, MMC_RSP_R3);
        self.send_command(&cmd, None)?;
        delay_us(10000);
        
        // Get response and store it
        let mut card_ocr = self.get_response().as_r3();
        
        info!("eMMC first CMD1 response (no args): {:#x}", card_ocr);
        
        // Calculate arg for next commands
        let ocr_hcs = 0x40000000; // High Capacity Support
        let ocr_busy = 0x80000000;
        let ocr_voltage_mask = 0x007FFF80;
        let ocr_access_mode = 0x60000000;
        
        let cmd_arg = ocr_hcs | (self.voltages & (card_ocr & ocr_voltage_mask)) | 
                        (card_ocr & ocr_access_mode);

        info!("eMMC CMD1 arg for retries: {:#x}", cmd_arg);

        // Now retry with the proper argument until ready or timeout
        let mut ready = false;
        while retry > 0 && !ready {
            cmd = EMmcCommand::new(MMC_SEND_OP_COND, cmd_arg, MMC_RSP_R3);
            self.send_command(&cmd, None)?;
            let resp = self.get_response().as_r3();
            card_ocr = resp;

            info!("CMD1 response raw: {:#x}", self.read_reg(EMMC_RESPONSE));
            info!("eMMC CMD1 response: {:#x}", resp);
            
            // Update card OCR
            {
                let card = self.card.as_mut().unwrap();
                card.ocr = resp;
                
                // Check if card is ready (OCR_BUSY flag set)
                if (resp & ocr_busy) != 0 {
                    ready = true;
                    if (resp & ocr_hcs) != 0 {
                        card.card_type = CardType::MmcHc;
                        card.state |= MMC_STATE_HIGHCAPACITY;
                    }
                }
            }
            
            if !ready {
                retry -= 1;
                // Delay between retries
                delay_us(1000);
            }
        }
        
        info!("eMMC initialization status: {}", ready);
        
        if !ready {
            return Err(SdError::UnsupportedCard);
        }
        
        delay_us(1000);
        
        debug!("Clock control before CMD2: 0x{:x}, stable: {}", 
            self.read_reg16(EMMC_CLOCK_CONTROL),
            self.is_clock_stable());
        
        Ok(card_ocr)
    }
    
    // Send CMD2 to get CID
    pub fn mmc_all_send_cid(&mut self) -> Result<[u32; 4], SdError> {
        let cmd = EMmcCommand::new(MMC_ALL_SEND_CID, 0, MMC_RSP_R2);
        self.send_command(&cmd, None)?;
        let response = self.get_response();

        // Now borrow card as mutable to update it
        let card = self.card.as_mut().unwrap();
        card.cid = response.as_r2();

        Ok(card.cid)
    }

    // Send CMD3 to set RCA for eMMC
    pub fn mmc_set_relative_addr(&self) -> Result<(), SdError> {
        // Get the RCA value before borrowing the card
        let rca = self.card.as_ref().unwrap().rca;

        let cmd = EMmcCommand::new(MMC_SET_RELATIVE_ADDR, rca << 16, MMC_RSP_R1);
        self.send_command(&cmd,None)?;

        info!("cmd3 0x{:x}", self.get_response().as_r1());

        Ok(())
    }

    // Send CMD9 to get CSD
    pub fn mmc_send_csd(&mut self) -> Result<[u32; 4], SdError> {
        // Get the RCA value before borrowing the card
        let rca = self.card.as_ref().unwrap().rca;
        
        let cmd = EMmcCommand::new(MMC_SEND_CSD, rca << 16, MMC_RSP_R2);
        self.send_command(&cmd, None)?;
        let response = self.get_response();
        
        // Now borrow card as mutable to update it
        let card = self.card.as_mut().unwrap();
        card.csd = response.as_r2();

        Ok(card.csd)
    }

    pub fn mmc_send_ext_csd(&mut self, ext_csd: &mut DVec<u8>) -> Result<(), SdError> {
        let cmd = EMmcCommand::new(MMC_SEND_EXT_CSD, 0, MMC_RSP_R1)
            .with_data(MMC_MAX_BLOCK_LEN as u16, 1, true);
        
        self.send_command(&cmd, Some(DataBuffer::Read(ext_csd)))?;

        debug!("CMD8: {:#x}",self.get_response().as_r1());
        
        debug!("EXT_CSD read successfully, rev: {}", ext_csd[EXT_CSD_REV as usize]);
        
        Ok(())
    }

    pub fn mmc_poll_for_busy(&self, send_status: bool) -> Result<(), SdError> {
        let mut busy = true;
        let mut timeout = 1000;

        // 轮询等待卡忙状态结束
        while busy {
            if send_status {
                let cmd = EMmcCommand::new(MMC_SEND_STATUS, self.card.as_ref().unwrap().rca << 16, MMC_RSP_R1);
                self.send_command(&cmd, None)?;
                let response = self.get_response().as_r1();
                debug!("cmd_d {:#x}", response);

                if response & MMC_STATUS_SWITCH_ERROR != 0 {
                    return Err(SdError::BadMessage);
                }
                busy = (response & MMC_STATUS_CURR_STATE) == MMC_STATE_PRG;
                if !busy {
                    break;
                }
            } else {
                busy = self.mmc_card_busy();
            }
            
            if timeout == 0 && busy {
                return Err(SdError::Timeout);
            }

            timeout -= 1;
            delay_us(1000);
        }

        Ok(())
    }

    pub fn mmc_card_busy(&self) -> bool {        
        let present_state = self.read_reg(EMMC_PRESENT_STATE);
        // 检查DATA[0]线是否为0（低电平表示忙）
        let is_busy = !(present_state & EMMC_DATA_0_LVL != 0);
        
        is_busy
    }
}
