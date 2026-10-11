fn main() {
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
