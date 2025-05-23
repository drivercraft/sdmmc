extern crate alloc;

mod block;
mod cmd;
mod config;
mod info;
mod regs;
mod rockchip;

pub mod aux;
pub mod clock;
pub mod constant;

use crate::{delay_us, err::*};
use aux::{
    MMC_VERSION_1_2, MMC_VERSION_1_4, MMC_VERSION_2_2, MMC_VERSION_3, MMC_VERSION_4,
    MMC_VERSION_4_1, MMC_VERSION_4_2, MMC_VERSION_4_3, MMC_VERSION_4_5, MMC_VERSION_4_41,
    MMC_VERSION_5_0, MMC_VERSION_5_1, MMC_VERSION_UNKNOWN, generic_fls, lldiv,
};
use block::EMmcCard;
use cmd::*;
use constant::*;
use core::fmt::Display;
#[cfg(feature = "dma")]
use dma_api::{DVec, Direction};
use info::CardType;
use log::{debug, info, trace};

// SD Host Controller structure
#[derive(Debug)]
pub struct EMmcHost {
    base_addr: usize,
    card: Option<EMmcCard>,
    caps: u32,
    clock_base: u32,
    voltages: u32,
    quirks: u32,
    // clock: u32,
    host_caps: u32,
    version: u16,
}

impl Display for EMmcHost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "EMMC Controller {{ base_addr: {:#x}, card: {:?}, caps: {:#x}, clock_base: {} }}",
            self.base_addr, self.card, self.caps, self.clock_base
        )
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
            // clock: 0,
            host_caps: 0,
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

    // 获取 card 的不可变引用
    pub fn card(&self) -> Option<&EMmcCard> {
        self.card.as_ref()
    }

    // 获取 card 的可变引用
    pub fn card_mut(&mut self) -> Option<&mut EMmcCard> {
        self.card.as_mut()
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

        self.host_caps = MMC_MODE_HS | MMC_MODE_HS_52MHZ | MMC_MODE_4BIT;

        if (version & EMMC_SPEC_VER_MASK) >= EMMC_SPEC_300 && (caps1 & EMMC_CAN_DO_8BIT) == 0 {
            self.host_caps &= !MMC_MODE_8BIT;
        }

        // 暂时写死
        self.host_caps |= 0x48;

        // debug!("self.host_caps {:#x}", self.host_caps);

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

        info!(
            "voltage range: {:#x}, {:#x}",
            voltages,
            generic_fls(voltages) - 1
        );

        // Perform full power cycle
        self.sdhci_set_power(generic_fls(voltages) - 1)?;

        // Enable interrupts
        self.write_reg(
            EMMC_NORMAL_INT_STAT_EN,
            EMMC_INT_CMD_MASK | EMMC_INT_DATA_MASK,
        );
        self.write_reg(EMMC_SIGNAL_ENABLE, 0x0);

        // Set initial bus width to 1-bit
        self.mmc_set_bus_width(1);

        // Set initial clock and wait for it to stabilize
        self.mmc_set_clock(400000);

        self.mmc_set_timing(MMC_TIMING_LEGACY);

        // Initialize the card
        self.init_card()?;

        info!("EMMC initialization completed successfully");
        Ok(())
    }

    // Reset the controller
    pub fn reset(&self, mask: u8) -> Result<(), SdError> {
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
        // debug!("EMMC Present State: {:#b}", state);
        (state & EMMC_CARD_INSERTED) != 0 && ((state & EMMC_CARD_STABLE) != 0)
    }

    // Check if card is write protected
    fn is_write_protected(&self) -> bool {
        let state = self.read_reg(EMMC_PRESENT_STATE);
        (state & EMMC_WRITE_PROTECT) != 0
    }

    // Initialize the eMMC card
    fn init_card(&mut self) -> Result<(), SdError> {
        info!("eMMC initialization started");

        // CMD0: Put card into idle state
        self.mmc_go_idle()?;

        // CMD1: Send operation condition (OCR) and wait for card ready
        let ocr = 0x00; // Voltage window: 2.7V to 3.6V
        let retry = 100;
        let ocr = self.mmc_send_op_cond(ocr, retry)?;

        // Set RCA (Relative Card Address)
        self.set_rca(1).unwrap();

        // Determine if card is high capacity (SDHC/SDXC/eMMC)
        let high_capacity = (ocr & OCR_HCS) == OCR_HCS;
        self.set_high_capacity(high_capacity).unwrap();

        // CMD2: Request CID (Card Identification)
        let _cid = self.mmc_all_send_cid()?;

        // CMD3: Set RCA and switch card to "standby" state
        self.mmc_set_relative_addr()?;

        // CMD9: Read CSD (Card-Specific Data) register
        let csd = self.mmc_send_csd()?;

        // Determine card version from CSD if unknown
        let card = self.card.as_mut().unwrap();
        if card.version() == MMC_VERSION_UNKNOWN {
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

        // Extract parameters from CSD for frequency, size, and block lengths
        let (freq, mult, dsr_imp, mut read_bl_len, mut write_bl_len, csize, cmult) = {
            let freq = FBASE[(csd[0] & 0x7) as usize];
            let mult = MULTIPLIERS[((csd[0] >> 3) & 0xf) as usize];
            let dsr_imp = (csd[1] >> 12) & 0x1;
            let read_bl_len = (csd[1] >> 16) & 0xf;
            let write_bl_len = (csd[3] >> 22) & 0xf;
            let (csize, cmult) = if high_capacity {
                ((csd[1] & 0x3f) << 16 | (csd[2] & 0xffff0000) >> 16, 8)
            } else {
                (
                    (csd[1] & 0x3ff) << 2 | (csd[2] & 0xc0000000) >> 30,
                    (csd[2] & 0x00038000) >> 15,
                )
            };
            (freq, mult, dsr_imp, read_bl_len, write_bl_len, csize, cmult)
        };

        card.dsr_imp = dsr_imp;

        // Calculate user capacity
        let _tran_speed = freq * mult as usize;
        let mut capacity_user = (csize as u64 + 1) << (cmult as u64 + 2);
        capacity_user *= read_bl_len as u64;
        card.capacity_user = capacity_user;

        let mut capacity_gp = [0; 4];

        // Clip read/write block lengths to max supported size
        if write_bl_len > MMC_MAX_BLOCK_LEN {
            write_bl_len = MMC_MAX_BLOCK_LEN;
        }
        if read_bl_len > MMC_MAX_BLOCK_LEN {
            read_bl_len = MMC_MAX_BLOCK_LEN;
        }

        card.read_bl_len = read_bl_len;
        card.write_bl_len = write_bl_len;

        // CMD4: Set DSR if required by card
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
            self.send_command(&cmd4, None)?;
        }

        // CMD7: Select the card
        let rca = {
            let card = self.card.as_ref().unwrap();
            card.rca
        };
        let cmd7 = EMmcCommand::new(MMC_SELECT_CARD, rca << 16, MMC_RSP_R1);
        self.send_command(&cmd7, None)?;
        debug!("cmd7: {:#x}", self.get_response().as_r1());

        // Set initial erase group size and partition config
        self.set_erase_grp_size(1).unwrap();
        self.set_part_config(MMCPART_NOAVAILABLE).unwrap();

        // For eMMC 4.0+, configure high-speed, EXT_CSD and partitions
        let is_version_4_plus = {
            let card = self.card.as_ref().unwrap();
            card.version >= MMC_VERSION_4
        };
        if is_version_4_plus {
            self.mmc_select_hs()?; // Switch to high speed
            self.mmc_set_clock(MMC_HIGH_52_MAX_DTR); // Set high-speed clock

            // Allocate buffer for EXT_CSD read
            cfg_if::cfg_if! {
                if #[cfg(feature = "dma")] {
                    let mut ext_csd: DVec<u8> = DVec::zeros(MMC_MAX_BLOCK_LEN as usize, 0x1000, Direction::FromDevice).unwrap();
                } else if #[cfg(feature = "pio")] {
                    let mut ext_csd: [u8; 512] = [0; 512];
                }
            }

            // CMD8: Read EXT_CSD
            self.mmc_send_ext_csd(&mut ext_csd)?;
            let mut ext_csd = ext_csd.to_vec();
            trace!("EXT_CSD: {:?}", ext_csd);

            // Extract capacity and version
            if ext_csd[EXT_CSD_REV as usize] >= 2 {
                let mut capacity: u64 = ext_csd[EXT_CSD_SEC_CNT as usize] as u64
                    | (ext_csd[EXT_CSD_SEC_CNT as usize + 1] as u64) << 8
                    | (ext_csd[EXT_CSD_SEC_CNT as usize + 2] as u64) << 16
                    | (ext_csd[EXT_CSD_SEC_CNT as usize + 3] as u64) << 24;
                capacity *= MMC_MAX_BLOCK_LEN as u64;
                if (capacity >> 20) > 2 * 1024 {
                    self.set_capacity_user(capacity).unwrap();
                }

                let card = self.card.as_mut().unwrap();
                match ext_csd[EXT_CSD_REV as usize] {
                    1 => card.version = MMC_VERSION_4_1,
                    2 => card.version = MMC_VERSION_4_2,
                    3 => card.version = MMC_VERSION_4_3,
                    5 => card.version = MMC_VERSION_4_41,
                    6 => card.version = MMC_VERSION_4_5,
                    7 => card.version = MMC_VERSION_5_0,
                    8 => card.version = MMC_VERSION_5_1,
                    _ => panic!("Unknown EXT_CSD revision"),
                }
            }

            // Parse partition configuration info
            let part_completed = (ext_csd[EXT_CSD_PARTITION_SETTING as usize] as u32
                & EXT_CSD_PARTITION_SETTING_COMPLETED)
                != 0;
            self.set_part_support(ext_csd[EXT_CSD_PARTITIONING_SUPPORT as usize])
                .unwrap();

            if (ext_csd[EXT_CSD_PARTITIONING_SUPPORT as usize] as u32 & PART_SUPPORT != 0)
                || ext_csd[EXT_CSD_BOOT_MULT as usize] != 0
            {
                self.set_part_config(ext_csd[EXT_CSD_PART_CONF as usize])
                    .unwrap();
            }

            // Save enhanced partition attributes
            if part_completed
                && (ext_csd[EXT_CSD_PARTITIONING_SUPPORT as usize] as u32 & ENHNCD_SUPPORT != 0)
            {
                let part_attr = ext_csd[EXT_CSD_PARTITIONS_ATTRIBUTE as usize];
                self.set_part_attr(part_attr).unwrap();
            }

            // Check secure erase support
            if ext_csd[EXT_CSD_SEC_FEATURE_SUPPORT as usize] as u32 & EXT_CSD_SEC_GB_CL_EN != 0 {
                let _mmc_can_trim = 1;
            }

            // Calculate boot and RPMB sizes
            let capacity_boot = (ext_csd[EXT_CSD_BOOT_MULT as usize] as u64) << 17;
            self.set_capacity_boot(capacity_boot).unwrap();
            let capacity_rpmb = (ext_csd[EXT_CSD_RPMB_MULT as usize] as u64) << 17;
            self.set_capacity_rpmb(capacity_rpmb).unwrap();
            debug!("Boot partition size: {:#x}", capacity_boot);
            debug!("RPMB partition size: {:#x}", capacity_rpmb);

            // Calculate general purpose partition sizes
            let mut has_parts = false;
            for i in 0..4 {
                let idx = EXT_CSD_GP_SIZE_MULT as usize + i * 3;
                let mult = ((ext_csd[idx + 2] as u32) << 16)
                    + ((ext_csd[idx + 1] as u32) << 8)
                    + (ext_csd[idx] as u32);
                if mult != 0 {
                    has_parts = true;
                }
                if !part_completed {
                    continue;
                }
                capacity_gp[i] = mult as u64;
                capacity_gp[i] *= ext_csd[EXT_CSD_HC_ERASE_GRP_SIZE as usize] as u64;
                capacity_gp[i] *= ext_csd[EXT_CSD_HC_WP_GRP_SIZE as usize] as u64;
                capacity_gp[i] <<= 19;
                self.set_capacity_gp(capacity_gp).unwrap();
            }
            debug!("GP partition sizes: {:?}", capacity_gp);

            // Calculate enhanced user data size and start
            if part_completed {
                let mut enh_user_size = ((ext_csd[EXT_CSD_ENH_SIZE_MULT as usize + 2] as u64)
                    << 16)
                    + ((ext_csd[EXT_CSD_ENH_SIZE_MULT as usize + 1] as u64) << 8)
                    + (ext_csd[EXT_CSD_ENH_SIZE_MULT as usize] as u64);
                enh_user_size *= ext_csd[EXT_CSD_HC_ERASE_GRP_SIZE as usize] as u64;
                enh_user_size *= ext_csd[EXT_CSD_HC_WP_GRP_SIZE as usize] as u64;
                enh_user_size <<= 19;
                self.set_enh_user_size(enh_user_size).unwrap();

                let mut enh_user_start = ((ext_csd[EXT_CSD_ENH_START_ADDR as usize + 3] as u64)
                    << 24)
                    + ((ext_csd[EXT_CSD_ENH_START_ADDR as usize + 2] as u64) << 16)
                    + ((ext_csd[EXT_CSD_ENH_START_ADDR as usize + 1] as u64) << 8)
                    + (ext_csd[EXT_CSD_ENH_START_ADDR as usize] as u64);
                if high_capacity {
                    enh_user_start <<= 9;
                }
                self.set_enh_user_start(enh_user_start).unwrap();
            }

            // If partitions are configured, enable ERASE_GRP_DEF
            if part_completed {
                has_parts = true;
            }

            if (ext_csd[EXT_CSD_PARTITIONING_SUPPORT as usize] as u32 & PART_SUPPORT != 0)
                && (ext_csd[EXT_CSD_PARTITIONS_ATTRIBUTE as usize] as u32 & PART_ENH_ATTRIB != 0)
            {
                has_parts = true;
            }

            if has_parts {
                let err = self.mmc_switch(EXT_CSD_CMD_SET_NORMAL, EXT_CSD_ERASE_GROUP_DEF, 1, true);
                if err.is_err() {
                    return Err(SdError::CommandError);
                } else {
                    ext_csd[EXT_CSD_ERASE_GROUP_DEF as usize] = 1;
                }
            }

            // Calculate erase group size
            if ext_csd[EXT_CSD_ERASE_GROUP_DEF as usize] & 0x01 != 0 {
                self.set_erase_grp_size(
                    (ext_csd[EXT_CSD_HC_ERASE_GRP_SIZE as usize] as u32) * 1024,
                )
                .unwrap();

                if high_capacity && part_completed {
                    let capacity = (ext_csd[EXT_CSD_SEC_CNT as usize] as u64)
                        | ((ext_csd[EXT_CSD_SEC_CNT as usize + 1] as u64) << 8)
                        | ((ext_csd[EXT_CSD_SEC_CNT as usize + 2] as u64) << 16)
                        | ((ext_csd[EXT_CSD_SEC_CNT as usize + 3] as u64) << 24);
                    self.set_capacity_user(capacity * (MMC_MAX_BLOCK_LEN as u64))
                        .unwrap();
                }
            } else {
                let erase_gsz = (csd[2] & 0x00007c00) >> 10;
                let erase_gmul = (csd[2] & 0x000003e0) >> 5;
                self.set_erase_grp_size((erase_gsz + 1) * (erase_gmul + 1))
                    .unwrap();
            }

            // Set high-capacity write-protect group size
            let hc_wp_grp_size = 1024
                * (ext_csd[EXT_CSD_HC_ERASE_GRP_SIZE as usize] as u64)
                * (ext_csd[EXT_CSD_HC_WP_GRP_SIZE as usize] as u64);
            self.set_hc_wp_grp_size(hc_wp_grp_size).unwrap();

            // Set write reliability and drive strength
            self.set_wr_rel_set(ext_csd[EXT_CSD_WR_REL_SET as usize])
                .unwrap();
            self.set_raw_driver_strength(ext_csd[EXT_CSD_DRIVER_STRENGTH as usize])
                .unwrap();
        }

        // Final initialization steps
        self.mmc_set_capacity(0)?;
        self.mmc_change_freq()?;
        self.set_initialized(true).unwrap();

        Ok(())
    }

    fn mmc_set_capacity(&mut self, part_num: u32) -> Result<(), SdError> {
        // part_num 暂时设置为 0
        match part_num {
            0 => match self.capacity_user() {
                Some(capacity_user) => self.set_capacity(capacity_user).unwrap(),
                None => return Err(SdError::InvalidArgument),
            },
            1 | 2 => match self.capacity_boot() {
                Some(capacity_boot) => self.set_capacity(capacity_boot).unwrap(),
                None => return Err(SdError::InvalidArgument),
            },
            3 => match self.capacity_rpmb() {
                Some(capacity_rpmb) => self.set_capacity(capacity_rpmb).unwrap(),
                None => return Err(SdError::InvalidArgument),
            },
            4..=7 => match self.capacity_gp() {
                Some(capacity_gp) => self
                    .set_capacity(capacity_gp[(part_num - 4) as usize])
                    .unwrap(),
                None => return Err(SdError::InvalidArgument),
            },
            _ => return Err(SdError::InvalidArgument),
        }

        let capacity = self.capacity().unwrap_or(0);
        let read_bl_len = self.read_bl_len().unwrap_or(0);
        let _lba = lldiv(capacity, read_bl_len);

        Ok(())
    }

    pub fn mmc_change_freq(&mut self) -> Result<(), SdError> {
        // Allocate buffer for EXT_CSD depending on whether DMA or PIO is enabled
        cfg_if::cfg_if! {
            if #[cfg(feature = "dma")] {
                let mut ext_csd: DVec<u8> = DVec::zeros(MMC_MAX_BLOCK_LEN as usize, 0x1000, Direction::FromDevice).unwrap();
            } else if #[cfg(feature = "pio")] {
                let mut ext_csd: [u8; 512] = [0; 512];
            }
        }

        // Initialize card capabilities flags
        self.set_card_caps(0).unwrap();

        // Get card version (default to 0 if not available)
        let version = self.version().unwrap_or(0);

        // Only cards version 4.0 and above support high-speed modes
        if version < MMC_VERSION_4 {
            return Ok(());
        }

        // Enable both 4-bit and 8-bit modes on the card
        self.set_card_caps(MMC_MODE_4BIT | MMC_MODE_8BIT).unwrap();

        // Read the EXT_CSD register from the card
        self.mmc_send_ext_csd(&mut ext_csd)?;

        // Determine supported high-speed modes from EXT_CSD
        let avail_type = self.mmc_select_card_type(&ext_csd);

        // Select the appropriate high-speed mode supported by both host and card
        let result = if avail_type & EXT_CSD_CARD_TYPE_HS200 != 0 {
            // HS200 mode
            self.mmc_select_hs200()
        } else if avail_type & EXT_CSD_CARD_TYPE_HS != 0 {
            // Standard high-speed mode
            self.mmc_select_hs()
        } else {
            Err(SdError::InvalidArgument)
        };

        // Apply the result of speed mode selection
        result?;

        // Configure the bus speed according to selected type
        self.mmc_set_bus_speed(avail_type as u32);

        // If HS200 mode was selected, perform tuning procedure
        if self.mmc_card_hs200() {
            let tuning_result = self.mmc_hs200_tuning();

            // Optionally upgrade to HS400 mode if supported and using 8-bit bus
            if avail_type & EXT_CSD_CARD_TYPE_HS400 != 0
                && self.bus_width().unwrap_or(0) == MMC_BUS_WIDTH_8BIT
            {
                // self.mmc_select_hs400()?; // Currently not executed
                self.mmc_set_bus_speed(avail_type as u32);
            }

            tuning_result
        } else if !self.mmc_card_hs400es() {
            // If not in HS400 Enhanced Strobe mode, try to switch bus width
            let width_result = self.mmc_select_bus_width()?;
            let err = if width_result > 0 {
                Ok(())
            } else {
                Err(SdError::BusWidth)
            };

            // If DDR52 mode is supported, implement selection (currently TODO)
            if err.is_ok() && avail_type & EXT_CSD_CARD_TYPE_DDR_52 as u16 != 0 {
                todo!("Implement HS-DDR selection");
            }

            err
        } else {
            // Already in HS400ES mode, no further action needed
            Ok(())
        }
    }

    pub fn mmc_set_bus_speed(&mut self, avail_type: u32) {
        let mut clock = 0;

        if self.mmc_card_hs() {
            clock = if (avail_type & EXT_CSD_CARD_TYPE_52 as u32) != 0 {
                MMC_HIGH_52_MAX_DTR
            } else {
                MMC_HIGH_26_MAX_DTR
            };
        } else if self.mmc_card_hs200() {
            clock = MMC_HS200_MAX_DTR;
        }

        self.mmc_set_clock(clock);
    }

    /// 检查卡是否为HS模式
    fn mmc_card_hs(&self) -> bool {
        let timing = self.timing().unwrap();
        (timing == MMC_TIMING_MMC_HS) || (timing == MMC_TIMING_SD_HS)
    }

    fn mmc_card_hs400es(&self) -> bool {
        let timing = self.timing().unwrap();
        timing == MMC_TIMING_MMC_HS400ES
    }

    /// 检查卡是否为HS200模式
    fn mmc_card_hs200(&self) -> bool {
        let timing = self.timing().unwrap();
        timing == MMC_TIMING_MMC_HS200
    }

    pub fn mmc_select_hs200(&mut self) -> Result<(), SdError> {
        let ret = self.mmc_select_bus_width()?;

        if ret > 0 {
            self.mmc_switch(
                EXT_CSD_CMD_SET_NORMAL,
                EXT_CSD_HS_TIMING,
                EXT_CSD_TIMING_HS200,
                false,
            )?;

            self.mmc_set_timing(MMC_TIMING_MMC_HS200);
        }

        Ok(())
    }

    fn mmc_select_bus_width(&mut self) -> Result<i32, SdError> {
        let ext_csd_bits: [u8; 2] = [EXT_CSD_BUS_WIDTH_8, EXT_CSD_BUS_WIDTH_4];
        let bus_widths: [u8; 2] = [MMC_BUS_WIDTH_8BIT, MMC_BUS_WIDTH_4BIT];

        cfg_if::cfg_if! {
            if #[cfg(feature = "dma")] {
                let mut ext_csd: DVec<u8> = DVec::zeros(MMC_MAX_BLOCK_LEN as usize, 0x1000, Direction::FromDevice).unwrap();
                let mut test_csd = DVec::zeros(MMC_MAX_BLOCK_LEN as usize, 0x1000, Direction::FromDevice)
        .ok_or(SdError::MemoryError)?;
            } else if #[cfg(feature = "pio")] {
                let mut ext_csd: [u8; 512] = [0; 512];
                let mut test_csd: [u8; 512] = [0; 512];
            }
        }

        // 版本检查和主机能力检查
        if self.version().unwrap_or(0) < MMC_VERSION_4
            || (self.host_caps & (MMC_MODE_4BIT | MMC_MODE_8BIT)) == 0
        {
            return Ok(0);
        }

        self.mmc_send_ext_csd(&mut ext_csd)?;

        let mut idx = if (self.host_caps & MMC_MODE_8BIT) != 0 {
            0
        } else {
            1
        };
        while idx < bus_widths.len() {
            let switch_result = self.mmc_switch(
                EXT_CSD_CMD_SET_NORMAL,
                EXT_CSD_BUS_WIDTH,
                ext_csd_bits[idx],
                true,
            );

            if switch_result.is_err() {
                idx += 1;
                continue;
            }

            let bus_width = bus_widths[idx];
            self.mmc_set_bus_width(bus_width);

            // 再次读取EXT_CSD进行验证
            let test_result = self.mmc_send_ext_csd(&mut test_csd);

            if test_result.is_err() {
                idx += 1;
                continue;
            }
            if (ext_csd[EXT_CSD_PARTITIONING_SUPPORT as usize]
                == test_csd[EXT_CSD_PARTITIONING_SUPPORT as usize])
                && (ext_csd[EXT_CSD_HC_WP_GRP_SIZE as usize]
                    == test_csd[EXT_CSD_HC_WP_GRP_SIZE as usize])
                && (ext_csd[EXT_CSD_REV as usize] == test_csd[EXT_CSD_REV as usize])
                && (ext_csd[EXT_CSD_HC_ERASE_GRP_SIZE as usize]
                    == test_csd[EXT_CSD_HC_ERASE_GRP_SIZE as usize])
                && self.compare_sector_count(&ext_csd, &test_csd)
            {
                return Ok(bus_width as i32);
            } else {
                idx += 1;
            }
        }

        Err(SdError::BadMessage)
    }

    #[cfg(feature = "dma")]
    fn compare_sector_count(&self, ext_csd: &DVec<u8>, test_csd: &DVec<u8>) -> bool {
        let sec_cnt_offset = EXT_CSD_SEC_CNT as usize;
        for i in 0..4 {
            if ext_csd[sec_cnt_offset + i] != test_csd[sec_cnt_offset + i] {
                return false;
            }
        }
        true
    }

    #[cfg(feature = "pio")]
    fn compare_sector_count(&self, ext_csd: &[u8], test_csd: &[u8]) -> bool {
        let sec_cnt_offset = EXT_CSD_SEC_CNT as usize;
        for i in 0..4 {
            if ext_csd[sec_cnt_offset + i] != test_csd[sec_cnt_offset + i] {
                return false;
            }
        }
        true
    }

    /// Perform HS200 tuning sequence (also used for HS400 initial tuning)
    fn mmc_hs200_tuning(&mut self) -> Result<(), SdError> {
        let opcode = MMC_SEND_TUNING_BLOCK_HS200;
        let timing = self.timing().unwrap();

        match timing {
            // HS400 tuning must be issued in HS200 mode; reject direct HS400 timing
            MMC_TIMING_MMC_HS400 => {
                return Err(SdError::InvalidArgument);
            }
            // HS200 timing: OK to proceed with tuning here
            MMC_TIMING_MMC_HS200 => {
                // HS400 re-tuning is not expected; leave periodic tuning disabled
            }
            // Any other timing mode is invalid for HS200 tuning
            _ => {
                return Err(SdError::InvalidArgument);
            }
        }

        // Set the EXEC_TUNING bit in Host Control2 to start tuning
        let mut ctrl = self.read_reg16(EMMC_HOST_CTRL2);
        ctrl |= MMC_CTRL_EXEC_TUNING;
        self.write_reg16(EMMC_HOST_CTRL2, ctrl);

        // Invoke the common tuning loop implementation
        self.__emmc_execute_tuning(opcode)
    }

    /// Core tuning loop: send tuning blocks until the controller indicates success or timeout
    fn __emmc_execute_tuning(&mut self, opcode: u8) -> Result<(), SdError> {
        const MAX_TUNING_LOOP: usize = 40;

        for _ in 0..MAX_TUNING_LOOP {
            // Send one tuning block command
            self.emmc_send_tuning(opcode)?;

            // Read back Host Control2 to check tuning status
            let ctrl = self.read_reg16(EMMC_HOST_CTRL2);

            // If the EXEC_TUNING bit has been cleared by hardware...
            if (ctrl & MMC_CTRL_EXEC_TUNING) == 0 {
                // ...and the TUNED_CLK bit is set, tuning succeeded
                if (ctrl & MMC_CTRL_TUNED_CLK) != 0 {
                    return Ok(());
                }
                // EXEC_TUNING cleared but no TUNED_CLK => break and report failure
                break;
            }
        }

        // Exceeded max loops without success: timeout
        Err(SdError::Timeout)
    }

    /// Send a single tuning block read command over the SDHCI interface
    fn emmc_send_tuning(&mut self, opcode: u8) -> Result<(), SdError> {
        // Helper to pack DMA boundary and block size fields
        let make_blksz = |dma: u16, blksz: u16| ((dma & 0x7) << 12) | (blksz & 0x0FFF);

        // Determine current bus width (1/4/8 bits)
        let bus_width = self.bus_width().unwrap();

        // Choose block size: 128 bytes for HS200 on 8-bit bus, else 64 bytes
        let block_size = if opcode == MMC_SEND_TUNING_BLOCK_HS200 && bus_width == MMC_BUS_WIDTH_8BIT
        {
            128
        } else {
            64
        };

        // Program block size and enable DMA boundary
        self.write_reg16(EMMC_BLOCK_SIZE, make_blksz(7, block_size));
        // Set transfer mode to single-block read
        self.write_reg16(EMMC_XFER_MODE, EMMC_TRNS_READ);

        // Build and send the tuning command
        let cmd = EMmcCommand::new(opcode, 0, MMC_RSP_R1);
        self.send_command(&cmd, None)?;

        Ok(())
    }

    #[allow(unused)]
    fn mmc_card_ddr(&self) -> bool {
        let timing = self.timing().unwrap();
        (timing == MMC_TIMING_UHS_DDR50)
            || (timing == MMC_TIMING_MMC_DDR52)
            || (timing == MMC_TIMING_MMC_HS400)
            || (timing == MMC_TIMING_MMC_HS400ES)
    }

    #[cfg(feature = "dma")]
    pub fn mmc_select_card_type(&self, ext_csd: &DVec<u8>) -> u16 {
        let card_type = ext_csd[EXT_CSD_CARD_TYPE as usize] as u16;
        let host_caps = self.host_caps;
        let mut avail_type = 0;

        if (host_caps & MMC_MODE_HS != 0) && (card_type & EXT_CSD_CARD_TYPE_26 != 0) {
            avail_type |= EXT_CSD_CARD_TYPE_26;
        }

        if (host_caps & MMC_MODE_HS != 0) && (card_type & EXT_CSD_CARD_TYPE_52 != 0) {
            avail_type |= EXT_CSD_CARD_TYPE_52;
        }

        if (host_caps & MMC_MODE_DDR_52MHZ != 0)
            && (card_type & EXT_CSD_CARD_TYPE_DDR_1_8V as u16 != 0)
        {
            avail_type |= EXT_CSD_CARD_TYPE_DDR_1_8V as u16;
        }

        if (host_caps & MMC_MODE_HS200 != 0) && (card_type & EXT_CSD_CARD_TYPE_HS200_1_8V != 0) {
            avail_type |= EXT_CSD_CARD_TYPE_HS200_1_8V;
        }

        if (host_caps & MMC_MODE_HS400 != 0)
            && (host_caps & MMC_MODE_8BIT != 0)
            && (card_type & EXT_CSD_CARD_TYPE_HS400_1_8V != 0)
        {
            avail_type |= EXT_CSD_CARD_TYPE_HS200_1_8V | EXT_CSD_CARD_TYPE_HS400_1_8V;
        }

        if (host_caps & MMC_MODE_HS400ES != 0)
            && (host_caps & MMC_MODE_8BIT != 0)
            && (ext_csd[EXT_CSD_STROBE_SUPPORT as usize] != 0)
            && (avail_type & EXT_CSD_CARD_TYPE_HS400_1_8V != 0)
        {
            avail_type |= EXT_CSD_CARD_TYPE_HS200_1_8V
                | EXT_CSD_CARD_TYPE_HS400_1_8V
                | EXT_CSD_CARD_TYPE_HS400ES;
        }

        avail_type
    }

    #[cfg(feature = "pio")]
    pub fn mmc_select_card_type(&self, ext_csd: &[u8]) -> u16 {
        let card_type = ext_csd[EXT_CSD_CARD_TYPE as usize] as u16;
        let host_caps = self.host_caps;
        let mut avail_type = 0;

        if (host_caps & MMC_MODE_HS != 0) && (card_type & EXT_CSD_CARD_TYPE_26 != 0) {
            avail_type |= EXT_CSD_CARD_TYPE_26;
        }

        if (host_caps & MMC_MODE_HS != 0) && (card_type & EXT_CSD_CARD_TYPE_52 != 0) {
            avail_type |= EXT_CSD_CARD_TYPE_52;
        }

        if (host_caps & MMC_MODE_DDR_52MHZ != 0)
            && (card_type & EXT_CSD_CARD_TYPE_DDR_1_8V as u16 != 0)
        {
            avail_type |= EXT_CSD_CARD_TYPE_DDR_1_8V as u16;
        }

        if (host_caps & MMC_MODE_HS200 != 0) && (card_type & EXT_CSD_CARD_TYPE_HS200_1_8V != 0) {
            avail_type |= EXT_CSD_CARD_TYPE_HS200_1_8V;
        }

        if (host_caps & MMC_MODE_HS400 != 0)
            && (host_caps & MMC_MODE_8BIT != 0)
            && (card_type & EXT_CSD_CARD_TYPE_HS400_1_8V != 0)
        {
            avail_type |= EXT_CSD_CARD_TYPE_HS200_1_8V | EXT_CSD_CARD_TYPE_HS400_1_8V;
        }

        if (host_caps & MMC_MODE_HS400ES != 0)
            && (host_caps & MMC_MODE_8BIT != 0)
            && (ext_csd[EXT_CSD_STROBE_SUPPORT as usize] != 0)
            && (avail_type & EXT_CSD_CARD_TYPE_HS400_1_8V != 0)
        {
            avail_type |= EXT_CSD_CARD_TYPE_HS200_1_8V
                | EXT_CSD_CARD_TYPE_HS400_1_8V
                | EXT_CSD_CARD_TYPE_HS400ES;
        }

        avail_type
    }

    fn mmc_select_hs(&mut self) -> Result<(), SdError> {
        let ret = self.mmc_switch(
            EXT_CSD_CMD_SET_NORMAL,
            EXT_CSD_HS_TIMING,
            EXT_CSD_TIMING_HS,
            true,
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
        debug!("Bus width set to {}", width);
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

    fn mmc_switch(
        &self,
        _set: u8,
        index: u32,
        value: u8,
        send_status: bool,
    ) -> Result<(), SdError> {
        let mut retries = 3;
        let cmd = EMmcCommand::new(
            MMC_SWITCH,
            (MMC_SWITCH_MODE_WRITE_BYTE << 24)
                | (index << 16)
                | ((value as u32) << 8),
            MMC_RSP_R1B,
        );

        loop {
            let ret = self.send_command(&cmd, None);

            if ret.is_ok() {
                debug!("cmd6 {:#x}", self.get_response().as_r1());
                return self.mmc_poll_for_busy(send_status);
            }

            retries -= 1;
            if retries <= 0 {
                debug!("Switch command failed after 3 retries");
                break;
            }
        }

        Err(SdError::Timeout)
    }
}
