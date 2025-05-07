# SD/MMC

本指南介绍如何在 Firefly ROC-RK3568-PC 开发板上快速测试 SD/MMC 功能。

## 快速上手

### 前提条件

+ **开发板**：[Firefly ROC-RK3568-PC](https://wiki.t-firefly.com/zh_CN/ROC-RK3568-PC/preface.html)
+ **工具依赖**：安装 [`ostool`](https://github.com/ZR233/ostool)（用于串口烧录与调试）

安装 ostool：
``` bash
cargo install ostool
```

### 配置串口连接

创建配置文件：

``` bash
touch .bare-test.toml
``` 
编辑 `.bare-test.toml` 文件，添加以下内容：

``` toml
serial = "/dev/ttyUSB1"
baud_rate = 1500000
dtb_file = "./firmware/rk3568-firefly-roc-pc-se.dtb"
```
请根据实际串口设备路径与 `dtb` 文件路径进行调整。

### 测试方法
编译并烧录 U-Boot：

``` bash
make uboot
```
系统会自动使用上述配置通过串口进行测试。确保设备已连接且权限允许访问 `/dev/ttyUSB1`。