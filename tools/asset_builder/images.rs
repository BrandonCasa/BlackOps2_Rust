use crate::{catalog::Catalog, core::*, fastfile::Address, ipak::Ipak, pack::Pack, texture};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs::File, path::Path, sync::Arc};

pub fn convert(
    install: &Path,
    catalog: &Catalog<'_>,
    pack: &mut Pack,
    required: Option<&std::collections::BTreeSet<String>>,
) -> Result<(Vec<Value>, BTreeMap<String, texture::Image>)> {
    let mut packages = Vec::new();
    let mut paths = Vec::new();
    for language in ["all", "english"] {
        paths.extend(
            std::fs::read_dir(install.join("zone").join(language))?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<std::io::Result<Vec<_>>>()?,
        );
    }
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "ipak")
    });
    paths.sort();
    for path in paths {
        let package = Ipak::read(File::open(&path)?)?;
        println!(
            "Read {} image-package index: {} entries",
            path.file_name().unwrap().to_string_lossy(),
            package.entries.len()
        );
        packages.push(package);
    }
    let mut index = BTreeMap::new();
    for (package, data) in packages.iter().enumerate() {
        for entry in &data.entries {
            index.insert((entry.data_key, entry.name_key), (package, *entry));
        }
    }
    let mut rows = Vec::new();
    let mut sources = BTreeMap::new();
    for (name, zone_index, root) in catalog.definitions(8) {
        if required.is_some_and(|names| !names.contains(&name.to_ascii_lowercase())) {
            continue;
        }
        let (zone_name, zone) = &catalog.zones[zone_index];
        let source = (|| -> Result<texture::Image> {
            let streams = &zone.streams;
            let load = Address::decode(streams.u32(root)?)?;
            let size = streams.u32(load.add(8)?)? as usize;
            if size == 0 {
                let key = (streams.u32(root.add(40)?)?, streams.u32(root.add(76)?)?);
                let (package, entry) = index.get(&key).ok_or_else(|| {
                    format!(
                        "Installed IPAKs lack image {name}, data {:08x}, name {:08x}",
                        key.0, key.1
                    )
                })?;
                return texture::read(&packages[*package].image(*entry)?);
            }
            let header = streams.read(load, 12)?;
            let format = match u32_at(header, 4) {
                0x1c => 1,
                0x41 => 5,
                0x47 => 11,
                0x4a => 12,
                0x4d => 13,
                0x53 => 14,
                0x0a => 19,
                value => return Err(format!("Unrecovered inline image DXGI format {value}").into()),
            };
            let flags = header[1];
            let width = streams.u16(root.add(20)?)? as usize;
            let height = streams.u16(root.add(22)?)? as usize;
            let depth = streams.u16(root.add(24)?)? as usize;
            let faces = if streams.u8(root.add(4)?)? == 5 { 6 } else { 1 };
            if width == 0 || height == 0 || depth == 0 {
                return Err("Invalid inline native image dimensions".into());
            }
            let count = if flags & 2 != 0 {
                1
            } else {
                (usize::BITS - (width.max(height).max(depth) - 1).leading_zeros()) as usize + 1
            };
            let (_, _, bw, bh, bs, _) = texture::format(format)?;
            let bytes = streams.read(load.add(12)?, size)?;
            let mut levels = vec![Vec::new(); count];
            let mut cursor = 0;
            for _ in 0..faces {
                for (level, data) in levels.iter_mut().enumerate() {
                    let length = (width >> level).max(1).div_ceil(bw)
                        * (height >> level).max(1).div_ceil(bh)
                        * (depth >> level).max(1)
                        * bs;
                    let end = cursor + length;
                    data.extend_from_slice(
                        bytes
                            .get(cursor..end)
                            .ok_or("Truncated inline native image mip")?,
                    );
                    cursor = end;
                }
            }
            if cursor != bytes.len() {
                return Err(format!(
                    "Inline image layout consumes {cursor} of {} bytes",
                    bytes.len()
                )
                .into());
            }
            Ok(texture::Image {
                width,
                height,
                depth,
                faces,
                format,
                flags,
                levels,
            })
        })()
        .map_err(|error| format!("Image {zone_name}/{name}: {error}"))?;
        let semantic = zone.streams.u8(root.add(5)?)?;
        let role = match semantic {
            5 => "normal",
            8 => "response",
            _ => "color",
        };
        let mut uris = BTreeMap::new();
        let mut linear: Option<Arc<Vec<u8>>> = None;
        for (transfer, srgb) in [("linear", false), ("srgb", true)] {
            let uri = format!("prepared/images/{}/{transfer}.ktx2", rows.len());
            let bytes = if srgb && (matches!(role, "normal" | "response") || source.format == 19) {
                linear
                    .as_ref()
                    .ok_or("Linear image encoding is missing")?
                    .clone()
            } else {
                let bytes = texture::encode(&source, srgb, role)?;
                texture::validate(&bytes)
                    .map_err(|error| format!("Converted image {name}/{transfer}: {error}"))?;
                Arc::new(bytes)
            };
            if !srgb {
                linear = Some(bytes.clone());
            }
            pack.entries.insert(uri.clone(), bytes);
            uris.insert(transfer, uri);
        }
        let language = if zone_name.starts_with("en_") {
            "english"
        } else {
            "all"
        };
        rows.push(json!({"name": name, "zone": format!("{language}-{zone_name}"), "uris": uris}));
        sources.insert(name.to_ascii_lowercase(), source);
        if rows.len() % 100 == 0 {
            println!("Converted {} native images to packed KTX2", rows.len());
        }
    }
    println!("Converted {} native images to packed KTX2", rows.len());
    Ok((rows, sources))
}
