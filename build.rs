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
    // Windows: icon and version info inside bibicord.exe.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let v = env!("CARGO_PKG_VERSION");
        let n: Vec<&str> = v.split(['.', '-']).take(3).collect();
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/icon/icon.ico");
        let rc = format!(
            r#"#include <winver.h>
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {a},{b},{c},0
PRODUCTVERSION {a},{b},{c},0
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "041504B0"
    BEGIN
      VALUE "CompanyName", "BibiCord"
      VALUE "FileDescription", "BibiCord"
      VALUE "FileVersion", "{v}"
      VALUE "OriginalFilename", "bibicord.exe"
      VALUE "ProductName", "BibiCord"
      VALUE "ProductVersion", "{v}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0415, 0x04B0
  END
END
"#,
            icon = icon.display().to_string().replace('\\', "/"),
            a = n[0],
            b = n[1],
            c = n[2],
        );
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bibicord.rc");
        std::fs::write(&out, rc).unwrap();
        embed_resource::compile(&out, embed_resource::NONE).manifest_optional().unwrap();
        println!("cargo:rerun-if-changed=packaging/icon/icon.ico");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
