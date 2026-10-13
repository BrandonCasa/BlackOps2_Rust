#![forbid(unsafe_code)]

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_GAME").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rerun-if-env-changed=DLSS_SDK");
        let sdk = std::path::PathBuf::from(
            std::env::var_os("DLSS_SDK")
                .expect("DLSS_SDK must point to the SDK required by the locked dlss_wgpu version"),
        );
        let architecture = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let runtime = sdk.join(format!("lib/Windows_{architecture}/rel"));
        let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let executable_dir = out_dir
            .ancestors()
            .nth(3)
            .expect("Cargo output directory is unavailable");
        for name in ["nvngx_dlss.dll", "nvngx_dlssd.dll"] {
            let source = runtime.join(name);
            println!("cargo:rerun-if-changed={}", source.display());
            std::fs::copy(&source, executable_dir.join(name)).unwrap_or_else(|error| {
                panic!(
                    "Cannot deploy DLSS runtime {} beside origins.exe: {error}",
                    source.display()
                )
            });
        }
        for binary in ["origins"] {
            println!("cargo:rustc-link-arg-bin={binary}=/MANIFEST:EMBED");
            println!(
                "cargo:rustc-link-arg-bin={binary}=/MANIFESTUAC:level='asInvoker' uiAccess='false'"
            );
        }
    }
}
