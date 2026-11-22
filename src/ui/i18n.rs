use crate::state::Language;

/// UI keys introduced/required by layout & UX polish (must exist in both En and Zh).
/// Used by unit tests; kept public so audits can list required keys.
#[allow(dead_code)]
pub const REQUIRED_POLISH_KEYS: &[&str] = &[
    "btn_more",
    "btn_less",
    "btn_edit",
    "btn_edit_url",
    "btn_show_preview",
    "btn_hide_preview",
    "traffic_used",
    "traffic_left",
    "chart_legend_down",
    "chart_legend_up",
    "tun_admin_banner",
    "core_not_running_hint",
    "no_proxy_groups",
    "toast_start_core_first",
    "toast_no_nodes",
    "toast_latency_done",
    "toast_clipboard_empty",
    "chart_waiting_traffic",
    "chart_now",
    "chart_seconds_ago",
    "sys_proxy_desc",
    "btn_start_core_short",
    "btn_clear_search",
    "btn_goto_dashboard",
    "btn_goto_profiles",
    "btn_goto_settings",
    "proxies_no_profile_hint",
    "proxies_no_core_hint",
    "btn_reinstall_core",
    "latency_test_url_label",
    "col_process",
    "empty_connections_hint",
    "empty_logs_start_hint",
    "status_starting",
    "status_stopping",
    "core_starting_hint",
    "toast_sub_ok",
    "toast_profile_deleted",
    "toast_core_stopped_profile_deleted",
    "btn_download_install",
    "status_downloading_update",
    "toast_update_downloading",
    "toast_update_installing",
    "sort_connections_by",
    "sort_nodes_by",
    "sort_latency",
    "sort_name",
    "sort_original",
    "nodes_unit",
    "protocols_unit",
    "groups_unit",
    "invalid_domain_rule",
    "invalid_ip_rule",
    "duplicate_rule",
    "invalid_dns_server",
    "port_empty_error",
    "port_invalid_error",
    "port_reserved_error",
    "port_conflict_error",
    "settings_saved_toast",
    "profile_selected_toast",
    "sort_host",
    "sort_process",
    "sort_network",
    "sort_rule",
    "sort_download",
    "sort_upload",
    "connections_hidden_hint",
    "btn_add_rule",
    "no_custom_rules",
    "appearance_settings",
    "settings_pending",
    "btn_discard_changes",
    "settings_invalid",
    "invalid_latency_url",
    "invalid_latency_timeout",
    "invalid_core_path",
    "core_path_default_draft",
    "config_save_failed",
    "autostart_apply_failed",
    "system_proxy_apply_failed",
    "update_apply_failed",
    "profile_delete_failed",
    "profile_name_required",
    "profile_url_invalid",
    "profile_url_update_required",
    "core_downloading",
    "core_verifying",
    "core_extracting",
    "core_installing",
    "core_install_success",
    "core_install_failed",
    "btn_retry_core",
    "core_download_requires_stop",
    "core_start_failed",
    "tray_show_window",
    "tray_start_core",
    "tray_stop_core",
    "tray_rules",
    "tray_global",
    "tray_direct",
    "tray_system_proxy",
    "tray_proxy_mode",
    "tray_exit",
    "clipboard_read_failed",
    "core_download_success",
    "core_not_running",
    "all_connections_closed",
    "routing_mode_pending",
    "routing_mode_switched",
    "routing_mode_failed",
    "logs_exported",
    "update_download_failed",
    "operation_in_progress",
    "status_installing_update",
    "update_asset_missing",
    "update_digest_missing",
    "update_size_missing",
    "logs_export_failed",
    "profile_download_failed",
    "core_unexpected_exit",
    "connection_close_failed",
    "connections_close_failed",
    "dash_current_profile",
    "import_formats_hint",
    "rules_direct_title",
    "rules_direct_desc",
    "rules_proxy_title",
    "rules_proxy_desc",
    "placeholder_rule",
    "rule_tag_domain",
    "rule_tag_ip",
    "settings_proxy",
    "settings_advanced",
    "settings_core",
    "help_mixed_port",
    "help_api_port",
    "help_dns_local",
    "help_dns_remote",
    "data_dir_label",
    "help_data_dir",
    "btn_open",
    "core_status_installed",
    "core_status_missing",
    "app_version_current",
    "help_config_preview",
    "btn_updating",
    "btn_import_profile_first",
    "btn_install_core_first",
    "mode_switching_hint",
    "onboarding_title",
    "onboarding_subtitle",
    "onboarding_step1_title",
    "onboarding_step1_ready",
    "onboarding_step1_action",
    "onboarding_step2_title",
    "onboarding_step2_ready",
    "onboarding_step3_title",
    "onboarding_step3_ready",
    "onboarding_step3_waiting",
    "fact_port",
    "showing_nodes_cap",
    "showing_nodes_all",
    "btn_show_all",
    "btn_show_less",
    "appearance_instant_hint",
    "settings_behavior",
    "btn_browse",
    "btn_copy",
    "toast_copied",
    "btn_reset_filters",
    "logs_count_fmt",
    "profile_kind_local",
    "native_json_ignored_toast",
    "ignored_custom_rules",
    "ignored_dns_fake_ip",
    "update_dir_readonly",
    "core_start_cancelled",
    "detached_core_detected",
    "tun_privilege_required",
    "connections_unit",
    "logs_live",
    "logs_paused",
    "ip_stack_label",
    "help_ip_stack",
    "ip_stack_prefer_ipv4",
    "ip_stack_prefer_ipv6",
    "ip_stack_ipv4_only",
    "no_logs_running_title",
    "no_logs_running_hint",
    "empty_connections_no_profile_hint",
    "col_traffic",
    "sort_asc_tooltip",
    "sort_desc_tooltip",
];

pub fn tr(lang: Language, key: &'static str) -> &'static str {
    match lang {
        Language::En => match key {
            // Tabs
            "tab_dashboard" => "Dashboard",
            "tab_proxies" => "Proxies",
            "tab_profiles" => "Profiles",
            "tab_logs" => "Logs",
            "tab_settings" => "Settings",

            "ON" => "ON",
            "OFF" => "OFF",
            "total_label" => "Total",
            "direct_chain" => "Direct",
            "placeholder_connections_search" => "Search host, IP, rule, proxy chain…",
            "btn_confirm_delete" => "Confirm Delete",
            "close_all_conn" => "Close All",
            "core_path_label" => "Custom Core Path",
            "placeholder_core_path" => "Leave empty to use built-in bin/sing-box",
            "btn_clear_core_path" => "Use Default Core",
            "help_core_path" => {
                "Absolute path to sing-box executable. Empty = managed core under data folder."
            }
            "help_tun_mode" => "Captures all traffic via virtual NIC. Requires Administrator.",
            "help_autostart" => "Launch this app when you sign in to your desktop.",
            "help_close_core" => "Stop sing-box process when the GUI exits.",
            "help_auto_start_core" => "Start core automatically if a profile is active.",
            "help_auto_sys_proxy" => {
                "Points the system proxy at the local proxy port once the core is up."
            }
            "help_tcp_fast_open" => "Enable TCP Fast Open on supported outbounds.",
            "help_tcp_multipath" => "Enable Multipath TCP when the OS supports it.",
            "help_fake_ip" => "Resolve remote domains to fake IPs for better routing.",
            "help_ip_stack" => "Dual-stack also proxies IPv6. Pick IPv4 only if IPv6 sites fail.",
            "disable_proxy_on_stop_label" => "Disable System Proxy on Core Stop",
            "help_disable_proxy_on_stop" => {
                "Turn off system proxy when the core is stopped. App exit always clears the system proxy for safety."
            }
            "auto_update_interval" => "Subscription Auto-Update",
            "help_auto_update" => "Periodically refresh remote subscription URLs.",
            "auto_update_off" => "Off",
            "auto_update_6h" => "Every 6 hours",
            "auto_update_12h" => "Every 12 hours",
            "auto_update_24h" => "Every 24 hours",
            "core_version_label" => "Core version",
            "btn_import_clipboard" => "From Clipboard",
            "btn_import_file" => "Import File",
            "expire_at_label" => "Expires",
            "empty_profiles_hint" => {
                "Paste a link above, or import from the clipboard or a local file."
            }
            "empty_dashboard_title" => "No active profile yet",
            "empty_dashboard_desc" => "Install the core and import a profile to get started.",
            "btn_goto_dashboard" => "Go to Dashboard",
            "btn_goto_profiles" => "Go to Profiles",
            "btn_goto_settings" => "Open Settings",
            "proxies_no_profile_hint" => "Import or select a profile to view proxy nodes.",
            "proxies_no_core_hint" => {
                "Install the sing-box core in Settings to start and load proxy groups."
            }
            "dash_connections" => "Active Connections",
            "dash_no_node" => "Not selected",
            "log_search_placeholder" => "Filter log text…",
            "export_logs" => "Export",
            "log_filter_all" => "All",
            "log_filter_info" => "Info+",
            "log_filter_warn" => "Warn+",
            "log_filter_error" => "Error",
            "no_matching_logs" => "No log lines match the current filter.",
            "rules_builtin_desc" => {
                "Rule mode already bypasses private LAN and China mainland (geosite-cn / geoip-cn). Custom rules below are applied on top and restart the core automatically."
            }

            // Dashboard
            "status_running" => "Running",
            "status_stopped" => "Stopped",
            "status_starting" => "Starting…",
            "status_stopping" => "Stopping…",
            "core_starting_hint" => "Validating config and starting core…",
            "toast_sub_ok" => "Subscription downloaded successfully",
            "toast_profile_deleted" => "Profile deleted",
            "toast_core_stopped_profile_deleted" => {
                "Core stopped because the active profile was deleted"
            }
            "system_proxy" => "System Proxy",
            "enabled" => "Enabled",
            "disabled" => "Disabled",
            "upload" => "Upload",
            "download" => "Download",
            "btn_stop_core" => "Stop Core",
            "btn_start_core" => "Start Core",
            "speed_history_chart" => "Live Traffic",
            "singbox_core" => "sing-box Core",

            // Proxies
            "proxy_nodes" => "Proxies",
            "test_latency" => "Test Latency",
            "testing_latency" => "Testing…",
            "no_nodes" => {
                "No nodes found in the active profile. Please select or update a profile."
            }
            "search_nodes_placeholder" => "Search nodes by name or server…",
            "no_matching_nodes" => "No nodes match your search query.",
            "latency_timeout" => "Timeout",
            "nodes_unit" => "nodes",
            "protocols_unit" => "protocols",
            "groups_unit" => "groups",
            "invalid_domain_rule" => {
                "Enter a valid domain (example.com, *.example.com) or IP/CIDR (10.0.0.0/8)."
            }
            "invalid_ip_rule" => "Enter a valid IPv4/IPv6 address or CIDR range.",
            "duplicate_rule" => "This rule already exists.",
            "invalid_dns_server" => "Enter a valid DNS server address, hostname, or DNS URL.",
            "port_empty_error" => "Port numbers cannot be empty.",
            "port_invalid_error" => "Ports must be valid numbers between 1 and 65535.",
            "appearance_settings" => "Appearance",
            "port_reserved_error" => "Ports below 1024 are reserved; choose higher ports.",
            "port_conflict_error" => "Proxy and controller ports must be different.",
            "settings_saved_toast" => "Settings saved and applied.",
            "profile_selected_toast" => "Active profile updated.",
            "settings_pending" => "You have unsaved changes",
            "settings_invalid" => "Please fix the highlighted settings.",
            "invalid_latency_url" => "Enter a valid HTTP(S) latency URL.",
            "invalid_latency_timeout" => "Latency timeout must be between 500 and 30000 ms.",
            "invalid_core_path" => "Core path must point to an existing executable file.",
            "core_path_default_draft" => "Default core path will be used after saving.",
            "config_save_failed" => "Could not save settings",
            "autostart_apply_failed" => "Could not update auto-start",
            "system_proxy_apply_failed" => "Could not update system proxy",
            "update_apply_failed" => "Could not apply update",
            "profile_delete_failed" => "Could not delete profile file",
            "profile_name_required" => "Profile name cannot be empty.",
            "profile_url_invalid" => "Enter a local file path or HTTP(S) URL.",
            "profile_url_update_required" => "URL changed; click Update to fetch the new profile.",
            "core_downloading" => "Downloading core…",
            "core_verifying" => "Verifying SHA-256…",
            "core_extracting" => "Extracting core…",
            "core_installing" => "Installing core…",
            "core_install_success" => "Core installed successfully",
            "core_install_failed" => "Core installation failed",
            "btn_retry_core" => "Retry",
            "core_download_requires_stop" => "Stop the core before downloading or reinstalling it.",
            "core_start_failed" => "Core failed to start",
            "tray_show_window" => "Show Window",
            "tray_start_core" => "Start Core",
            "tray_stop_core" => "Stop Core",
            "tray_rules" => "Rules",
            "tray_global" => "Global",
            "tray_direct" => "Direct",
            "tray_system_proxy" => "System Proxy",
            "tray_proxy_mode" => "Proxy Mode",
            "tray_exit" => "Exit",
            "clipboard_read_failed" => "Could not read clipboard",
            "core_download_success" => "Core downloaded successfully",
            "core_not_running" => "Core is not running",
            "all_connections_closed" => "All connections closed",
            "routing_mode_pending" => "Routing mode set to {} (applies on next start)",
            "routing_mode_switched" => "Routing mode: {}",
            "routing_mode_failed" => "Failed to set mode: {}",
            "logs_exported" => "Logs exported: {}",
            "update_download_failed" => "Update download failed: {}",
            "operation_in_progress" => "Another core, proxy, or update operation is in progress.",
            "status_installing_update" => "Installing update…",
            "update_asset_missing" => {
                "Automatic installation is unavailable: the official platform asset was not found."
            }
            "update_digest_missing" => {
                "Automatic installation is unavailable: GitHub did not provide a SHA-256 digest."
            }
            "update_size_missing" => {
                "Automatic installation is unavailable: GitHub did not provide the asset size."
            }
            "logs_export_failed" => "Could not export logs",
            "profile_download_failed" => "Could not download or import profile",
            "core_unexpected_exit" => "Core stopped unexpectedly",
            "connection_close_failed" => "Could not close connection",
            "node_select_failed" => "Failed to switch node",
            "logs_follow" => "Follow",
            "proxy_applying" => "Applying…",
            "connections_close_failed" => "Could not close all connections",

            // Profiles
            "btn_download" => "Import",
            "btn_downloading" => "Importing…",
            "btn_updating" => "Updating…",
            "sub_url_placeholder" => "Paste a subscription URL, share link or config",
            "btn_select" => "Use",
            "btn_delete" => "Delete",
            "active_profile" => "In use",
            "import_sub" => "Import Profile",
            "no_profiles" => "No profiles yet",
            "updated_at_label" => "Updated",

            // Rules
            "tab_rules" => "Routing Rules",
            "btn_add_rule" => "Add",
            "no_custom_rules" => "No custom rules yet",

            // Logs
            "clear_logs" => "Clear",
            "no_logs" => "No logs yet",
            "no_logs_running_title" => "Log buffer is empty",
            "no_logs_running_hint" => "Core is running — new log lines will appear here.",

            // Settings
            "mixed_port" => "Mixed Port",
            "api_port" => "Controller Port",
            "dns_servers" => "DNS",
            "dns_local" => "DNS Local (Direct)",
            "dns_remote" => "DNS Remote (Proxy)",
            "app_language" => "App Language",
            "app_theme" => "App Theme",
            "theme_auto" => "Follow System",
            "theme_dark" => "Dark Mode",
            "theme_light" => "Light Mode",
            "btn_download_core" => "Download & Install sing-box Core",
            "mixed_port_label" => "Proxy Port",
            "api_port_label" => "Controller Port",
            "tun_mode_label" => "TUN Mode",
            "autostart_label" => "Start on Boot",
            "btn_save_apply" => "Save & Apply",
            "btn_discard_changes" => "Discard",
            "fake_ip_label" => "Fake-IP DNS Mode",
            "ip_stack_label" => "IP Stack",
            "ip_stack_prefer_ipv4" => "Dual · IPv4 first",
            "ip_stack_prefer_ipv6" => "Dual · IPv6 first",
            "ip_stack_ipv4_only" => "IPv4 only",
            "tcp_fast_open_label" => "TCP Fast Open (TFO)",
            "tcp_multipath_label" => "TCP Multi-Path (MPTCP)",
            "close_core_on_exit_label" => "Auto-Close Core on Exit",
            "auto_start_core_label" => "Auto-Start Core on Launch",
            "auto_sys_proxy_label" => "Auto-Enable System Proxy on Start",
            "mode_rules" => "Rules",
            "mode_global" => "Global",
            "mode_direct" => "Direct",
            "mode_rules_desc" => "Bypass LAN & China Mainland",
            "mode_global_desc" => "Proxy all network traffic",
            "mode_direct_desc" => "Bypass all proxy servers",

            "active_mode" => "Routing Mode",
            "listen_port" => "Listen Port",
            "btn_update" => "Update",
            "imported_profiles" => "Imported Profiles",
            "btn_open_folder" => "Open Folder",
            "btn_edit" => "Open Config",
            "btn_edit_url" => "Edit Info",
            "edit_link_title" => "Edit Profile Info",
            "btn_save" => "Save",
            "btn_cancel" => "Cancel",
            "btn_more" => "More",
            "btn_less" => "Less",
            "placeholder_profile_name" => "Profile Name",
            "placeholder_profile_url" => "Subscription URL or Path",
            "core_config_preview" => "Configuration Preview (Read-only)",
            "btn_show_preview" => "Show Config Preview",
            "btn_hide_preview" => "Hide Config Preview",
            "traffic_used" => "used",
            "traffic_left" => "left",
            "chart_legend_down" => "Download",
            "chart_legend_up" => "Upload",
            "chart_waiting_traffic" => "Waiting for traffic…",
            "chart_now" => "Now",
            "chart_seconds_ago" => "{}s ago",
            "sys_proxy_desc" => "Point system HTTP/HTTPS traffic at the local mixed port.",
            "btn_start_core_short" => "Start Core",
            "btn_clear_search" => "Clear Search",
            "btn_reinstall_core" => "Reinstall Core",
            "latency_test_url_label" => "Latency Test URL",
            "latency_test_timeout_label" => "Latency Timeout (ms)",
            "help_latency_test" => {
                "Requested when testing node latency; a lightweight 204 page works best."
            }
            "col_process" => "Process",
            "col_traffic" => "Down / Up",
            "process_unknown" => "—",
            "empty_connections_hint" => "Start the core to see live connections.",
            "empty_connections_no_profile_hint" => {
                "Import and select a profile first, then start the core to see live connections."
            }
            "empty_logs_start_hint" => "Start the sing-box core to stream live logs here.",
            "tun_admin_banner" => {
                "TUN mode requires running as Administrator. Without elevation, the core may fail to start."
            }
            "core_not_running_hint" => "Start the core to load proxy groups and test latency.",
            "no_proxy_groups" => "No selector groups available from the core.",
            "toast_start_core_first" => "Start the core before testing latency",
            "toast_no_nodes" => "No nodes available for latency test",
            "toast_latency_done" => "Latency test complete",
            "toast_clipboard_empty" => "Clipboard is empty",

            // Connections
            "tab_connections" => "Connections",
            "host" => "Host / IP",
            "network" => "Network",
            "type" => "Type",
            "chains" => "Chains",
            "rule" => "Rule",
            "close_conn" => "Close",
            "no_active_connections" => "No active connections.",
            "no_matching_connections" => "No matching connections found.",
            "connections_hidden_hint" => {
                "additional connections hidden; refine the search to inspect them."
            }

            // Software Update
            "app_update" => "Software Update",
            "btn_check_update" => "Check for Updates",
            "status_not_checked" => "Not checked yet",
            "status_checking" => "Checking for updates…",
            "status_uptodate" => "Up to date",
            "status_new_version" => "New version available",
            "btn_view_release_on_github" => "View Release on GitHub",
            "btn_download_install" => "Download & Restart",
            "status_downloading_update" => "Downloading update…",
            "toast_update_downloading" => "Downloading update in the background…",
            "toast_update_installing" => "Installing update and restarting…",
            "status_check_failed" => "Update check failed",

            // Sorting
            "sort_connections_by" => "Sort by",
            "sort_nodes_by" => "Sort by",
            "sort_latency" => "Latency",
            "sort_name" => "Name",
            "sort_original" => "Original",
            "sort_host" => "Host",
            "sort_process" => "Process",
            "sort_network" => "Network",
            "sort_rule" => "Rule",
            "sort_download" => "Download",
            "sort_upload" => "Upload",
            "sort_asc_tooltip" => "Ascending — click to sort descending",
            "sort_desc_tooltip" => "Descending — click to sort ascending",

            // Layout redesign
            "dash_current_profile" => "Active Profile",
            "import_formats_hint" => {
                "Accepts subscription URLs, share links (vless://, hy2://, …), sing-box JSON, Clash/Mihomo YAML and Base64 — all converted to sing-box."
            }
            "rules_direct_title" => "Direct",
            "rules_direct_desc" => "Matching domains (and subdomains) and IPs skip the proxy.",
            "rules_proxy_title" => "Proxy",
            "rules_proxy_desc" => "Matching domains (and subdomains) and IPs always use the proxy.",
            "placeholder_rule" => "e.g. example.com, 10.0.0.0/8",
            "rule_tag_domain" => "Domain",
            "rule_tag_ip" => "IP",
            "settings_proxy" => "Proxy",
            "settings_advanced" => "Advanced",
            "settings_core" => "Core",
            "help_mixed_port" => "Shared HTTP and SOCKS5 port.",
            "help_api_port" => "Used for node switching, latency tests and stats.",
            "help_dns_local" => "Resolves China and direct domains.",
            "help_dns_remote" => "Resolves everything else through the proxy.",
            "data_dir_label" => "Data Folder",
            "help_data_dir" => "Profiles, settings, logs and the core live here.",
            "btn_open" => "Open",
            "core_status_installed" => "Installed",
            "core_status_missing" => "Not installed",
            "app_version_current" => "Installed {}",
            "help_config_preview" => "The full configuration handed to sing-box.",

            // UI polish additions
            "btn_import_profile_first" => "Import a profile first",
            "btn_install_core_first" => "Install core first",
            "mode_switching_hint" => "Applying mode…",
            "onboarding_title" => "Quick Setup",
            "onboarding_subtitle" => "Complete these steps right here to start connecting.",
            "onboarding_step1_title" => "1. Install sing-box Core",
            "onboarding_step1_ready" => "Core installed and ready",
            "onboarding_step1_action" => "Download Core",
            "onboarding_step2_title" => "2. Import Profile",
            "onboarding_step2_ready" => "Active profile",
            "onboarding_step3_title" => "3. Connect",
            "onboarding_step3_ready" => "Ready — click Power to start proxying.",
            "onboarding_step3_waiting" => "Finish the steps above to connect.",
            "fact_port" => "Port",
            "showing_nodes_cap" => "Showing {} of {} matching nodes",
            "showing_nodes_all" => "Showing all {} matching nodes",
            "btn_show_all" => "Show All",
            "btn_show_less" => "Show Top 120",
            "appearance_instant_hint" => "Applied immediately — no save required.",
            "settings_behavior" => "Startup & Updates",
            "btn_browse" => "Browse…",
            "btn_copy" => "Copy",
            "toast_copied" => "Copied to clipboard",
            "btn_reset_filters" => "Reset Filters",
            "logs_count_fmt" => "{} / {} lines",
            "profile_kind_local" => "Local / Share Link",
            "native_json_ignored_toast" => "Native sing-box JSON active; GUI settings skipped: {}",
            "ignored_custom_rules" => "Custom Rules",
            "ignored_dns_fake_ip" => "DNS / Fake-IP",
            "update_dir_readonly" => {
                "Install folder is read-only; opened release page to download and replace manually."
            }
            "core_start_cancelled" => "Core startup cancelled",
            "detached_core_detected" => {
                "Detected an active background sing-box instance; disconnect before re-starting."
            }
            "tun_privilege_required" => {
                "TUN mode requires administrator privileges or cap_net_admin on sing-box."
            }
            "connections_unit" => "connections",
            "logs_live" => "Live",
            "logs_paused" => "Paused",

            _ => key,
        },
        Language::Zh => match key {
            // Tabs
            "tab_dashboard" => "控制台",
            "tab_proxies" => "代理节点",
            "tab_profiles" => "配置管理",
            "tab_logs" => "实时日志",
            "tab_settings" => "系统设置",

            "ON" => "开启",
            "OFF" => "关闭",
            "total_label" => "累计",
            "direct_chain" => "直连",
            "placeholder_connections_search" => "搜索域名、IP、分流规则或代理链…",
            "btn_confirm_delete" => "确认删除",
            "close_all_conn" => "关闭全部",
            "core_path_label" => "自定义内核路径",
            "placeholder_core_path" => "留空则使用内置 bin/sing-box",
            "btn_clear_core_path" => "恢复默认内核",
            "help_core_path" => "sing-box 可执行文件的绝对路径。留空则使用数据目录下的托管内核。",
            "help_tun_mode" => "通过虚拟网卡接管全部流量，需要管理员权限。",
            "help_autostart" => "登录桌面时自动启动本程序。",
            "help_close_core" => "退出图形界面时关闭 sing-box 进程。",
            "help_auto_start_core" => "若已有激活配置，启动客户端时自动运行内核。",
            "help_auto_sys_proxy" => "内核就绪后，将系统代理指向本机代理端口。",
            "help_tcp_fast_open" => "在支持的出站上启用 TCP Fast Open。",
            "help_tcp_multipath" => "在系统支持时启用多径 TCP。",
            "help_fake_ip" => "将远程域名解析为虚假 IP，便于分流。",
            "help_ip_stack" => "双栈会同时代理 IPv6；IPv6 网站打不开时选仅 IPv4。",
            "disable_proxy_on_stop_label" => "停止内核时关闭系统代理",
            "help_disable_proxy_on_stop" => {
                "关闭内核时自动关闭系统代理。退出应用时始终清除系统代理（安全策略）。"
            }
            "auto_update_interval" => "订阅自动更新",
            "help_auto_update" => "按间隔自动刷新远程订阅链接。",
            "auto_update_off" => "关闭",
            "auto_update_6h" => "每 6 小时",
            "auto_update_12h" => "每 12 小时",
            "auto_update_24h" => "每 24 小时",
            "core_version_label" => "内核版本",
            "btn_import_clipboard" => "从剪贴板导入",
            "btn_import_file" => "导入本地文件",
            "expire_at_label" => "到期",
            "empty_profiles_hint" => "在上方粘贴链接，或从剪贴板 / 本地文件导入。",
            "empty_dashboard_title" => "尚未选择配置",
            "empty_dashboard_desc" => "先安装内核并导入配置，即可开始使用。",
            "btn_goto_dashboard" => "前往控制台",
            "btn_goto_profiles" => "前往配置管理",
            "btn_goto_settings" => "打开系统设置",
            "proxies_no_profile_hint" => "请先导入或选择一个配置以查看代理节点。",
            "proxies_no_core_hint" => "请先在系统设置中安装 sing-box 内核，再启动以加载策略组。",
            "dash_connections" => "活动连接数",
            "dash_no_node" => "未选择节点",
            "log_search_placeholder" => "搜索日志内容…",
            "export_logs" => "导出",
            "log_filter_all" => "全部",
            "log_filter_info" => "Info+",
            "log_filter_warn" => "Warn+",
            "log_filter_error" => "Error",
            "no_matching_logs" => "没有匹配当前筛选条件的日志。",
            "rules_builtin_desc" => {
                "规则模式下已默认绕过局域网与中国大陆（geosite-cn / geoip-cn）。下方自定义规则会叠加生效，修改后将重启内核应用。"
            }

            // Dashboard
            "status_running" => "正在运行",
            "status_stopped" => "已停止",
            "status_starting" => "启动中…",
            "status_stopping" => "停止中…",
            "core_starting_hint" => "正在校验配置并启动内核…",
            "toast_sub_ok" => "订阅下载/更新成功",
            "toast_profile_deleted" => "配置已删除",
            "toast_core_stopped_profile_deleted" => "已停止内核：当前使用的配置已被删除",
            "system_proxy" => "系统代理",
            "enabled" => "已启用",
            "disabled" => "已禁用",
            "upload" => "上传",
            "download" => "下载",
            "btn_stop_core" => "停止内核",
            "btn_start_core" => "启动内核",
            "speed_history_chart" => "实时流量",
            "singbox_core" => "sing-box 内核",

            // Proxies
            "proxy_nodes" => "代理节点",
            "test_latency" => "测试延迟",
            "testing_latency" => "测速中…",
            "no_nodes" => "活动配置中没有节点，请选择或更新配置。",
            "search_nodes_placeholder" => "按名称或服务器搜索节点…",
            "no_matching_nodes" => "未找到匹配搜索条件的节点。",
            "latency_timeout" => "超时",
            "nodes_unit" => "个节点",
            "protocols_unit" => "种协议",
            "groups_unit" => "个策略组",
            "invalid_domain_rule" => {
                "请输入有效的域名（如 example.com、*.example.com）或 IP / CIDR（如 10.0.0.0/8）。"
            }
            "invalid_ip_rule" => "请输入有效的 IPv4/IPv6 地址或 CIDR 网段。",
            "duplicate_rule" => "该规则已经存在。",
            "invalid_dns_server" => "请输入有效的 DNS 服务器 IP、主机名或 DNS URL。",
            "port_empty_error" => "端口号不能为空。",
            "port_invalid_error" => "端口必须是 1 到 65535 之间的有效数字。",
            "appearance_settings" => "外观与语言",
            "port_reserved_error" => "1024 以下为系统保留端口，请选择更高的端口。",
            "port_conflict_error" => "代理端口与控制端口不能相同。",
            "settings_saved_toast" => "设置已保存并应用。",
            "profile_selected_toast" => "已切换活动配置。",
            "settings_pending" => "有尚未保存的更改",
            "settings_invalid" => "请修正标红的设置项。",
            "invalid_latency_url" => "请输入有效的 HTTP(S) 延迟测试网址。",
            "invalid_latency_timeout" => "延迟超时必须在 500 到 30000 毫秒之间。",
            "invalid_core_path" => "内核路径必须指向已存在的可执行文件。",
            "core_path_default_draft" => "保存后将使用默认内核路径。",
            "config_save_failed" => "设置保存失败",
            "autostart_apply_failed" => "自启动设置失败",
            "system_proxy_apply_failed" => "系统代理设置失败",
            "update_apply_failed" => "更新应用失败",
            "profile_delete_failed" => "配置文件删除失败",
            "profile_name_required" => "配置名称不能为空。",
            "profile_url_invalid" => "请输入本地文件路径或 HTTP(S) 网址。",
            "profile_url_update_required" => "网址已修改，请点击“更新”重新抓取。",
            "core_downloading" => "正在下载内核…",
            "core_verifying" => "正在校验 SHA-256…",
            "core_extracting" => "正在解压内核…",
            "core_installing" => "正在安装内核…",
            "core_install_success" => "内核安装成功",
            "core_install_failed" => "内核安装失败",
            "btn_retry_core" => "重试",
            "core_download_requires_stop" => "下载或重装内核前请先停止内核。",
            "core_start_failed" => "内核启动失败",
            "tray_show_window" => "显示主界面",
            "tray_start_core" => "启动内核",
            "tray_stop_core" => "停止内核",
            "tray_rules" => "规则分流",
            "tray_global" => "全局代理",
            "tray_direct" => "直接连接",
            "tray_system_proxy" => "系统代理",
            "tray_proxy_mode" => "代理模式",
            "tray_exit" => "退出",
            "clipboard_read_failed" => "无法读取剪贴板",
            "core_download_success" => "内核下载安装成功",
            "core_not_running" => "内核未运行",
            "all_connections_closed" => "已关闭全部连接",
            "routing_mode_pending" => "路由模式已设为 {}（下次启动生效）",
            "routing_mode_switched" => "路由模式已切换为 {}",
            "routing_mode_failed" => "切换路由模式失败：{}",
            "logs_exported" => "日志已导出：{}",
            "update_download_failed" => "更新下载失败：{}",
            "operation_in_progress" => "另一个内核、代理或更新操作正在进行中。",
            "status_installing_update" => "正在安装更新…",
            "update_asset_missing" => "无法自动安装：未找到适用于当前平台的官方安装包。",
            "update_digest_missing" => "无法自动安装：GitHub 未提供 SHA-256 摘要。",
            "update_size_missing" => "无法自动安装：GitHub 未提供文件大小信息。",
            "logs_export_failed" => "日志导出失败",
            "profile_download_failed" => "配置下载或导入失败",
            "core_unexpected_exit" => "内核意外停止",
            "connection_close_failed" => "连接关闭失败",
            "node_select_failed" => "切换节点失败",
            "logs_follow" => "跟随",
            "proxy_applying" => "应用中…",
            "connections_close_failed" => "未能关闭全部连接",

            // Profiles
            "btn_download" => "导入",
            "btn_downloading" => "正在导入…",
            "btn_updating" => "正在更新…",
            "sub_url_placeholder" => "粘贴订阅链接、节点分享链接或配置内容",
            "btn_select" => "使用",
            "btn_delete" => "删除",
            "active_profile" => "使用中",
            "import_sub" => "导入配置",
            "no_profiles" => "还没有配置",
            "updated_at_label" => "更新于",

            // Rules
            "tab_rules" => "分流规则",
            "btn_add_rule" => "添加",
            "no_custom_rules" => "暂无自定义规则",

            // Logs
            "clear_logs" => "清空",
            "no_logs" => "暂无日志",
            "no_logs_running_title" => "日志缓冲区为空",
            "no_logs_running_hint" => "内核正在运行，新的实时日志将显示在此处。",

            // Settings
            "mixed_port" => "混合代理端口",
            "api_port" => "控制端口",
            "dns_servers" => "DNS",
            "dns_local" => "本地 DNS（直连）",
            "dns_remote" => "远程 DNS（代理）",
            "app_language" => "界面语言",
            "app_theme" => "应用主题",
            "theme_auto" => "跟随系统",
            "theme_dark" => "深色模式",
            "theme_light" => "浅色模式",
            "btn_download_core" => "下载并安装 sing-box 内核",
            "mixed_port_label" => "代理端口",
            "api_port_label" => "控制端口",
            "tun_mode_label" => "TUN 虚拟网卡模式",
            "autostart_label" => "开机自启",
            "btn_save_apply" => "保存并应用",
            "btn_discard_changes" => "放弃更改",
            "fake_ip_label" => "Fake-IP DNS 模式",
            "ip_stack_label" => "IP 协议栈",
            "ip_stack_prefer_ipv4" => "双栈 · IPv4 优先",
            "ip_stack_prefer_ipv6" => "双栈 · IPv6 优先",
            "ip_stack_ipv4_only" => "仅 IPv4",
            "tcp_fast_open_label" => "TCP 快速打开（TFO）",
            "tcp_multipath_label" => "TCP 多径传输（MPTCP）",
            "close_core_on_exit_label" => "退出时自动关闭内核",
            "auto_start_core_label" => "客户端启动时自动运行内核",
            "auto_sys_proxy_label" => "内核启动时自动启用系统代理",
            "mode_rules" => "规则分流",
            "mode_global" => "全局代理",
            "mode_direct" => "直接连接",
            "mode_rules_desc" => "自动分流，绕过局域网及大陆地区",
            "mode_global_desc" => "所有流量接管，全部通过代理访问",
            "mode_direct_desc" => "直接连接网络，不通过代理服务器",

            "active_mode" => "路由模式",
            "listen_port" => "监听端口",
            "btn_update" => "更新",
            "imported_profiles" => "已导入的配置",
            "btn_open_folder" => "打开目录",
            "btn_edit" => "打开配置",
            "btn_edit_url" => "编辑信息",
            "edit_link_title" => "修改配置信息",
            "btn_save" => "保存",
            "btn_cancel" => "取消",
            "btn_more" => "更多",
            "btn_less" => "收起",
            "placeholder_profile_name" => "配置名称",
            "placeholder_profile_url" => "订阅链接或文件路径",
            "core_config_preview" => "内核配置预览（只读）",
            "btn_show_preview" => "显示配置预览",
            "btn_hide_preview" => "隐藏配置预览",
            "traffic_used" => "已用",
            "traffic_left" => "剩余",
            "chart_legend_down" => "下载",
            "chart_legend_up" => "上传",
            "chart_waiting_traffic" => "等待流量数据…",
            "chart_now" => "此刻",
            "chart_seconds_ago" => "{} 秒前",
            "sys_proxy_desc" => "让系统 HTTP/HTTPS 流量经由本地混合端口转发。",
            "btn_start_core_short" => "启动内核",
            "btn_clear_search" => "清除搜索",
            "btn_reinstall_core" => "重新安装内核",
            "latency_test_url_label" => "测速 URL",
            "latency_test_timeout_label" => "测速超时（毫秒）",
            "help_latency_test" => "节点测速时请求的地址，返回 204 的轻量页面最佳。",
            "col_process" => "进程",
            "col_traffic" => "下载 / 上传",
            "process_unknown" => "—",
            "empty_connections_hint" => "启动内核后可查看活动连接。",
            "empty_connections_no_profile_hint" => {
                "请先导入并选择配置，启动内核后即可查看活动连接。"
            }
            "empty_logs_start_hint" => "启动 sing-box 内核后将在此显示实时日志。",
            "tun_admin_banner" => "TUN 模式需要以管理员身份运行。未提权时内核可能启动失败。",
            "core_not_running_hint" => "请先启动内核以加载策略组并测速。",
            "no_proxy_groups" => "内核未返回可用的选择器策略组。",
            "toast_start_core_first" => "请先启动内核再测速",
            "toast_no_nodes" => "当前没有可测速的节点",
            "toast_latency_done" => "延迟测试完成",
            "toast_clipboard_empty" => "剪贴板为空",

            // Connections
            "tab_connections" => "连接管理",
            "host" => "目标主机 / IP",
            "network" => "网络",
            "type" => "类型",
            "chains" => "代理链",
            "rule" => "规则",
            "close_conn" => "断开",
            "no_active_connections" => "当前没有活动的连接。",
            "no_matching_connections" => "未找到匹配的活动连接。",
            "connections_hidden_hint" => "个额外连接已隐藏，请缩小搜索范围后查看。",

            // Software Update
            "app_update" => "软件更新",
            "btn_check_update" => "检查更新",
            "status_not_checked" => "尚未检查",
            "status_checking" => "正在检查更新…",
            "status_uptodate" => "已是最新版本",
            "status_new_version" => "检测到新版本",
            "btn_view_release_on_github" => "在 GitHub 上查看发布",
            "btn_download_install" => "下载并重启",
            "status_downloading_update" => "正在下载更新…",
            "toast_update_downloading" => "正在后台下载更新…",
            "toast_update_installing" => "正在安装更新并重启…",
            "status_check_failed" => "检查更新失败",

            // Sorting
            "sort_connections_by" => "排序依据",
            "sort_nodes_by" => "节点排序",
            "sort_latency" => "延迟优先",
            "sort_name" => "名称排序",
            "sort_original" => "原始顺序",
            "sort_host" => "目标域名",
            "sort_process" => "应用进程",
            "sort_network" => "网络协议",
            "sort_rule" => "分流规则",
            "sort_download" => "下载流量",
            "sort_upload" => "上传流量",
            "sort_asc_tooltip" => "当前升序，点击切换降序",
            "sort_desc_tooltip" => "当前降序，点击切换升序",

            // Layout redesign
            "dash_current_profile" => "当前配置",
            "import_formats_hint" => {
                "支持订阅链接、vless:// hy2:// 等分享链接、sing-box JSON、Clash/Mihomo YAML 与 Base64，统一转换为 sing-box 配置。"
            }
            "rules_direct_title" => "直连",
            "rules_direct_desc" => "匹配的域名（含子域名）与 IP 不经过代理。",
            "rules_proxy_title" => "代理",
            "rules_proxy_desc" => "匹配的域名（含子域名）与 IP 始终经过代理。",
            "placeholder_rule" => "如 example.com, 10.0.0.0/8，可一次添加多条",
            "rule_tag_domain" => "域名",
            "rule_tag_ip" => "IP",
            "settings_proxy" => "代理",
            "settings_advanced" => "高级",
            "settings_core" => "内核",
            "help_mixed_port" => "HTTP 与 SOCKS5 共用端口。",
            "help_api_port" => "供切换节点、测速与统计使用。",
            "help_dns_local" => "解析国内及直连域名。",
            "help_dns_remote" => "经代理解析其他域名。",
            "data_dir_label" => "数据目录",
            "help_data_dir" => "配置、设置、日志与内核都保存在这里。",
            "btn_open" => "打开",
            "core_status_installed" => "已安装",
            "core_status_missing" => "未安装",
            "app_version_current" => "当前版本 {}",
            "help_config_preview" => "即将交给 sing-box 的完整配置。",

            // UI polish additions
            "btn_import_profile_first" => "请先导入配置",
            "btn_install_core_first" => "请先安装内核",
            "mode_switching_hint" => "正在切换模式…",
            "onboarding_title" => "快速开始",
            "onboarding_subtitle" => "按顺序完成以下步骤即可开始连接。",
            "onboarding_step1_title" => "1. 安装 sing-box 内核",
            "onboarding_step1_ready" => "内核已就绪",
            "onboarding_step1_action" => "下载内核",
            "onboarding_step2_title" => "2. 导入配置或订阅",
            "onboarding_step2_ready" => "当前配置",
            "onboarding_step3_title" => "3. 启动连接",
            "onboarding_step3_ready" => "一切就绪，点击开关按钮即可启动代理。",
            "onboarding_step3_waiting" => "完成以上两步后即可启动连接。",
            "fact_port" => "端口",
            "showing_nodes_cap" => "当前显示前 {} 个（共 {} 个匹配节点）",
            "showing_nodes_all" => "已显示全部 {} 个匹配节点",
            "btn_show_all" => "显示全部",
            "btn_show_less" => "仅看前 120",
            "appearance_instant_hint" => "修改后立即生效，无需点击保存。",
            "settings_behavior" => "启动与更新",
            "btn_browse" => "浏览…",
            "btn_copy" => "复制",
            "toast_copied" => "已复制到剪贴板",
            "btn_reset_filters" => "重置筛选",
            "logs_count_fmt" => "{} / {} 条日志",
            "profile_kind_local" => "本地配置 / 分享链接",
            "native_json_ignored_toast" => "当前为原生 sing-box JSON 配置，以下 GUI 设置不生效：{}",
            "ignored_custom_rules" => "自定义分流规则",
            "ignored_dns_fake_ip" => "DNS 与 Fake-IP",
            "update_dir_readonly" => "安装目录为只读，已打开下载页面，请手动下载并替换程序。",
            "core_start_cancelled" => "已取消启动内核",
            "detached_core_detected" => {
                "检测到后台独立运行的 sing-box 内核，如需重启请先断开停止。"
            }
            "tun_privilege_required" => {
                "TUN 模式需要管理员权限或为 sing-box 授予 cap_net_admin 权限。"
            }
            "connections_unit" => "个连接",
            "logs_live" => "实时追踪",
            "logs_paused" => "已暂停",

            _ => key,
        },
    }
}

/// Error phrases produced by the networking, parsing, core and platform
/// layers, keyed by their English text the way a gettext catalog is. Those
/// layers stay language-neutral; [`localize_error`] translates at the
/// display boundary and leaves OS / library detail untouched.
const ERROR_CATALOG: &[(&str, &str)] = &[
    // Profile import and subscription download
    ("Profile source is empty", "配置来源为空"),
    (
        "Profile is larger than the 16 MiB safety limit",
        "配置超过 16 MiB 安全上限",
    ),
    (
        "Subscription is larger than the 16 MiB safety limit",
        "订阅内容超过 16 MiB 安全上限",
    ),
    (
        "Profile is neither valid UTF-8 nor GBK-compatible text",
        "配置不是有效的 UTF-8 或 GBK 文本",
    ),
    ("Download request failed", "下载请求失败"),
    ("Download failed with status", "服务器返回错误状态"),
    ("Failed to read local file", "读取本地文件失败"),
    ("Failed to read content", "读取内容失败"),
    ("Failed to read active profile", "读取当前配置失败"),
    ("Failed to read profile file", "读取配置文件失败"),
    ("Profile parse task failed", "配置解析任务失败"),
    ("Profile save task failed", "配置保存任务失败"),
    ("No active profile selected", "尚未选择配置"),
    (
        "Active profile config file not found!",
        "找不到当前配置文件",
    ),
    // Profile parsing
    ("Invalid JSON structure", "JSON 结构无效"),
    ("Invalid YAML structure", "YAML 结构无效"),
    ("Invalid Clash configuration format", "Clash 配置格式无效"),
    ("YAML parsing failed", "YAML 解析失败"),
    ("JSON parsing failed", "JSON 解析失败"),
    (
        "No 'proxies' key found in config file",
        "配置文件缺少 proxies 字段",
    ),
    (
        "No 'proxies' key found in Clash config",
        "Clash 配置缺少 proxies 字段",
    ),
    (
        "'proxies' must be a sequence/list",
        "proxies 字段必须是列表",
    ),
    ("Invalid native Sing-Box JSON", "原生 sing-box JSON 无效"),
    (
        "Native Sing-Box JSON must be a JSON object",
        "原生 sing-box JSON 必须是对象",
    ),
    (
        "Native sing-box JSON must be a configuration object",
        "原生 sing-box JSON 必须是配置对象",
    ),
    (
        "Native sing-box JSON must contain an 'outbounds' or 'endpoints' array",
        "原生 sing-box JSON 必须包含 outbounds 或 endpoints 数组",
    ),
    (
        "Native sing-box JSON contains only unsupported ShadowsocksR (shadowsocksr) outbounds, which were removed in sing-box 1.6.0.",
        "原生 sing-box JSON 只包含 ShadowsocksR 节点，sing-box 1.6.0 起已不再支持。",
    ),
    (
        "No supported proxy nodes remain after removing unsupported ShadowsocksR (shadowsocksr) outbounds, which were removed in sing-box 1.6.0.",
        "移除 ShadowsocksR 节点后没有可用节点（sing-box 1.6.0 起已不再支持 ShadowsocksR）。",
    ),
    (
        "Profile contained no parseable proxy nodes (unsupported schemes or bad links).",
        "未解析到任何代理节点（协议不支持或链接有误）。",
    ),
    (
        "No supported proxy nodes could be converted.",
        "没有可转换的代理节点。",
    ),
    ("Unsupported proxy types", "不支持的类型"),
    (
        "ShadowsocksR was removed in sing-box 1.6.0.",
        "sing-box 1.6.0 起已不再支持 ShadowsocksR。",
    ),
    (
        "Use a native sing-box JSON profile for endpoint-only or custom outbound types.",
        "仅 endpoint 或自定义出站类型请改用原生 sing-box JSON 配置。",
    ),
    // Core lifecycle
    ("Core not installed", "内核未安装"),
    (
        "sing-box core not found! Please download it or specify correct path in Settings.",
        "未找到 sing-box 内核！请先下载或在设置中指定正确路径。",
    ),
    ("Core startup cancelled", "已取消启动内核"),
    ("Exit code", "退出码"),
    (
        "Check the logs for FATAL/ERROR details.",
        "请在日志中查看 FATAL/ERROR 详情。",
    ),
    ("Failed to inspect core process", "无法读取内核进程状态"),
    ("Config check failed", "配置检查未通过"),
    ("Configuration check failed", "配置检查未通过"),
    ("Failed to run config check", "无法运行配置检查"),
    ("Failed to run sing-box check", "无法运行 sing-box 检查"),
    ("sing-box check failed", "sing-box 检查失败"),
    ("Failed to start process", "无法启动内核进程"),
    ("Failed to capture stdout/stderr", "无法捕获内核输出"),
    ("Failed to poll core process", "查询内核进程状态失败"),
    ("Failed to signal core process", "向内核进程发送信号失败"),
    ("Failed to create JobObject", "创建进程作业对象失败"),
    ("Failed to set JobObject info", "设置进程作业对象属性失败"),
    (
        "Failed to assign process to JobObject",
        "将进程加入作业对象失败",
    ),
    ("Failed to run core version", "无法读取内核版本"),
    ("Core version check failed", "内核版本检查失败"),
    ("Empty version output", "内核版本输出为空"),
    ("Failed to serialize final config", "生成运行配置失败"),
    // Core download and install
    (
        "Failed to fetch core release metadata",
        "获取内核版本信息失败",
    ),
    (
        "Failed to parse core release metadata",
        "解析内核版本信息失败",
    ),
    (
        "Core release metadata returned status",
        "内核版本信息返回错误状态",
    ),
    (
        "Official core asset not found",
        "未找到适用于本平台的官方内核",
    ),
    (
        "Core release asset URL is not an official sing-box download",
        "内核下载地址不是 sing-box 官方地址",
    ),
    (
        "Core release asset has an invalid size",
        "内核发布文件大小无效",
    ),
    (
        "Release asset contains an invalid SHA-256 digest",
        "发布文件包含无效的 SHA-256 摘要",
    ),
    ("Failed to download core", "下载内核失败"),
    (
        "Downloaded core archive exceeds the 256 MiB safety limit",
        "内核压缩包超过 256 MiB 安全上限",
    ),
    (
        "Downloaded core archive exceeds the expected release asset size",
        "下载的内核压缩包超过预期的发布文件大小",
    ),
    (
        "Core download size does not match release metadata",
        "内核下载大小与发布信息不符",
    ),
    ("Core download size mismatch", "内核下载大小不符"),
    (
        "Downloaded core binary has an invalid size",
        "下载的内核程序大小无效",
    ),
    (
        "Downloaded core binary size does not match archive entry",
        "下载的内核程序大小与压缩包记录不符",
    ),
    (
        "Core archive SHA-256 verification failed",
        "内核压缩包 SHA-256 校验失败",
    ),
    (
        "Core archive contains ambiguous duplicate binaries",
        "内核压缩包包含重复的可执行文件",
    ),
    ("Invalid zip archive", "zip 压缩包无效"),
    ("Failed to open temp zip", "打开临时 zip 文件失败"),
    ("Failed to read zip entry", "读取 zip 条目失败"),
    ("Failed to create staged core", "创建暂存内核文件失败"),
    ("Failed to flush staged core", "刷新暂存内核文件缓存失败"),
    ("Failed to open temp archive", "打开临时压缩包失败"),
    ("Failed to inspect tar.gz", "检查 tar.gz 压缩包失败"),
    ("Failed to read tar entry", "读取 tar 条目失败"),
    ("Invalid tar entry path", "tar 条目路径无效"),
    (
        "Failed to mark staged core executable",
        "设置暂存内核可执行权限失败",
    ),
    (
        "Core destination path has no parent directory",
        "内核目标路径没有上级目录",
    ),
    ("Failed to extract core", "解压内核失败"),
    ("Core extraction task failed", "内核解压任务失败"),
    (
        "Could not find sing-box.exe inside downloaded zip package",
        "下载的 zip 包中找不到 sing-box.exe",
    ),
    (
        "Could not find sing-box inside downloaded zip package",
        "下载的 zip 包中找不到 sing-box",
    ),
    (
        "Could not find sing-box inside downloaded tar package",
        "下载的 tar 包中找不到 sing-box",
    ),
    (
        "Downloaded file does not match this platform's binary format",
        "下载的文件与本平台的程序格式不符",
    ),
    (
        "Failed to install the downloaded core",
        "安装下载的内核失败",
    ),
    ("Failed to create bin directory", "创建内核目录失败"),
    // Clash API
    ("Invalid Clash API port", "Clash API 端口无效"),
    ("Proxy selector cannot be empty", "代理选择器不能为空"),
    ("Proxy node tag cannot be empty", "代理节点标签不能为空"),
    ("Connection ID cannot be empty", "连接 ID 不能为空"),
    ("Routing mode cannot be empty", "路由模式不能为空"),
    ("Failed to fetch proxies", "获取节点列表失败"),
    ("Proxy API returned an error", "节点接口返回错误"),
    ("Failed to parse proxies response", "解析节点列表响应失败"),
    ("Failed to select proxy", "切换节点失败"),
    ("Latency test request failed", "测速请求失败"),
    ("Failed to parse delay response", "解析测速响应失败"),
    ("Failed to fetch connections", "获取连接列表失败"),
    ("Connections API returned an error", "连接接口返回错误"),
    (
        "Failed to parse connections response",
        "解析连接列表响应失败",
    ),
    ("Failed to close connection", "关闭连接失败"),
    ("Failed to close all connections", "关闭全部连接失败"),
    ("Failed to set mode", "切换路由模式失败"),
    ("Server returned status code", "服务器返回状态码"),
    ("Server returned error code", "服务器返回错误码"),
    ("Server returned error status", "服务器返回错误状态"),
    ("Server returned error", "服务器返回错误"),
    ("API unavailable", "控制接口不可用"),
    // App update
    ("No GitHub release published yet", "尚未发布任何版本"),
    ("Failed to parse JSON", "解析响应失败"),
    (
        "Update asset is missing a valid SHA-256 digest",
        "更新文件缺少有效的 SHA-256 摘要",
    ),
    ("Update asset has an invalid size", "更新文件大小无效"),
    ("Invalid update URL", "更新地址无效"),
    (
        "Update asset size does not match release metadata",
        "更新文件大小与发布信息不符",
    ),
    (
        "Update asset exceeds its declared size limit",
        "更新文件超过声明的大小限制",
    ),
    (
        "Downloaded update size does not match release metadata",
        "下载的更新大小与发布信息不符",
    ),
    (
        "Downloaded update is too small to be a binary",
        "下载的更新文件过小，不是有效程序",
    ),
    (
        "Update SHA-256 verification failed",
        "更新文件 SHA-256 校验失败",
    ),
    (
        "Update URL is not an official sing-box-gui release asset",
        "更新地址不是 sing-box-gui 官方发布文件",
    ),
    (
        "Current executable has no parent directory",
        "当前程序没有上级目录",
    ),
    ("Failed to read download body", "读取下载数据失败"),
    ("Failed to create update file", "创建更新文件失败"),
    ("Failed to write update file", "写入更新文件失败"),
    ("Failed to flush update file", "刷新更新文件缓存失败"),
    ("Failed to sync update file", "同步更新文件到磁盘失败"),
    ("Failed to stat update file", "读取更新文件信息失败"),
    ("Failed to chmod update file", "设置更新文件权限失败"),
    ("Failed to write update script", "写入更新脚本失败"),
    ("Failed to stat update script", "读取更新脚本信息失败"),
    ("Failed to chmod update script", "设置更新脚本权限失败"),
    ("Failed to spawn update script", "启动更新脚本失败"),
    ("Failed to build HTTP client", "构建 HTTP 客户端失败"),
    (
        "Failed to build direct HTTP client",
        "构建直连 HTTP 客户端失败",
    ),
    (
        "In-app update is not supported on this platform",
        "此平台不支持应用内更新",
    ),
    ("Request failed", "请求失败"),
    // System proxy and autostart
    (
        "System proxy is not supported on this platform",
        "此平台不支持系统代理",
    ),
    (
        "Autostart is not supported on this platform",
        "此平台不支持开机自启",
    ),
    (
        "No enabled macOS network service was found",
        "未找到已启用的 macOS 网络服务",
    ),
    (
        "Failed to open Windows proxy settings",
        "无法打开 Windows 代理设置",
    ),
    (
        "Failed to read Windows proxy settings",
        "读取 Windows 代理设置失败",
    ),
    (
        "Failed to update Windows proxy server",
        "更新 Windows 代理服务器失败",
    ),
    (
        "Failed to update Windows proxy state",
        "更新 Windows 代理状态失败",
    ),
    ("Failed to disable Windows proxy", "关闭 Windows 代理失败"),
    ("Failed to run gsettings", "无法运行 gsettings"),
    (
        "Failed to open autostart registry key",
        "无法打开开机自启注册表项",
    ),
    (
        "Failed to write autostart registry",
        "写入开机自启注册表失败",
    ),
    (
        "Failed to delete autostart registry value",
        "删除开机自启注册表项失败",
    ),
    (
        "Failed to create autostart directory",
        "创建开机自启目录失败",
    ),
    ("Failed to remove autostart entry", "移除开机自启项失败"),
    ("Failed to remove LaunchAgent", "移除 LaunchAgent 失败"),
    (
        "Failed to serialize system proxy backup",
        "序列化系统代理备份失败",
    ),
    (
        "Failed to clear Windows proxy backup marker",
        "清除 Windows 代理备份标记失败",
    ),
    (
        "Failed to restore Windows bypass list",
        "恢复 Windows 代理例外列表失败",
    ),
    (
        "Failed to restore Windows proxy server",
        "恢复 Windows 代理服务器失败",
    ),
    (
        "Failed to restore Windows proxy state",
        "恢复 Windows 代理状态失败",
    ),
    (
        "Failed to back up Windows proxy state",
        "备份 Windows 代理状态失败",
    ),
    (
        "Failed to back up Windows proxy server",
        "备份 Windows 代理服务器失败",
    ),
    (
        "Failed to back up Windows bypass list",
        "备份 Windows 代理例外列表失败",
    ),
    (
        "Failed to mark Windows proxy backup",
        "标记 Windows 代理备份失败",
    ),
    (
        "Failed to update Windows proxy bypass list",
        "更新 Windows 代理例外列表失败",
    ),
    (
        "Failed to parse GNOME proxy port",
        "无法解析 GNOME 代理端口",
    ),
    (
        "Failed to parse macOS proxy port",
        "无法解析 macOS 代理端口",
    ),
    ("Invalid Linux proxy backup", "Linux 代理备份无效"),
    (
        "Invalid macOS proxy backup service",
        "macOS 代理备份中的网络服务无效",
    ),
    (
        "Invalid macOS proxy backup entries",
        "macOS 代理备份条目无效",
    ),
    ("Failed to run", "无法运行"),
    ("rollback failed", "回滚失败"),
    // Saving app data
    ("Failed to serialize config", "序列化配置失败"),
    (
        "Target file has no parent directory",
        "目标文件没有上级目录",
    ),
    ("Failed to create data directory", "创建数据目录失败"),
    ("Failed to create temporary file", "创建临时文件失败"),
    ("Failed to write temporary file", "写入临时文件失败"),
    ("Failed to flush temporary file", "临时文件写入磁盘失败"),
    (
        "Failed to restrict temporary file permissions",
        "限制临时文件权限失败",
    ),
    ("Failed to create recovery backup", "创建恢复备份失败"),
    ("Failed to commit temporary file", "替换目标文件失败"),
    // Shared
    ("Failed to resolve current executable", "无法定位当前程序"),
    ("permission denied", "权限不足"),
    ("Permission denied", "权限不足"),
    ("Access is denied.", "拒绝访问。"),
    ("timed out", "请求超时"),
];

/// Translate the known phrases of an error message for display. Chained
/// errors ("Failed to download core: Server returned error: 404") translate
/// link by link; anything the catalog does not know (OS errors, URLs,
/// sing-box's own output) passes through verbatim.
pub fn localize_error(lang: Language, message: &str) -> std::borrow::Cow<'_, str> {
    if lang == Language::En {
        return std::borrow::Cow::Borrowed(message);
    }
    let mut out = String::with_capacity(message.len() + 16);
    let mut rest = message;
    let mut changed = false;
    while let Some(c) = rest.chars().next() {
        let at_word_start = !out
            .chars()
            .next_back()
            .is_some_and(|p| p.is_ascii_alphanumeric());
        let hit = at_word_start
            .then(|| {
                ERROR_CATALOG
                    .iter()
                    .filter(|(en, _)| {
                        rest.starts_with(en)
                            && !rest[en.len()..]
                                .chars()
                                .next()
                                .is_some_and(|n| n.is_ascii_alphanumeric())
                    })
                    .max_by_key(|(en, _)| en.len())
            })
            .flatten();
        let Some((en, zh)) = hit else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
            continue;
        };
        changed = true;
        // A translated phrase carries CJK punctuation: "：" for a following
        // detail, and no stray space after a full stop.
        while out.ends_with(' ') && is_cjk_punct_end(&out[..out.len() - 1]) {
            out.pop();
        }
        // A translated sentence turns the ASCII full stop before it into "。".
        if zh.ends_with('。') && out.ends_with(". ") {
            out.truncate(out.len() - 2);
            out.push('。');
        }
        out.push_str(zh);
        rest = &rest[en.len()..];
        if let Some(after) = rest.strip_prefix(": ") {
            out.push('：');
            rest = after;
        } else if zh.ends_with('。') {
            rest = rest.trim_start_matches(' ');
        }
    }
    if changed {
        std::borrow::Cow::Owned(out)
    } else {
        std::borrow::Cow::Borrowed(message)
    }
}

fn is_cjk_punct_end(s: &str) -> bool {
    s.ends_with(['。', '：', '）'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Language;

    #[test]
    fn localize_error_translates_known_phrases_and_keeps_detail() {
        let en = "Download failed with status: 404 Not Found";
        assert_eq!(localize_error(Language::En, en), en);
        assert_eq!(
            localize_error(Language::Zh, en),
            "服务器返回错误状态：404 Not Found"
        );
        // Chained errors translate link by link; unknown detail passes through.
        assert_eq!(
            localize_error(
                Language::Zh,
                "Failed to download core: Server returned error: 503 Service Unavailable"
            ),
            "下载内核失败：服务器返回错误：503 Service Unavailable"
        );
        // Multi-sentence messages lose the ASCII sentence spacing.
        assert_eq!(
            localize_error(
                Language::Zh,
                "No supported proxy nodes could be converted. Unsupported proxy types: foo, bar. Use a native sing-box JSON profile for endpoint-only or custom outbound types."
            ),
            "没有可转换的代理节点。不支持的类型：foo, bar。仅 endpoint 或自定义出站类型请改用原生 sing-box JSON 配置。"
        );
        assert_eq!(
            localize_error(
                Language::Zh,
                "Exit code 1. Check the logs for FATAL/ERROR details."
            ),
            "退出码 1。请在日志中查看 FATAL/ERROR 详情。"
        );
        // Phrases only match on word boundaries.
        assert_eq!(localize_error(Language::Zh, "xtimed out"), "xtimed out");
        // Nothing recognised: borrowed, untouched.
        let raw = "FATAL[0000] start service: initialize inbound";
        assert!(matches!(
            localize_error(Language::Zh, raw),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn error_catalog_is_translated_and_unambiguous() {
        let mut seen = std::collections::HashSet::new();
        for (en, zh) in ERROR_CATALOG {
            assert!(seen.insert(*en), "duplicate catalog phrase: {en}");
            assert!(
                zh.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "untranslated catalog entry: {en}"
            );
            assert!(!zh.contains(": "), "ASCII colon in translation: {zh}");
        }
    }

    #[test]
    fn polish_keys_resolve_in_en_and_zh() {
        for key in REQUIRED_POLISH_KEYS {
            let en = tr(Language::En, key);
            let zh = tr(Language::Zh, key);
            assert_ne!(en, *key, "missing En translation for {key}");
            assert_ne!(zh, *key, "missing Zh translation for {key}");
            assert!(!en.is_empty());
            assert!(!zh.is_empty());
        }
    }

    #[test]
    fn en_and_zh_differ_for_user_facing_polish_keys() {
        // Spot-check that Chinese is not English copy-paste for key banners.
        assert_ne!(
            tr(Language::En, "tun_admin_banner"),
            tr(Language::Zh, "tun_admin_banner")
        );
        assert_ne!(tr(Language::En, "btn_more"), tr(Language::Zh, "btn_more"));
    }

    #[test]
    fn literal_ui_translation_keys_resolve_in_both_languages() {
        let sources = [
            include_str!("../main.rs"),
            include_str!("../tray.rs"),
            include_str!("dashboard.rs"),
            include_str!("profiles.rs"),
            include_str!("proxies.rs"),
            include_str!("connections.rs"),
            include_str!("logs.rs"),
            include_str!("rules.rs"),
            include_str!("settings.rs"),
            include_str!("mod.rs"),
            include_str!("toast.rs"),
            include_str!("util.rs"),
            include_str!("../smoke.rs"),
        ];
        let needles = [
            "tr(lang, \"",
            "self.tr(\"",
            "self.tr_detail(\"",
            "tr_detail(\"",
        ];
        let mut checked = std::collections::BTreeSet::new();

        for source in sources {
            for needle in needles {
                let mut rest = source;
                while let Some(start) = rest.find(needle) {
                    rest = &rest[start + needle.len()..];
                    let Some(end) = rest.find('"') else { break };
                    let key = &rest[..end];
                    rest = &rest[end + 1..];
                    if !checked.insert(key) {
                        continue;
                    }
                    let key: &'static str = Box::leak(key.to_string().into_boxed_str());
                    let en_str = tr(Language::En, key);
                    let zh_str = tr(Language::Zh, key);
                    assert_ne!(en_str, key, "missing English translation: {key}");
                    assert_ne!(zh_str, key, "missing Chinese translation: {key}");

                    let en_placeholders = en_str.matches("{}").count();
                    let zh_placeholders = zh_str.matches("{}").count();
                    assert_eq!(
                        en_placeholders, zh_placeholders,
                        "Placeholder count mismatch for key '{key}': EN has {en_placeholders}, ZH has {zh_placeholders}"
                    );
                }
            }
        }

        assert!(checked.len() > 80, "translation audit scanned too few keys");
    }

    #[test]
    fn all_en_and_zh_keys_and_placeholders_match() {
        let src = include_str!("i18n.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("i18n source");
        let en_block = src
            .split("Language::En => match key {")
            .nth(1)
            .and_then(|s| s.split("_ => key,").next())
            .expect("En block");
        let zh_block = src
            .split("Language::Zh => match key {")
            .nth(1)
            .and_then(|s| s.split("_ => key,").next())
            .expect("Zh block");

        let collect_keys = |block: &str| {
            let mut keys = Vec::new();
            for line in block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('"')
                    && let Some((raw_key, rest)) = trimmed[1..].split_once('"')
                    && rest.trim_start().starts_with("=>")
                {
                    keys.push(raw_key.to_string());
                }
            }
            keys
        };

        let en_keys = collect_keys(en_block);
        let zh_keys = collect_keys(zh_block);
        let en_set: std::collections::BTreeSet<_> = en_keys.iter().cloned().collect();
        let zh_set: std::collections::BTreeSet<_> = zh_keys.iter().cloned().collect();
        assert_eq!(en_keys.len(), en_set.len(), "duplicate keys in En");
        assert_eq!(zh_keys.len(), zh_set.len(), "duplicate keys in Zh");
        assert_eq!(en_set, zh_set, "En/Zh key mismatch");

        for key in &en_set {
            let key_static: &'static str = Box::leak(key.clone().into_boxed_str());
            let en = tr(Language::En, key_static);
            let zh = tr(Language::Zh, key_static);
            assert!(!en.is_empty(), "empty En value for {key}");
            assert!(!zh.is_empty(), "empty Zh value for {key}");
            assert_eq!(
                en.matches("{}").count(),
                zh.matches("{}").count(),
                "placeholder count mismatch for {key}"
            );
        }
    }

    #[test]
    fn localize_error_handles_api_and_update_errors() {
        let out = localize_error(
            Language::Zh,
            "Failed to parse proxies response: expected value",
        );
        assert!(out.starts_with("解析节点列表响应失败："), "{out}");
        let upd = localize_error(Language::Zh, "Failed to spawn update script: denied");
        assert!(upd.starts_with("启动更新脚本失败："), "{upd}");
    }

    #[test]
    fn localize_error_covers_new_core_api_and_platform_phrases() {
        let samples = [
            "Invalid Clash API port: 0",
            "Proxy selector cannot be empty",
            "Proxy node tag cannot be empty",
            "Connection ID cannot be empty",
            "Routing mode cannot be empty",
            "Downloaded core archive exceeds the expected release asset size: 123",
            "Downloaded core binary size does not match archive entry: 123",
            "Config check failed: sing-box check failed: exit status 1",
            "Core archive contains ambiguous duplicate binaries",
            "Failed to open temp zip: io error",
            "Failed to read zip entry: corrupt",
            "Failed to create staged core: denied",
            "Failed to flush staged core: disk full",
            "Could not find sing-box.exe inside downloaded zip package",
            "Could not find sing-box inside downloaded zip package",
            "Failed to open temp archive: io error",
            "Failed to inspect tar.gz: corrupt",
            "Failed to read tar entry: truncated",
            "Invalid tar entry path: ../bin",
            "Failed to mark staged core executable: permission denied",
            "Core destination path has no parent directory",
            "sing-box core not found! Please download it or specify correct path in Settings.",
            "Failed to remove LaunchAgent: permission denied",
        ];

        for sample in samples {
            assert_eq!(localize_error(Language::En, sample), sample);
            let zh = localize_error(Language::Zh, sample);
            assert_ne!(zh, sample, "phrase was not translated: {sample}");
            assert!(
                zh.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "translation lacks CJK characters for {sample}: {zh}"
            );
            assert!(
                !zh.contains(": "),
                "ASCII colon remained after localize_error for {sample}: {zh}"
            );
        }

        assert_eq!(
            localize_error(Language::Zh, "Invalid Clash API port: 0"),
            "Clash API 端口无效：0"
        );
        assert_eq!(
            localize_error(Language::Zh, "Config check failed: sing-box check failed"),
            "配置检查未通过：sing-box 检查失败"
        );
    }

    #[test]
    fn no_update_required_marker_or_unlocalized_zh_colons_in_ui_keys() {
        let src = include_str!("i18n.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("i18n source");
        let en_block = src
            .split("Language::En => match key {")
            .nth(1)
            .and_then(|s| s.split("_ => key,").next())
            .expect("En block");
        let zh_block = src
            .split("Language::Zh => match key {")
            .nth(1)
            .and_then(|s| s.split("_ => key,").next())
            .expect("Zh block");

        for line in en_block.lines().chain(zh_block.lines()) {
            let trimmed = line.trim();
            if trimmed.starts_with('"')
                && let Some((raw_key, rest)) = trimmed[1..].split_once('"')
                && rest.trim_start().starts_with("=>")
            {
                let key_static: &'static str = Box::leak(raw_key.to_string().into_boxed_str());
                for lang in [Language::En, Language::Zh] {
                    let val = tr(lang, key_static);
                    assert!(
                        !val.contains("UPDATE_REQUIRED"),
                        "stale UPDATE_REQUIRED marker in {key_static} ({lang:?})"
                    );
                }
                let zh_val = tr(Language::Zh, key_static);
                assert!(
                    !zh_val.contains(": "),
                    "ASCII ': ' in Zh translation for {key_static}: {zh_val}"
                );
            }
        }
    }
}
