# sing-box GUI

![Build Status](https://github.com/zangge8855/sing-box-gui/actions/workflows/build.yml/badge.svg)
![Version](https://img.shields.io/badge/version-2026.7.11-blue.svg)

<p align="center">
  <img src="assets/logo.jpg" alt="sing-box GUI Logo" width="128" height="128" style="border-radius: 20px;"/>
</p>

An elegant, feature-rich, and clearly-structured sing-box client written in pure Rust using [Iced 0.14.0](https://github.com/iced-rs/iced), natively adapted and polished for the Windows platform. The version number for this release is **2026.7.11**.

## 🌟 Core Features

- **High-Aesthetic Modern UI**: Exquisite dark theme and modern design (soft glowing shadows, rounded cards, smooth status indicators) providing a premium visual experience.
- **Bilingual Support (i18n)**: Fully supports English and Chinese interfaces, toggleable with a single click in Settings.
- **Core Auto-Management**: Automatically downloads and installs the official `sing-box.exe` (v1.13.14 stable) core release on first launch; supports custom external core paths.
- **Universal Subscription & Link Import**: Supports parsing standard Clash YAML subscriptions as well as raw/Base64 sharing node links. Features a one-click **Update** button to fetch the latest configurations on the fly.
- **Smart Routing & Split Rules**: Built-in Rule/Global/Direct proxy modes. Contains split rules based on remote rule-sets for bypassing LAN and Chinese mainland IPs/domains (GeoIP/GeoSite) to prevent DNS leaks.
- **Dashboard Traffic Chart**: Real-time traffic rate display (download/upload speed), cumulative total traffic tracking for the session, and responsive SVG wave history plots.
- **Connection Summary**: Quick overview of your active Routing Mode and Listen Port directly on the Dashboard.
- **Proxy Cards & Latency Test**: Interactive grid card layout highlighting active proxies; supports multi-threaded parallel latency testing (colorized green/yellow/red according to delay values).
- **Colorized Core Logs**: Real-time pipe streaming of standard output and error logs from the core, colored by severity level (Error, Warning, Info) with a clear console log button.
- **Windows System Integration**:
  - **System Proxy Toggle**: Modifies Windows registry keys and broadcasts system messages, cleaning up registry states automatically on client shutdown.
  - **Start on Boot**: Writes to the registry `Run` key for headless, silent startup.
  - **TUN Mode**: Seamless virtual network card traffic capturing (requires Administrator privileges).
  - **Auto-Close Core**: Optional toggle to gracefully shut down the sing-box core when the application exits.

---

## 🛠️ Project Architecture

The project adheres to the classic **Elm Architecture** (The Elm Architecture), modularized into:

- `src/state.rs`: Managed state, local configurations, and node data structure models.
- `src/message.rs`: Actions and asynchronous events definitions.
- `src/config.rs`: Saves/loads app configurations, parses subscriptions, and converts configurations to sing-box JSON rules.
- `src/core.rs`: Manages the silent Windows child subprocess `sing-box.exe` life cycle, auto-downloads, and pipes logs.
- `src/api.rs`: Integrates Clash REST API for selector nodes, multi-threaded latency tests, and streams `/traffic` metrics.
- `src/sysproxy.rs`: System-wide registry proxy configurations.
- `src/ui/`: Contains Tab renders (Dashboard, Proxies, Profiles, Logs, Settings) and theme coloring schemes.

---

## 🚀 Local Development & Compilation

Ensure you have Rust 1.70+ installed, then begin local development and debugging:

```bash
# Clone the repository and enter the directory
cd sing-box-gui

# Check codebase compilation
cargo check

# Run unit tests
cargo test

# Run GUI in development mode
cargo run

# Build release executable for Windows (output binary is located at target/release/sing-box-gui.exe)
cargo build --release
```

---

## 🤖 GitHub Actions Remote Build

The project integrates GitHub Actions. Any push to the `master` or `main` branches triggers automatic remote Windows AMD64 compilation, producing release binaries published directly to GitHub Releases.

CI configuration file is located at `.github/workflows/build.yml`.

---

## 🙏 Acknowledgments

Special thanks to the following open-source projects for providing core libraries and inspiration:
- [iced-rs/iced](https://github.com/iced-rs/iced) (GUI Framework)
- [SagerNet/sing-box](https://github.com/SagerNet/sing-box) (Proxy Core Engine)
- [GUI-for-Cores/GUI.for.SingBox](https://github.com/GUI-for-Cores/GUI.for.SingBox) (Modern sing-box desktop client which inspired features of this project)

---

## 📄 License

This project is open-sourced under the **GNU General Public License v3.0** (GPL-3.0) license, ensuring full compatibility with dependent component licenses (GPLv3 for `sing-box` and MIT/Apache-2.0 for `iced`).

================================================================================

# sing-box GUI (中文说明)

![Build Status](https://github.com/zangge8855/sing-box-gui/actions/workflows/build.yml/badge.svg)
![Version](https://img.shields.io/badge/version-2026.7.11-blue.svg)

<p align="center">
  <img src="assets/logo.jpg" alt="sing-box GUI Logo" width="128" height="128" style="border-radius: 20px;"/>
</p>

一个用纯 Rust + [Iced 0.14.0](https://github.com/iced-rs/iced) 编写的高颜值、功能丰富、逻辑清晰的 sing-box 客户端，专为 Windows 平台进行原生适配与美化。 本次发布版本号为 **2026.7.9**。

## 🌟 核心特性

- **高颜值现代 UI**：精美的深色主题与现代化设计（柔和发光阴影、圆角卡片、平滑状态指示），带来极致视觉体验。
- **多语言切换 (i18n)**：完整支持中文和英文界面，可在“设置”中一键进行语言切换。
- **内核自动管理**：首次运行自动从 GitHub 官方 Releases 下载并安装 `sing-box.exe`（v1.13.14 稳定版）内核；支持自定义外部内核路径。
- **订阅热更新与链接导入**：支持解析标准的 Clash YAML 订阅配置及各种原始分享链接，并支持一键**更新**订阅配置，实时获取最新节点。
- **智能分流规则**：内置 Rule/Global/Direct 三种经典代理模式。内置基于远端规则集（rule-set）的局域网与国内直连（GeoIP/GeoSite）分流逻辑，防 DNS 泄漏。
- **仪表盘流量监控**：实时显示当前下载/上传速度、**会话总流量统计**，并利用原生 SVG 绘制响应式流量波形图。附加当前的路由模式与端口状态总览。
- **节点网格与测速**：直观的卡片网格布局，高亮当前活跃代理节点；支持多线程并发测速，延迟根据响应时间绿/黄/红实时渲染。
- **彩码实时日志**：实时管道流传输内核的标准输出和错误日志，按错误、警告、信息进行行级色彩渲染，支持一键清空日志。
- **Windows 系统集成**：
  - **系统代理一键切换**：自动修改 Windows 注册表并发送系统广播，内核退出时自动清理系统代理。
  - **开机自启动**：直接写入注册表 Run 键，方便无感开机启动。
  - **TUN 虚拟网卡**：支持无感虚拟网卡接管系统级流量（需管理员权限）。
  - **自动关闭内核**：支持设置在退出图形界面时自动安全关闭内核进程，拒绝残留。

---

## 🛠️ 项目架构

项目遵循经典的 **Elm 架构**（The Elm Architecture），划分为：

- `src/state.rs`: 核心状态、本地配置及节点数据结构模型。
- `src/message.rs`: 系统所有交互和异步事件的 Message 定义。
- `src/config.rs`: 负责保存/读取 GUI 配置，解析 Clash YAML/分享链接并输出完整的 sing-box 配置 JSON。
- `src/core.rs`: 管理 Windows 隐藏子进程 `sing-box.exe` 的生命周期，自动下载核心并双管道读取终端日志。
- `src/api.rs`: 封装 Clash REST API，实现多线程延迟测试、节点选择与实时的 `/traffic` 流量流解析。
- `src/sysproxy.rs`: 跨平台系统代理设定（Windows 注册表）。
- `src/ui/`: 模块化渲染各个 Tab 视图（仪表盘、节点列表、配置订阅、日志台、设置项）以及主框架与配色方案。

---

## 🚀 本地开发与编译

确保已安装 Rust 1.70+ 版本，随后进行本地开发调试：

```bash
# 克隆项目并进入目录
cd sing-box-gui

# 检查代码
cargo check

# 运行单元测试
cargo test

# 以开发模式启动 GUI
cargo run

# 编译 Windows 平台 Release 版本二进制包（编译产物位于 target/release/sing-box-gui.exe）
cargo build --release
```

---

## 🤖 GitHub Actions 远程编译

项目已集成 GitHub Actions。任何推送至 `master` 或 `main` 分支的代码，均会自动在 GitHub 的 Windows 云端虚拟机中执行远程编译，并将产出的 release 级可执行程序作为 Releases 供用户下载使用。

CI 配置文件位于 `.github/workflows/build.yml`.

---

## 🙏 致谢

感谢以下优秀项目为本项目提供的核心支持与灵感：
- [iced-rs/iced](https://github.com/iced-rs/iced) (GUI 框架)
- [SagerNet/sing-box](https://github.com/SagerNet/sing-box) (代理核心引擎)
- [GUI-for-Cores/GUI.for.SingBox](https://github.com/GUI-for-Cores/GUI.for.SingBox) (优秀的 sing-box 客户端，本项目参考并汲取了其丰富的功能特性及界面灵感)

---

本项目采用 **GNU General Public License v3.0** (GPL-3.0) 协议开源。该协议与本项目所依赖的开源组件完全兼容。

<!--
## 🏷️ Keywords & Search Tags (搜索优化标签)

`sing-box`, `sing-box-gui`, `sing-box client`, `sing-box windows`, `rust-gui`, `iced-gui`, `proxy-client`, `clash-alternative`, `v2ray-client`, `shadowsocks-client`, `trojan-client`, `vless-client`, `vless`, `network-proxy`, `tun-mode`, `rules-routing`, `rust-proxy`, `cross-platform`, `high-aesthetic-gui`, `singbox`, `clash`, `proxy`
-->

