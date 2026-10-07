#![forbid(unsafe_code)]

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // The game runs with its launcher's ordinary permissions, never elevation.
        println!("cargo:rustc-link-arg-bin=origins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=origins=/MANIFESTUAC:level='asInvoker' uiAccess='false'"
        );
    }
}
