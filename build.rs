fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    #[cfg(target_os = "windows")]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            const ICON_PATH: &str = "assets/icon.ico";
            println!("cargo:rerun-if-changed={ICON_PATH}");
            let mut res = winres::WindowsResource::new();
            if std::path::Path::new(ICON_PATH).is_file() {
                res.set_icon(ICON_PATH);
            } else {
                println!(
                    "cargo:warning=sing-box-gui: missing {ICON_PATH}; \
                     the .exe will use the default icon."
                );
            }
            if let Err(e) = res.compile() {
                // Print as single-line cargo warning so the build keeps rolling;
                // the resource metadata is cosmetic and shouldn't hard-fail CI
                // on machines without the Windows SDK resource compiler installed.
                let detail = e.to_string().replace(['\r', '\n'], " ");
                println!(
                    "cargo:warning=sing-box-gui: failed to embed icon ({detail}). \
                     The build will continue but the .exe will use the default icon."
                );
            }
        }
    }
}
