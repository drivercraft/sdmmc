use core::sync::atomic::Ordering;
use crate::err::SdError;
use super::{block::EMmcCard, cmd::EMmcCommand, constant::*, EMmcHost};

// Card information structure
#[derive(Debug)]
pub struct CardInfo {
    pub card_type: CardType,
    pub manufacturer_id: u8,
    pub application_id: u16,
    pub serial_number: u32,
    pub manufacturing_month: u8,
    pub manufacturing_year: u16,
    pub capacity_bytes: u64,
    pub block_size: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum CardType {
    Unknown,
    Mmc,
    SdV1,
    SdV2,
    SdHc,
    MmcHc,
}

impl EMmcHost {
    // Get card status
    pub fn get_status(&self) -> Result<u32, SdError> {
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Send SEND_STATUS command
        let cmd = EMmcCommand::new(MMC_SEND_STATUS, card.rca, MMC_RSP_R1);
        self.send_command(&cmd, None)?;
        let response = self.get_response();

        Ok(response.as_r1())
    }

    // Get card info
    pub fn get_card_info(&self) -> Result<CardInfo, SdError> {
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        // Extract information from CID
        let cid = card.cid;
        
        // SD card CID format
        let manufacturer_id = (cid[0] >> 24) as u8;
        let application_id = ((cid[0] >> 8) & 0xFFFF) as u16;
        let serial_number = ((cid[0] & 0xFF) << 24) | ((cid[1] >> 8) & 0xFFFFFF);

        // Extract manufacturing date
        let manufacturing_year = (((cid[1] & 0xF) << 4) | ((cid[2] >> 28) & 0xF)) as u16 + 2000;
        let manufacturing_month = ((cid[2] >> 24) & 0xF) as u8;

        let card_info = CardInfo {
            card_type: card.card_type,
            manufacturer_id,
            application_id,
            serial_number,
            manufacturing_month,
            manufacturing_year,
            capacity_bytes: card.capacity_blocks * 512,
            block_size: 512,
        };

        Ok(card_info)
    }

    // Get card capacity in bytes
    pub fn get_capacity(&self) -> Result<u64, SdError> {
        // Check if card is initialized
        let card = match &self.card {
            Some(card) => card,
            None => return Err(SdError::NoCard),
        };

        if !card.initialized.load(Ordering::SeqCst) {
            return Err(SdError::UnsupportedCard);
        }

        Ok(card.capacity_blocks * 512)
    }

    pub fn get_block_num(&self) -> u64 {
        if let Some(card) = &self.card {
            card.capacity_user / 512
        } else {
            0
        }
    }

    pub fn get_block_size(&self) -> usize {
        if let Some(card) = &self.card {
            1 << (card.read_bl_len as usize)
        } else {
            0
        }
    }
}

// EMmcCard 代理访问宏 - 为 Host 实现直接访问 Card 参数的方法
macro_rules! impl_emmc_card_proxy {
    ($($field:ident: $type:ty),*) => {
        impl EMmcHost {
            $(
                // 代理 Getter 方法
                pub fn $field(&self) -> Option<$type> {
                    self.card.as_ref().map(|card| card.$field)
                }

                // 代理 Setter 方法
                paste::paste! {
                    pub fn [<set_ $field>](&mut self, value: $type) -> Result<(), &'static str> {
                        if let Some(card) = self.card.as_mut() {
                            card.$field = value;
                            Ok(())
                        } else {
                            Err("No card present")
                        }
                    }
                }
            )*
        }

        impl EMmcCard {
            $(
                // Getter 方法
                pub fn $field(&self) -> $type {
                    self.$field
                }

                // Setter 方法名：set_field_name
                paste::paste! {
                    pub fn [<set_ $field>](&mut self, value: $type) {
                        self.$field = value;
                    }
                }
            )*
        }
    };
}

impl_emmc_card_proxy!(
    card_type: CardType,
    rca: u32,
    ocr: u32,
    state: u32,
    block_size: u32,
    capacity_blocks: u64,
    high_capacity: bool,
    version: u32,
    dsr: u32,
    timing: u32,
    bus_width: u8,
    part_support: u8,
    part_attr: u8,
    wr_rel_set: u8,
    part_config: u8,
    dsr_imp: u32,
    card_caps: u32,
    read_bl_len: u32,
    write_bl_len: u32,
    erase_grp_size: u32,
    hc_wp_grp_size: u64,
    capacity: u64,
    capacity_user: u64,
    capacity_boot: u64,
    capacity_rpmb: u64,
    ext_csd_rev: u8,
    ext_csd_sectors: u64,
    hs_max_dtr: u32,
    raw_driver_strength: u8
);

impl EMmcHost {
    pub fn set_card(&mut self, card: Option<EMmcCard>) {
        self.card = card;
    }
    
    // CID 数组代理方法
    pub fn cid(&self) -> Option<[u32; 4]> {
        self.card.as_ref().map(|card| card.cid)
    }
    
    pub fn set_cid(&mut self, value: [u32; 4]) -> Result<(), &'static str> {
        if let Some(card) = self.card.as_mut() {
            card.cid = value;
            Ok(())
        } else {
            Err("No card present")
        }
    }
    
    // CSD 数组代理方法
    pub fn csd(&self) -> Option<[u32; 4]> {
        self.card.as_ref().map(|card| card.csd)
    }
    
    pub fn set_csd(&mut self, value: [u32; 4]) -> Result<(), &'static str> {
        if let Some(card) = self.card.as_mut() {
            card.csd = value;
            Ok(())
        } else {
            Err("No card present")
        }
    }
    
    // capacity_gp 数组代理方法
    pub fn capacity_gp(&self) -> Option<[u64; 4]> {
        self.card.as_ref().map(|card| card.capacity_gp)
    }
    
    pub fn set_capacity_gp(&mut self, value: [u64; 4]) -> Result<(), &'static str> {
        if let Some(card) = self.card.as_mut() {
            card.capacity_gp = value;
            Ok(())
        } else {
            Err("No card present")
        }
    }
    
    // AtomicBool 代理方法
    pub fn initialized(&self) -> Option<bool> {
        self.card.as_ref().map(|card| card.initialized.load(Ordering::Relaxed))
    }
    
    pub fn set_initialized(&mut self, value: bool) -> Result<(), &'static str> {
        if let Some(card) = self.card.as_mut() {
            card.initialized.store(value, Ordering::Relaxed);
            Ok(())
        } else {
            Err("No card present")
        }
    }
    
    // enh_user 相关字段代理方法
    pub fn enh_user_size(&self) -> Option<u64> {
        self.card.as_ref().map(|card| card.enh_user_size)
    }
    
    pub fn set_enh_user_size(&mut self, value: u64) -> Result<(), &'static str> {
        if let Some(card) = self.card.as_mut() {
            card.enh_user_size = value;
            Ok(())
        } else {
            Err("No card present")
        }
    }
    
    pub fn enh_user_start(&self) -> Option<u64> {
        self.card.as_ref().map(|card| card.enh_user_start)
    }
    
    pub fn set_enh_user_start(&mut self, value: u64) -> Result<(), &'static str> {
        if let Some(card) = self.card.as_mut() {
            card.enh_user_start = value;
            Ok(())
        } else {
            Err("No card present")
        }
    }
}