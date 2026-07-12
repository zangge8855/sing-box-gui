use sysproxy::Sysproxy;
use std::sync::atomic::{AtomicBool, Ordering};

static SYSTEM_PROXY_OWNED: AtomicBool = AtomicBool::new(false);

pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    let proxy = Sysproxy {
        enable,
        host: "127.0.0.1".into(),
        port,
        bypass: "localhost;127.0.0.1;::1;<local>".into(),
    };
    
    proxy.set_system_proxy()
        .map_err(|e| format!("Failed to set system proxy: {}", e))?;

    SYSTEM_PROXY_OWNED.store(enable, Ordering::Release);
        
    Ok(())
}

/// Whether this process successfully enabled the current system proxy.
/// A proxy that was already active before launch is deliberately not owned.
pub fn is_system_proxy_owned() -> bool {
    SYSTEM_PROXY_OWNED.load(Ordering::Acquire)
}

pub fn check_system_proxy(port: u16) -> Result<bool, String> {
    let proxy = Sysproxy::get_system_proxy()
        .map_err(|e| format!("Failed to get system proxy: {}", e))?;
    Ok(proxy.enable && (proxy.host == "127.0.0.1" || proxy.host == "localhost" || proxy.host == "::1") && proxy.port == port)
}
