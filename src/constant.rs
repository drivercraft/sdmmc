use bitflags::bitflags;

pub static ARCH_DMA_MINALIGN: u8 = 64;

/* Registers to support IDMAC 32-bit address mode */
pub const DWMCI_DBADDR : u32 = 0x088;
pub const DWMCI_IDSTS : u32 = 0x08c;
pub const DWMCI_IDINTEN : u32 = 0x090;
pub const DWMCI_DSCADDR : u32 = 0x094;
pub const DWMCI_BUFADDR : u32 = 0x098;
/* Registers to support IDMAC 64-bit address mode */
pub const  DWMCI_DBADDRL: u32 = 0x088;
pub const  DWMCI_DBADDRU: u32 = 0x08c;
pub const  DWMCI_IDSTS64: u32 = 0x090;
pub const  DWMCI_IDINTEN64: u32 = 0x094;
pub const  DWMCI_DSCADDRL: u32 = 0x098;
pub const  DWMCI_DSCADDRU: u32 = 0x09c;
pub const  DWMCI_BUFADDRL: u32 = 0x0a0;
pub const  DWMCI_BUFADDRU: u32 = 0x0a4;