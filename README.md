# sing-box GUI

[![Build](https://github.com/zangge8855/sing-box-gui/actions/workflows/build.yml/badge.svg)](https://github.com/zangge8855/sing-box-gui/actions/workflows/build.yml)
![Version](https://img.shields.io/badge/version-2026.7.13-6d5ce7.svg)

<p align="center">
  <img src="assets/logo.jpg" alt="sing-box GUI" width="112" height="112" />
</p>

A native-style, cross-platform desktop client for [sing-box](https://github.com/SagerNet/sing-box), built with Rust and Iced. It focuses on reliable profile conversion, low-overhead runtime monitoring, restrained desktop UI, and consistent behavior on Windows, macOS, and Linux.

中文说明见 [下方](#中文说明)。

## Highlights

- Native desktop layout with compact navigation, flat workspaces, responsive panes, light/dark themes, and system Sans Serif/CJK fallback.
- Dashboard traffic history, session totals, core state, routing mode, and system proxy state.
- Proxy selector groups, node search/sort, active-node state, bounded rendering, and parallel latency tests.
- Subscription import/update, raw sharing links, Base64 link bundles, Clash YAML conversion, and native sing-box JSON passthrough.
- Rule, Global, and Direct modes; DNS, TUN, remote rule sets, custom bypass/proxy domains, and Clash API integration.
- Bounded log/traffic channels and throttled UI updates to prevent long-running sessions from accumulating unbounded work.
- Atomic settings/profile writes, update confirmation, download size limits, and recoverable core startup errors.
- Windows Registry integration, macOS LaunchAgent integration, and Linux XDG autostart/system-proxy support.

## Import and protocol matrix

| Protocol | Sharing link | Clash YAML conversion | Native sing-box JSON |
| --- | :---: | :---: | :---: |
| Shadowsocks (SIP002, plugins) | Yes | Yes | Passthrough |
| ShadowsocksR | Yes | Yes | Passthrough |
| VMess | Yes | Yes | Passthrough |
| VLESS / Reality | Yes | Yes | Passthrough |
| Trojan | Yes | Yes | Passthrough |
| Hysteria | Yes | Yes | Passthrough |
| Hysteria2 | Yes | Yes | Passthrough |
| TUIC | Yes | Yes | Passthrough |
| AnyTLS | Yes | Yes | Passthrough |
| Naive | Yes | Yes | Passthrough |
| SSH | Yes | Yes | Passthrough |
| SOCKS / HTTP | Clash YAML | Yes | Passthrough |
| WireGuard endpoint and custom types | Native JSON | No fabricated legacy outbound | Passthrough |

Converted transport metadata includes WebSocket, HTTP/H2, HTTPUpgrade, QUIC, and gRPC. TLS conversion preserves SNI, insecure mode, ALPN, uTLS fingerprint, and Reality options where supported. Hysteria-family and TUIC imports preserve common bandwidth, obfuscation, port-hopping, congestion, UDP relay, 0-RTT, and heartbeat options.

Native sing-box JSON is the compatibility path for endpoint-only features such as current WireGuard endpoints and for custom/experimental objects that cannot be represented safely by Clash proxy entries. The application keeps the native object graph instead of inventing an invalid outbound.

## Platform builds

Every push and pull request is verified by GitHub Actions:

| Platform | Architectures | Artifact |
| --- | --- | --- |
| Windows | x86_64, ARM64 | `.exe` |
| macOS | Intel + Apple Silicon | universal binary |
| Linux | x86_64, ARM64 | native binary |

The workflow runs unit tests before release builds. Windows additionally runs formatting and Clippy checks. Release assets are published only for `v*` tags.

## Development

Rust 1.85 or newer is required by the Rust 2024 edition used by this project.

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run
```

Important modules:

- `src/config.rs` — profile validation, sharing-link parsing, Clash conversion, native JSON merging, DNS/routing generation.
- `src/core.rs` — sing-box download, startup validation, process lifecycle, bounded log forwarding.
- `src/api.rs` — Clash API, selector changes, latency tests, connections, and traffic polling.
- `src/ui/` — responsive desktop workspaces, localization, typography, and visual tokens.
- `src/sysproxy.rs` / `src/autostart.rs` — platform integration.

## 中文说明

sing-box GUI 是使用 Rust 与 Iced 编写的跨平台桌面客户端，强调稳定的配置转换、低开销运行监控，以及接近 Windows、macOS、Linux 原生软件的克制界面。

### 主要功能

- 紧凑侧边栏、扁平工作区、响应式分栏、亮色/深色主题；统一使用系统 Sans Serif 字体并保留中文字体回退，避免乱码和方框字。
- 仪表盘展示实时上下行、会话流量、内核状态、路由模式与系统代理状态。
- 支持策略组、节点搜索与排序、当前节点状态、并发测速；大订阅采用有界渲染，避免界面卡顿。
- 支持订阅更新、分享链接、Base64 链接集合、Clash YAML 转换及原生 sing-box JSON 直通。
- 支持 Rule / Global / Direct、DNS、TUN、远程规则集、自定义直连/代理域名和 Clash API。
- 日志与流量采用有界通道，界面更新限频，长时间运行不会无限堆积消息或渲染任务。
- 配置与订阅原子写入；应用更新需要确认；下载具有大小限制；内核启动失败会返回可操作的错误信息。
- 支持 Windows 注册表、macOS LaunchAgent、Linux XDG 自启动和各平台系统代理集成。

### 协议兼容性

分享链接与 Clash 转换支持 Shadowsocks（含 SIP002 插件）、ShadowsocksR、VMess、VLESS/Reality、Trojan、Hysteria、Hysteria2、TUIC、AnyTLS、Naive、SSH，以及 Clash 中的 SOCKS/HTTP 节点。可保留 WS、HTTP/H2、HTTPUpgrade、QUIC、gRPC、SNI、ALPN、uTLS 指纹、Reality、混淆、带宽、端口跳跃、UDP 转发、0-RTT、心跳及 SSH 私钥等常用参数。

新版 sing-box 的 WireGuard 属于 endpoint，而不是传统 outbound。此类 endpoint、实验性类型和自定义对象请使用原生 sing-box JSON；程序会保留原始对象结构，不会生成无效的旧式 WireGuard outbound。

### GitHub Actions

每次推送都会在 GitHub 上执行 Windows x64/ARM64、macOS Universal、Linux x64/ARM64 的单元测试和 Release 构建。普通分支构建只上传 Actions Artifacts；仅版本标签会创建 GitHub Release。

## License

[GNU General Public License v3.0](LICENSE)
