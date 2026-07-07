use sysproxy::Sysproxy;

pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    let proxy = Sysproxy {
        enable,
        host: "127.0.0.1".into(),
        port,
        bypass: "localhost;127.0.0.1;::1;<local>".into(),
    };
    
    proxy.set_system_proxy()
        .map_err(|e| format!("Failed to set system proxy: {}", e))?;
        
    Ok(())
}

pub fn check_system_proxy(port: u16) -> Result<bool, String> {
    let proxy = Sysproxy::get_system_proxy()
        .map_err(|e| format!("Failed to get system proxy: {}", e))?;
    Ok(proxy.enable && (proxy.host == "127.0.0.1" || proxy.host == "localhost" || proxy.host == "::1") && proxy.port == port)
}
