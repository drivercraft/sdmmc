use log::{debug, info};
use crate::{delay_us, err::SdError};
use super::{clock::RK3568ClkPriv, constant::*, EMmcHost};

impl EMmcHost {
    // Rockchip EMMC设置时钟函数
    pub fn rockchip_emmc_set_clock(&mut self, freq: u32, clk: &mut RK3568ClkPriv) -> Result<(), SdError> {
        // wait for command and data inhibit to be cleared
        let mut timeout = 20;
        while (self.read_reg(EMMC_PRESENT_STATE) & (EMMC_CMD_INHIBIT | EMMC_DATA_INHIBIT)) != 0 {
            if timeout == 0 {
                debug!("Timeout waiting for cmd & data inhibit");
                return Err(SdError::Timeout);
            }
            timeout -= 1;
            delay_us(1000);
        }

        // first disable the clock
        self.write_reg16(EMMC_CLOCK_CONTROL, 0x0000);

        if freq == 0 {
            return Ok(());
        }

        // 计算输入时钟
        let input_clk = clk.emmc_set_clk(freq as u64).unwrap() as u32;
        info!("input_clk: {}", input_clk);

        // 根据SDHCI规范版本计算分频器
        let mut div = 0;
        let mut clk = 0u16;
        let sdhci_version = self.read_reg16(EMMC_HOST_CNTRL_VER);
        
        if (sdhci_version & 0xFF) >= EMMC_SPEC_300 {
            let caps2 = self.read_reg(EMMC_CAPABILITIES2);
            let clk_mul = (caps2 & EMMC_CLOCK_MUL_MASK) >> EMMC_CLOCK_MUL_SHIFT;

            info!("EMMC Clock Mul: {}", clk_mul);

            // Check if the Host Controller supports Programmable Clock Mode.
            if clk_mul != 0 {
                for i in 1..=1024 {
                    if (input_clk / i) <= freq {
                        div = i;
                        break;
                    }
                }
                // Set Programmable Clock Mode in the Clock Control register.
                clk = EMMC_PROG_CLOCK_MODE;
                div -= 1;
            } else {
                // Version 3.00 divisors must be a multiple of 2.
                if input_clk <= freq {
                    div = 1;
                } else {
                    for i in (2 ..= 2046).step_by(2) {
                        if (input_clk / i ) <= freq {
                            div = i;
                            break;
                        }
                    }
                }
                div >>= 1;
            }
        } else {
            // Version 2.00 divisors must be a power of 2.
            let mut i = 1;
            while i < 256 && (input_clk / i) > freq {
                i *= 2;
            }
            div = i >> 1;
        }

        info!("EMMC Clock Divisor: {:x}", div);

        clk |= ((div as u16) & 0xFF) << EMMC_DIVIDER_SHIFT;
        clk |= (((div as u16) & 0x300) >> 8) << EMMC_DIVIDER_HI_SHIFT;

        self.write_reg16(EMMC_CLOCK_CONTROL, clk);
        self.enable_card_clock(clk)?;

        Ok(())
    }

    pub fn enable_card_clock(&mut self, mut clk: u16) -> Result<(), SdError> {
        clk |= EMMC_CLOCK_INT_EN;
        clk &= !EMMC_CLOCK_INT_STABLE;
        self.write_reg16(EMMC_CLOCK_CONTROL, clk);

        let mut timeout = 20;
        while (self.read_reg16(EMMC_CLOCK_CONTROL) & EMMC_CLOCK_INT_STABLE) == 0 {
            timeout -= 1;
            delay_us(1000);
            if timeout == 0 {
                info!("Internal clock never stabilised.");
                return Err(SdError::Timeout);
            }
        }    

        self.write_reg16(EMMC_CLOCK_CONTROL, clk | EMMC_CLOCK_CARD_EN);

        debug!("EMMC Clock Control: {:#x}", self.read_reg16(EMMC_CLOCK_CONTROL));

        Ok(())
    }

    pub fn is_clock_stable(&self) -> bool {
        let clock_ctrl = self.read_reg16(EMMC_CLOCK_CONTROL);
        return (clock_ctrl & EMMC_CLOCK_INT_STABLE) != 0;
    }

    pub fn sdhci_set_power(&mut self, power: u32) -> Result<(), SdError> {
        let mut pwr: u8= 0;
    
        if power != 0xFFFF {
            match 1 << power {
                MMC_VDD_165_195 => {
                    pwr = EMMC_POWER_180;
                },
                MMC_VDD_29_30 | MMC_VDD_30_31 => {
                    pwr = EMMC_POWER_300;
                },
                MMC_VDD_32_33 | MMC_VDD_33_34 => {
                    pwr = EMMC_POWER_330;
                },
                _ => {}
            }
        }
    
        if pwr == 0 {
            self.write_reg8(EMMC_POWER_CTRL, 0);
            return Ok(());
        }
    
        pwr |= EMMC_POWER_ON;
        self.write_reg8(EMMC_POWER_CTRL, pwr);

        info!("EMMC Power Control: {:#x}", self.read_reg8(EMMC_POWER_CTRL));

        // Small delay for power to stabilize
        delay_us(10000);

        Ok(())
    }

    // DWCMSHC SDHCI EMMC设置时钟
    pub fn dwcmshc_sdhci_emmc_set_clock(&mut self, freq: u32, clk: &mut RK3568ClkPriv) -> Result<(), SdError> {
        self.rockchip_emmc_set_clock(freq, clk)?;
        // Disable output clock while config DLL
        self.write_reg16(EMMC_CLOCK_CONTROL, 0);
        
        // DLL配置基于频率
        if freq >= 100_000_000 { // 100 MHz
            // Enable DLL
        } else {
            // Disable dll
            self.write_reg(DWCMSHC_EMMC_DLL_CTRL, 0);
            
            // Disable cmd conflict check
            let mut extra = self.read_reg(DWCMSHC_HOST_CTRL3);
            debug!("extra: {:#b}", extra);
            extra &= !0x1;
            self.write_reg(DWCMSHC_HOST_CTRL3, extra);
            
            // reset the clock phase when the frequency is lower than 100MHz
            self.write_reg(DWCMSHC_EMMC_DLL_CTRL, DWCMSHC_EMMC_DLL_BYPASS | DWCMSHC_EMMC_DLL_START);
            self.write_reg(DWCMSHC_EMMC_DLL_RXCLK, DLL_RXCLK_ORI_GATE);
            self.write_reg(DWCMSHC_EMMC_DLL_TXCLK, 0);
            self.write_reg(DECMSHC_EMMC_DLL_CMDOUT, 0);
            
            // Before switching to hs400es mode, the driver 
            // will enable enhanced strobe first. PHY needs to 
            // configure the parameters of enhanced strobe first.
            let ddr50_strbin_delay_num = 16;
            let extra = DWCMSHC_EMMC_DLL_DLYENA |
                        DLL_STRBIN_DELAY_NUM_SEL |
                        (ddr50_strbin_delay_num << DLL_STRBIN_DELAY_NUM_OFFSET);
            info!("extra: {:#b}", extra);
            self.write_reg(DWCMSHC_EMMC_DLL_STRBIN, extra);
        }

        // Enable card clock
        self.enable_card_clock(0)?;

        info!("Clock {:#x}", self.read_reg16(EMMC_CLOCK_CONTROL));

        Ok(())
    }
    
}

const EMMC_PROG_CLOCK_MODE: u16 = 0x0020;
const EMMC_DIVIDER_SHIFT: u16 = 8;
const EMMC_DIVIDER_HI_SHIFT: u16 = 6;