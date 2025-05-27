// ===== Block Device Interface =====

use aux::MMC_VERSION_UNKNOWN;
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "dma")]
use {
    dma_api::DVec,
    crate::delay_us,
    log::{debug, info},
};

use log::trace;

use crate::err::SdError;

use super::{CardType, EMmcHost, aux, cmd::EMmcCommand, constant::*};

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

    /// Read one or more data blocks from the card
    #[cfg(feature = "dma")]
    pub fn read_blocks(
        &self,
        block_id: u32,
        blocks: u16,
        buffer: &mut DVec<u8>,
    ) -> Result<(), SdError> {
        // Check if buffer size matches the expected size based on number of blocks
        let expected_size = blocks as usize * 512;
        if buffer.len() != expected_size {
            return Err(SdError::IoError);
        }

        // Check if card is initialized and extract card information
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // Adjust block address based on card type
        // High capacity cards use block addressing, standard capacity cards use byte addressing
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id // High capacity card: use block address directly
        } else {
            block_id * 512 // Standard capacity card: convert to byte address
        };

        trace!(
            "Reading {} blocks starting at address: {:#x}",
            blocks, card_addr
        );

        // Select appropriate command based on number of blocks
        if blocks == 1 {
            // Single block read operation
            let cmd = EMmcCommand::new(MMC_READ_SINGLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, 1, true); // Configure for reading 512 bytes with DMA
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
        } else {
            // Multiple block read operation
            let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, true); // Configure for reading multiple blocks with DMA

            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;

            // Must send stop transmission command after multiple block read
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    /// Write multiple blocks to the card
    #[cfg(feature = "dma")]
    pub fn write_blocks(
        &self,
        block_id: u32,
        blocks: u16,
        buffer: &DVec<u8>,
    ) -> Result<(), SdError> {
        // Verify that buffer size matches the requested number of blocks
        let expected_size = blocks as usize * 512;
        if buffer.len() != expected_size {
            return Err(SdError::IoError);
        }

        // Extract card information and check if card exists
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // Check if card is properly initialized
        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Check if card is write protected
        if self.is_write_protected() {
            return Err(SdError::IoError);
        }

        // Determine the correct address based on card capacity type
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id // High capacity card: use block address directly
        } else {
            block_id * 512 // Standard capacity card: convert to byte address
        };

        trace!(
            "Writing {} blocks starting at address: {:#x}",
            blocks, card_addr
        );

        // Select appropriate command based on number of blocks
        if blocks == 1 {
            // Single block write operation
            let cmd =
                EMmcCommand::new(MMC_WRITE_BLOCK, card_addr, MMC_RSP_R1).with_data(512, 1, false); // Configure for writing 512 bytes with DMA (false = write)
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
        } else {
            // Multiple block write operation
            let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, false); // Configure for writing multiple blocks

            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;

            // Must send stop transmission command after multiple block write
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    /// Transfer data using DMA mode
    /// This function polls for transfer completion or errors
    #[cfg(feature = "dma")]
    pub fn transfer_data_by_dma(&self) -> Result<(), SdError> {
        let mut timeout = 100;

        loop {
            // Read the interrupt status register
            let stat = self.read_reg16(EMMC_NORMAL_INT_STAT);
            trace!("Transfer status: {:#b}", stat);

            // Check for any errors during transfer
            if stat & EMMC_INT_ERROR as u16 != 0 {
                let err_status = self.read_reg16(EMMC_ERROR_INT_STAT);
                trace!(
                    "Data transfer error: status={:#b}, err_status={:#b}",
                    stat, err_status
                );

                // Reset the data circuit to recover from error
                self.reset_data()?;

                // Determine specific error type based on error status bits
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

            // Check if data transfer is complete
            if stat & EMMC_INT_DATA_END as u16 != 0 {
                // Clear the data end interrupt flag
                self.write_reg16(EMMC_NORMAL_INT_STAT, EMMC_INT_DATA_END as u16);
                break;
            }

            // Handle timeout to prevent infinite loop
            if timeout > 0 {
                timeout -= 1;
                delay_us(1000); // Wait 1ms before checking again
            } else {
                info!("Data transfer timeout");
                return Err(SdError::DataTimeout);
            }
        }

        Ok(())
    }

    /// Read blocks from SD card using PIO (Programmed I/O) mode
    /// Parameters:
    /// - block_id: Starting block address to read from
    /// - blocks: Number of blocks to read
    /// - buffer: Buffer to store the read data
    #[cfg(feature = "pio")]
    pub fn read_blocks(
        &self,
        block_id: u32,
        blocks: u16,
        buffer: &mut [u8],
    ) -> Result<(), SdError> {
        trace!(
            "pio read_blocks: block_id = {}, blocks = {}",
            block_id, blocks
        );
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // Adjust block address based on card type
        // High capacity cards use block addressing, standard capacity cards use byte addressing
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id // High capacity card: use block address directly
        } else {
            block_id * 512 // Standard capacity card: convert to byte address
        };

        trace!(
            "Reading {} blocks starting at address: {:#x}",
            blocks, card_addr
        );

        if blocks == 1 {
            // Single block read operation
            let cmd = EMmcCommand::new(MMC_READ_SINGLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, 1, true);
            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;
        } else {
            // Multiple block read operation
            let cmd = EMmcCommand::new(MMC_READ_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, true);

            self.send_command(&cmd, Some(DataBuffer::Read(buffer)))?;

            // Must send stop transmission command after multiple block read
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    /// Write blocks to SD card using PIO (Programmed I/O) mode
    /// Parameters:
    /// - block_id: Starting block address to write to
    /// - blocks: Number of blocks to write
    /// - buffer: Buffer containing data to write
    #[cfg(feature = "pio")]
    pub fn write_blocks(&self, block_id: u32, blocks: u16, buffer: &[u8]) -> Result<(), SdError> {
        use log::trace;

        trace!(
            "pio write_blocks: block_id = {}, blocks = {}",
            block_id, blocks
        );
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        // Check if card is write protected
        if self.is_write_protected() {
            return Err(SdError::IoError);
        }

        // Determine the correct address based on card capacity type
        let card_addr = if card.state & MMC_STATE_HIGHCAPACITY != 0 {
            block_id // High capacity card: use block address directly
        } else {
            block_id * 512 // Standard capacity card: convert to byte address
        };

        trace!(
            "Writing {} blocks starting at address: {:#x}",
            blocks, card_addr
        );

        // Select appropriate command based on number of blocks
        if blocks == 1 {
            // Single block write operation
            let cmd =
                EMmcCommand::new(MMC_WRITE_BLOCK, card_addr, MMC_RSP_R1).with_data(512, 1, false);
            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;
        } else {
            // Multiple block write operation
            let cmd = EMmcCommand::new(MMC_WRITE_MULTIPLE_BLOCK, card_addr, MMC_RSP_R1)
                .with_data(512, blocks, false);

            self.send_command(&cmd, Some(DataBuffer::Write(buffer)))?;

            // Must send stop transmission command after multiple block write
            let stop_cmd = EMmcCommand::new(MMC_STOP_TRANSMISSION, 0, MMC_RSP_R1B);
            self.send_command(&stop_cmd, None)?;
        }

        Ok(())
    }

    /// Transfer data using PIO (Programmed I/O) mode
    /// This function manually reads/writes data to/from the controller buffer
    /// Parameters:
    /// - data_dir_read: True for read operation, false for write
    /// - buffer: Buffer to read data into or write data from
    pub fn transfer_data_by_pio(
        &self,
        data_dir_read: bool,
        buffer: &mut [u8],
    ) -> Result<(), SdError> {
        // Process data in 16-byte chunks (4 words at a time)
        for i in (0..buffer.len()).step_by(16) {
            if data_dir_read {
                // Read operation: controller buffer -> memory
                let mut values = [0u32; 4];
                for j in 0..4 {
                    if i + j * 4 < buffer.len() {
                        // Read 32-bit word from controller buffer
                        values[j] = self.read_reg(EMMC_BUF_DATA);

                        if i + j * 4 + 3 < buffer.len() {
                            // Unpack 32-bit word into 4 bytes in little-endian order
                            buffer[i + j * 4] = (values[j] & 0xFF) as u8;
                            buffer[i + j * 4 + 1] = ((values[j] >> 8) & 0xFF) as u8;
                            buffer[i + j * 4 + 2] = ((values[j] >> 16) & 0xFF) as u8;
                            buffer[i + j * 4 + 3] = ((values[j] >> 24) & 0xFF) as u8;
                        }
                    }
                }

                trace!(
                    "0x{:08x}: 0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x}",
                    buffer.as_ptr() as usize + i,
                    values[0],
                    values[1],
                    values[2],
                    values[3]
                );
            } else {
                // Write operation: memory -> controller buffer
                let mut values = [0u32; 4];
                for j in 0..4 {
                    if i + j * 4 + 3 < buffer.len() {
                        // Pack 4 bytes into 32-bit word in little-endian order
                        values[j] = (buffer[i + j * 4] as u32)
                            | ((buffer[i + j * 4 + 1] as u32) << 8)
                            | ((buffer[i + j * 4 + 2] as u32) << 16)
                            | ((buffer[i + j * 4 + 3] as u32) << 24);

                        // Write 32-bit word to controller buffer
                        self.write_reg(EMMC_BUF_DATA, values[j]);
                    }
                }

                trace!(
                    "0x{:08x}: 0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x}",
                    buffer.as_ptr() as usize + i,
                    values[0],
                    values[1],
                    values[2],
                    values[3]
                );
            }
        }

        Ok(())
    }

    /// Write data to SD card buffer register
    /// This is a lower-level function used by data transfer operations
    pub fn write_buffer(&self, buffer: &[u8]) -> Result<(), SdError> {
        // Wait until space is available in the controller buffer
        self.wait_for_interrupt(EMMC_INT_SPACE_AVAIL, 100000)?;

        let len = buffer.len();
        // Write data in 4-byte chunks
        for i in (0..len).step_by(4) {
            // Pack bytes into a 32-bit word, handling potential buffer underrun
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

            // Write the 32-bit word to the buffer data register
            self.write_reg(EMMC_BUF_DATA, val);
        }

        // Wait for data transfer to complete
        self.wait_for_interrupt(EMMC_INT_DATA_END, 1000000)?;

        Ok(())
    }

    /// Read data from SD card buffer register
    /// This is a lower-level function used by data transfer operations
    pub fn read_buffer(&self, buffer: &mut [u8]) -> Result<(), SdError> {
        // Wait until data is available in the controller buffer
        self.wait_for_interrupt(EMMC_INT_DATA_AVAIL, 100000)?;

        // Read data into buffer in 4-byte chunks
        let len = buffer.len();
        for i in (0..len).step_by(4) {
            // Read 32-bit word from buffer data register
            let val = self.read_reg(EMMC_BUF_DATA);

            // Unpack the 32-bit word into individual bytes, handling buffer boundary
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

        // Wait for data transfer to complete
        self.wait_for_interrupt(EMMC_INT_DATA_END, 100000)?;

        Ok(())
    }

    /// Wait for a specific interrupt flag to be set
    /// Helper function used by data transfer operations
    /// Parameters:
    /// - flag: The interrupt flag to wait for
    /// - timeout_count: Maximum number of iterations to wait
    fn wait_for_interrupt(&self, flag: u32, timeout_count: u32) -> Result<(), SdError> {
        let mut timeout = timeout_count;
        while timeout > 0 {
            // Read the current interrupt status
            let int_status = self.read_reg(EMMC_NORMAL_INT_STAT);

            // Check if the target flag is set
            if int_status & flag != 0 {
                // Clear the flag by writing back to the register
                self.write_reg16(EMMC_NORMAL_INT_STAT, flag as u16);
                return Ok(());
            }

            // Check for any error flags
            if int_status & EMMC_INT_ERROR_MASK != 0 {
                // Clear error flags
                self.write_reg16(
                    EMMC_NORMAL_INT_STAT,
                    (int_status & EMMC_INT_ERROR_MASK) as u16,
                );
                // Reset the data circuit
                self.reset_data()?;
                return Err(SdError::DataError);
            }

            timeout -= 1;
        }

        // If we reached the timeout limit, return timeout error
        if timeout == 0 {
            return Err(SdError::DataTimeout);
        }

        Ok(())
    }
}
