#![forbid(unsafe_code)]

#[path = "asset_builder/core.rs"]
pub mod core;
#[path = "asset_builder/convert.rs"]
mod convert;
#[path = "asset_builder/atmosphere.rs"]
mod atmosphere;
#[path = "asset_builder/animation.rs"]
mod animation;
#[path = "asset_builder/bake.rs"]
mod bake;
#[path = "asset_builder/collision.rs"]
mod collision;
#[path = "asset_builder/entities.rs"]
mod entities;
#[path = "asset_builder/geometry.rs"]
mod geometry;
#[path = "asset_builder/gltf.rs"]
mod gltf;
#[path = "asset_builder/material.rs"]
mod material;
#[path = "asset_builder/fastfile.rs"]
pub mod fastfile;
#[path = "asset_builder/ipak.rs"]
pub mod ipak;
#[path = "asset_builder/native.rs"]
pub mod native;
#[path = "../src/pack.rs"]
pub mod pack;
#[path = "asset_builder/source.rs"]
mod source;
#[path = "asset_builder/scenes.rs"]
mod scenes;
#[path = "asset_builder/sound.rs"]
mod sound;
#[path = "asset_builder/texture.rs"]
pub mod texture;

use core::*;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

fn dependency(pack: &pack::Pack, directory: &Path, uri: &str) -> Result<()> {
    let mut path = directory.to_path_buf();
    let uri = uri.split('#').next().unwrap_or(uri);
    if uri.contains([':', '\\', '%']) || uri.starts_with('/') {
        return Err(format!("Unsupported runtime dependency URI: {uri}").into());
    }
    for component in Path::new(uri).components() {
        match component {
            Component::Normal(value) => path.push(value),
            Component::CurDir => (),
            Component::ParentDir if path.pop() => (),
            _ => return Err(format!("Runtime dependency escapes pack: {uri}").into()),
        }
    }
    pack.get(&path.to_string_lossy().replace('\\', "/"))?;
    Ok(())
}

fn check(path: &Path) -> Result<()> {
    let pack = pack::Pack::read(path)?;
    validate_pack(&pack)
}

fn validate_pack(pack: &pack::Pack) -> Result<()> {
    let _: Value = serde_json::from_slice(pack.get("game.json")?)?;
    let art: Value = serde_json::from_slice(pack.get("prepared/runtime.json")?)?;
    if art["schema_version"] != 1 {
        return Err("Unsupported runtime art schema".into());
    }
    fn links(pack: &pack::Pack, directory: &Path, data: &Value) -> Result<()> {
        match data {
            Value::Object(fields) => {
                for (key, value) in fields {
                    if (key.ends_with("_uri") || matches!(key.as_str(), "uri" | "scene" | "texture" | "path"))
                        && let Some(uri) = value.as_str().filter(|uri| !uri.is_empty())
                    { dependency(pack, directory, uri)?; }
                    if key == "uris" {
                        for uri in value.as_object().into_iter().flat_map(|uris| uris.values()) {
                            dependency(pack, directory, uri.as_str().ok_or("Runtime image URI is not a string")?)?;
                        }
                    }
                    if key == "frames" {
                        for uri in value.as_array().into_iter().flatten().filter_map(Value::as_str) {
                            dependency(pack, directory, uri)?;
                        }
                    }
                    if key == "models" {
                        for uri in value.as_object().into_iter().flat_map(|models| models.values()).filter_map(Value::as_str) {
                            dependency(pack, directory, uri)?;
                        }
                    }
                    links(pack, directory, value)?;
                }
            }
            Value::Array(values) => for value in values { links(pack, directory, value)?; },
            _ => (),
        }
        Ok(())
    }
    for (name, bytes) in &pack.entries {
        let path = Path::new(name);
        match path.extension().and_then(|value| value.to_str()) {
            Some("ktx2") => { texture::validate(bytes)?; }
            Some("gltf") => {
                let doc: Value = serde_json::from_slice(bytes)?;
                for section in ["buffers", "images"] {
                    links(pack, path.parent().unwrap_or(Path::new("")), &doc[section])?;
                }
                for material in doc["materials"].as_array().into_iter().flatten() {
                    links(pack, Path::new(""), &material["extras"])?;
                }
            }
            Some("json" | "nativefx") => {
                let doc: Value = serde_json::from_slice(bytes)?;
                links(pack, Path::new(""), &doc)?;
            }
            _ => (),
        }
    }
    println!("Runtime pack contents and dependencies resolved: {} entries", pack.entries.len());
    Ok(())
}

fn execute() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("build") => {
            if args.len() != 3 || args[1] != "--output" {
                return Err("Usage: build_assets build --output PATH/origins.pack".into());
            }
            let output = PathBuf::from(&args[2]);
            if output.file_name().and_then(|name| name.to_str()) != Some("origins.pack") {
                return Err("The runtime consumer expects origins.pack beside origins.exe".into());
            }
            source::build(&source::installation()?, &output, None)
        }
        Some("material" | "model") if args.len() == 4 && args[2] == "--output" =>
            source::build(&source::installation()?, Path::new(&args[3]), Some(&args[1])),
        Some("check") if args.len() == 2 => check(Path::new(&args[1])),
        _ => Err("Usage: build_assets build --output PATH/origins.pack | build_assets check PATH/origins.pack | build_assets material NAME --output PATH.pack | build_assets model NAME --output PATH.pack".into()),
    }
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("Asset build failed: {error}");
        std::process::exit(1);
    }
}
