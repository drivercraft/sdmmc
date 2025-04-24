use log::{debug, info};

use crate::{delay_us, emmc::CardType, err::SdError};

use super::{block::DataBuffer, constant::*, EMmcHost};

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
    pub fn send_command(&self, cmd: &EMmcCommand, data_buffer: Option<DataBuffer>) -> Result<(), SdError> {
        // 动态超时时间配置
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

        // 设置参数
        self.write_reg(EMMC_ARGUMENT, cmd.arg);
        
        // 设置预期中断掩码
        let mut int_mask = EMMC_INT_RESPONSE as u16;
        
        // 如果有数据且响应类型包含BUSY标志，则也等待数据结束中断
        if cmd.data_present && (cmd.resp_type & MMC_RSP_BUSY != 0) {
            int_mask |= EMMC_INT_DATA_END as u16;
        }

        // 设置数据传输相关寄存器
        if cmd.data_present {
            // 设置更长的超时
            self.write_reg8(EMMC_TIMEOUT_CONTROL, 0xe);
            
            // 设置块大小和数量
            self.write_reg16(EMMC_BLOCK_SIZE, cmd.block_size);
            self.write_reg16(EMMC_BLOCK_COUNT, cmd.block_count);

            // 设置传输模式
            let mut mode = EMMC_TRNS_BLK_CNT_EN;
            if cmd.block_count > 1 {
                mode |= EMMC_TRNS_MULTI;
            }

            if cmd.data_dir_read {
                mode |= EMMC_TRNS_READ;
            }

            // 传输模式配置
            self.write_reg16(EMMC_XFER_MODE, mode);
            
            // 预处理数据缓冲区
            if let Some(buffer) = &data_buffer {
                match (buffer, cmd.data_dir_read) {
                    (DataBuffer::Read(_), true) => {
                        // 读操作，不需要预处理
                    },
                    (DataBuffer::Write(write_buf), false) => {
                        // 写操作，准备数据
                        if cmd.data_dir_read {
                            return Err(SdError::InvalidArgument); // 缓冲区类型与操作类型不匹配
                        }
                        
                        // 对于多块写入，仅准备第一块数据
                        // 其余数据将在命令执行后写入
                        if cmd.block_count == 1 {
                            self.write_data_buffer(write_buf)?;
                        }
                    },
                    _ => return Err(SdError::InvalidArgument), // 缓冲区类型与操作类型不匹配
                }
            } else if cmd.data_present {
                return Err(SdError::InvalidArgument); // 需要缓冲区但未提供
            }
        } else if cmd.resp_type & MMC_RSP_BUSY != 0 {
            // 对于带BUSY的命令，但没有数据的情况，仍然设置超时控制
            self.write_reg8(EMMC_TIMEOUT_CONTROL, 0xe);
        }

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

        // 发送命令
        self.write_reg16(EMMC_COMMAND, command);
        
        // 特殊命令特殊处理
        // 使用更长的超时时间进行初始化命令
        let mut timeout_val = if cmd.opcode == MMC_GO_IDLE_STATE || cmd.opcode == MMC_SEND_OP_COND {
            CMD_MAX_TIMEOUT // 初始化命令的更长超时
        } else {
            CMD_DEFAULT_TIMEOUT
        };
        
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
            info!("Data transfer: block_size={}, block_count={}", 
                cmd.block_size, cmd.block_count);
            if let Some(buffer) = data_buffer {
                match (buffer, cmd.data_dir_read) {
                    (DataBuffer::Read(read_buf), true) => {
                        // 等待数据准备好
                        let mut data_timeout = CMD_DEFAULT_TIMEOUT;
                        debug!("Waiting for data to be ready...");
                        loop {
                            status = self.read_reg16(EMMC_NORMAL_INT_STAT);

                            info!("Data Status: {:#b}", status);
                            
                            // 检查数据传输完成
                            if status & EMMC_INT_DATA_END as u16 != 0 {
                                info!("Data transfer completed: status={:#b}", status);
                                self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_DATA_END as u16);
                                
                                // 读取数据
                                self.read_data_buffer(read_buf)?;
                                
                                break;
                            }
                            
                            // 检查数据错误
                            if status & EMMC_INT_ERROR as u16 != 0 {
                                let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
                                info!("Data error: status={:#b}, err_status={:#b}", status, err_status);
                                
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
                            
                            // 检查超时
                            if data_timeout <= 0 {
                                info!("Data timeout");
                                self.reset_data()?;
                                return Err(SdError::DataTimeout);
                            }
                            
                            data_timeout -= 1;
                            delay_us(100000);
                        }
                    },
                    (DataBuffer::Write(write_buf), false) => {
                        // 对于多块写入，写入剩余的块
                        if cmd.block_count > 1 {
                            let block_size = cmd.block_size as usize;
                            
                            // 第一块已经在命令之前写入
                            // 写入剩余的块
                            for i in 0..cmd.block_count {
                                let start = i as usize * block_size;
                                let end = start + block_size;
                                
                                if end <= write_buf.len() {
                                    // 等待缓冲区准备好接收数据
                                    let mut buffer_ready_timeout = CMD_DEFAULT_TIMEOUT;
                                    
                                    while (self.read_reg16(EMMC_NORMAL_INT_STAT)) == 0 {
                                        if buffer_ready_timeout <= 0 {
                                            info!("Buffer write ready timeout");
                                            self.reset_data()?;
                                            return Err(SdError::DataTimeout);
                                        }
                                        
                                        buffer_ready_timeout -= 1;
                                        delay_us(100);
                                    }
                                    
                                    // // 清除缓冲区就绪中断
                                    // self.write_reg16(EMMC_NORMAL_INT_STAT,  as u16);
                                    
                                    // 写入数据块
                                    self.write_data_buffer(&write_buf[start..end])?;
                                }
                            }
                        }
                        
                        // 等待数据传输完成
                        let mut data_timeout = CMD_DEFAULT_TIMEOUT;
                        
                        loop {
                            status = self.read_reg16(EMMC_NORMAL_INT_STAT);
                            
                            // 检查数据传输完成
                            if status & EMMC_INT_DATA_END as u16 != 0 {
                                info!("Data transfer completed: status={:#b}", status);
                                self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_DATA_END as u16);
                                break;
                            }
                            
                            // 检查数据错误
                            if status & EMMC_INT_ERROR as u16 != 0 {
                                let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
                                info!("Data error: status={:#b}, err_status={:#b}", status, err_status);
                                
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
                            
                            // 检查超时
                            if data_timeout <= 0 {
                                info!("Data timeout");
                                self.reset_data()?;
                                return Err(SdError::DataTimeout);
                            }
                            
                            data_timeout -= 1;
                            delay_us(100000);
                        }
                    },
                    _ => return Err(SdError::InvalidArgument), // 缓冲区类型与操作类型不匹配
                }
            } else {
                // 无缓冲区但有数据传输，返回错误
                return Err(SdError::InvalidArgument);
            }
        }
        
        // 成功完成，复位命令和数据线以准备下一个命令
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
    pub fn mmc_send_op_cond(&mut self, ocr: u32, mut retry: u32) -> Result<(), SdError> {
        // First command to get capabilities
        let mut cmd = EMmcCommand::new(MMC_SEND_OP_COND, ocr, MMC_RSP_R3);
        self.send_command(&cmd, None)?;
        delay_us(10000);
        
        // Get response and store it
        let response = self.get_response().as_r3();
        {
            let card = self.card.as_mut().unwrap();
            card.ocr = response;
        }
        
        info!("eMMC first CMD1 response (no args): {:#x}", response);
        
        // Calculate arg for next commands
        let ocr_hcs = 0x40000000; // High Capacity Support
        let ocr_busy = 0x80000000;
        let ocr_voltage_mask = 0x007FFF80;
        let ocr_access_mode = 0x60000000;
        
        // Get card OCR for calculation
        let card_ocr;
        {
            card_ocr = self.card.as_ref().unwrap().ocr;
        }
        
        let cmd_arg = ocr_hcs | (self.voltages & (card_ocr & ocr_voltage_mask)) | 
                        (card_ocr & ocr_access_mode);

        info!("eMMC CMD1 arg for retries: {:#x}", cmd_arg);
        
        // Now retry with the proper argument until ready or timeout
        let mut ready = false;
        while retry > 0 && !ready {
            cmd = EMmcCommand::new(MMC_SEND_OP_COND, cmd_arg, MMC_RSP_R3);
            self.send_command(&cmd, None)?;
            let resp = self.get_response().as_r3();

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
        
        Ok(())
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
}
