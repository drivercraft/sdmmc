use core::sync::atomic::Ordering;

use crate::err::SdError;

use super::{cmd::EMmcCommand, constant::*, EMmcHost};

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
}