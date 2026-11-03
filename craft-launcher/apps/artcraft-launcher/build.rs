//! Windows only: embed the app icon and version info (VERSIONINFO) into `gesso-launcher.exe`.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `ARTCRAFT_LAUNCHER_REQUIRE_WINRES=1` (set
//! by the release packaging) turns it into an error.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/gesso-launcher.ico");
    println!("cargo:rerun-if-env-changed=ARTCRAFT_LAUNCHER_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/gesso-launcher.ico")
        .set("ProductName", "Gesso Launcher")
        .set("FileDescription", "Gesso Launcher")
        .set("CompanyName", "Jason Baldwin")
        .set("LegalCopyright", "Copyright (c) the ArtCraft Launcher authors; modifications (c) 2026 Jason Baldwin. MIT OR Apache-2.0.")
        .set("OriginalFilename", "gesso-launcher.exe")
        .set("InternalName", "gesso-launcher");
    if let Err(e) = res.compile() {
        if std::env::var_os("ARTCRAFT_LAUNCHER_REQUIRE_WINRES").is_some() {
            println!("cargo:warning=embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=gesso-launcher.exe built without icon/version resources: {e}");
    }
}
