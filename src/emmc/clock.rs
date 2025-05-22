use spin::Once;

#[derive(Debug, Clone, Copy)]
pub enum ClkError {
    InvalidClockRate,
    RegisterOperationFailed,
    InvalidPeripheralId,
    ResetTimeout,
    NotInitialized,
}

pub trait Clk {
    fn emmc_get_clk(&self) -> Result<u64, ClkError>;
    fn emmc_set_clk(&self, rate: u64) -> Result<u64, ClkError>;
}

static INIT: Once = Once::new();
static mut GLOBAL_CLK_INSTANCE: Option<&'static dyn Clk> = None;

pub fn init_global_clk(clk: &'static dyn Clk) {
    INIT.call_once(|| {
        unsafe {
            GLOBAL_CLK_INSTANCE = Some(clk);
        }
    });
}

pub fn global_clk() -> Result<&'static dyn Clk, ClkError> {
    unsafe {
        match GLOBAL_CLK_INSTANCE {
            Some(instance) => Ok(instance),
            None => Err(ClkError::NotInitialized),
        }
    }
}

pub fn emmc_set_clk(rate: u64) -> Result<u64, ClkError> {
    global_clk()?.emmc_set_clk(rate)
}

pub fn emmc_get_clk() -> Result<u64, ClkError> {
    global_clk()?.emmc_get_clk()
}