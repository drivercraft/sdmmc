use log::{debug, info};

use crate::{delay_us, emmc::CardType, err::SdError};

use super::{block::EMmcCard, constant::*, EMmcHost};

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
        self.raw
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
    // Send a command to the card
    pub fn send_command(&self, cmd: &EMmcCommand) -> Result<(), SdError> {
        // Check if command or data lines are busy
        let mut timeout = 100;
        while (self.read_reg(EMMC_PRESENT_STATE) & (EMMC_CMD_INHIBIT | EMMC_DATA_INHIBIT)) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
            delay_us(1000);
        }
        
        self.write_reg16(EMMC_NORMAL_INT_STAT, 0xFFFF);
        self.write_reg16(EMMC_ERROR_INT_STAT, 0xFFFF);

        info!(
            "Sending command: opcode={:#x}, arg={:#x}, resp_type={:#x}",
            cmd.opcode, cmd.arg, cmd.resp_type
        );

        // Set argument
        self.write_reg(EMMC_ARGUMENT, cmd.arg);

        // Set up transfer mode if data is present
        if cmd.data_present {
            // Set block size and count
            self.write_reg16(EMMC_BLOCK_SIZE, cmd.block_size);
            self.write_reg16(EMMC_BLOCK_COUNT, cmd.block_count);

            // Set transfer mode
            let mut mode = EMMC_TRNS_BLK_CNT_EN;
            if cmd.block_count > 1 {
                mode |= EMMC_TRNS_MULTI;
            }

            if cmd.data_dir_read {
                mode |= EMMC_TRNS_READ;
            }

            // For simplicity, we use programmed I/O, not DMA
            self.write_reg16(EMMC_XFER_MODE, mode);
        }

        // Set command register
        let mut command = (cmd.opcode as u16) << 8;

        // Map response type to EMMC format
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

        // Send the command
        self.write_reg16(EMMC_COMMAND, command);

        // Use longer timeout for initialization commands
        let timeout_val = if cmd.opcode == MMC_GO_IDLE_STATE || cmd.opcode == MMC_SEND_OP_COND {
            500 // Longer timeout for initialization commands
        } else {
            100 // Standard timeout
        };

        // Wait for command completion using polling
        let mut timeout = timeout_val;
        while timeout > 0 {
            let status = self.read_reg16(EMMC_NORMAL_INT_STAT);

            info!("Repsonse Status: {:#b}", status);

            // Check for command completion
            if status & EMMC_INT_RESPONSE as u16 != 0 {
                // Command completed successfully
                info!("Command completed: status={:#b}", status);

                debug!(
                    "EMMC Normal Int Status: 0x{:x}",
                    self.read_reg16(EMMC_NORMAL_INT_STAT)
                );
                debug!(
                    "EMMC Error Int Status: 0x{:x}",
                    self.read_reg16(EMMC_ERROR_INT_STAT)
                );

                // Check for errors
                if status & (1 << 15) != 0 {
                    // ERROR_INT_STAT bit
                    let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
                    info!(
                        "Command error: status={:#b}, err_status={:#b}",
                        status, err_status
                    );

                    // Reset command line
                    self.reset_cmd()?;
                    if cmd.data_present {
                        self.reset_data()?;
                    }

                    // Map specific error types
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

                break;
            }

            timeout -= 1;
            delay_us(1000); // Delay to avoid busy-waiting
        }

        if timeout == 0 {
            info!("Command timeout waiting for response");
            self.reset_cmd()?;
            return Err(SdError::Timeout);
        }

        // If data is present, wait for data completion
        if cmd.data_present {
            timeout = 100;
            while timeout > 0 {
                let status = self.read_reg16(EMMC_NORMAL_INT_STAT);

                // Check for data completion
                if status & EMMC_INT_DATA_END as u16 != 0 {
                    // Data transfer completed
                    info!("Data transfer completed: status={:#b}", status);
                    self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_DATA_END as u16);
                    break;
                }

                // Check for data errors
                if status & (1 << 15) != 0 {
                    // ERROR_INT_STAT bit
                    let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
                    info!(
                        "Data error: status={:#b}, err_status={:#b}",
                        status, err_status
                    );

                    // Reset data line
                    self.reset_data()?;

                    // Map specific data error
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

                timeout -= 1;
                delay_us(1000); // Delay to avoid busy-waiting
            }

            if timeout == 0 {
                info!("Data timeout");
                self.reset_data()?;
                return Err(SdError::DataTimeout);
            }
        }

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
        self.send_command(&cmd)?;

        delay_us(10000);

        info!("eMMC reset complete");
        Ok(())
    }
    
    // Send CMD1 to set OCR and check if card is ready
    pub fn mmc_send_op_cond(&mut self, ocr: u32, mut retry: u32) -> Result<(), SdError> {
        // First command to get capabilities
        let mut cmd = EMmcCommand::new(MMC_SEND_OP_COND, ocr, MMC_RSP_R3);
        self.send_command(&cmd)?;
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
            self.send_command(&cmd)?;
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
    pub fn mmc_all_send_cid(&mut self) -> Result<(), SdError> {
        let cmd = EMmcCommand::new(MMC_ALL_SEND_CID, 0, MMC_RSP_R2);
        self.send_command(&cmd)?;
        let response = self.get_response();

        // Now borrow card as mutable to update it
        let card = self.card.as_mut().unwrap();
        card.cid = response.as_r2();

        info!("eMMC Card CID: 0x{:x} 0x{:x} 0x{:x} 0x{:x}", 
            response.as_r2()[0], response.as_r2()[1], 
            response.as_r2()[2], response.as_r2()[3]);

        Ok(())
    }

    // Send CMD3 to set RCA for eMMC
    pub fn mmc_set_relative_addr(&self) -> Result<(), SdError> {
        // Get the RCA value before borrowing the card
        let rca = self.card.as_ref().unwrap().rca;

        let cmd = EMmcCommand::new(MMC_SET_RELATIVE_ADDR, rca << 16, MMC_RSP_R1);
        self.send_command(&cmd)?;

        info!("cmd3 0x{:x}", self.get_response().as_r1());

        Ok(())
    }

    // Send CMD9 to get CSD
    pub fn mmc_send_csd(&mut self) -> Result<(), SdError> {
        // Get the RCA value before borrowing the card
        let rca = self.card.as_ref().unwrap().rca;
        
        let cmd = EMmcCommand::new(MMC_SEND_CSD, rca << 16, MMC_RSP_R2);
        self.send_command(&cmd)?;
        let response = self.get_response();
        
        // Now borrow card as mutable to update it
        let card = self.card.as_mut().unwrap();
        card.csd = response.as_r2();

        info!("eMMC Card CSD: 0x{:x} 0x{:x} 0x{:x} 0x{:x}",
            response.as_r2()[0], response.as_r2()[1], 
            response.as_r2()[2], response.as_r2()[3]);

        Ok(())
    }
}
