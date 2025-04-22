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
pub const MMC_VERSION_4_4: u32 = make_mmc_version(4, 4, 1);
pub const MMC_VERSION_4_5: u32 = make_mmc_version(4, 5, 0);
pub const MMC_VERSION_5_0: u32 = make_mmc_version(5, 0, 0);
pub const MMC_VERSION_5_1: u32 = make_mmc_version(5, 1, 0);

const DWCMSHC_EMMC_DLL_LOCKED: u32 = 1 << 8;
const DWCMSHC_EMMC_DLL_TIMEOUT: u32 = 1 << 9;

pub fn dll_lock_wo_tmout(x: u32) -> bool {
    ((x & DWCMSHC_EMMC_DLL_LOCKED) == DWCMSHC_EMMC_DLL_LOCKED) && 
    ((x & DWCMSHC_EMMC_DLL_TIMEOUT) == 0)
}