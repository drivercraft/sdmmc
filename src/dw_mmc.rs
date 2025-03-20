use crate::constant::{DWMCI_BUFADDR, DWMCI_BUFADDRL, DWMCI_BUFADDRU, DWMCI_DBADDR, DWMCI_DBADDRL, DWMCI_DBADDRU, DWMCI_DSCADDR, DWMCI_DSCADDRL, DWMCI_DSCADDRU, DWMCI_IDINTEN, DWMCI_IDINTEN64, DWMCI_IDSTS, DWMCI_IDSTS64};

pub const DWMCIIDMACREGS32: DwmciIdmacRegs = DwmciIdmacRegs {
    dbaddrl: Some(DWMCI_DBADDR),
    dbaddru: None,
    idsts: Some(DWMCI_IDSTS),
    idinten: Some(DWMCI_IDINTEN),
    dscaddrl: Some(DWMCI_DSCADDR),
    dscaddru: None,
    bufaddrl: Some(DWMCI_BUFADDR),
    bufaddru: None,
};

pub const DWMCIIDMACREGS64: DwmciIdmacRegs = DwmciIdmacRegs {
    dbaddrl: Some(DWMCI_DBADDRL),
    dbaddru: Some(DWMCI_DBADDRU),
    idsts: Some(DWMCI_IDSTS64),
    idinten: Some(DWMCI_IDINTEN64),
    dscaddrl: Some(DWMCI_DSCADDRL),
    dscaddru: Some(DWMCI_DSCADDRU),
    bufaddrl: Some(DWMCI_BUFADDRL),
    bufaddru: Some(DWMCI_BUFADDRU),
};

// Internal DMA Controller (IDMAC) descriptor for 32-bit addressing mode
#[repr(align(64))] // Replace 64 with the actual value of ARCH_DMA_MINALIGN
struct DwmciIdmac32 {
    des0: u32,
    des1: u32,
    des2: u32,
    des3: u32,
}

// Internal DMA Controller (IDMAC) descriptor for 64-bit addressing mode
#[repr(align(64))]
struct DwmciIdmac64 {
    des0: u32,
    des1: u32,
    des2: u32,
    des3: u32,
    des4: u32,
    des5: u32,
    des6: u32,
    des7: u32,
}

/**
 * struct dwmci_idmac_regs - Offsets of IDMAC registers
 *
 * @dbaddrl:	Descriptor base address, lower 32 bits
 * @dbaddru:	Descriptor base address, upper 32 bits
 * @idsts:	Internal DMA status
 * @idinten:	Internal DMA interrupt enable
 * @dscaddrl:	IDMAC descriptor address, lower 32 bits
 * @dscaddru:	IDMAC descriptor address, upper 32 bits
 * @bufaddrl:	Current data buffer address, lower 32 bits
 * @bufaddru:	Current data buffer address, upper 32 bits
 */
struct DwmciIdmacRegs {
    dbaddrl: Option<u32>,
    dbaddru: Option<u32>,
    idsts: Option<u32>,
    idinten: Option<u32>,
    dscaddrl: Option<u32>,
	dscaddru: Option<u32>, 
	bufaddrl: Option<u32>, 
	bufaddru: Option<u32>, 
}

fn dwmci_wait_reset() -> i32 {
    todo!()
}

fn dwmci_sert_idma_desc32() {
    todo!()
}

fn dwmci_sert_idma_desc64() {
    todo!()
}

fn dwmci_prepare_desc() {
    todo!()
}

fn dwmci_prepare_data() {
    todo!()
}

fn dwmci_fifo_ready() -> i32 {
    todo!()
}

fn dwmci_get_timeout () -> i32 {
    todo!()
}

fn dwmci_data_transfer() -> i32 {
    todo!()
}

fn dwmci_dma_transfer() ->i32 {
    todo!()
}

fn dwmci_set_transfer_mode() -> i32 {
    todo!()
}

fn dwmci_wait_while_busy() -> i32 {
    todo!()
}

fn dwmci_send_cmd_common() -> i32 {
    todo!()
}

// #ifdef CONFIG_DM_MMC
// static int dwmci_send_cmd(struct udevice *dev, struct mmc_cmd *cmd,
// 			  struct mmc_data *data)
// {
// 	struct mmc *mmc = mmc_get_mmc_dev(dev);
// #else
// static int dwmci_send_cmd(struct mmc *mmc, struct mmc_cmd *cmd,
// 			  struct mmc_data *data)
// {
// #endif
// 	struct dwmci_host *host = mmc->priv;
// 	const size_t buf_size = data ? DIV_ROUND_UP(data->blocks, 8) : 0;

// 	if (host->dma_64bit_address) {
// 		ALLOC_CACHE_ALIGN_BUFFER(struct dwmci_idmac64, idmac, buf_size);
// 		return dwmci_send_cmd_common(host, cmd, data, idmac);
// 	} else {
// 		ALLOC_CACHE_ALIGN_BUFFER(struct dwmci_idmac32, idmac, buf_size);
// 		return dwmci_send_cmd_common(host, cmd, data, idmac);
// 	}
// }


fn dwmci_control_clken() {
    todo!()
}

fn dwmci_update_div () -> i32 {
    todo!()
}

fn dwmci_setup_bus() -> i32 {
    todo!()
}

// #ifdef CONFIG_DM_MMC
// static int dwmci_set_ios(struct udevice *dev)
// {
// 	struct mmc *mmc = mmc_get_mmc_dev(dev);
// #else
// static int dwmci_set_ios(struct mmc *mmc)
// {
// #endif
// 	struct dwmci_host *host = (struct dwmci_host *)mmc->priv;
// 	u32 ctype, regs;

// 	debug("Bus width = %d, clock: %d\n", mmc->bus_width, mmc->clock);

// 	dwmci_setup_bus(host, mmc->clock);
// 	switch (mmc->bus_width) {
// 	case 8:
// 		ctype = DWMCI_CTYPE_8BIT;
// 		break;
// 	case 4:
// 		ctype = DWMCI_CTYPE_4BIT;
// 		break;
// 	default:
// 		ctype = DWMCI_CTYPE_1BIT;
// 		break;
// 	}

// 	dwmci_writel(host, DWMCI_CTYPE, ctype);

// 	regs = dwmci_readl(host, DWMCI_UHS_REG);
// 	if (mmc->ddr_mode)
// 		regs |= DWMCI_DDR_MODE;
// 	else
// 		regs &= ~DWMCI_DDR_MODE;

// 	dwmci_writel(host, DWMCI_UHS_REG, regs);

// 	if (host->clksel) {
// 		int ret;

// 		ret = host->clksel(host);
// 		if (ret)
// 			return ret;
// 	}

// #if CONFIG_IS_ENABLED(DM_REGULATOR)
// 	if (mmc->vqmmc_supply) {
// 		int ret;

// 		ret = regulator_set_enable_if_allowed(mmc->vqmmc_supply, false);
// 		if (ret)
// 			return ret;

// 		if (mmc->signal_voltage == MMC_SIGNAL_VOLTAGE_180)
// 			regulator_set_value(mmc->vqmmc_supply, 1800000);
// 		else
// 			regulator_set_value(mmc->vqmmc_supply, 3300000);

// 		ret = regulator_set_enable_if_allowed(mmc->vqmmc_supply, true);
// 		if (ret)
// 			return ret;
// 	}
// #endif

// 	return 0;
// }


fn dwmci_init_fifo() {
    todo!()
}

fn dwmci_init_dma() -> i32 {
    todo!()
}

fn dwmci_init() -> i32 {
    todo!()
}

// const struct dm_mmc_ops dm_dwmci_ops = {
// 	.send_cmd	= dwmci_send_cmd,
// 	.set_ios	= dwmci_set_ios,
// };

// #else
// static const struct mmc_ops dwmci_ops = {
// 	.send_cmd	= dwmci_send_cmd,
// 	.set_ios	= dwmci_set_ios,
// 	.init		= dwmci_init,
// };
// #endif


fn dwmci_setup_cfg() {
    todo!()
}

// if CONFIG_BLK
fn dwmci_bind() -> i32 {
    todo!()
}
// else
fn add_dwmci() -> i32 {
    todo!()
}
// endif