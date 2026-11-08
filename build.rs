fn main() {
    // ScreenCaptureKit's Swift bridge needs the Swift compatibility libs.
    // With only the Command Line Tools installed they live next to the
    // toolchain's `swift` binary, which the bridge's build script misses.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        if let Ok(out) = std::process::Command::new("xcrun").args(["--find", "swift"]).output() {
            let swift = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(bin) = std::path::Path::new(&swift).parent() {
                let lib = bin.join("../lib/swift/macosx");
                if lib.exists() {
                    println!("cargo:rustc-link-search=native={}", lib.display());
                }
            }
        }
        println!("cargo:rustc-link-search=native=/usr/lib/swift");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
