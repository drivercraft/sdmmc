const MMC_VERSION_MMC: u32 = 1 << 30;

const fn make_sdmmc_version(a: u32, b: u32, c: u32) -> u32 {
    (a << 16) | (b << 8) | c
}

const fn make_mmc_version(a: u32, b: u32, c: u32) -> u32 {
    MMC_VERSION_MMC | make_sdmmc_version(a, b, c)
}

pub const MMC_VERSION_UNKNOWN: u32 = make_mmc_version(0, 0, 0);
pub const MMC_VERSION_1_2: u32 = make_mmc_version(1, 2, 0);
pub const MMC_VERSION_1_4: u32 = make_mmc_version(1, 4, 0);
pub const MMC_VERSION_2_2: u32 = make_mmc_version(2, 2, 0);
pub const MMC_VERSION_3: u32 = make_mmc_version(3, 0, 0);
pub const MMC_VERSION_4: u32 = make_mmc_version(4, 0, 0);
pub const MMC_VERSION_4_1: u32 = make_mmc_version(4, 1, 0);
pub const MMC_VERSION_4_2: u32 = make_mmc_version(4, 2, 0);
pub const MMC_VERSION_4_3: u32 = make_mmc_version(4, 3, 0);
pub const MMC_VERSION_4_41: u32 = make_mmc_version(4, 4, 1);
pub const MMC_VERSION_4_5: u32 = make_mmc_version(4, 5, 0);
pub const MMC_VERSION_5_0: u32 = make_mmc_version(5, 0, 0);
pub const MMC_VERSION_5_1: u32 = make_mmc_version(5, 1, 0);

const DWCMSHC_EMMC_DLL_LOCKED: u32 = 1 << 8;
const DWCMSHC_EMMC_DLL_TIMEOUT: u32 = 1 << 9;

pub fn dll_lock_wo_tmout(x: u32) -> bool {
    ((x & DWCMSHC_EMMC_DLL_LOCKED) == DWCMSHC_EMMC_DLL_LOCKED) && 
    ((x & DWCMSHC_EMMC_DLL_TIMEOUT) == 0)
}

#[inline]
pub fn lldiv(dividend: u64, divisor: u32) -> u64 {
    let mut result = dividend;    
    let _ = do_div(&mut result, divisor);

    result
}

#[inline]
fn do_div(n: &mut u64, base: u32) -> u32 {
    let remainder = (*n % base as u64) as u32;
    *n /= base as u64;
    remainder
}

pub fn generic_fls(x: u32) -> u32 {
    let mut r = 32;
    let mut val = x;

    if val == 0 {
        return 0;
    }
    
    if (val & 0xffff0000) == 0 {
        val <<= 16;
        r -= 16;
    }
    
    if (val & 0xff000000) == 0 {
        val <<= 8;
        r -= 8;
    }
    
    if (val & 0xf0000000) == 0 {
        val <<= 4;
        r -= 4;
    }
    
    if (val & 0xc0000000) == 0 {
        val <<= 2;
        r -= 2;
    }
    
    if (val & 0x80000000) == 0 {
        val <<= 1;
        r -= 1;
    }
    
    r
}