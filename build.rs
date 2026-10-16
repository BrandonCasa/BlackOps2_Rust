#![forbid(unsafe_code)]

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_GAME").is_some() {
        let checkout = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        for name in [
            "game.json",
            "cinematics.json",
            "enemies.json",
            "weapons.json",
            "audio.json",
            "fx_catalog.json",
        ] {
            let source = checkout.join("gameplay/origins").join(name);
            let destination = checkout.join("target/assets").join(name);
            println!("cargo:rerun-if-changed={}", source.display());
            println!("cargo:rerun-if-changed={}", destination.display());
            // Stage authored gameplay data; external model/audio extraction stays explicit.
            let bytes = std::fs::read(&source).expect("read authored Origins gameplay definition");
            if std::fs::read(&destination).ok().as_deref() != Some(bytes.as_slice()) {
                std::fs::create_dir_all(destination.parent().unwrap())
                    .expect("create runtime asset root");
                std::fs::write(&destination, bytes).expect("stage Origins gameplay definition");
            }
        }
    }

    if std::env::var_os("CARGO_FEATURE_GAME").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // NGX loads Super Resolution dynamically from the executable directory.
        let sdk = std::path::PathBuf::from(
            std::env::var_os("DLSS_SDK").expect("DLSS_SDK is required for the DLSS-enabled game"),
        );
        let dll = sdk.join("lib/Windows_x86_64/rel/nvngx_dlss.dll");
        println!("cargo:rerun-if-env-changed=DLSS_SDK");
        println!("cargo:rerun-if-changed={}", dll.display());
        let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let profile_dir = out_dir.ancestors().nth(3).expect("Cargo profile directory");
        let bytes = std::fs::read(&dll).expect("read DLSS Super Resolution runtime");
        for directory in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
            let destination = directory.join("nvngx_dlss.dll");
            if std::fs::read(&destination).ok().as_deref() != Some(bytes.as_slice()) {
                std::fs::write(destination, &bytes).expect("stage DLSS Super Resolution runtime");
            }
        }
        println!("cargo:rustc-link-arg-bin=origins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=origins=/MANIFESTUAC:level='asInvoker' uiAccess='false'"
        );
    }
}
