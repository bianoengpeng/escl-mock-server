# eSCL Mock Server

一个用 Rust 编写的 eSCL (eScan Cloud Services) 模拟扫描仪服务器，可以被 Windows、macOS、Linux 等操作系统发现并作为网络扫描仪使用。

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

## 📋 项目概述

eSCL Mock Server 是一个功能完整的网络扫描仪模拟器，实现了 eSCL 协议的所有核心功能。它可以帮助开发人员测试扫描应用程序，或者在没有物理扫描仪的情况下进行 eSCL 协议的学习和调试。

> 🍴 **项目来源**: 本项目 Fork 自 [@Chrisimx/escl-mock-server](https://github.com/Chrisimx/escl-mock-server)，在原项目基础上进行了功能增强和优化。

### 🎯 主要用途

- **扫描应用测试**: 为扫描软件开发提供稳定的测试环境
- **协议学习**: 学习和调试 eSCL 协议实现
- **兼容性测试**: 测试 Windows、macOS、Linux 的扫描仪发现机制
- **网络诊断**: 调试网络扫描仪连接问题

## ✨ 功能特性

### 🔌 协议支持

- **eSCL 2.97**: 完整实现 eSCL 协议规范
- **mDNS 服务发现**: 自动网络设备发现 (`_uscan._tcp`)
- **WS-Discovery (WSD)**: Windows 设备发现协议
- **UPnP/SSDP**: 通用即插即用设备发现
- **HTTP REST API**: 标准的 eSCL RESTful 接口

### 🖥️ 系统兼容性

- **Windows 11/10**: 完全兼容，支持"添加打印机或扫描仪"
- **Windows 设备验证**: 专门的验证端点确保设备正常添加
- **NAPS2**: 兼容 NAPS2 扫描软件的 eSCL 驱动
- **跨平台**: 支持 Linux、macOS、Windows

### 📄 扫描功能

- **平板扫描 (Platen)**: 模拟平板扫描仪
- **自动输稿器 (ADF)**: 模拟多页文档扫描
- **双面扫描**: 支持双面扫描模拟
- **多种色彩模式**: 黑白、灰度、彩色 (RGB24)
- **多种分辨率**: 100、200、300、600 DPI
- **多种格式**: PDF、JPEG 输出

### 🔍 调试功能

- **详细请求日志**: 实时显示所有 HTTP 请求
- **客户端识别**: 自动识别 NAPS2、Windows 等客户端类型
- **端点分析**: 清晰标识不同类型的 eSCL 请求
- **网络信息**: 显示服务器 IP、端口等网络配置

## 🚀 快速开始

### 前置要求

- **Rust**: 1.70+ (推荐使用 `rustup` 安装)
- **网络**: 确保防火墙允许所选端口的访问

### 安装

1. **克隆仓库**
   ```bash
   git clone https://github.com/your-repo/escl-mock-server.git
   cd escl-mock-server
   ```

2. **构建项目**
   ```bash
   cargo build --release
   ```

### 使用方法

#### 💨 快速启动 (推荐)

```bash
# Windows 用户可以直接运行批处理文件
run_escl_server.bat

# 或者使用 Cargo 命令
cargo run -- -a 0.0.0.0 -p 8080 -i res/portrait-color.jpg
```

#### ⚙️ 自定义配置

```bash
cargo run -- [选项]
```

**命令行选项:**

| 选项 | 短选项 | 默认值 | 说明 |
|------|--------|--------|------|
| `--bindaddr` | `-a` | `127.0.0.1` | 服务器绑定地址 |
| `--port` | `-p` | `8080` | HTTP 服务端口 |
| `--scope` | `-s` | `/eSCL` | eSCL 服务路径 |
| `--scannercaps` | `-c` | 内置默认值 | 自定义扫描仪能力 XML 文件 |
| `--image` | `-i` | 内置默认图片 | 自定义扫描返回的图片文件 |

**配置示例:**

```bash
# 基本配置 - 监听所有接口
cargo run -- -a 0.0.0.0 -p 8080

# Windows 11 兼容模式
cargo run -- -a 0.0.0.0 -p 8080 -s "/eSCL" -i res/portrait-color.jpg

# 自定义图片和端口
cargo run -- -a 192.168.1.100 -p 9000 -i /path/to/your/image.jpg

# 自定义扫描仪能力
cargo run -- -c /path/to/custom_caps.xml
```

## 🌐 服务端点

启动后，服务器将提供以下端点：

### 📡 核心 eSCL 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/eSCL/ScannerCapabilities` | GET | 扫描仪能力查询 |
| `/eSCL/ScannerStatus` | GET | 扫描仪状态查询 |
| `/eSCL/ScanJobs` | POST | 创建扫描任务 |
| `/eSCL/ScanJobs/{uuid}/NextDocument` | GET | 获取扫描文档 |
| `/eSCL/ScanBufferInfo` | PUT | 扫描缓冲区信息验证 |

### 🔍 设备发现端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/` | GET | 设备根信息 (UPnP) |
| `/device.xml` | GET | Windows 设备元数据 |
| `/wsd` | GET/POST | WS-Discovery 支持 |
| `/ssdp` | GET | SSDP 设备发现 |

### 🛡️ Windows 兼容端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/eSCL/Validate` | POST | Windows 设备验证 |
| `/eSCL/DeviceCapabilities` | GET | 设备能力验证 |
| `/eSCL/DeviceUUID` | GET | 设备唯一标识 |
| `/eSCL/Configuration` | GET | 设备配置信息 |

### 🎛️ 管理和调试端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/admin` | GET | 设备管理页面 |
| `/icon.png` | GET | 设备图标 |
| `/health` | GET | 健康检查 |
| `/system` | GET | 系统信息 |

## 📱 客户端配置

### Windows 11/10

1. **打开设置** → **蓝牙和设备** → **打印机和扫描仪**
2. **点击"添加设备"**
3. **等待自动发现** (通常几秒内出现 "eSCL Scanner")
4. **点击添加**，Windows 会自动安装 eSCL 驱动

> ⚠️ **重要提示 - mDNS 端口冲突**
> 
> 在 Windows 系统上运行此服务器时，由于 mDNS 端口冲突，**当前运行服务的 Windows 电脑无法通过自动发现功能添加扫描设备**（包括系统设置和 NAPS2 的自动发现）。这是技术限制，不是软件缺陷。
> 
> **解决方案:**
> - ✅ **推荐**: 在局域网中的**其他电脑**上添加扫描仪（其他 Windows/Mac/Linux 设备可以正常发现）
> - ✅ **最佳**: 使用 NAPS2 手动配置 IP 地址，可以在同一台电脑上正常使用
> - ✅ **备选**: 使用其他支持手动 URL 配置的扫描软件

**故障排除:**
- 确保 Windows 防火墙允许端口 8080
- 检查网络连接在同一子网
- 在浏览器访问 `http://[server-ip]:8080/admin` 验证服务可用
- 如果本机无法发现，尝试在其他设备上添加扫描仪

### NAPS2

#### 自动发现方式 (仅限其他电脑):
1. **打开 NAPS2**
2. **选择"eSCL 驱动程序"**
3. **点击"选择设备"**
4. **从列表中选择"eSCL Scanner"**

> ⚠️ **注意**: 由于 mDNS 端口冲突，在运行服务的同一台电脑上，NAPS2 的自动发现功能也无法正常工作。

#### 手动配置方式 (推荐用于同机运行):
1. **打开 NAPS2**
2. **选择"eSCL 驱动程序"**
3. **点击"选择设备"**
4. **点击"输入 URL"或"手动添加"**
5. **输入服务器地址**: `http://[服务器IP]:8080/eSCL`
   - 例如: `http://192.168.1.100:8080/eSCL`
   - 本机运行时: `http://127.0.0.1:8080/eSCL`

> ✅ **推荐**: 即使在局域网其他电脑上使用 NAPS2，也建议使用手动配置方式，更加稳定可靠。

### macOS

1. **系统偏好设置** → **打印机与扫描仪**
2. **点击"+"添加**
3. **选择自动发现的扫描仪**

## 🏗️ 项目结构

```
escl-mock-server/
├── src/
│   ├── main.rs              # 主程序入口，HTTP服务器和mDNS设置
│   ├── cli.rs               # 命令行参数解析
│   ├── escl_server.rs       # eSCL协议端点实现
│   └── model.rs             # 数据模型定义
├── res/
│   ├── default_scanner_caps.xml  # 默认扫描仪能力配置
│   ├── example_image.jpg         # 默认扫描图片
│   ├── portrait-color.jpg        # 彩色示例图片
│   └── regexes/                   # 网络地址验证正则表达式
├── run_escl_server.bat           # Windows 快速启动脚本
├── Cargo.toml                    # Rust项目配置
└── README.md                     # 项目文档
```

## 🔧 开发

### 依赖项

```toml
[dependencies]
actix-web = "4.9.0"          # Web 框架
clap = "4.5.26"              # 命令行解析
tokio = "1.43.0"             # 异步运行时
mdns-sd = "0.10.0"           # mDNS 服务发现
uuid = "1.12.0"              # UUID 生成
chrono = "0.4"               # 时间处理
```

### 构建和测试

```bash
# 开发模式运行
cargo run

# 发布模式构建
cargo build --release

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

### 自定义扫描仪能力

创建自定义的 `scanner_caps.xml` 文件来定义扫描仪的具体能力：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<scan:ScannerCapabilities xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03">
    <pwg:Version>2.97</pwg:Version>
    <pwg:MakeAndModel>My Custom Scanner</pwg:MakeAndModel>
    <!-- 更多配置... -->
</scan:ScannerCapabilities>
```

然后使用 `-c` 参数指定：

```bash
cargo run -- -c /path/to/custom_caps.xml
```

## 🐛 故障排除

### 常见问题

**问题**: Windows 无法发现扫描仪
- **解决**: 检查防火墙设置，确保允许端口 8080
- **解决**: 确保服务器绑定到正确的网络接口 (`-a 0.0.0.0`)
- **已知限制**: 在运行服务的 Windows 电脑上，由于 mDNS 端口冲突无法自动发现设备。请在其他设备上添加或使用手动配置

**问题**: mDNS 服务注册失败
- **解决**: 在 Linux 上安装 `avahi-daemon`
- **解决**: 在 Windows 上启用"Bonjour 服务"

**问题**: 运行服务的电脑无法添加扫描仪 (Windows)
- **原因**: mDNS 端口 5353 冲突，Windows 系统服务占用该端口
- **解决**: 在局域网其他电脑上添加扫描仪，或使用 NAPS2 手动配置 IP 地址

**问题**: NAPS2 无法自动发现扫描仪 (同机运行)
- **原因**: 同样受到 mDNS 端口冲突影响
- **解决**: 使用 NAPS2 的手动 URL 配置功能: `http://127.0.0.1:8080/eSCL`

**问题**: 扫描返回错误
- **解决**: 检查图片文件路径是否正确
- **解决**: 确保图片文件格式为 JPEG

### 调试技巧

1. **查看详细日志**: 程序会输出详细的请求日志
2. **浏览器测试**: 访问 `http://[ip]:8080/admin` 检查服务状态
3. **网络测试**: 使用 `curl` 测试端点响应

```bash
# 测试服务器连接
curl http://192.168.1.100:8080/

# 测试扫描仪能力
curl http://192.168.1.100:8080/eSCL/ScannerCapabilities

# 测试扫描仪状态
curl http://192.168.1.100:8080/eSCL/ScannerStatus
```

---

**注意**: 这是一个模拟服务器，仅用于测试和开发目的。

