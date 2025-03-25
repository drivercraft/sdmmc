// SDHCI register offsets
#[allow(unused)]
pub const SDHCI_DMA_ADDRESS: u32 = 0x00;

const SDHCI_ARGUMENT2: u32 = SDHCI_DMA_ADDRESS;
const SDHCI_32BIT_BLK_CNT: u32 = SDHCI_DMA_ADDRESS;

pub const SDHCI_BLOCK_SIZE: u32 = 0x04;
pub const SDHCI_BLOCK_COUNT: u32 = 0x06;
pub const SDHCI_ARGUMENT: u32 = 0x08;
pub const SDHCI_TRANSFER_MODE: u32 = 0x0C;
pub const SDHCI_COMMAND: u32 = 0x0E;
pub const SDHCI_RESPONSE: u32 = 0x10; // 0x10-0x1F, 4 registers
pub const SDHCI_BUFFER: u32 = 0x20;
pub const SDHCI_PRESENT_STATE: u32 = 0x24;
pub const SDHCI_HOST_CONTROL: u32 = 0x28;
pub const SDHCI_POWER_CONTROL: u32 = 0x29;
pub const SDHCI_BLOCK_GAP_CONTROL: u32 = 0x2A;
pub const SDHCI_WAKE_UP_CONTROL: u32 = 0x2B;
pub const SDHCI_CLOCK_CONTROL: u32 = 0x2C;
pub const SDHCI_TIMEOUT_CONTROL: u32 = 0x2E;
pub const SDHCI_SOFTWARE_RESET: u32 = 0x2F;
pub const SDHCI_INT_STATUS: u32 = 0x30;
pub const SDHCI_INT_ENABLE: u32 = 0x34;
pub const SDHCI_SIGNAL_ENABLE: u32 = 0x38;
pub const SDHCI_AUTO_CMD_STATUS: u32 = 0x3C;
pub const SDHCI_HOST_CONTROL2: u32 = 0x3E;
pub const SDHCI_CAPABILITIES: u32 = 0x40;
pub const SDHCI_CAPABILITIES_1: u32 = 0x44;
pub const SDHCI_MAX_CURRENT: u32 = 0x48;

/* 4C-4F reserved for more max current */
#[allow(unused)]
pub const SDHCI_SET_ACMD12_ERROR: u32 = 0x50;
#[allow(unused)]
pub const SDHCI_SET_INT_ERROR: u32 = 0x52;

#[allow(unused)]
pub const SDHCI_ADMA_ERROR: u32 = 0x54;

/* 55-57 reserved */

#[allow(unused)]
pub const SDHCI_ADMA_ADDRESS: u32 = 0x58;
#[allow(unused)]
pub const SDHCI_ADMA_ADDRESS_HI: u32 = 0x5C;

/* 60-FB reserved */

#[allow(unused)]
pub const SDHCI_PRESET_FOR_HIGH_SPEED: u32 = 0x64;
#[allow(unused)]
pub const SDHCI_PRESET_FOR_SDR12: u32 = 0x66;
#[allow(unused)]
pub const SDHCI_PRESET_FOR_SDR25: u32 = 0x68;
#[allow(unused)]
pub const SDHCI_PRESET_FOR_SDR50: u32 = 0x6A;
#[allow(unused)]
pub const SDHCI_PRESET_FOR_SDR104: u32 = 0x6C;
#[allow(unused)]
pub const SDHCI_PRESET_FOR_DDR50: u32 = 0x6E;
#[allow(unused)]
pub const SDHCI_PRESET_FOR_HS400: u32 = 0x74; /* Non-standard */

#[allow(unused)]
pub const SDHCI_SLOT_INT_STATUS: u32 = 0xFC;

#[allow(unused)]
pub const SDHCI_HOST_VERSION: u32 = 0xFE;

/*
 * End of controller registers.
 */

// SDHCI flags
pub const SDHCI_CMD_RESP_MASK: u16 = 0x03;
pub const SDHCI_CMD_CRC: u16 = 0x08;
pub const SDHCI_CMD_INDEX: u16 = 0x10;
pub const SDHCI_CMD_DATA: u16 = 0x20;
pub const SDHCI_CMD_ABORTCMD: u32	= 0xC0;

pub const SDHCI_CMD_RESP_NONE: u16 = 0x00;
pub const SDHCI_CMD_RESP_LONG: u16 = 0x01;
pub const SDHCI_CMD_RESP_SHORT: u16 = 0x02;
pub const SDHCI_CMD_RESP_SHORT_BUSY: u16 = 0x03;

// SDHCI transfer mode flags
pub const SDHCI_TRNS_DMA: u16 = 0x01;
pub const SDHCI_TRNS_BLK_CNT_EN: u16 = 0x02;
pub const SDHCI_TRNS_AUTO_CMD12: u16 = 0x04;
pub const SDHCI_TRNS_AUTO_CMD23: u16 = 0x08;
pub const SDHCI_TRNS_AUTO_SEL: u32 = 0x0C;
pub const SDHCI_TRNS_READ: u16 = 0x10;
pub const SDHCI_TRNS_MULTI: u16 = 0x20;

// SDHCI present state flags
pub const SDHCI_DATA_INHIBIT: u32 = 0x00000001;
pub const SDHCI_CMD_INHIBIT: u32 = 0x00000002;
pub const SDHCI_CARD_INSERTED: u32 = 0x00010000;
pub const SDHCI_WRITE_PROTECT: u32 = 0x00080000;

// const SDHCI_CMD_INHIBIT: u32 = 0x00000001;
// const SDHCI_DATA_INHIBIT: u32 = 0x00000002;
// const SDHCI_DOING_WRITE: u32 = 0x00000100;
// const SDHCI_DOING_READ: u32 = 0x00000200;
// const SDHCI_SPACE_AVAILABLE: u32 = 0x00000400;
// const SDHCI_DATA_AVAILABLE: u32 =	0x00000800;
// const SDHCI_CARD_PRESENT: u32 = 0x00010000;
// const SDHCI_CARD_PRES_SHIFT: u32 = 16;
// const SDHCI_CD_STABLE: u32 = 0x00020000;
// const SDHCI_CD_LVL: u32 = 0x00040000;
// const SDHCI_CD_LVL_SHIFT: u32 = 18;
// const SDHCI_DATA_LVL_MASK: u32 = 0x00F00000;
// const SDHCI_DATA_LVL_SHIFT: u32 = 20;
// const SDHCI_DATA_0_LVL_MASK: u32 = 0x00100000;
// const SDHCI_CMD_LVL: u32 = 0x01000000;

// SDHCI host control flags
pub const SDHCI_CTRL_LED: u8 = 0x01;
pub const SDHCI_CTRL_4BITBUS: u8 = 0x02;
pub const SDHCI_CTRL_HISPD: u8 = 0x04;
pub const SDHCI_CTRL_DMA_MASK: u8 = 0x18;
pub const SDHCI_CTRL_SDMA: u8 = 0x00;
pub const SDHCI_CTRL_ADMA1: u8 = 0x08;
pub const SDHCI_CTRL_ADMA32: u8 = 0x10;
pub const SDHCI_CTRL_ADMA64: u8 = 0x18;
pub const SDHCI_CTRL_8BITBUS: u8 = 0x20;
// const SDHCI_CTRL_ADMA3: u32 = 0x18;
// const SDHCI_CTRL_CDTEST_INS: u32 = 0x40;
// const SDHCI_CTRL_CDTEST_EN: u32 = 0x80;

// SDHCI clock control flags
pub const SDHCI_CLOCK_INT_EN: u16 = 0x0001;
pub const SDHCI_CLOCK_INT_STABLE: u16 = 0x0002;
pub const SDHCI_CLOCK_CARD_EN: u16 = 0x0004;
pub const SDHCI_CLOCK_DIV_SHIFT: u8 = 8;

// SDHCI reset flags
pub const SDHCI_RESET_ALL: u8 = 0x01;
pub const SDHCI_RESET_CMD: u8 = 0x02;
pub const SDHCI_RESET_DATA: u8 = 0x04;

// SDHCI interrupt flags
pub const SDHCI_INT_RESPONSE: u32 = 0x00000001;
pub const SDHCI_INT_DATA_END: u32 = 0x00000002;
pub const SDHCI_INT_BLK_GAP: u32 = 0x00000004;
pub const SDHCI_INT_DMA_END: u32 = 0x00000008;
pub const SDHCI_INT_SPACE_AVAIL: u32 = 0x00000010;
pub const SDHCI_INT_DATA_AVAIL: u32 = 0x00000020;
pub const SDHCI_INT_CARD_INSERT: u32 = 0x00000040;
pub const SDHCI_INT_CARD_REMOVE: u32 = 0x00000080;
pub const SDHCI_INT_CARD_INT: u32 = 0x00000100;
pub const SDHCI_INT_RETUNE: u32 = 0x00001000;
pub const SDHCI_INT_CQE	: u32 = 0x00004000;
pub const SDHCI_INT_ERROR: u32 = 0x00008000;
pub const SDHCI_INT_TIMEOUT: u32 = 0x00010000;
pub const SDHCI_INT_CRC: u32 = 0x00020000;
pub const SDHCI_INT_END_BIT: u32 = 0x00040000;
pub const SDHCI_INT_INDEX: u32 = 0x00080000;
pub const SDHCI_INT_DATA_TIMEOUT: u32 = 0x00100000;
pub const SDHCI_INT_DATA_CRC: u32 = 0x00200000;
pub const SDHCI_INT_DATA_END_BIT: u32 = 0x00400000;
pub const SDHCI_INT_BUS_POWER: u32 = 0x00800000;
const SDHCI_INT_AUTO_CMD_ERR: u32 = 0x01000000;
const SDHCI_INT_ADMA_ERROR: u32 = 0x02000000;

pub const SDHCI_INT_NORMAL_MASK: u32 = 0x00007FFF;
pub const SDHCI_INT_ERROR_MASK: u32 = 0xFFFF8000;

pub const SDHCI_INT_CMD_MASK: u32 = SDHCI_INT_RESPONSE | SDHCI_INT_TIMEOUT | SDHCI_INT_CRC | SDHCI_INT_END_BIT | SDHCI_INT_INDEX | SDHCI_INT_AUTO_CMD_ERR;
pub const SDHCI_INT_DATA_MASK: u32 = SDHCI_INT_DATA_END | SDHCI_INT_DATA_TIMEOUT | SDHCI_INT_DATA_CRC | SDHCI_INT_DATA_END_BIT | SDHCI_INT_ADMA_ERROR;
pub const SDHCI_INT_ALL_MASK: u32 = SDHCI_INT_CMD_MASK | SDHCI_INT_DATA_MASK;

// pub const SDHCI_INT_CMD_MASK: u32 = SDHCI_INT_RESPONSE | SDHCI_INT_TIMEOUT | SDHCI_INT_CRC | SDHCI_INT_END_BIT | SDHCI_INT_INDEX | SDHCI_INT_AUTO_CMD_ERR;
// pub const SDHCI_INT_DATA_MASK: u32 = SDHCI_INT_DATA_END | SDHCI_INT_DMA_END | SDHCI_INT_DATA_AVAIL | SDHCI_INT_SPACE_AVAIL | SDHCI_INT_DATA_TIMEOUT | SDHCI_INT_DATA_CRC | SDHCI_INT_DATA_END_BIT | SDHCI_INT_ADMA_ERROR | SDHCI_INT_BLK_GAP;
// pub const SDHCI_INT_ALL_MASK: u32 = (u32::MAX) as u32;

// pub const SDHCI_CQE_INT_ERR_MASK: u32 = SDHCI_INT_ADMA_ERROR | SDHCI_INT_BUS_POWER | SDHCI_INT_DATA_END_BIT | SDHCI_INT_DATA_CRC | SDHCI_INT_DATA_TIMEOUT | SDHCI_INT_INDEX | SDHCI_INT_END_BIT | SDHCI_INT_CRC | SDHCI_INT_TIMEOUT;

// pub const SDHCI_CQE_INT_MASK: u32 = SDHCI_CQE_INT_ERR_MASK | SDHCI_INT_CQE;


// SD/MMC Command definitions
pub const MMC_GO_IDLE_STATE: u8 = 0;

pub const MMC_ALL_SEND_CID: u8 = 2;
pub const MMC_SET_RELATIVE_ADDR: u8 = 3;
pub const MMC_SET_DSR: u8 = 4;
pub const MMC_SWITCH: u8 = 6;
pub const MMC_SELECT_CARD: u8 = 7;
pub const MMC_SEND_EXT_CSD: u8 = 8;
pub const MMC_SEND_CSD: u8 = 9;
pub const MMC_SEND_CID: u8 = 10;
pub const MMC_STOP_TRANSMISSION: u8 = 12;
pub const MMC_SEND_STATUS: u8 = 13;
pub const MMC_SET_BLOCKLEN: u8 = 16;
pub const MMC_READ_SINGLE_BLOCK: u8 = 17;
pub const MMC_READ_MULTIPLE_BLOCK: u8 = 18;
pub const MMC_WRITE_BLOCK: u8 = 24;
pub const MMC_WRITE_MULTIPLE_BLOCK: u8 = 25;
pub const MMC_APP_CMD: u8 = 55;

// SD-specific commands
pub const SD_SEND_RELATIVE_ADDR: u8 = 3;
pub const SD_SWITCH_FUNC: u8 = 6;
pub const SD_SEND_IF_COND: u8 = 8;
pub const SD_APP_OP_COND: u8 = 41;
pub const SD_APP_SEND_SCR: u8 = 51;

// Response types
pub const MMC_RSP_PRESENT: u32 = 1 << 0;
pub const MMC_RSP_136: u32 = 1 << 1; // 136-bit response
pub const MMC_RSP_CRC: u32 = 1 << 2; // Expect valid CRC
pub const MMC_RSP_BUSY: u32 = 1 << 3; // Card may send busy
pub const MMC_RSP_OPCODE: u32 = 1 << 4; // Response contains opcode

pub const MMC_RSP_NONE: u32 = 0;
pub const MMC_RSP_R1: u32 = MMC_RSP_PRESENT | MMC_RSP_CRC | MMC_RSP_OPCODE;
pub const MMC_RSP_R1B: u32 = MMC_RSP_PRESENT | MMC_RSP_CRC | MMC_RSP_OPCODE | MMC_RSP_BUSY;
pub const MMC_RSP_R2: u32 = MMC_RSP_PRESENT | MMC_RSP_136 | MMC_RSP_CRC;
pub const MMC_RSP_R3: u32 = MMC_RSP_PRESENT;
pub const MMC_RSP_R4: u32 = MMC_RSP_PRESENT;
pub const MMC_RSP_R5: u32 = MMC_RSP_PRESENT | MMC_RSP_CRC | MMC_RSP_OPCODE;
pub const MMC_RSP_R6: u32 = MMC_RSP_PRESENT | MMC_RSP_CRC | MMC_RSP_OPCODE;
pub const MMC_RSP_R7: u32 = MMC_RSP_PRESENT | MMC_RSP_CRC | MMC_RSP_OPCODE;

// Card states
pub const MMC_STATE_PRESENT: u32 = 1 << 0;
pub const MMC_STATE_READONLY: u32 = 1 << 1;
pub const MMC_STATE_HIGHSPEED: u32 = 1 << 2;
pub const MMC_STATE_BLOCKADDR: u32 = 1 << 3;
pub const MMC_STATE_HIGHCAPACITY: u32 = 1 << 4;
pub const MMC_STATE_ULTRAHIGHSPEED: u32 = 1 << 5;
pub const MMC_STATE_DDR_MODE: u32 = 1 << 6;
pub const MMC_STATE_HS200: u32 = 1 << 7;
pub const MMC_STATE_HS400: u32 = 1 << 8;

// const SDHCI_POWER_ON: u32 = 0x01;
// const SDHCI_POWER_180: u32 = 0x0A;
// const SDHCI_POWER_300: u32 = 0x0C;
// const SDHCI_POWER_330: u32 = 0x0E;

// const SDHCI_WAKE_ON_INT: u32 = 0x01;
// const SDHCI_WAKE_ON_INSERT: u32 = 0x02;
// const SDHCI_WAKE_ON_REMOVE: u32 = 0x04;

// const SDHCI_DIVIDER_SHIFT: u32 = 8;
// const SDHCI_DIVIDER_HI_SHIFT: u32 = 6;
// const SDHCI_DIV_MASK: u32 = 0xFF;
// const SDHCI_DIV_MASK_LEN: u32 = 8;
// const SDHCI_DIV_HI_MASK: u32 = 0x300;
// const SDHCI_PROG_CLOCK_MODE: u32 = 0x0020;
// const SDHCI_CLOCK_CARD_EN: u32 = 0x0004;
// const SDHCI_CLOCK_PLL_EN: u32 = 0x0008;
// const SDHCI_CLOCK_INT_STABLE: u32 = 0x0002;
// const SDHCI_CLOCK_INT_EN: u32 = 0x0001;

// const SDHCI_AUTO_CMD_TIMEOUT: u32 = 0x00000002;
// const SDHCI_AUTO_CMD_CRC: u32 = 0x00000004;
// const SDHCI_AUTO_CMD_END_BIT: u32 = 0x00000008;
// const SDHCI_AUTO_CMD_INDEX: u32 = 0x00000010;


// const SDHCI_CTRL_UHS_MASK: u32 = 0x0007;
// const SDHCI_CTRL_UHS_SDR12: u32 = 0x0000;
// const SDHCI_CTRL_UHS_SDR25: u32 = 0x0001;
// const SDHCI_CTRL_UHS_SDR50: u32 = 0x0002;
// const SDHCI_CTRL_UHS_SDR104: u32 = 0x0003;
// const SDHCI_CTRL_UHS_DDR50: u32 = 0x0004;
// const SDHCI_CTRL_HS400: u32 = 0x0005; /* Non-standard */
// const SDHCI_CTRL_VDD_180: u32 = 0x0008;
// const SDHCI_CTRL_DRV_TYPE_MASK: u32 = 0x0030;
// const SDHCI_CTRL_DRV_TYPE_B: u32 = 0x0000;
// const SDHCI_CTRL_DRV_TYPE_A: u32 = 0x0010;
// const SDHCI_CTRL_DRV_TYPE_C: u32 = 0x0020;
// const SDHCI_CTRL_DRV_TYPE_D: u32 = 0x0030;
// const SDHCI_CTRL_EXEC_TUNING: u32 = 0x0040;
// const SDHCI_CTRL_TUNED_CLK: u32 = 0x0080;
// const SDHCI_CMD23_ENABLE: u32 = 0x0800;
// const SDHCI_CTRL_V4_MODE: u32 = 0x1000;
// const SDHCI_CTRL_64BIT_ADDR: u32 = 0x2000;
// const SDHCI_CTRL_PRESET_VAL_ENABLE: u32 = 0x8000;
