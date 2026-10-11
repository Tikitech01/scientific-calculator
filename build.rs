fn main() {
    // The native menu bar is built on macOS (and can be forced with --cfg numera_menu for checks).
    println!("cargo::rustc-check-cfg=cfg(numera_menu)");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cfg=numera_menu");
    }
    // Embed the application icon and version info into the Windows executable.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icon.ico");
        res.set("ProductName", "Numera");
        res.set("FileDescription", "Numera — scientific and graphing calculator");
        res.set("CompanyName", "Tiki Tech");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
