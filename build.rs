fn main() {
    // The native menu bar is built on macOS (and can be forced with --cfg native_menu for checks).
    println!("cargo::rustc-check-cfg=cfg(native_menu)");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cfg=native_menu");
    }

    // Build date for the About window, e.g. 2026-10-11.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    println!("cargo:rustc-env=NUMERA_BUILD_DATE={:04}-{:02}-{:02}", year, month, day);
    // Embed the application icon and version info into the Windows executable.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icon.ico");
        res.set("ProductName", "Scientific Calculator");
        res.set("FileDescription", "Scientific Calculator");
        res.set("CompanyName", "Sope Technology");
        res.set("LegalCopyright", "(c) 2026 Sope Technology");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
