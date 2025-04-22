use super::constant::RK_RXCLK_NO_INVERTER;

#[derive(Debug, Clone, Copy)]
pub struct EMmcChipConfig {
    pub flags: u32,
    pub hs200_tx_tap: u8,
    pub hs400_tx_tap: u8,
    pub hs400_cmd_tap: u8,
    pub hs400_strbin_tap: u8,
    pub ddr50_strbin_delay_num: u8,
}

impl EMmcChipConfig {
    pub fn rk3568_config() -> Self {
        Self {
            flags: RK_RXCLK_NO_INVERTER,
            hs200_tx_tap: 16,
            hs400_tx_tap: 8,
            hs400_cmd_tap: 8,
            hs400_strbin_tap: 3,
            ddr50_strbin_delay_num: 16,
        }
    }
}