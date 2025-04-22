extern crate alloc;

mod cmd;
mod block;
mod config;
mod constant;
mod rockchip;
mod regs;
mod info;

pub mod aux;
pub mod clock;

use core::fmt::Display;
use aux::{MMC_VERSION_1_2, MMC_VERSION_1_4, MMC_VERSION_2_2, MMC_VERSION_3, MMC_VERSION_4, MMC_VERSION_UNKNOWN};
use block::EMmcCard;
use clock::emmc_get_clk;
use constant::*;
use cmd::*;
use info::CardType;
use smccc::arch::Version;
use crate::{delay_us, err::*, generic_fls};
use log::{debug, info};

// SD Host Controller structure
#[derive(Debug)]
pub struct EMmcHost {
    base_addr: usize,
    card: Option<EMmcCard>,
    caps: u32,
    clock_base: u32,
    voltages: u32,
    quirks: u32,
    clock: u32,
    version: u16,
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
            voltages: 0,
            quirks: 0,
            clock: 0,
            version: 0,
        };

        // Read capabilities
        host.caps = host.read_reg(EMMC_CAPABILITIES1);

        // Calculate base clock from capabilities
        host.clock_base = (host.caps >> 8) & 0xFF;
        host.clock_base *= 1000000; // convert to Hz

        info!("EMMC Controller created: {}", host);

        host
    }

    // Initialize the host controller
    pub fn init(&mut self) -> Result<(), SdError> {
        info!("Init EMMC Controller");

        // Create card structure
        self.add_card(EMmcCard::init(CardType::Unknown));

        // Reset the controller
        self.reset(EMMC_RESET_ALL)?;

        let is_card_inserted = self.is_card_present();
        debug!("Card inserted: {}", is_card_inserted);

        let version = self.read_reg16(EMMC_HOST_CNTRL_VER);
        // version = 4.2
        self.version = version;
        info!("EMMC Version: 0x{:x}", version);

        let caps1 = self.read_reg(EMMC_CAPABILITIES1);
        info!("EMMC Capabilities 1: 0b{:b}", caps1);

        let mut clk_mul: u32 = 0;

        if (version & EMMC_SPEC_VER_MASK) >= EMMC_SPEC_300 {
            let caps2 = self.read_reg(EMMC_CAPABILITIES2);
            info!("EMMC Capabilities 2: 0b{:b}", caps2);
            clk_mul = (caps2 & EMMC_CLOCK_MUL_MASK) >> EMMC_CLOCK_MUL_SHIFT;
        }

        if self.clock_base == 0 {
            if (version & EMMC_SPEC_VER_MASK) >= EMMC_SPEC_300 {
                self.clock_base = (caps1 & EMMC_CLOCK_V3_BASE_MASK) >> EMMC_CLOCK_BASE_SHIFT
            } else {
                self.clock_base = (caps1 & EMMC_CLOCK_BASE_MASK) >> EMMC_CLOCK_BASE_SHIFT
            }

            self.clock_base *= 1000000; // convert to Hz
            if clk_mul != 0 {
                self.clock_base *= clk_mul;
            }
        }

        if self.clock_base == 0 {
            info!("Hardware doesn't specify base clock frequency");
            return Err(SdError::UnsupportedCard);
        }

        let mut voltages = 0;

        if (caps1 & EMMC_CAN_VDD_330) != 0 {
            voltages |= MMC_VDD_32_33 | MMC_VDD_33_34;
        } else if (caps1 & EMMC_CAN_VDD_300) != 0 {
            voltages |= MMC_VDD_29_30 | MMC_VDD_30_31;
        } else if (caps1 & EMMC_CAN_VDD_180) != 0 {
            voltages |= MMC_VDD_165_195;
        } else {
            info!("Unsupported voltage range");
            return Err(SdError::UnsupportedCard);
        } 

        self.voltages = voltages;

        info!("voltage range: {:#x}, {:#x}", voltages, generic_fls(voltages as u32) - 1);

        // Perform full power cycle
        self.sdhci_set_power(generic_fls(voltages as u32) - 1)?;

        // Enable interrupts
        self.write_reg(EMMC_NORMAL_INT_STAT_EN, EMMC_INT_CMD_MASK | EMMC_INT_DATA_MASK);
        self.write_reg(EMMC_SIGNAL_ENABLE, 0x0);

        // Set initial bus width to 1-bit
        self.mmc_set_bus_width(1);

        // Set initial clock and wait for it to stabilize
        debug!("emmc_get_clk {:?}", emmc_get_clk());
        self.mmc_set_clock(400000);

        self.mmc_set_timing(MMC_TIMING_LEGACY);

        // Initialize the card
        self.init_card()?;
        
        info!("EMMC initialization completed successfully");
        Ok(())
    }

    // Reset the controller
    fn reset(&self, mask: u8) -> Result<(), SdError> {
        // Request reset
        self.write_reg8(EMMC_SOFTWARE_RESET, mask);

        // Wait for reset to complete with timeout
        let mut timeout = 20; // Increased timeout
        while (self.read_reg8(EMMC_SOFTWARE_RESET) & mask) != 0 {
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
            delay_us(1000);
        }

        Ok(())
    }

    // Check if card is present
    fn is_card_present(&self) -> bool {
        let state = self.read_reg(EMMC_PRESENT_STATE);
        debug!("EMMC Present State: {:#b}", state);
        ((state & EMMC_CARD_INSERTED)) != 0 && ((state & EMMC_CARD_STABLE) != 0)
    }

    // Check if card is write protected
    fn is_write_protected(&self) -> bool {
        let state = self.read_reg(EMMC_PRESENT_STATE);
        (state & EMMC_WRITE_PROTECT) != 0
    }

    // Initialize the eMMC card
    fn init_card(&mut self) -> Result<(), SdError> {
        info!("eMMC initialization started");
        // For eMMC, we use CMD1 instead of ACMD41
        // HCS=1, voltage window for eMMC as per specs
        let ocr = 0x00; // 2.7V to 3.6V
        let retry = 100;

        // Avoid long-lived mutable borrow
        // Instead of: let card = self.card.as_mut().unwrap();
        
        self.mmc_go_idle()?;

        // Send CMD1 to set OCR and check if card is ready
        self.mmc_send_op_cond(ocr, retry)?;

        // Set RCA - use short-lived borrow
        let card = self.card.as_mut().unwrap();
        card.rca = 1; // Typical RCA value for eMMC is 1

        // Get high_capacity status using short-lived immutable borrow
        let high_capacity = {
            let card = self.card.as_ref().unwrap();
            (card.ocr & OCR_HCS) == OCR_HCS
        };

        // Send CMD2 to get CID
        self.mmc_all_send_cid()?;

        // Send CMD3 to get RCA
        self.mmc_set_relative_addr()?;

        // Send CMD9 to get CSD
        self.mmc_send_csd()?;

        // Process card version and CSD data - use short-lived borrow
        let card = self.card.as_mut().unwrap();
        if card.version == MMC_VERSION_UNKNOWN {
            let csd_version = (card.csd[0] >> 26) & 0xf;
            debug!("eMMC CSD version: {}", csd_version);

            match csd_version {
                0 => card.version = MMC_VERSION_1_2,
                1 => card.version = MMC_VERSION_1_4,
                2 => card.version = MMC_VERSION_2_2,
                3 => card.version = MMC_VERSION_3,
                4 => card.version = MMC_VERSION_4,
                _ => card.version = MMC_VERSION_1_2,
            }
        }

        // Calculate capacity and block lengths - use short-lived borrow
        let (freq, mult, dsr_imp, mut read_bl_len, mut write_bl_len, mut csize, mut cmult) = {
            let card = self.card.as_ref().unwrap();
            let freq = FBASE[(card.csd[0] & 0x7) as usize];
            let mult = MULTIPLIERS[((card.csd[0] >> 3) & 0xf) as usize];
            let dsr_imp = (card.csd[1] >> 12) & 0x1;
            let read_bl_len = (card.csd[1] >> 16) & 0xf;
            let write_bl_len = (card.csd[3] >> 22) & 0xf;

            let (csize, cmult) = if high_capacity {
                (
                    (card.csd[1] & 0x3f) << 16 | (card.csd[2] & 0xffff0000) >> 16,
                    8
                )
            } else {
                (
                    (card.csd[1] & 0x3ff) << 2 | (card.csd[2] & 0xc0000000) >> 30,
                    (card.csd[2] & 0x00038000) >> 15
                )
            };
            
            (freq, mult, dsr_imp, read_bl_len, write_bl_len, csize, cmult)
        };

        let tran_speed = freq * mult as usize;
        let mut capacity_user = (csize + 1) << (cmult + 2);
        capacity_user *= read_bl_len;
        
        if write_bl_len > MMC_MAX_BLOCK_LEN {
            write_bl_len = MMC_MAX_BLOCK_LEN;
        }

        if read_bl_len > MMC_MAX_BLOCK_LEN {
            read_bl_len = MMC_MAX_BLOCK_LEN;
        }

        // Check DSR and send DSR command if needed
        let dsr_needed = {
            let card = self.card.as_ref().unwrap();
            dsr_imp != 0 && 0xffffffff != card.dsr
        };
        
        if dsr_needed {
            let dsr_value = {
                let card = self.card.as_ref().unwrap();
                (card.dsr & 0xffff) << 16
            };
            let cmd4 = EMmcCommand::new(MMC_SET_DSR, dsr_value, MMC_RSP_NONE);
            self.send_command(&cmd4)?;
        }

        // Select card with CMD7
        let rca = {
            let card = self.card.as_ref().unwrap();
            card.rca
        };
        let cmd7 = EMmcCommand::new(MMC_SELECT_CARD, rca << 16, MMC_RSP_R1);
        self.send_command(&cmd7)?;

        // Check eMMC version 4+
        let is_version_4_plus = {
            let card = self.card.as_ref().unwrap();
            card.version >= MMC_VERSION_4
        };
        
        if is_version_4_plus {
            self.mmc_select_hs()?;
            self.mmc_set_clock(MMC_HIGH_52_MAX_DTR);
        }

        info!("eMMC initialization complete");
        Ok(())
    }

    fn mmc_select_hs(&mut self) -> Result<(), SdError> {
        let ret = self.mmc_switch(
            EXT_CSD_CMD_SET_NORMAL,
            EXT_CSD_HS_TIMING,
            EXT_CSD_TIMING_HS
        );

        if ret.is_ok() {
            self.mmc_set_timing(MMC_TIMING_MMC_HS);
        }
    
        ret
    }

    fn mmc_set_bus_width(&mut self, width: u8) {
        /* Set bus width */
        let card = self.card.as_mut().unwrap();
        card.bus_width = width;
        self.sdhci_set_ios();
    }

    fn mmc_set_timing(&mut self, timing: u32) {
        /* Set timing */
        let card = self.card.as_mut().unwrap();
        card.timing = timing;
        self.sdhci_set_ios();
    }

    fn mmc_set_clock(&mut self, clk: u32) {
        /* Set clock */
        let card = self.card.as_mut().unwrap();
        card.clock = clk;
        self.sdhci_set_ios();
    }

    fn mmc_switch(&self, _set: u8, index: u8, value: u8) -> Result<(), SdError> {
        let mut retries = 3;
        let cmd = EMmcCommand::new(MMC_SWITCH, ((MMC_SWITCH_MODE_WRITE_BYTE as u32) << 24) | ((index as u32) << 16) | ((value as u32) << 8), MMC_RSP_R1B);

        loop {
            let ret = self.send_command(&cmd);

            if ret.is_ok() {
                return self.mmc_poll_for_busy(true);
            }

            retries -= 1;
            if retries <= 0 {
                break;
            }
        }

        return Err(SdError::Timeout);
    }

    pub fn mmc_poll_for_busy(&self, send_status: bool) -> Result<(), SdError> {
        let mut busy = true;
        let mut timeout = 1000;
        loop {
            if !send_status {
                todo!("Implement mmc_card_busy");
            } else {
                let cmd = EMmcCommand::new(MMC_SEND_STATUS, self.card.as_ref().unwrap().rca << 16, MMC_RSP_R1);
                self.send_command(&cmd)?;
                let response = self.get_response().as_r1();

                if response & MMC_STATUS_SWITCH_ERROR != 0 {
                    return Err(SdError::BadMessage);
                }
                busy = (response & MMC_STATUS_CURR_STATE) == MMC_STATE_PRG;
                if !busy {
                    break;
                }
            }

            if timeout == 0 && busy {
                return Err(SdError::Timeout);
            }
            timeout -= 1;
        }

        Ok(())
    }

    // Helper function to check if controller supports 8-bit bus
    fn supports_8bit_bus(&self) -> bool {
        // Read controller capabilities register
        // This is a placeholder - actual implementation depends on your EMMC controller
        let caps = self.read_reg(EMMC_CAPABILITIES1);
        (caps & EMMC_CAN_DO_8BIT) != 0
    }
}