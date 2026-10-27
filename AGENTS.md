# AGENTS.md

Notes for AI agents (and humans) working in this repository.

## Build / verify commands

The project does not ship a Makefile — use cargo directly.

- **Compile check (fast)**: `cargo check`
- **Unit tests**: `cargo test --quiet`
- **Lint**: `cargo clippy --quiet` (currently informative only; **not** `-D warnings`)
- **Wayland UI + tray**: `cargo build --features ui-smoke` then
  `python3 scripts/ui_smoke.py` (requires Weston, dbus-run-session, PyGObject).
  Repeat with `--scale 2`; `--no-tray` checks the missing-tray fallback.
- **Release build (Windows)**: `cargo build --release`

CI (`.github/workflows/build.yml`) runs `cargo test` (must pass) and
`cargo clippy` (non-fatal, surfaces warnings) before producing artifacts.

## Architecture overview

- `src/main.rs`: iced daemon `App` + Message handlers + GUI subscriptions
  (log/traffic/tray streams), window recreation on tray activation, and helpers (`is_running_elevated`,
  `normalize_version_tag`, `is_remote_version_newer`).
- `src/core.rs`: sing-box child lifecycle. Liveness is cached in
  `CORE_RUNNING_CACHED` so per-second UI ticks use `is_core_running_fast()`
  (`is_core_running()` re-takes the process lock — call it sparingly).
- `src/config.rs`: subscription/Clash parsing + sing-box JSON conversion +
  `mitigate_run_config()` startup footgun fixes.
- `src/api.rs`: Clash REST API client. `with_secret()` attaches the
  `experimental.clash_api.secret` from the generated `run_config.json`
  to every request, so native profiles with non-empty secrets still work.
- `src/tray.rs`: Linux StatusNotifierItem over D-Bus (no GTK event loop);
  Windows/macOS native tray menus on the window event-loop thread.
- `src/sysproxy.rs`: Windows registry, macOS networksetup, and Linux desktop
  proxy integration with ownership tracking and rollback.
- `src/smoke.rs`: offline UI fixtures, screenshots, and tray lifecycle checks,
  compiled only with the `ui-smoke` feature. Uses isolated application data.

## Conventions

- All user-facing strings (English + Chinese) must be added to BOTH
  `Language::En` and `Language::Zh` arms in `src/ui/i18n.rs`. The
  `polish_keys_resolve_in_en_and_zh` test enforces UPDATE_REQUIRED.
- The `windows_subsystem` attribute is gated by `cfg(target_os = "windows")`
  so macOS/Linux builds keep a console for stdio.
- `Mutex::lock().unwrap_or_else(|e| e.into_inner())` is the deliberate
  poison-recovery pattern across the codebase.
