fn main() {
    println!("cargo:rerun-if-changed=../../../assets/icon.ico");

    // Give the Windows executable the app icon (shown in Explorer, taskbar, Task Manager).
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../../assets/icon.ico");
        res.set("ProductName", "ClipStack");
        res.set("FileDescription", "ClipStack — clipboard stack for your tray");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows icon: {e}");
        }
    }
}
