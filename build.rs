fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // Print as cargo warning so the build keeps rolling; the icon is
            // cosmetic and shouldn't hard-fail CI on machines without the
            // Windows SDK resource compiler installed.
            println!(
                "cargo:warning=sing-box-gui: failed to embed icon ({e}). \
                      The build will continue but the .exe will use the default icon."
            );
        }
    }
}
