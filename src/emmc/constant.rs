#![allow(unused)]

// EMMC register offsets
pub const EMMC_SDMASA: u32 = 0x0000;                    // SDMA System Address Register
pub const EMMC_BLOCK_SIZE : u32 = 0x0004;               // Block Size Register
pub const EMMC_BLOCK_COUNT : u32 = 0x0006;              // 16-bit Block Count Register
pub const EMMC_ARGUMENT: u32 = 0x0008;                  // Command Argument Register
pub const EMMC_XFER_MODE: u32 = 0x000C;                 // Transfer Mode Register
pub const EMMC_COMMAND: u32 = 0x000E;                   // Command Register
pub const EMMC_RESPONSE: u32 = 0x0010;                  // 0x10-0x1F, 4 Response Registers
pub const EMMC_BUF_DATA: u32 = 0x0020;                  // Buffer Data Port Register
pub const EMMC_PRESENT_STATE: u32 = 0x0024;             // Present State Register
pub const EMMC_HOST_CTRL1: u32 = 0x0028;                // Host Control 1 Register
pub const EMMC_POWER_CTRL: u32 = 0x0029;                // Power Control Register
pub const EMMC_BLOCK_GAP_CONTROL: u32 = 0x002A;         // Block Gap Control Register
pub const EMMC_CLOCK_CONTROL: u32 = 0x002C;             // Clock Control Register
pub const EMMC_TIMEOUT_CONTROL: u32 = 0x002E;           // Timeout Control Register
pub const EMMC_SOFTWARE_RESET: u32 = 0x002F;            // Software Reset Register
pub const EMMC_NORMAL_INT_STAT: u32 = 0x0030;           // Normal Interrupt Status Register
pub const EMMC_ERROR_INT_STAT: u32 = 0x0032;            // Error Interrupt Status Register
pub const EMMC_NORMAL_INT_STAT_EN: u32 = 0x0034;        // Normal Interrupt Status Enable Register
pub const EMMC_ERROR_INT_STAT_EN: u32 = 0x0036;         // Error Interrupt Status Enable Register
pub const EMMC_SIGNAL_ENABLE: u32 = 0x0038;             // Normal Interrupt Signal Enable Register
pub const EMMC_ERROR_INT_SIGNAL_EN: u32 = 0x003A;       // Error Interrupt Signal Enable Register
pub const EMMC_AUTO_CMD_STAT: u32 = 0x003C;             // Auto CMD Error Status Register
pub const EMMC_HOST_CTRL2: u32 = 0x003E;                // Host Control 2 Register
pub const EMMC_CAPABILITIES1: u32 = 0x0040;             // Capabilities Register 1
pub const EMMC_CAPABILITIES2: u32 = 0x0044;             // Capabilities Register 2

pub const EMMC_FORCE_AUTO_CMD_STAT: u32 = 0x0050;       // Force Event Register for Auto CMD Error Status Register
pub const EMMC_FORCE_ERR_INT_STAT: u32 = 0x0052;        // Force Event Register for Error Interrupt Status Register

pub const EMMC_ADMA_ERR_STAT: u32 = 0x0054;             // ADMA Error Status Register
pub const EMMC_ADMA_SA: u32 = 0x0058;                   // ADMA System Address Register

pub const EMMC_PRESET_INIT: u32 =  0x0060;              // Preset Value for Initialization
pub const EMMC_PRESET_DS: u32 = 0x0062;                 // Preset Value for Default Speed
pub const EMMC_PRESET_HS: u32 = 0x0064;                 // Preset Value for High Speed
pub const EMMC_PRESET_SDR12: u32 = 0x0066;              // Preset Value for SDR12
pub const EMMC_PRESET_SDR25: u32 = 0x0068;              // Preset Value for SDR25
pub const EMMC_PRESET_SDR50: u32 = 0x006A;              // Preset Value for SDR50
pub const EMMC_PRESET_SDR104: u32 = 0x006C;             // Preset Value for SDR104
pub const EMMC_PRESET_DDR50: u32 = 0x006E;              // Preset Value for DDR50
pub const EMMC_ADMA_ID: u32 = 0x0078;                   // ADMA3 Integrated Descriptor Address Register
pub const EMMC_SLOT_INTR_STATUS: u32 = 0x00FC;          // Slot Interrupt Status Register
pub const EMMC_HOST_CNTRL_VER: u32 = 0x00FE;            // Host Controller Version

pub const EMMC_COVER: u32 = 0x0180;                     // Command Queuing Version Register
pub const EMMC_CQCAP: u32 = 0x0184;                     // Command Queuing Capabilities Register
pub const EMMC_CQCFG: u32 = 0x0188;                     // Command Queuing Configuration Register
pub const EMMC_CQCTRL: u32 = 0x018C;                    // Command Queuing Control Register
pub const EMMC_CQIS: u32 = 0x0190;                      // Command Queuing Interrupt Status Register
pub const EMMC_CQISE: u32 = 0x0194;                     // Command Queuing Interrupt Status Enable Register
pub const EMMC_CQISGE: u32 = 0x0198;                    // Command Queuing Interrupt Signal Enable Register
pub const EMMC_CQIC: u32 = 0x019C;                      // Command Queuing Interrupt Coalescing Register
pub const EMMC_CQTDLBA: u32 = 0x01A0;                   // Command Queuing Task Descriptor List Base Address Register
pub const EMMC_CQTDBR: u32 = 0x01A8;                    // Command Queuing Doorbell Register
pub const EMMC_CQTDBN: u32 = 0x01AC;                    // Command Queuing Task Clear Notification Register
pub const EMMC_CQDOS: u32 = 0x01B0;                     // Command Queuing Device Queue Status Register
pub const EMMC_CQDPT: u32 = 0x01B4;                     // Command Queuing Device Pending Tasks Register
pub const EMMC_COTCLR: u32 = 0x01B8;                    // Command Queuing Task Clear Register
pub const EMMC_QSSC1: u32 = 0x01C0;                     // Command Queuing Send Status Configuration Register 1
pub const EMMC_QSSC2: u32 = 0x01C4;                     // Command Queuing Send Status Configuration Register 2
pub const EMMC_CQRDCT: u32 = 0x01C8;                    // Command Queuing Command Response For Direct Command Register
pub const EMMC_CQRMEM: u32 = 0x01D0;                    // Command Queuing Command Response Mode Error Mask Register
pub const EMMC_CQTERRI: u32 = 0x01D4;                   // Command Queuing Task Error Information Register
pub const EMMC_CQCRI: u32 = 0x01D8;                     // Command Queuing Command Response Index Register
pub const EMMC_CQCRA: u32 = 0x01DC;                     // Command Queuing Command Response Argument Register

pub const EMMC_VER_ID: u32 = 0x0500;                    // Host Version ID Register
pub const EMMC_VER_TYPE: u32 = 0x0504;                  // Host Version Type Register
pub const EMMC_HOST_CTRL3: u32 = 0x0508;                // Host Control 3 Register
pub const EMMC_EMMC_CTRL: u32 = 0x052C;                 // EMMC Control Register
pub const EMMC_BOOT_CTRL: u32 = 0x052E;                 // Boot Control Register
pub const EMMC_AT_CTRL: u32 = 0x0540;                   // Boot Control Register
pub const EMMC_AT_STAT: u32 = 0x0544;                   // Boot Control Register

pub const EMMC_DLL_CTRL: u32 = 0x0800;                  // DLL Global Control Register
pub const EMMC_DLL_RXCLK: u32 = 0x0804;                 // DLL Control Register For RXCLK
pub const EMMC_DLL_TXCLK: u32 = 0x0808;                 // DLL Control Register For TXCLK
pub const EMMC_DLL_STRBIN: u32 = 0x080C;                // DLL Control Register For STRBIN
pub const EMMC_DLL_STATUS0: u32 = 0x0840;               // DLL Status Register 0
pub const EMMC_DLL_STATUS1: u32 = 0x0844;               // DLL Status Register 1

/*
 * End of controller registers.
 */

// EMMC flags
pub const EMMC_CMD_RESP_MASK: u16 = 0x03;
pub const EMMC_CMD_CRC: u16 = 0x08;
pub const EMMC_CMD_INDEX: u16 = 0x10;
pub const EMMC_CMD_DATA: u16 = 0x20;
pub const EMMC_CMD_ABORTCMD: u32	= 0xC0;

pub const EMMC_CMD_RESP_NONE: u16 = 0x00;
pub const EMMC_CMD_RESP_LONG: u16 = 0x01;
pub const EMMC_CMD_RESP_SHORT: u16 = 0x02;
pub const EMMC_CMD_RESP_SHORT_BUSY: u16 = 0x03;

// EMMC transfer mode flags
pub const EMMC_TRNS_DMA: u16 = 0x01;
pub const EMMC_TRNS_BLK_CNT_EN: u16 = 0x02;
pub const EMMC_TRNS_AUTO_CMD12: u16 = 0x04;
pub const EMMC_TRNS_AUTO_CMD23: u16 = 0x08;
pub const EMMC_TRNS_AUTO_SEL: u32 = 0x0C;
pub const EMMC_TRNS_READ: u16 = 0x10;
pub const EMMC_TRNS_MULTI: u16 = 0x20;

// EMMC present state flags
pub const EMMC_DATA_INHIBIT: u32 = 0x00000001;
pub const EMMC_CMD_INHIBIT: u32 = 0x00000002;
pub const EMMC_CARD_INSERTED: u32 = 0x00010000;
pub const EMMC_WRITE_PROTECT: u32 = 0x00080000;

// EMMC host control flags
pub const EMMC_CTRL_LED: u8 = 0x01;
pub const EMMC_CTRL_4BITBUS: u8 = 0x02;
pub const EMMC_CTRL_HISPD: u8 = 0x04;
pub const EMMC_CTRL_DMA_MASK: u8 = 0x18;
pub const EMMC_CTRL_SDMA: u8 = 0x00;
pub const EMMC_CTRL_ADMA1: u8 = 0x08;
pub const EMMC_CTRL_ADMA32: u8 = 0x10;
pub const EMMC_CTRL_ADMA64: u8 = 0x18;
pub const EMMC_CTRL_8BITBUS: u8 = 0x20;

// EMMC clock control flags
pub const EMMC_CLOCK_INT_EN: u16 = 0x0001;
pub const EMMC_CLOCK_INT_STABLE: u16 = 0x0002;
pub const EMMC_CLOCK_CARD_EN: u16 = 0x0004;
pub const EMMC_CLOCK_DIV_SHIFT: u8 = 8;

// EMMC reset flags
pub const EMMC_RESET_ALL: u8 = 0x01;
pub const EMMC_RESET_CMD: u8 = 0x02;
pub const EMMC_RESET_DATA: u8 = 0x04;

// EMMC interrupt flags
pub const EMMC_INT_RESPONSE: u32 = 0x00000001;
pub const EMMC_INT_DATA_END: u32 = 0x00000002;
pub const EMMC_INT_BLK_GAP: u32 = 0x00000004;
pub const EMMC_INT_DMA_END: u32 = 0x00000008;
pub const EMMC_INT_SPACE_AVAIL: u32 = 0x00000010;
pub const EMMC_INT_DATA_AVAIL: u32 = 0x00000020;
pub const EMMC_INT_CARD_INSERT: u32 = 0x00000040;
pub const EMMC_INT_CARD_REMOVE: u32 = 0x00000080;
pub const EMMC_INT_CARD_INT: u32 = 0x00000100;
pub const EMMC_INT_RETUNE: u32 = 0x00001000;
pub const EMMC_INT_CQE	: u32 = 0x00004000;
pub const EMMC_INT_ERROR: u32 = 0x00008000;
pub const EMMC_INT_TIMEOUT: u32 = 0x00010000;
pub const EMMC_INT_CRC: u32 = 0x00020000;
pub const EMMC_INT_END_BIT: u32 = 0x00040000;
pub const EMMC_INT_INDEX: u32 = 0x00080000;
pub const EMMC_INT_DATA_TIMEOUT: u32 = 0x00100000;
pub const EMMC_INT_DATA_CRC: u32 = 0x00200000;
pub const EMMC_INT_DATA_END_BIT: u32 = 0x00400000;
pub const EMMC_INT_BUS_POWER: u32 = 0x00800000;
const EMMC_INT_AUTO_CMD_ERR: u32 = 0x01000000;
const EMMC_INT_ADMA_ERROR: u32 = 0x02000000;

pub const EMMC_INT_NORMAL_MASK: u32 = 0x00007FFF;
pub const EMMC_INT_ERROR_MASK: u32 = 0xFFFF8000;

pub const EMMC_INT_CMD_MASK: u32 = EMMC_INT_RESPONSE | EMMC_INT_TIMEOUT | EMMC_INT_CRC | EMMC_INT_END_BIT | EMMC_INT_INDEX | EMMC_INT_AUTO_CMD_ERR;
pub const EMMC_INT_DATA_MASK: u32 = EMMC_INT_DATA_END | EMMC_INT_DATA_TIMEOUT | EMMC_INT_DATA_CRC | EMMC_INT_DATA_END_BIT | EMMC_INT_ADMA_ERROR;
pub const EMMC_INT_ALL_MASK: u32 = EMMC_INT_CMD_MASK | EMMC_INT_DATA_MASK;

// SD/MMC Command definitions
// Basic commands (class 0 and class 1)
pub const MMC_GO_IDLE_STATE: u8 = 0;
pub const MMC_SEND_OP_COND: u8 = 1;
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
pub const MMC_BUSTEST_R: u8 = 14;
pub const MMC_GO_INACTIVE_STATE: u8 = 15;
pub const MMC_BUSTEST_W: u8 = 19;

// Block-oriented read commands (class 2) 
pub const MMC_SET_BLOCKLEN: u8 = 16;
pub const MMC_READ_SINGLE_BLOCK: u8 = 17;
pub const MMC_READ_MULTIPLE_BLOCK: u8 = 18;
pub const MMC_SEND_TUNING_BLOCK: u8 = 21;

// Block-oriented write commands (class 4)
pub const MMC_SET_BLOCK_COUNT: u8 = 23;
pub const MMC_WRITE_BLOCK: u8 = 24;
pub const MMC_WRITE_MULTIPLE_BLOCK: u8 = 25;
pub const MMC_PROGRAM_CID: u8 = 26;
pub const MMC_PROGRAM_CSD: u8 = 27;
pub const MMC_SET_TIME: u8 = 49;

// Block-oriented write protection commands (class 6)
pub const MMC_SET_WRITE_PROT: u8 = 28;
pub const MMC_CLR_WRITE_PROT: u8 = 29;
pub const MMC_SEND_WRITE_PROT: u8 = 30;
pub const MMC_SEND_WRITE_PROT_TYPE: u8 = 31;

// Erase commands (class 5) 
pub const MMC_EARSE_GROUP_START: u8 = 35;
pub const MMC_EARSE_GROUP_END: u8 = 36;
pub const MMC_ERASE: u8 = 38;

// Table 55 — I/O mode commands (class 9) 
pub const MMC_FAST_IO: u8 = 39;
pub const MMC_GO_IRQ_STATE: u8 = 40;

// Lock Device commands (class 7)
pub const MMC_LOCK_UNLOCK: u8 = 42;

// Application-specific commands (class 8) 
pub const MMC_APP_CMD: u8 = 55;
pub const MMC_GEN_CMD: u8 = 56;

// Security Protocols (class 10)
pub const MMC_PROTOCOL_RD: u8 = 53;
pub const MMC_PROTOCOL_WR: u8 = 54;

// Command Queue (Class 11)
pub const MMC_QUEUED_TASK_PARAMS: u8 = 44;
pub const MMC_QUEUED_TASK_ADDRESS: u8 = 45;
pub const MMC_EXECUTE_READ_TASK: u8 = 46;
pub const MMC_EXECUTE_WRITE_TASK: u8 = 47;
pub const MMC_CMDQ_TASK_MGMT: u8 = 48;

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

pub const EMMC_CAN_DO_8BIT: u32 = 0x00040000; // 支持8位数据总线位掩码

pub const EXT_CSD_BUS_WIDTH: u8 = 183;      // 总线宽度索引
pub const EXT_CSD_HS_TIMING: u8 = 185;      // 高速时序索引

pub const EXT_CSD_BUS_WIDTH_1: u8 = 0;      // 1位模式
pub const EXT_CSD_BUS_WIDTH_4: u8 = 1;      // 4位模式
pub const EXT_CSD_BUS_WIDTH_8: u8 = 2;      // 8位模式
