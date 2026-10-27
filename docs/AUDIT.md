# 本地检查与修复记录

检查日期：2026-09-06。仓库基线：`fe9f7f1`（`main`）。本轮修改保留在本地工作区。

项目是 Rust / Iced 原生桌面应用。本次在 Fedora 44 上运行原生 Wayland 检查，并对 Windows、macOS 的代码和测试代码进行交叉编译检查。

## 已修复的问题

| 范围 | 问题与修复 |
| --- | --- |
| Linux 托盘 | 原来的 GTK 托盘缺少正确的 GTK 事件循环，可能在启动时失败。改为 `ksni` / D-Bus StatusNotifierItem，直接支持 Wayland；Windows/macOS 托盘在窗口事件线程创建。 |
| 窗口生命周期 | Wayland 不支持原实现依赖的隐藏/显示方式。改用 Iced daemon 保留应用状态，关闭窗口表面后由托盘重建；保留未保存输入和窗口尺寸。没有托盘时正常退出，托盘服务消失时自动恢复隐藏窗口。过滤旧窗口迟到的尺寸事件。 |
| 字体与图标 | Linux 不再要求安装 Ubuntu 字体；macOS 不再引用未必安装的 SF Pro。分别使用 Fontconfig、Helvetica Neue、Segoe UI，并保留内置 Noto CJK。修复缺失的节点页图标、英文节点名标点方框；日志使用等宽字体。 |
| 节点展示 | 国旗 emoji 改为稳定的地区代码徽标，避免跨平台缺字；修复从 `node`、`business`、`Auto` 等普通词错误推断地区的问题。网格按实际可用宽度计算，长名称保留悬停提示，策略组比较函数符合排序要求。 |
| 页面排版 | 支持 640×480 至宽屏窗口；正文居中限宽，标题和主要操作保持可见。修复窄窗口日志工具栏拥挤、订阅表单挤占列表、导入按钮文字未居中等问题；设置页按内容分栏，配置预览高度随窗口调整。 |
| 深浅主题 | 提高状态、错误、辅助文字、节点选中状态的对比度；调整危险按钮和选中徽标。对语义文字与主要按钮的活动/悬停/按下状态检查至少 4.5:1 的颜色对比度。系统主题改为原生通知，去掉周期性启动外部命令的检测。 |
| 导入与 API | 剪贴板中的分享链接、Base64 集合、YAML、JSON 直接进入配置解析器，不再错误地全部作为订阅 URL 下载；本地与内联内容限制为 16 MiB，内联凭据不复制到订阅来源元数据。区分 HTTP 代理分享链接和订阅 URL，包括显式 80/443 端口。回环 Clash API 请求绕过环境代理。 |
| 运行状态 | 路由模式在内核 API 确认成功后才更新界面和持久化；失败保留原模式，阻止重复切换。内核停止期间忽略迟到的节点和连接快照。 |
| 平台配置 | macOS 自动生成的 TUN 交给系统分配 `utunN`，保留用户原生配置明确指定的接口名。旧设置缺少新增字段时保留订阅与已有设置；所有平台按系统语言设置初始语言，取消会覆盖用户语言选择的旧迁移。 |
| 键盘与工程 | 增加输入框 Tab/Shift+Tab、页面 Ctrl/Command+1…7，编辑表单支持 Enter，输入框聚焦时 Escape 和保存快捷键仍有效。修正文档中的键盘能力说明；Rust 最低版本改为 1.88，精简不需要的归档/图像及 Linux 托盘依赖。 |

主要实现见 [main.rs](../src/main.rs)、[tray.rs](../src/tray.rs)、[config.rs](../src/config.rs) 和 [ui](../src/ui)。新增的 [桌面入口](../assets/sing-box-gui.desktop) 与 Wayland 应用 ID 一致。

## 验证结果

| 检查 | 结果 |
| --- | --- |
| 原始基线测试 | 120 项通过 |
| 修复后 `cargo test --quiet --locked --offline` | 131 项通过，0 失败 |
| `cargo clippy --all-targets --quiet --locked --offline --features ui-smoke` | 通过，0 警告 |
| `cargo fmt --all -- --check` | 通过 |
| Linux `cargo build --quiet --locked --offline` | 通过；开发版本位于 `target/debug/sing-box-gui` |
| Windows `x86_64-pc-windows-msvc`，`cargo check --all-targets --locked --offline` | 通过 |
| macOS `aarch64-apple-darwin`，`cargo check --all-targets --locked --offline` | 通过 |
| 原生 Wayland / wgpu / 1 倍缩放 | 172 张截图，托盘验证通过 |
| 原生 Wayland / wgpu / 2 倍缩放 | 172 张截图，托盘验证通过 |
| Wayland / wgpu / 无托盘服务 | 22 张截图，关闭退出验证通过 |
| Python 语法、工作流 YAML、desktop-file-validate | 通过 |

每个完整截图矩阵覆盖 7 个页面 × 2 种语言 × 2 种主题 × 4 种窗口尺寸（640×480、720×600、1080×750、1600×1000），再加 60 个空态、编辑、删除、输入错误、无搜索结果和滚动位置场景。截图尺寸和缩放比例由程序断言；通过查看截图检查各页面、深浅主题、窄/宽窗口及关键状态的排版。

托盘检查运行在私有 D-Bus 会话中，实际调用 StatusNotifierItem 的激活接口，读取菜单与图标，并移除、恢复托盘服务。验证了窗口恢复、未保存输入保留，以及托盘服务丢失后的恢复行为。测试数据为模拟配置，不启动代理内核。

截图由 Iced 实际渲染并捕获。Weston headless 驱动原生 Wayland 窗口、帧与缩放事件；使用 noop 输出合成避免本机 Pixman 2 倍缩放崩溃。最终矩阵使用应用默认的 wgpu 渲染器，1 倍和 2 倍检查分别约 125 秒、154 秒。

### 截图与本地完整输出

- [浅色控制台](screenshots/wayland-dashboard-light.png)
- [深色节点页](screenshots/wayland-proxies-dark.png)
- [宽屏设置页](screenshots/wayland-settings-wide.png)
- [640×480 订阅编辑](screenshots/wayland-profile-editor-compact.png)

完整的 366 张截图、日志和 `result.json` 已保存在本地 `target/ui-audit/` 下的 `wayland-1x`、`wayland-2x`、`wayland-no-tray` 目录。`target/` 不进入版本控制；可使用 [ui_smoke.py](../scripts/ui_smoke.py) 重新生成。

## 验证边界

Windows 和 macOS 已检查平台条件编译与测试代码；本机没有这两个系统，未宣称完成其真实窗口、字体、DPI、原生菜单或辅助功能验收。Linux 截图来自独立的 Weston Wayland 会话。

没有修改当前桌面的系统代理、启动真实 TUN 或使用真实订阅连接外部节点。相关解析和状态逻辑有测试覆盖，真实网络及权限集成仍需在相应环境验证。Iced 0.14 的按钮、开关、下拉菜单键盘焦点和屏幕阅读器支持仍受框架限制，README 已说明实际能力。

CI 已增加 Windows/macOS 原生测试，以及 Wayland 截图和托盘检查；本轮没有在 GitHub 执行或发布这些工作流。修改后的工作流仍保留原有依赖安全检查，本机没有额外执行 `cargo audit`。
