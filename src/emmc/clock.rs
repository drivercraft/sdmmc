use core::fmt;
use core::ptr::{read_volatile, write_volatile};
use log::debug;

/// RK3568 eMMC时钟源选择常量
const CCLK_EMMC_SEL_24M: u32 = 0;  // OSC (24MHz)
const CCLK_EMMC_SEL_200M: u32 = 1; // 200 MHz
const CCLK_EMMC_SEL_150M: u32 = 2; // 150 MHz
const CCLK_EMMC_SEL_100M: u32 = 3; // 100 MHz
const CCLK_EMMC_SEL_50M: u32 = 4;  // 50 MHz
const CCLK_EMMC_SEL_375K: u32 = 5; // 375 KHz

/// eMMC 总线时钟(BCLK)选择常量
const BCLK_EMMC_SEL_200M: u32 = 0; // 200 MHz
const BCLK_EMMC_SEL_150M: u32 = 1; // 150 MHz
const BCLK_EMMC_SEL_125M: u32 = 2; // 125 MHz

/// eMMC 总线时钟选择掩码和偏移
const BCLK_EMMC_SEL_MASK: u32 = 0x3 << BCLK_EMMC_SEL_SHIFT;
const BCLK_EMMC_SEL_SHIFT: u32 = 8;

/// 频率常量
const MHZ: u64 = 1_000_000;
const KHZ: u64 = 1_000;
const OSC_HZ: u64 = 24 * MHZ;      // 24 MHz

/// eMMC时钟选择掩码和偏移
const CCLK_EMMC_SEL_MASK: u32 = 0x7 << CCLK_EMMC_SEL_SHIFT;
const CCLK_EMMC_SEL_SHIFT: u32 = 12;

/// 错误类型
#[derive(Debug, Clone, Copy)]
pub enum RK3568Error {
    InvalidClockRate,
    RegisterOperationFailed,
    InvalidPeripheralId,
    ResetTimeout,
}

/// RK3568 时钟控制单元寄存器结构
#[repr(C)]
pub struct RK3568Cru {
    cru_apll_con: [u32; 5],             // APLL 寄存器 /* 0x0000~0x0014 */
    reserved0: [u32; 3],                // reserved
    cru_dpll_con: [u32; 5],             // GPLL 寄存器 /* 0x0020~0x0034 */
    reserved1: [u32; 3],                // reserved
    cru_gpll_con: [u32; 5],             // CPLL 寄存器 /* 0x0040~0x0054 */
    reserved2: [u32; 3],                // reserved
    cru_cpll_con: [u32; 5],             // DPLL 寄存器 /* 0x0060~0x0074 */
    reserved3: [u32; 3],                // reserved
    cru_npll_con: [u32; 2],             // NPLL 寄存器 /* 0x0080~0x0088 */
    reserved4: [u32; 6],                // reserved
    cru_vpll_con: [u32; 2],             // GPLL2 寄存器 /* 0x00A0~0x00A8 */
    reserved5: [u32; 6],                // reserved
    
    cru_mode_con00: u32,                // 模式控制寄存器 /* 0x00C0 */
    cru_misc_con: [u32; 3],             // 杂项控制寄存器 
    cru_glb_cnt_th: u32,                // 全局计数阈值 /*  */
    cru_glb_srst_fst: u32,              // 全局软复位
    cru_glb_srsr_snd: u32,              // 全局软复位
    cru_glb_rst_con: u32,               // 全局软复位阈值
    cru_glb_rst_st: u32,                // 全局软复位状态
    
    reserved6: [u32; 7],                // reserved
    clksel_con: [u32; 85],              // 时钟选择寄存器
    reserved7: [u32; 43],               // reserved
    clk_gate_con: [u32; 36],            // 时钟门控寄存器
    reserved8: [u32; 28],               // reserved
    
    cru_softrst_con: [u32; 30],         // 软复位寄存器
    reserved9: [u32; 2],                // reserved
    cru_ssgtbl: [u32; 32],              // SSG表寄存器

    cru_autocs_core_con: [u32; 2],      // Pdcore auto clock swith control
    cru_autocs_gpu_con: [u32; 2],       // Pdgpu auto clock swith control
    cru_autocs_bus_con: [u32; 2],       // Pdbus auto clock swith control
    cru_autocs_top_con: [u32; 2],       // Top auto clock swith control
    cru_autocs_rkvdec_con: [u32; 2],    // Rkvdec auto clock swith control
    cru_autocs_rkvenc_con: [u32; 2],    // Rkvenc auto clock swith control
    cru_autocs_vpu_con: [u32; 2],       // Vpu auto clock swith control
    cru_autocs_peri_con: [u32; 2],      // Pdperi auto clock swith control
    cru_autocs_gpll_con: [u32; 2],      // Gpll auto clock swith control
    cru_autocs_cpll_con: [u32; 2],      // Cpll auto clock swith control

    reserved10: [u32; 12],              // reserved
    sdmmc0_con: [u32; 2],               // SDMMC0 control
    sdmmc1_con: [u32; 2],               // SDMMC1 control
    sdmmc2_con: [u32; 2],               // SDMMC2 control
    emmc_con: [u32; 2],                 // EMMC control
}

/// RK3568 时钟驱动
pub struct RK3568ClkPriv {
    cru: usize,
}

impl fmt::Display for RK3568Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RK3568Error::InvalidClockRate => write!(f, "Invalid clock rate"),
            RK3568Error::RegisterOperationFailed => write!(f, "Register operation failed"),
            RK3568Error::InvalidPeripheralId => write!(f, "Invalid peripheral ID"),
            RK3568Error::ResetTimeout => write!(f, "Reset operation timed out"),
        }
    }
}

impl RK3568ClkPriv {
    pub unsafe fn new(cru_ptr: *mut RK3568Cru) -> Self {
        // cru的基地址
        Self {
            cru: cru_ptr as usize,
        }
    }

    /// 获取当前eMMC时钟频率
    pub fn emmc_get_clk(&self) -> Result<u64, RK3568Error> {
        // 安全地读取寄存器
        let con = unsafe { read_volatile(&(*((self.cru) as *mut RK3568Cru)).clksel_con[28]) };
        
        // 提取时钟选择位
        let sel = (con & CCLK_EMMC_SEL_MASK) >> CCLK_EMMC_SEL_SHIFT;
        
        // 根据选择返回对应频率
        match sel {
            CCLK_EMMC_SEL_200M => Ok(200 * MHZ),
            CCLK_EMMC_SEL_150M => Ok(150 * MHZ),
            CCLK_EMMC_SEL_100M => Ok(100 * MHZ),
            CCLK_EMMC_SEL_50M => Ok(50 * MHZ),
            CCLK_EMMC_SEL_375K => Ok(375 * KHZ),
            CCLK_EMMC_SEL_24M => Ok(OSC_HZ),
            _ => Err(RK3568Error::InvalidClockRate),
        }
    }
    
    /// 设置eMMC时钟频率
    pub fn emmc_set_clk(&self, rate: u64) -> Result<u64, RK3568Error> {
        debug!("cru = {:#x}, rate = {}", self.cru, rate);
        
        // 根据请求的频率选择对应的时钟源
        let src_clk = match rate {
            OSC_HZ => CCLK_EMMC_SEL_24M,
            r if r == 52 * MHZ || r == 50 * MHZ => CCLK_EMMC_SEL_50M,
            r if r == 100 * MHZ => CCLK_EMMC_SEL_100M,
            r if r == 150 * MHZ => CCLK_EMMC_SEL_150M,
            r if r == 200 * MHZ => CCLK_EMMC_SEL_200M,
            r if r == 400 * KHZ || r == 375 * KHZ => CCLK_EMMC_SEL_375K,
            _ => return Err(RK3568Error::InvalidClockRate),
        };
        
        unsafe {
            let addr = &mut (*((self.cru) as *mut RK3568Cru)).clksel_con[28];

            self.rk_clrsetreg(
                addr,
                CCLK_EMMC_SEL_MASK,
                src_clk << CCLK_EMMC_SEL_SHIFT
            );
        }
        
        // 返回实际设置的频率
        self.emmc_get_clk()
    }

    /// 获取当前 eMMC 总线时钟频率
    pub fn emmc_get_bclk(&self) -> Result<u64, RK3568Error> {
        // 安全地读取寄存器
        let con = unsafe { read_volatile(&(*((self.cru) as *mut RK3568Cru)).clksel_con[28]) };
        
        // 提取时钟选择位
        let sel = (con & BCLK_EMMC_SEL_MASK) >> BCLK_EMMC_SEL_SHIFT;

        debug!("emmc_get_bclk con = {:#x} sel = {}", con, sel);
        
        // 根据选择返回对应频率
        match sel {
            BCLK_EMMC_SEL_200M => Ok(200 * MHZ),
            BCLK_EMMC_SEL_150M => Ok(150 * MHZ),
            BCLK_EMMC_SEL_125M => Ok(125 * MHZ),
            _ => Err(RK3568Error::InvalidClockRate),
        }
    }
    
    /// 设置 eMMC 总线时钟频率
    pub fn emmc_set_bclk(&mut self, rate: u64) -> Result<u64, RK3568Error> {
        // 根据请求的频率选择对应的时钟源
        let src_clk = match rate {
            r if r == 200 * MHZ => BCLK_EMMC_SEL_200M,
            r if r == 150 * MHZ => BCLK_EMMC_SEL_150M,
            r if r == 125 * MHZ => BCLK_EMMC_SEL_125M,
            _ => return Err(RK3568Error::InvalidClockRate),
        };
        
        unsafe {
            // 读取-修改-写入操作
            let addr = &mut (*((self.cru) as *mut RK3568Cru)).clksel_con[28];

            self.rk_clrsetreg(
                addr,
                BCLK_EMMC_SEL_MASK,
                src_clk << BCLK_EMMC_SEL_SHIFT
            );
        }
        
        // 返回实际设置的频率
        self.emmc_get_bclk()
    }
    
    /// 清除并设置寄存器的特定位
    pub fn rk_clrsetreg(&self, addr: *mut u32, clr: u32, set: u32) {
        let val = ((clr | set) << 16) | set;

        debug!("addr {:#x?}, clr, {:#x}, set {:#x} val {:#x}", addr, clr, set, val);

        unsafe { write_volatile(addr, val) };
    }
}

/// 相位调整相关常量
const ROCKCHIP_MMC_DELAY_SEL: u32 = 0x1;
const ROCKCHIP_MMC_DEGREE_MASK: u32 = 0x3;
const ROCKCHIP_MMC_DELAYNUM_OFFSET: u32 = 2;
const ROCKCHIP_MMC_DELAYNUM_MASK: u32 = 0xff << ROCKCHIP_MMC_DELAYNUM_OFFSET;
const ROCKCHIP_MMC_DELAY_ELEMENT_PSEC: u32 = 100;

/// MMC采样相位时钟ID
#[derive(Debug, Clone, Copy)]
pub enum RK3568MmcClockId {
    SclkEmmcSample,
    SclkSdmmc0Sample,
    SclkSdmmc1Sample,
    SclkSdmmc2Sample,
}

/// 时钟结构
pub struct Clock {
    pub id: RK3568MmcClockId,
    pub rate: u64,
}

impl RK3568ClkPriv {
    /// 获取MMC时钟相位(以度为单位)
    pub fn mmc_get_phase(&self, clk: &Clock) -> Result<u16, RK3568Error> {
        // 获取时钟频率
        let rate = clk.rate;
        if rate == 0 {
            return Err(RK3568Error::InvalidClockRate);
        }
        
        // 根据时钟ID读取相应的控制寄存器
        let raw_value = unsafe {
            match clk.id {
                RK3568MmcClockId::SclkEmmcSample => 
                    read_volatile(&(*((self.cru) as *mut RK3568Cru)).emmc_con[1]),
                RK3568MmcClockId::SclkSdmmc0Sample => 
                    read_volatile(&(*((self.cru) as *mut RK3568Cru)).sdmmc0_con[1]),
                RK3568MmcClockId::SclkSdmmc1Sample => 
                    read_volatile(&(*((self.cru) as *mut RK3568Cru)).sdmmc1_con[1]),
                RK3568MmcClockId::SclkSdmmc2Sample => 
                    read_volatile(&(*((self.cru) as *mut RK3568Cru)).sdmmc2_con[1]),
            }
        };
        
        let raw_value = raw_value >> 1;
        
        // 计算粗调相位(90度增量)
        let mut degrees = (raw_value & ROCKCHIP_MMC_DEGREE_MASK) * 90;
        
        // 检查是否启用了细调
        if (raw_value & ROCKCHIP_MMC_DELAY_SEL) != 0 {
            // 计算延迟元素带来的额外度数
            let factor = (ROCKCHIP_MMC_DELAY_ELEMENT_PSEC / 10) as u64 *
                            36 * (rate / 1_000_000);
            
            let delay_num = (raw_value & ROCKCHIP_MMC_DELAYNUM_MASK) >> ROCKCHIP_MMC_DELAYNUM_OFFSET;
            
            // 添加细调相位
            degrees += div_round_closest((delay_num as u64 * factor) as u32, 10000);
        }
        
        // 返回总相位(限制在0-359度)
        Ok(degrees as u16 % 360)
    }
    
    /// 设置MMC时钟相位
    pub fn mmc_set_phase(&mut self, clk: &Clock, degrees: u32) -> Result<(), RK3568Error> {
        let rate = clk.rate;
        if rate == 0 {
            return Err(RK3568Error::InvalidClockRate);
        }
        
        // 将请求的相位分解为90度步进和余数
        let nineties = degrees / 90;
        let remainder = degrees % 90;
        
        // 将余数转换为延迟元素数量
        let mut delay = 10_000_000; // PSECS_PER_SEC / 10000 / 10
        delay *= remainder;
        delay = div_round_closest(
            delay,
            (rate / 1000) * 36 * (ROCKCHIP_MMC_DELAY_ELEMENT_PSEC / 10) as u64
        ) as u32;
        
        // 限制延迟元素数量到最大允许值
        let delay_num = core::cmp::min(delay, 255) as u8;
        
        // 构建寄存器值
        let mut raw_value = if delay_num > 0 { ROCKCHIP_MMC_DELAY_SEL } else { 0 };
        raw_value |= (delay_num as u32) << ROCKCHIP_MMC_DELAYNUM_OFFSET;
        raw_value |= nineties;
        
        // 左移1位，以匹配寄存器布局
        raw_value <<= 1;
        
        // 向寄存器写入新值 (0xffff0000用于保留高16位)
        unsafe {
            let addr = match clk.id {
                RK3568MmcClockId::SclkEmmcSample => 
                    &mut (*((self.cru) as *mut RK3568Cru)).emmc_con[1],
                RK3568MmcClockId::SclkSdmmc0Sample => 
                    &mut (*((self.cru) as *mut RK3568Cru)).sdmmc0_con[1],
                RK3568MmcClockId::SclkSdmmc1Sample => 
                    &mut (*((self.cru) as *mut RK3568Cru)).sdmmc1_con[1],
                RK3568MmcClockId::SclkSdmmc2Sample => 
                    &mut (*((self.cru) as *mut RK3568Cru)).sdmmc2_con[1],
            };
            write_volatile(addr, raw_value | 0xffff0000);
        }
        
        if let Ok(actual_degrees) = self.mmc_get_phase(clk) {
            debug!(
                "mmc set_phase({}) delay_nums={} reg={:#x} actual_degrees={}", 
                degrees, delay_num, raw_value, actual_degrees
            );
        }
        
        Ok(())
    }
}

/// 辅助函数: 四舍五入的除法
fn div_round_closest(dividend: u32, divisor: u64) -> u32 {
    ((dividend as u64 + divisor / 2) / divisor) as u32
}

use spin::Mutex;
use core::cell::UnsafeCell;

static CLKRK3568_CLK: Mutex<Option<UnsafeCell<RK3568ClkPriv>>> = Mutex::new(None);

pub fn init_clk(clk: usize) -> bool {
    let clock = unsafe { RK3568ClkPriv::new(clk as *mut _) };
    
    let mut guard = CLKRK3568_CLK.lock();
    if guard.is_none() {
        *guard = Some(UnsafeCell::new(clock));
        debug!("Clock initialized successfully");
        true
    } else {
        debug!("Clock already initialized");
        false
    }
}

pub fn get_clk() -> Option<&'static RK3568ClkPriv> {
    let guard = CLKRK3568_CLK.lock();
    match &*guard {
        Some(cell) => Some(unsafe { &*cell.get() }),
        None => None
    }
}

pub fn emmc_get_clk() -> Result<u64, RK3568Error> {
    let binding = CLKRK3568_CLK.lock();
    let clk = binding.as_ref().ok_or(RK3568Error::RegisterOperationFailed)?;
    unsafe { (*clk.get()).emmc_get_clk() }
}

pub fn emmc_set_clk(rate: u64) -> Result<u64, RK3568Error> {
    let binding = CLKRK3568_CLK.lock();
    let clk = binding.as_ref().ok_or(RK3568Error::RegisterOperationFailed)?;
    unsafe { (*clk.get()).emmc_set_clk(rate) }
}
