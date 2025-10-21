# SD/MMC 驱动库

## 项目简介

SD/MMC 驱动库是一个专为 ARM64 平台设计的 Rust SD/MMC 控制器驱动库，支持 eMMC、SD 和 SDIO 设备。该库提供了完整的存储控制器功能，包括命令发送、时钟配置、块读写操作等。

本项目采用 `no_std` 设计，完全适用于裸机和嵌入式环境，特别针对 U-Boot 引导加载程序环境进行了优化。通过类型安全的寄存器访问，确保了硬件操作的可靠性和安全性。

## 功能特性

- **完整的 MMC/eMMC 支持**: 支持 eMMC 4.x/5.x 标准，包括高速模式、DDR 模式、HS200 和 HS400 模式
- **SD/SDIO 支持**: 支持 SD 1.0/2.0 标准和 SDIO 设备
- **多种数据传输模式**: 支持 PIO 和 DMA 两种数据传输模式
- **Rockchip 平台优化**: 针对 RK3568 平台进行了专门优化，支持 DWCMSHC 控制器
- **类型安全寄存器访问**: 基于直接内存访问提供类型安全的硬件寄存器操作
- **no_std 兼容**: 完全不依赖标准库，适用于裸机和嵌入式环境
- **ARM64 架构优化**: 专门针对 ARM64 平台进行优化
- **U-Boot 环境支持**: 在 U-Boot 引导环境下提供稳定可靠的存储访问功能

## 快速开始

### 环境要求

- Rust 2024 Edition
- ARM64 开发环境
- 支持 U-Boot 的 Rockchip RK3568 硬件平台
- ostool 工具 (用于测试)

### 安装步骤

1. 安装 `ostool` 依赖工具：

```bash
cargo install ostool
```

2. 将项目添加到 `Cargo.toml`：

```toml
[dependencies]
sdmmc = { git = "https://github.com/drivercraft/sdmmc.git" }
```

### 基本使用

```rust
use sdmmc::emmc::EMmcHost;
use core::ptr::NonNull;

// 创建 EMMC 控制器实例
let emmc_addr = 0xfe2e0000; // RK3568 EMMC 控制器基地址
let mut emmc = EMmcHost::new(emmc_addr);

// 初始化控制器和存储卡
match emmc.init() {
    Ok(_) => {
        println!("EMMC 初始化成功");
        
        // 读取存储卡信息
        match emmc.get_card_info() {
            Ok(card_info) => {
                println!("卡类型: {:?}", card_info.card_type);
                println!("容量: {} MB", card_info.capacity_bytes / (1024 * 1024));
            }
            Err(e) => println!("获取卡信息失败: {:?}", e),
        }
        
        // 读取数据块
        let mut buffer: [u8; 512] = [0; 512];
        match emmc.read_blocks(0, 1, &mut buffer) {
            Ok(_) => println!("读取数据块成功"),
            Err(e) => println!("读取数据块失败: {:?}", e),
        }
    }
    Err(e) => println!("EMMC 初始化失败: {:?}", e),
}
```

## 项目结构

```
src/
├── lib.rs              # 主入口和核心功能
├── err.rs              # 错误类型定义
├── emmc/
│   ├── mod.rs          # EMMC 模块主文件
│   ├── cmd.rs          # 命令发送和响应处理
│   ├── block.rs        # 块读写操作
│   ├── regs.rs         # 寄存器访问接口
│   ├── constant.rs     # 硬件常量定义
│   ├── clock.rs        # 时钟控制接口
│   ├── rockchip.rs     # Rockchip 平台特定实现
│   ├── config.rs       # 平台配置
│   ├── aux.rs          # 辅助函数
│   └── info.rs         # 卡信息处理
└── 

tests/
└── test.rs             # 集成测试，包含 EMMC 功能测试
```

## API 文档

### 核心结构体

- **`EMmcHost`**: 主要的 EMMC 控制器接口结构体，提供所有存储控制功能
- **`EMmcCard`**: 存储卡信息结构体，包含卡的详细信息

### 主要接口

#### EMMC 控制器管理

- `EMmcHost::new(addr)`: 创建新的 EMMC 控制器实例
- `EMmcHost::init()`: 初始化 EMMC 控制器和存储卡
- `EMmcHost::get_card_info()`: 获取存储卡信息
- `EMmcHost::get_status()`: 获取控制器状态

#### 数据读写操作

- `EMmcHost::read_blocks(block_id, blocks, buffer)`: 读取数据块
- `EMmcHost::write_blocks(block_id, blocks, buffer)`: 写入数据块

#### 时钟和总线控制

- `EMmcHost::mmc_set_clock(freq)`: 设置时钟频率
- `EMmcHost::mmc_set_bus_width(width)`: 设置总线宽度
- `EMmcHost::mmc_set_timing(timing)`: 设置时序模式

### 支持的传输模式

- **PIO 模式**: 默认启用，适用于小数据量传输
- **DMA 模式**: 通过 `dma` feature 启用，适用于大数据量传输

## 使用示例

### EMMC 初始化示例

```rust
use sdmmc::emmc::EMmcHost;
use core::ptr::NonNull;

fn init_emmc_controller(emmc_addr: usize) -> Result<(), &'static str> {
    // 创建 EMMC 控制器实例
    let mut emmc = EMmcHost::new(emmc_addr);
    
    // 初始化控制器
    match emmc.init() {
        Ok(_) => {
            println!("EMMC 控制器初始化成功");
            
            // 获取卡信息
            match emmc.get_card_info() {
                Ok(card_info) => {
                    println!("卡类型: {:?}", card_info.card_type);
                    println!("制造商 ID: 0x{:02X}", card_info.manufacturer_id);
                    println!("容量: {} MB", card_info.capacity_bytes / (1024 * 1024));
                    println!("块大小: {} 字节", card_info.block_size);
                }
                Err(e) => {
                    println!("获取卡信息失败: {:?}", e);
                    return Err("获取卡信息失败");
                }
            }
            
            Ok(())
        }
        Err(e) => {
            println!("EMMC 控制器初始化失败: {:?}", e);
            Err("控制器初始化失败")
        }
    }
}
```

### 数据读写示例

```rust
use sdmmc::emmc::EMmcHost;

fn read_write_test(emmc: &mut EMmcHost) -> Result<(), &'static str> {
    // 读取第一个数据块
    let mut read_buffer: [u8; 512] = [0; 512];
    match emmc.read_blocks(0, 1, &mut read_buffer) {
        Ok(_) => {
            println!("读取数据块成功");
            println!("前 16 字节: {:02X?}", &read_buffer[0..16]);
        }
        Err(e) => {
            println!("读取数据块失败: {:?}", e);
            return Err("读取失败");
        }
    }
    
    // 写入测试数据到第三个数据块
    let mut write_buffer: [u8; 512] = [0; 512];
    // 填充测试数据
    for i in 0..512 {
        write_buffer[i] = (i % 256) as u8;
    }
    
    match emmc.write_blocks(2, 1, &write_buffer) {
        Ok(_) => println!("写入数据块成功"),
        Err(e) => {
            println!("写入数据块失败: {:?}", e);
            return Err("写入失败");
        }
    }
    
    // 读回验证
    let mut verify_buffer: [u8; 512] = [0; 512];
    match emmc.read_blocks(2, 1, &mut verify_buffer) {
        Ok(_) => {
            // 验证数据一致性
            let mut data_match = true;
            for i in 0..512 {
                if write_buffer[i] != verify_buffer[i] {
                    data_match = false;
                    break;
                }
            }
            
            if data_match {
                println!("数据验证成功");
            } else {
                println!("数据验证失败");
                return Err("数据验证失败");
            }
        }
        Err(e) => {
            println!("验证读取失败: {:?}", e);
            return Err("验证失败");
        }
    }
    
    Ok(())
}
```

### 完整使用示例

```rust
use sdmmc::emmc::EMmcHost;
use core::ptr::NonNull;

fn main() -> Result<(), &'static str> {
    // EMMC 控制器基地址 (RK3568)
    let emmc_addr = 0xfe2e0000;
    
    // 创建控制器实例
    let mut emmc = EMmcHost::new(emmc_addr);
    
    // 初始化控制器
    println!("初始化 EMMC 控制器...");
    if let Err(e) = emmc.init() {
        println!("EMMC 控制器初始化失败: {:?}", e);
        return Err("初始化失败");
    }
    
    // 获取卡信息
    println!("获取存储卡信息...");
    match emmc.get_card_info() {
        Ok(card_info) => {
            println!("卡类型: {:?}", card_info.card_type);
            println!("制造商 ID: 0x{:02X}", card_info.manufacturer_id);
            println!("容量: {} MB", card_info.capacity_bytes / (1024 * 1024));
            println!("块大小: {} 字节", card_info.block_size);
        }
        Err(e) => {
            println!("获取卡信息失败: {:?}", e);
            return Err("获取卡信息失败");
        }
    }
    
    // 执行读写测试
    println!("执行读写测试...");
    if let Err(e) = read_write_test(&mut emmc) {
        println!("读写测试失败: {}", e);
        return Err(e);
    }
    
    println!("所有测试完成");
    Ok(())
}

fn read_write_test(emmc: &mut EMmcHost) -> Result<(), &'static str> {
    // 读取第一个数据块
    let mut read_buffer: [u8; 512] = [0; 512];
    match emmc.read_blocks(0, 1, &mut read_buffer) {
        Ok(_) => {
            println!("读取数据块成功");
            println!("前 16 字节: {:02X?}", &read_buffer[0..16]);
        }
        Err(e) => {
            println!("读取数据块失败: {:?}", e);
            return Err("读取失败");
        }
    }
    
    // 写入测试数据到第三个数据块
    let mut write_buffer: [u8; 512] = [0; 512];
    // 填充测试数据
    for i in 0..512 {
        write_buffer[i] = (i % 256) as u8;
    }
    
    match emmc.write_blocks(2, 1, &write_buffer) {
        Ok(_) => println!("写入数据块成功"),
        Err(e) => {
            println!("写入数据块失败: {:?}", e);
            return Err("写入失败");
        }
    }
    
    // 读回验证
    let mut verify_buffer: [u8; 512] = [0; 512];
    match emmc.read_blocks(2, 1, &mut verify_buffer) {
        Ok(_) => {
            // 验证数据一致性
            let mut data_match = true;
            for i in 0..512 {
                if write_buffer[i] != verify_buffer[i] {
                    data_match = false;
                    break;
                }
            }
            
            if data_match {
                println!("数据验证成功");
            } else {
                println!("数据验证失败");
                return Err("数据验证失败");
            }
        }
        Err(e) => {
            println!("验证读取失败: {:?}", e);
            return Err("验证失败");
        }
    }
    
    Ok(())
}
```

## 测试结果

### 运行测试

#### 带U-Boot环境的硬件测试

```bash
# 带uboot的开发板测试
make uboot
```

### 测试输出示例

```
     _____                                         __
    / ___/ ____   ____ _ _____ _____ ___   ____ _ / /
    \__ \ / __ \ / __ `// ___// ___// _ \ / __ `// / 
   ___/ // /_/ // /_/ // /   / /   /  __// /_/ // /  
  /____// .___/ \__,_//_/   /_/    \___/ \__,_//_/   
/_/                                           

Version                       : 0.12.2
Platfrom                      : RK3588 OPi 5 Plus
Start CPU                     : 0x0
FDT                           : 0xffff900000f29000
🐛 0.000ns    [sparreal_kernel::driver:16] add registers
🐛 0.000ns    [rdrive::probe::fdt:168] Probe [interrupt-controller@fe600000]->[GICv3]
🐛 0.000ns    [somehal::arch::mem::mmu:181] Map `iomap       `: RW- | [0xffff9000fe600000, 0xffff9000fe610000) -> [0xfe600000, 0xfe610000)
🐛 0.000ns    [somehal::arch::mem::mmu:181] Map `iomap       `: RW- | [0xffff9000fe680000, 0xffff9000fe780000) -> [0xfe680000, 0xfe780000)
🐛 0.000ns    [rdrive::probe::fdt:168] Probe [timer]->[ARMv8 Timer]
🐛 0.000ns    [sparreal_rt::arch::timer:78] ARMv8 Timer IRQ: IrqConfig { irq: 0x1e, trigger: LevelHigh, is_private: true }
🐛 0.000ns    [rdrive::probe::fdt:168] Probe [psci]->[ARM PSCI]
🐛 0.000ns    [spar:power:76] PCSI [Smc]
🐛 0.000ns    [sparreal_kernel::irq:39] [GICv3](405) open
🔍 0.000ns    [arm_gic_driver::version::v3:342] Initializing GICv3 Distributor@0xffff9000fe600000, security state: NonSecure...
🔍 0.000ns    [arm_gic_driver::version::v3:356] GICv3 Distributor disabled
🔍 0.000ns    [arm_gic_driver::version::v3:865] CPU interface initialization for CPU: 0x0
🔍 0.000ns    [arm_gic_driver::version::v3:921] CPU interface initialized successfully
🐛 0.000ns    [sparreal_kernel::irq:64] [GICv3](405) init cpu: CPUHardId(0)
🐛 0.000ns    [sparreal_rt::arch::timer:30] ARMv8 Timer: Enabled
🐛 17.681s    [sparreal_kernel::irq:136] Enable irq 0x1e on chip 405
🐛 17.681s    [sparreal_kernel::hal_al::run:33] Driver initialized
🐛 18.304s    [rdrive:132] probe pci devices
begin test
Run test: test_platform
💡 18.358s    [test::tests:243] Found node: mmc@fe2e0000
💡 18.359s    [test::tests:248💡 18.390s    [test::tests:243] Found node: clock-controller@fd7c0000
💡 18.390s    [teests:48] clk ptr: 0xffff9000fd7c0000
💡 18.395s    [test::tests:53] emmc addr: 0xffff9000fe2e0000
💡 18.396s    [test::tests:54] clk addr: 0xffff9000fd7c0000
💡 18.397s    [sdmmc::emmc:74] EMMC Controller created: EMMC Controller { base_addr: 0xffff9000fe2e0000, card: None, caps: 0x226dc881, clock_base: 200000000 }
💡 18.398s    [sdmmc::emmc:91] Init EMMC Controller
🐛 18.399s    [sdmmc::emmc:100] Card inserted: true
💡 18.399s    [sdmmc::emmc:105] EMMC Version: 0x5
💡 18.400s    [sdmmc::emmc:108] EMMC Capabilities 1: 0b100010011011011100100010000001
💡 18.401s    [sdmmc::emmc:114] EMMC Capabilities 2: 0b1000000000000000000000000111
💡 18.402s    [sdmmc::emmc:162] voltage range: 0x60000, 0x12
💡 18.402s    [sdmmc::emmc::rockchip:145] EMMC Power Control: 0xd
🐛 18.413s    [sdmmc::emmc:974] Bus width set to 1
🐛 18.414s    [sdmmc::emmc::rockchip:318] card_clock: 0, bus_width: 1, timing: 0
💡 18.415s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x0
🐛 18.415s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.416s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.417s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x0
🐛 18.417s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x0
🐛 18.418s    [sdmmc::emmc::rockchip:318] card_clock: 400000, bus_width: 1, timing: 0
🐛 18.419s    [rk3588_clk:111] Setting clk_id 314 to rate 400000
🐛 18.420s    [rk3588_clk:152] CCLK_EMMC: src_clk 2, div 60, new_value 0xbb00, final_value 0xff00bb00
🐛 18.421s    [rk3588_clk:73] Getting clk_id 314
💡 18.421s    [sdmmc::emmc::rockchip:32] input_clk: 400000
💡 18.422s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.423s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x0
🐛 18.423s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.424s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x2
🐛 18.425s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.426s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.426s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x0
🐛 18.427s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x0
🐛 18.428s    [sdmmc::emmc::rockchip:318] card_clock: 400000, bus_width: 1, timing: 0
🐛 18.428s    [rk3588_clk:111] Setting clk_id 314 to rate 400000
🐛 18.429s    [rk3588_clk:152] CCLK_EMMC: src_clk 2, div 60, new_value 0xbb00, final_value 0xff00bb00
🐛 18.430s    [rk3588_clk:73] Getting clk_id 314
💡 18.431s    [sdmmc::emmc::rockchip:32] input_clk: 400000
💡 18.431s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.432s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x0
🐛 18.433s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.434s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x2
🐛 18.434s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.435s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.436s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x0
🐛 18.436s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x0
💡 18.437s    [sdmmc::emmc:226] eMMC initialization started
🔍 18.438s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x0, arg=0x0, resp_type=0x0, command=0x0
🔍 18.439s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.440s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.440s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.453s    [sdmmc::emmc::cmd:416] eMMC reset complete
🔍 18.454s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x0, resp_type=0x1, command=0x102
🔍 18.455s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.455s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.456s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.469s    [sdmmc::emmc::cmd:431] eMMC first CMD1 response (no args): 0xff8080
🔍 18.470s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x40060000, resp_type=0x1, command=0x102
🔍 18.471s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.472s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.472s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.475s    [sdmmc::emmc::cmd:453] CMD1 response raw: 0xff8080
💡 18.476s    [sdmmc::emmc::cmd:454] eMMC CMD1 response: 0xff8080
🔍 18.477s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x40060000, resp_type=0x1, command=0x102
🔍 18.479s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.479s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.480s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.483s    [sdmmc::emmc::cmd:453] CMD1 response raw: 0xff8080
💡 18.484s    [sdmmc::emmc::cmd:454] eMMC CMD1 response: 0xff8080
🔍 18.485s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x40060000, resp_type=0x1, command=0x102
🔍 18.486s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.487s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.488s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.491s    [sdmmc::emmc::cmd:453] CMD1 response raw: 0xff8080
💡 18.491s    [sdmmc::emmc::cmd:454] eMMC CMD1 response: 0xff8080
🔍 18.493s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x40060000, resp_type=0x1, command=0x102
🔍 18.494s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.495s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.496s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.498s    [sdmmc::emmc::cmd:453] CMD1 response raw: 0xff8080
💡 18.499s    [sdmmc::emmc::cmd:454] eMMC CMD1 response: 0xff8080
🔍 18.501s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x40060000, resp_type=0x1, command=0x102
🔍 18.502s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.503s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.503s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.506s    [sdmmc::emmc::cmd:453] CMD1 response raw: 0xff8080
💡 18.507s    [sdmmc::emmc::cmd:454] eMMC CMD1 response: 0xff8080
🔍 18.508s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x1, arg=0x40060000, resp_type=0x1, command=0x102
🔍 18.510s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.510s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.511s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.514s    [sdmmc::emmc::cmd:453] CMD1 response raw: 0xc0ff8080
💡 18.514s    [sdmmc::emmc::cmd:454] eMMC CMD1 response: 0xc0ff8080
💡 18.515s    [sdmmc::emmc::cmd:478] eMMC initialization status: true
🐛 18.517s    [sdmmc::emmc::cmd:486] Clock control before CMD2: 0x7, stable: true
🔍 18.518s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x2, arg=0x0, resp_type=0x7, command=0x209
🔍 18.519s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.520s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.520s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.523s    [sdmmc::emmc::cmd:69] eMMC response: 0x45010044 0x56343033 0x3201bb29 0x7a017c00
🔍 18.524s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x3, arg=0x10000, resp_type=0x15, command=0x31a
🔍 18.525s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.526s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.527s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🔍 18.529s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x9, arg=0x10000, resp_type=0x7, command=0x909
🔍 18.530s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.531s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.532s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
💡 18.535s    [sdmmc::emmc::cmd:69] eMMC response: 0xd00f0032 0x8f5903ff 0xffffffef 0x8a404000
🐛 18.536s    [sdmmc::emmc:256] eMMC CSD version: 4
🔍 18.536s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x7, arg=0x10000, resp_type=0x15, command=0x71a
🔍 18.537s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.538s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.539s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🐛 18.541s    [sdmmc::emmc:327] cmd7: 0x700
🔍 18.542s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x6, arg=0x3b90100, resp_type=0x1d, command=0x61b
🔍 18.543s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.544s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.545s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🐛 18.547s    [sdmmc::emmc:1010] cmd6 0x800
🔍 18.548s    [sdmmc::emmc::cmd:244] Sending command: opcode=0xd, arg=0x10000, resp_type=0x15, command=0xd1a
🔍 18.549s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.550s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.550s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🔍 18.553s    [sdmmc::emmc::cmd:583] cmd_d 0x900
🐛 18.554s    [sdmmc::emmc::rockchip:318] card_clock: 400000, bus_width: 1, timing: 1
🐛 18.555s    [rk3588_clk:111] Setting clk_id 314 to rate 400000
🐛 18.555s    [rk3588_clk:152] CCLK_EMMC: src_clk 2, div 60, new_value 0xbb00, final_value 0xff00bb00
🐛 18.556s    [rk3588_clk:73] Getting clk_id 314
💡 18.557s    [sdmmc::emmc::rockchip:32] input_clk: 400000
💡 18.558s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.558s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x0
🐛 18.559s    [sdmmc::emmc::rockchip:106] EMMC Clocckchip:106] EMMC Clock Control: 0x7
💡 18.561s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.562s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x4
🐛 18.563s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x2
🐛 18.563s    [sdmmc::emmc::rockchip:318] card_clock: 52000000, bus_width: 1, timing: 1
🐛 18.564s    [rk3588_clk:111] Setting clk_id 314 to rate 52000000
🐛 18.565s    [rk3588_clk:152] CCLK_EMMC: src_clk 1, div 23, new_value 0x5600, final_value 0xff005600
🐛 18.566s    [rk3588_clk:73] Getting clk_id 314
💡 18.567s    [sdmmc::emmc::rockchip:32] input_clk: 65217391
💡 18.567s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.568s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x1
🐛 18.569s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x107
💡 18.569s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x2
🐛 18.570s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.571s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.571s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x4
🐛 18.572s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x2
🔍 18.573s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x8, arg=0x0, resp_type=0x15, command=0x83a
🔍 18.574s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.575s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.575s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🔍 18.576s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
🔍 18.577s    [sdmmc::emmc:354] EXT_CSD: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 3, 0, 144, 23, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 146, 4, 0, 7, 0, 0, 2, 0, 0, 21, 31, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 13, 0, 0, 0, 0, 8, 0, 2, 0, 87, 31, 10, 3, 221, 221, 0, 0, 0, 10, 10, 10, 10, 10, 10, 1, 0, 224, 163, 3, 23, 19, 23, 7, 7, 16, 1, 3, 1, 8, 32, 0, 7, 166, 166, 85, 3, 0, 0, 0, 0, 221, 221, 0, 1, 255, 0, 0, 0, 0, 1, 25, 25, 0, 16, 0, 0, 221, 82, 67, 51, 48, 66, 48, 48, 55, 81, 80, 8, 8, 8, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 31, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 16, 0, 3, 3, 0, 5, 3, 3, 1, 63, 63, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]
🐛 18.591s    [sdmmc::emmc:412] Boot partition size: 0x400000
🐛 18.591s    [sdmmc::emmc:413] RPMB partition size: 0x1000000
🐛 18.592s    [sdmmc::emmc:434] GP partition sizes: [0, 0, 0, 0]
🔍 18.593s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x8, arg=0x0, resp_type=0x15, command=0x83a
🔍 18.594s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.595s    [sdmmc::emmc::cmd:263] Response Status: 0b100001
🔍 18.595s    [sdmmc::emmc::cmd:288] Command completed: status=0b100001
🔍 18.596s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
🔍 18.597s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x8, arg=0x0, resp_type=0x15, command=0x83a
🔍 18.598s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.599s    [sdmmc::emmc::cmd:263] Response Status: 0b100001
🔍 18.599s    [sdmmc::emmc::cmd:288] Command completed: status=0b100001
🔍 18.600s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
🔍 18.601s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x6, arg=0x3b70200, resp_type=0x1d, command=0x61b
🔍 18.602s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.603s    [sdmmc::emmc::cmd:263] Response Status: 0b11
🔍 18.604s    [sdmmc::emmc::cmd:288] Command completed: status=0b11
🐛 18.604s    [sdmmc::emmc:1010] cmd6 0x800
🔍 18.605s    [sdmmc::emmc::cmd:244] Sending command: opcode=0xd, arg=0x10000, resp_type=0x15, command=0xd1a
🔍 18.606s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.607s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.608s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🔍 18.608s    [sdmmc::emmc::cmd:583] cmd_d 0x900
🐛 18.609s    [sdmmc::emmc:974] Bus width set to 8
🐛 18.609s    [sdmmc::emmc::rockchip:318] card_clock: 52000000, bus_width: 8, timing: 1
🐛 18.610s    [rk3588_clk:111] Setting clk_id 314 to rate 52000000
🐛 18.611s    [rk3588_clk:152] CCLK_EMMC: src_clk 1, div 23, new_value 0x5600, final_value 0xff005600
🐛 18.612s    [rk3588_clk:73] Getting clk_id 314
💡 18.613s    [sdmmc::emmc::rockchip:32] input_clk: 65217391
💡 18.613s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.614s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x1
🐛 18.615s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x107
💡 18.616s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x2
🐛 18.616s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.617s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.618s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x24
🐛 18.618s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x2
🔍 18.619s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x8, arg=0x0, resp_type=0x15, command=0x83a
🔍 18.620s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.621s    [sdmmc::emmc::cmd:263] Response Status: 0b1
🔍 18.622s    [sdmmc::emmc::cmd:288] Command completed: status=0b1
🔍 18.622s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
🔍 18.623s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x6, arg=0x3b90200, resp_type=0x1d, command=0x61b
🔍 18.624s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.625s    [sdmmc::emmc::cmd:263] Response Status: 0b11
🔍 18.626s    [sdmmc::emmc::cmd:288] Command completed: status=0b11
🐛 18.626s    [sdmmc::emmc:1010] cmd6 0x800
🐛 18.628s    [sdmmc::emmc::rockchip:318] card_clock: 52000000, bus_width: 8, timing: 9
🐛 18.629s    [rk3588_clk:111] Setting clk_id 314 to rate 52000000
🐛 18.630s    [rk3588_clk:152] CCLK_EMMC: src_clk 1, div 23, new_value 0x5600, final_value 0xff005600
🐛 18.631s    [rk3588_clk:73] Getting clk_id 314
💡 18.631s    [sdmmc::emmc::rockchip:32] input_clk: 65217391
💡 18.632s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.633s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x1
🐛 18.633s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x107
💡 18.634s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x2
🐛 18.635s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.636s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.636s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x24
💡 18.637s    [sdmmc::emmc::rockchip:145] EMMC Power Control: 0xb
🐛 18.648s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x1b
🐛 18.648s    [sdmmc::emmc::rockchip:318] card_clock: 200000000, bus_width: 8, timing: 9
🐛 18.649s    [rk3588_clk:111] Setting clk_id 314 to rate 200000000
🐛 18.650s    [rk3588_clk:152] CCLK_EMMC: src_clk 1, div 6, new_value 0x4500, final_value 0xff004500
🐛 18.651s    [rk3588_clk:73] Getting clk_id 314
💡 18.652s    [sdmmc::emmc::rockchip:32] input_clk: 250000000
💡 18.652s    [sdmmc::emmc::rockchip:42] EMMC Clock Mul: 0
💡 18.653s    [sdmmc::emmc::rockchip:78] EMMC Clock Divisor: 0x1
🐛 18.654s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x107
💡 18.654s    [sdmmc::emmc::rockchip:163] EMMC Clock Control: 0x2
🐛 18.657s    [sdmmc::emmc::rockchip:106] EMMC Clock Control: 0x7
💡 18.658s    [sdmmc::emmc::rockchip:275] Clock 0x7
🐛 18.658s    [sdmmc::emmc::rockchip:353] EMMC Host Control 1: 0x24
💡 18.659s    [sdmmc::emmc::rockchip:145] EMMC Power Control: 0xb
🐛 18.670s    [sdmmc::emmc::rockchip:307] EMMC Host Control 2: 0x1b
🔍 18.671s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.672s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.673s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.673s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.674s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.675s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.676s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.677s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.677s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.678s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.679s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.680s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.681s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.681s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.683s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.683s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.684s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.685s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.686s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.687s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.687s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.688s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.689s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.690s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.691s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.692s    [sdmmc::emmc::cmd:24resp_type=0x15, command=0x153a
🔍 18.693s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.693s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.694s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.695s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.696s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.697s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.697s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.698s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.699s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.700s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.701s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.702s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.703s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.703s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.704s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.705s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.706s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.707s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.707s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.708s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.709s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.710s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.711s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.712s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.713s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.713s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.714s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.715s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.716s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.717s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.717s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.718s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.719s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.720s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.721s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.722s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.723s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.723s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.724s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 1    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.726s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.727s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.727s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.728s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.729s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.730s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.731s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.732s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.733s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.733s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.734s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.735s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.736s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.737s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.738s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.738s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.739s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.740s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.741s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.742s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.743s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.743s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.744s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.745s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.746s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.747s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.748s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.748s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.749s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.750s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.751s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.752s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.753s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.754s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.754s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.755s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.756s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.757s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.758s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.758s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.759s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.760s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.761s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.762s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.763s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.764s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.764s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.765s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.766s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.767s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.768s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.768s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.769s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.770s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.771s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
🔍 18.772s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x15, arg=0x0, resp_type=0x15, command=0x153a
🔍 18.773s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.774s    [sdmmc::emmc::cmd:263] Response Status: 0b100000
🔍 18.774s    [sdmmc::emmc::cmd:nse Status: 0b100000
🔍 18.778s    [sdmmc::emmc::cmd:288] Command completed: status=0b100000
ully
SD card initialization successful!
Card type: MmcHc
Manufacturer ID: 0x45
Capacity: 0 MB
Block size: 512 bytes
Attempting to read first block...
🔍 18.780s    [sdmmc::emmc::block:365] pio read_blocks: block_id = 5034498, blocks = 1
🔍 18.781s    [sdmmc::emmc::block:383] Reading 1 blocks starting at address: 0x4cd202
🔍 18.782s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x11, arg=0x4cd202, resp_type=0x15, command=0x113a
🔍 18.783s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.784s    [sdmmc::emmc::cmd:263] Response Status: 0b100001
🔍 18.785s    [sdmmc::emmc::cmd:288] Command completed: status=0b100001
🔍 18.786s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
Successfully read first block!
First 16 bytes of first block: [40, E2, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 8F, D2, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, DB, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 80, E0, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, C0, EC, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, E9, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 80, EE, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, E4, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, C0, DE, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, F0, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, DD, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 80, E7, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, A9, D5, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, 5B, D7, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 80, 50, D6, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, 4E, D6, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 60, 4F, D6, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 80, CE, CD, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, 48, DF, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 8E, D2, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 60, D6, CD, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 90, D2, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, A0, 09, DD, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 80, B9, E1, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, EB, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 60, DD, E0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 20, D1, CD, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, E0, 7E, E2, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 20, A8, D5, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 40, D7, CD, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 91, D2, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, C0, E5, D0, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00]
Testing write and read back...
🔍 18.804s    [sdmmc::emmc::block:417] pio write_blocks: block_id = 3, blocks = 1
🔍 18.805s    [sdmmc::emmc::block:439] Writing 1 blocks starting at address: 0x3
🔍 18.806s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x18, arg=0x3, resp_type=0x15, command=0x183a
🔍 18.807s    [sdmmc::emmc::cmd:263] Response Status: 0b10000
🔍 18.808s    [sdmmc::emmc::cmd:263] Response Status: 0b10001
🔍 18.808s    [sdmmc::emmc::cmd:288] Command completed: status=0b10001
🔍 18.809s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
Successfully wrote to block 3!
🔍 18.811s    [sdmmc::emmc::block:365] pio read_blocks: block_id = 3, blocks = 1
🔍 18.812s    [sdmmc::emmc::block:383] Reading 1 blocks starting at address: 0x3
🔍 18.813s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x11, arg=0x3, resp_type=0x15, command=0x113a
🔍 18.814s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.815s    [sdmmc::emmc::cmd:263] Response Status: 0b100001
🔍 18.816s    [sdmmc::emmc::cmd:288] Command completed: status=0b100001
🔍 18.816s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
Successfully read back block 3!
First 16 bytes of read block: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
Data verification successful: written and read data match perfectly!
Testing multi-block read...
🔍 18.831s    [sdmmc::emmc::block:365] pio read_blocks: block_id = 200, blocks = 4
🔍 18.832s    [sdmmc::emmc::block:383] Reading 4 blocks starting at address: 0xc8
🔍 18.833s    [sdmmc::emmc::cmd:244] Sending command: opcode=0x12, arg=0xc8, resp_type=0x15, command=0x123a
🔍 18.834s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.835s    [sdmmc::emmc::cmd:263] Response Status: 0b100001
🔍 18.835s    [sdmmc::emmc::cmd:288] Command completed: status=0b100001
🔍 18.836s    [sdmmc::emmc::cmd:339] Data transfer: cmd.data_present=true
🔍 18.837s    [sdmmc::emmc::cmd:244] Sending command: opcode=0xc, arg=0x0, resp_type=0x1d, command=0xc1b
🔍 18.838s    [sdmmc::emmc::cmd:263] Response Status: 0b0
🔍 18.839s    [sdmmc::emmc::cmd:263] Response Status: 0b11
🔍 18.840s    [sdmmc::emmc::cmd:288] Command completed: status=0b11
Successfully read 4 blocks starting at block address 200!
First 16 bytes of first block: [A0, 2F, 00, B9, A1, 8B, 0D, A9, A0, 07, 42, A9, A0, 07, 04, A9]
First 16 bytes of last block: [B5, 01, BD, 01, C6, 01, CE, 01, D6, 01, DE, 01, E7, 01, EF, 01]
SD card test complete
💡 18.843s    [test::tests:58] test uboot
test test_platform passed
All tests passed
```

### 测试功能说明

测试程序会执行以下操作：

1. **设备树解析**: 从设备树中查找 EMMC 控制器硬件节点地址
2. **EMMC 控制器初始化**: 初始化 DWCMSHC EMMC 控制器
3. **存储卡检测**: 检测并初始化连接的 eMMC 存储卡
4. **基本读写测试**:
   - 读取存储卡信息
   - 读取数据块
   - 写入数据块并验证
   - 多块读取测试
5. **数据一致性验证**: 验证写入和读取的数据是否一致

**注意**: 完整测试需要支持 RK3568 的 ARM 硬件平台和 U-Boot 环境

## 开发板支持

### Firefly ROC-RK3568-PC

- **控制器**: DWCMSHC SDHCI
- **基地址**: 0xfe2e0000
- **时钟控制**: 通过 RK3568 CRU 控制器管理
- **串口调试**: 通过 `/dev/ttyUSB1`，波特率 1500000

### QEMU 模拟器

- **平台**: virt
- **CPU**: Cortex-A72
- **SD卡模拟**: sdhci-pci + sd-card 设备

## 故障排除

### 常见问题

1. **"No card detected" 错误**
   - 检查 SD 卡是否正确插入
   - 确认硬件连接是否正常
   - 验证控制器基地址是否正确

2. **时钟初始化失败**
   - 检查时钟控制器是否正确初始化
   - 确认平台特定配置是否正确

3. **数据传输错误**
   - 检查总线宽度设置是否正确
   - 确认时序模式是否匹配
   - 验证 DMA 缓冲区是否正确对齐

### 调试技巧

1. **启用详细日志**
   ```bash
   # 在测试时启用详细输出
   make uboot
   ```

2. **使用内存转储**
   ```rust
   // 在代码中添加内存转储调用
   unsafe { dump_memory_region(emmc_addr, 0x100); }
   ```

3. **检查寄存器状态**
   - 查看控制器寄存器状态
   - 验证命令和数据线路状态

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情