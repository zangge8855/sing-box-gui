# sing-box GUI

![Build Status](https://github.com/zangge8855/sing-box-gui/actions/workflows/build.yml/badge.svg)
![Version](https://img.shields.io/badge/version-2026.7.6-blue.svg)

<p align="center">
  <img src="assets/icon.jpg" alt="sing-box GUI Logo" width="128" height="128" style="border-radius: 20px;"/>
</p>

一个用纯 Rust + [Iced 0.14.0](https://github.com/iced-rs/iced) 编写的高颜值、功能丰富、逻辑清晰的 sing-box 客户端，专为 Windows 平台进行原生适配与美化。 本次发布版本号为 **2026.7.6**。

## 🌟 核心特性

- **高颜值现代 UI**：精美的深色主题与现代化设计（柔和发光阴影、圆角卡片、平滑状态指示），带来极致视觉体验。
- **内核自动管理**：首次运行自动从 GitHub 官方 Releases 下载并安装 `sing-box.exe`（v1.13.14 稳定版）内核；支持自定义外部内核路径。
- **订阅一键导入**：支持解析标准的 Clash YAML 订阅配置，自动转化为 sing-box 原生 JSON 配置，完美融入多入站和 DNS 绕过规则。
- **智能分流规则**：内置 Rule/Global/Direct 三种经典代理模式。内置基于远端规则集（rule-set）的局域网与国内直连（GeoIP/GeoSite）分流逻辑，防 DNS 泄漏。
- **仪表盘流量图**：实时显示当前下载/上传速度，并利用原生 SVG 绘制响应式流量波形图。
- **节点网格与测速**：直观的卡片网格布局，高亮当前活跃代理节点；支持多线程并发测速，延迟根据响应时间绿/黄/红实时渲染。
- **彩码实时日志**：实时管道流传输内核的标准输出和错误日志，按错误、警告、信息进行行级色彩渲染，支持一键清空日志。
- **Windows 系统集成**：
  - **系统代理一键切换**：自动修改 Windows 注册表并发送系统广播，内核退出时自动清理系统代理。
  - **开机自启动**：直接写入注册表 Run 键，方便无感开机启动。
  - **TUN 虚拟网卡**：支持无感虚拟网卡接管系统级流量（需管理员权限）。

---

## 🛠️ 项目架构

项目遵循经典的 **Elm 架构**（The Elm Architecture），划分为：

- `src/state.rs`: 核心状态、本地配置及节点数据结构模型。
- `src/message.rs`: 系统所有交互和异步事件的 Message 定义。
- `src/config.rs`: 负责保存/读取 GUI 配置，解析 Clash YAML 并输出完整的 sing-box 配置 JSON。
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

项目已集成 GitHub Actions。任何推送至 `master` 或 `main` 分支的代码，均会自动在 GitHub 的 Windows 云端虚拟机中执行远程编译，并将产出的 release 级可执行程序作为 Artifacts 供用户下载使用。

CI 配置文件位于 `.github/workflows/build.yml`。
