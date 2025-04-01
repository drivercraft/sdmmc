use log::{debug, info};

use crate::err::SdError;

use super::{constant::*, EMmcHost};

impl EMmcHost {
    // Rockchip EMMC设置时钟函数
    fn rockship_emmc_set_clock(&mut self, freq: u32) -> Result<(), SdError> {
        // wait for command and data inhibit to be cleared
        let mut timeout = 200000;
        while (self.read_reg(EMMC_PRESENT_STATE) & (EMMC_CMD_INHIBIT | EMMC_DATA_INHIBIT)) != 0 {
            timeout -= 1;
            if timeout == 0 {
                debug!("Timeout waiting for cmd & data inhibit");
                return Err(SdError::Timeout);
            }
        }

        // first disable the clock
        self.write_reg16(EMMC_CLOCK_CONTROL, 0x0000);

        // 如果请求的频率为0，则直接返回
        if freq == 0 {
            return Ok(());
        }

        // 计算输入时钟
        let input_clk = self.clock_base;
        info!("input_clk: {}", input_clk);

        // 根据SDHCI规范版本计算分频器
        let mut div = 0;
        let sdhci_version = self.read_reg16(EMMC_HOST_CNTRL_VER);
        
        // SDHCI规范3.0及以上
        if (sdhci_version & 0xFF) >= 0x30 {
            // 检查是否支持可编程时钟模式
            let clk_mul = (self.caps >> 16) & 0xFF;
            
            if clk_mul != 0 {
                // 可编程时钟模式
                for i in 1..=1024 {
                    if (input_clk / i) <= freq {
                        div = i;
                        break;
                    }
                }
                
                div -= 1;
            } else {
                // 版本3.00的分频器必须是2的倍数
                if input_clk <= freq {
                    div = 0;
                } else {
                    for i in (2..=2046).step_by(2) {
                        if (input_clk / i) <= freq {
                            div = i;
                            break;
                        }
                    }
                }
                div >>= 1; // 右移1位相当于除以2
            }
        } else {
            // 版本2.00的分频器必须是2的幂
            let mut i = 1;
            while i < 256 && (input_clk / i) > freq {
                i *= 2;
            }
            div = i >> 1; // 右移1位相当于除以2
        }

        // 构建时钟控制寄存器值
        let mut clk = 0u16;
        
        // 如果支持可编程时钟模式且使用它
        if (sdhci_version & 0xFF) >= 0x30 && (self.caps >> 16) & 0xFF != 0 {
            clk = EMMC_PROG_CLOCK_MODE;
        }
        
        // 设置分频器
        clk |= ((div as u16) & 0xFF) << EMMC_DIVIDER_SHIFT;
        clk |= (((div as u16) & 0x300) >> 8) << EMMC_DIVIDER_HI_SHIFT;
        clk |= EMMC_CLOCK_INT_EN;
        
        // 写入时钟控制寄存器
        self.write_reg16(EMMC_CLOCK_CONTROL, clk);
        
        // 等待内部时钟稳定
        timeout = 100000;
        while (self.read_reg16(EMMC_CLOCK_CONTROL) & EMMC_CLOCK_INT_STABLE) == 0 {
            timeout -= 1;
            if timeout == 0 {
                return Err(SdError::Timeout);
            }
        }
        
        // 使能卡时钟
        self.enable_card_clock()?;
        
        Ok(())
    }

    // DWCMSHC SDHCI EMMC设置时钟
    fn dwcmshc_sdhci_emmc_set_clock(&mut self, freq: u32) -> Result<(), SdError> {
        self.rockship_emmc_set_clock(freq)?;
        
        // 禁用输出时钟，以便配置DLL
        self.write_reg16(EMMC_CLOCK_CONTROL, 0);
        
        // // DLL配置基于频率
        // if freq >= 100_000_000 { // 100 MHz
        //     // reset DLL
        //     self.write_reg(DWCMSHC_EMMC_DLL_CTRL, DWCMSHC_EMMC_DLL_CTRL_RESET);
        //     // 小延迟
        //     for _ in 0..1000 {
        //         let _ = self.read_reg8(EMMC_POWER_CTRL);
        //     }
        //     self.write_reg(DWCMSHC_EMMC_DLL_CTRL, 0);
            
        //     // 配置 EMMC_ATCTRL 寄存器
        //     let extra = (0x1 << 16) | (0x2 << 17) | (0x3 << 19); // tune clock stop en, pre-change delay, post-change delay
        //     self.write_reg(DWCMSHC_EMMC_ATCTRL, extra);
            
        //     // 初始化DLL设置
        //     let extra = (DWCMSHC_EMMC_DLL_START_DEFAULT << DWCMSHC_EMMC_DLL_START_POINT) |
        //                 (DWCMSHC_EMMC_DLL_INC_VALUE << DWCMSHC_EMMC_DLL_INC) |
        //                 DWCMSHC_EMMC_DLL_START;
        //     self.write_reg(DWCMSHC_EMMC_DLL_CTRL, extra);
            
        //     // 等待DLL锁定
        //     let mut timeout = 500;
        //     let mut dll_lock_value = 0;
        //     while timeout > 0 {
        //         let status = self.read_reg(DWCMSHC_EMMC_DLL_STATUS0);
        //         if dll_lock_wo_tmout(status) {
        //             dll_lock_value = ((status & 0xFF) * 2) & 0xFF;
        //             break;
        //         }
        //         timeout -= 1;
        //         if timeout == 0 {
        //             return Err(SdError::Timeout);
        //         }
                
        //         // 小延迟
        //         for _ in 0..1000 {
        //             let _ = self.read_reg8(EMMC_POWER_CTRL);
        //         }
        //     }
            
        //     // 配置RX时钟
        //     let mut extra = DWCMSHC_EMMC_DLL_DLYENA | DLL_RXCLK_ORI_GATE;
            
        //     // 假设设备数据配置类似于C代码中的RK_RXCLK_NO_INVERTER
        //     let use_rxclk_no_inverter = true; // 这应该从设备配置中获取
        //     if use_rxclk_no_inverter {
        //         extra |= DLL_RXCLK_NO_INVERTER;
        //     }
            
        //     // 假设设备数据配置类似于C代码中的RK_TAP_VALUE_SEL
        //     let use_tap_value_sel = true; // 这应该从设备配置中获取
        //     if use_tap_value_sel {
        //         extra |= DLL_TAP_VALUE_SEL | (dll_lock_value << DLL_TAP_VALUE_OFFSET);
        //     }
            
        //     self.write_reg(DWCMSHC_EMMC_DLL_RXCLK, extra);
            
        //     // 设置TX时钟
        //     // 假设设备数据配置中有hs200_tx_tap和hs400_tx_tap
        //     let hs200_tx_tap = 16; // 这应该从设备配置中获取
        //     let hs400_tx_tap = 8;  // 这应该从设备配置中获取
        //     let mut txclk_tapnum = hs200_tx_tap;
            
        //     // 获取当前MMC时序模式
        //     let timing = MMC_TIMING_MMC_HS200; // 这应该从当前MMC时序状态中获取
            
        //     // 假设设备数据配置类似于C代码中的RK_DLL_CMD_OUT
        //     let use_dll_cmd_out = true; // 这应该从设备配置中获取
            
        //     if use_dll_cmd_out && (timing == MMC_TIMING_MMC_HS400 || timing == MMC_TIMING_MMC_HS400ES) {
        //         txclk_tapnum = hs400_tx_tap;
                
        //         // 配置命令输出DLL
        //         let hs400_cmd_tap = 8; // 这应该从设备配置中获取
        //         let mut extra = DLL_CMDOUT_SRC_CLK_NEG |
        //                         DLL_CMDOUT_BOTH_CLK_EDGE |
        //                         DWCMSHC_EMMC_DLL_DLYENA |
        //                         hs400_cmd_tap |
        //                         DLL_CMDOUT_TAPNUM_FROM_SW;
                                
        //         if use_tap_value_sel {
        //             extra |= DLL_TAP_VALUE_SEL | (dll_lock_value << DLL_TAP_VALUE_OFFSET);
        //         }
                
        //         self.write_reg(DECMSHC_EMMC_DLL_CMDOUT, extra);
        //     }
            
        //     // 配置TX时钟DLL
        //     let mut extra = DWCMSHC_EMMC_DLL_DLYENA |
        //                     DLL_TXCLK_TAPNUM_FROM_SW |
        //                     DLL_TXCLK_NO_INVERTER |
        //                     txclk_tapnum;
                            
        //     if use_tap_value_sel {
        //         extra |= DLL_TAP_VALUE_SEL | (dll_lock_value << DLL_TAP_VALUE_OFFSET);
        //     }
            
        //     self.write_reg(DWCMSHC_EMMC_DLL_TXCLK, extra);
            
        //     // 配置STRBIN DLL
        //     let hs400_strbin_tap = 3; // 这应该从设备配置中获取
        //     let mut extra = DWCMSHC_EMMC_DLL_DLYENA |
        //                     hs400_strbin_tap |
        //                     DLL_STRBIN_TAPNUM_FROM_SW;
                            
        //     if use_tap_value_sel {
        //         extra |= DLL_TAP_VALUE_SEL | (dll_lock_value << DLL_TAP_VALUE_OFFSET);
        //     }
            
        //     self.write_reg(DWCMSHC_EMMC_DLL_STRBIN, extra);
            
        // } else {
            // disable dll
            self.write_reg(DWCMSHC_EMMC_DLL_CTRL, 0);
            
            // Disable cmd conflict check
            let mut extra = self.read_reg(DWCMSHC_HOST_CTRL3);
            debug!("extra: {:#b}", extra);
            extra &= !0x1; // ~BIT(0)
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
            self.write_reg(DWCMSHC_EMMC_DLL_STRBIN, extra);
        // }
        
        // enable output clock
        self.enable_card_clock()?;
        
        Ok(())
    }
    
    // 启用卡时钟辅助函数
    pub fn enable_card_clock(&mut self) -> Result<(), SdError> {
        let clk = self.read_reg16(EMMC_CLOCK_CONTROL);
        self.write_reg16(EMMC_CLOCK_CONTROL, clk | EMMC_CLOCK_CARD_EN);
        Ok(())
    }
    
    // DWCMSHC SDHCI设置增强选通
    fn dwcmshc_sdhci_set_enhanced_strobe(&mut self) -> Result<(), SdError> {
        // 获取当前MMC时序模式
        let timing = MMC_TIMING_MMC_HS400; // 这应该从当前MMC时序状态中获取
        
        let mut vendor = self.read_reg(DWCMSHC_EMMC_CONTROL);
        
        if timing == MMC_TIMING_MMC_HS400ES {
            vendor |= DWCMSHC_ENHANCED_STROBE;
        } else {
            vendor &= !DWCMSHC_ENHANCED_STROBE;
        }
        
        self.write_reg(DWCMSHC_EMMC_CONTROL, vendor);
        
        // 一些eMMC设备在发送命令前需要延迟
        for _ in 0..100000 {
            let _ = self.read_reg8(EMMC_POWER_CTRL);
        }
        
        Ok(())
    }
    
    // DWCMSHC SDHCI设置IO后处理
    fn dwcmshc_sdhci_set_ios_post(&mut self) {
        // 获取当前MMC时序模式
        let timing = MMC_TIMING_MMC_HS400; // 这应该从当前MMC时序状态中获取
        
        if timing == MMC_TIMING_MMC_HS400 || timing == MMC_TIMING_MMC_HS400ES {
            // 设置主机控制2寄存器
            let mut ctrl = self.read_reg16(EMMC_HOST_CTRL2);
            ctrl &= !SDHCI_CTRL_UHS_MASK;
            ctrl |= DWCMSHC_CTRL_HS400;
            self.write_reg16(EMMC_HOST_CTRL2, ctrl);
            
            // 设置CARD_IS_EMMC位以启用HS400的数据选通
            let mut ctrl = self.read_reg16(DWCMSHC_EMMC_CONTROL);
            ctrl |= DWCMSHC_CARD_IS_EMMC as u16;
            self.write_reg16(DWCMSHC_EMMC_CONTROL, ctrl);
        }
    }
    
    // Rockchip SDHCI设置时钟
    pub fn rockchip_sdhci_set_clock(&mut self, freq: u32) -> Result<(), SdError> {
        self.dwcmshc_sdhci_emmc_set_clock(freq)
    }
    
    // Rockchip SDHCI设置IO后处理
    pub fn rockchip_sdhci_set_ios_post(&mut self) {
        // 根据设备类型选择适当的IO后处理函数
        // 这里假设我们使用DWCMSHC控制器
        self.dwcmshc_sdhci_set_ios_post();
    }
    
    // Rockchip SDHCI设置增强选通
    pub fn rockchip_sdhci_set_enhanced_strobe(&mut self) -> Result<(), SdError> {
        // 根据设备类型选择适当的增强选通设置函数
        // 这里假设我们使用DWCMSHC控制器
        self.dwcmshc_sdhci_set_enhanced_strobe()
    }
}

// DLL锁定检查辅助函数
fn dll_lock_wo_tmout(x: u32) -> bool {
    ((x & DWCMSHC_EMMC_DLL_LOCKED) == DWCMSHC_EMMC_DLL_LOCKED) && ((x & DWCMSHC_EMMC_DLL_TIMEOUT) == 0) // DWCMSHC_EMMC_DLL_LOCKED && !DWCMSHC_EMMC_DLL_TIMEOUT
}

const EMMC_PROG_CLOCK_MODE: u16 = 0x0020;
const EMMC_DIVIDER_SHIFT: u16 = 8;
const EMMC_DIVIDER_HI_SHIFT: u16 = 6;

const SDHCI_CTRL_UHS_MASK: u16 = 0x0007;
