use log::info;

use crate::err::SdError;

use super::{constant::*, EMmcHost};

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
        // Clear any pending interrupts before sending command
        self.write_reg(EMMC_NORMAL_INT_STAT, EMMC_INT_ALL_MASK);
        
        // Check if command or data lines are busy
        let mut timeout = 100000;
        while (self.read_reg(EMMC_PRESENT_STATE) & (EMMC_CMD_INHIBIT | EMMC_DATA_INHIBIT)) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

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

        // Send the command
        self.write_reg16(EMMC_COMMAND, command);

        // Use longer timeout for initialization commands
        let timeout_val = if cmd.opcode == MMC_GO_IDLE_STATE || cmd.opcode == MMC_SEND_OP_COND {
            500000  // Longer timeout for initialization commands
        } else {
            100000  // Standard timeout
        };

        // Wait for command completion
        let mut timeout = timeout_val;
        let mut int_status = 0;
        while timeout > 0 {
            int_status = self.read_reg(EMMC_NORMAL_INT_STAT);
            if int_status & EMMC_INT_ERROR_MASK != 0 {
                // Command error
                self.reset_cmd()?;
                if cmd.data_present {
                    self.reset_data()?;
                }

                // Map error to appropriate type
                let err = if int_status & EMMC_INT_TIMEOUT != 0 {
                    SdError::Timeout
                } else if int_status & EMMC_INT_CRC != 0 {
                    SdError::Crc
                } else if int_status & EMMC_INT_END_BIT != 0 {
                    SdError::EndBit
                } else if int_status & EMMC_INT_INDEX != 0 {
                    SdError::Index
                } else if int_status & EMMC_INT_DATA_TIMEOUT != 0 {
                    SdError::DataTimeout
                } else if int_status & EMMC_INT_DATA_CRC != 0 {
                    SdError::DataCrc
                } else if int_status & EMMC_INT_DATA_END_BIT != 0 {
                    SdError::DataEndBit
                } else {
                    SdError::CommandError
                };

                return Err(err);
            }

            if int_status & EMMC_INT_RESPONSE != 0 {
                // Command completed successfully
                break;
            }

            timeout -= 1;
        }

        if timeout == 0 {
            return Err(SdError::Timeout);
        }

        // If data is present, wait for data completion
        if cmd.data_present {
            timeout = 1000000;
            while timeout > 0 {
                int_status = self.read_reg(EMMC_NORMAL_INT_STAT);
                if int_status & EMMC_INT_ERROR_MASK != 0 {
                    // Data error
                    self.reset_data()?;
                    return Err(SdError::DataCrc);
                }

                if int_status & EMMC_INT_DATA_END != 0 {
                    // Data transfer completed
                    break;
                }

                timeout -= 1;
            }

            if timeout == 0 {
                return Err(SdError::DataTimeout);
            }
        }

        // Clear interrupt status
        self.write_reg(EMMC_NORMAL_INT_STAT, int_status);

        Ok(())
    }

    // Reset command line
    fn reset_cmd(&self) -> Result<(), SdError> {
        self.write_reg8(EMMC_SOFTWARE_RESET, EMMC_RESET_CMD);

        // Wait for reset to complete
        let mut timeout = 100000;
        while (self.read_reg8(EMMC_SOFTWARE_RESET) & EMMC_RESET_CMD) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
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
}