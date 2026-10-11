use bevy::{
    asset::{
        RenderAssetUsages,
        saver::{AssetSaver, SavedAsset},
    },
    image::{
        CompressedImageFormats, CompressedImageSaver, CompressedImageSaverSettings, Image,
        ImageType,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    tasks::{block_on, futures_lite::io::Cursor},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    id: String,
    input: PathBuf,
    output: PathBuf,
    role: String,
    is_srgb: bool,
    settings: CompressedImageSaverSettings,
    raw: Option<RawImage>,
    container: Option<ContainerImage>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContainerImage {
    format: u32,
    width: u32,
    height: u32,
    depth: u32,
    layers: u32,
    faces: u32,
    payloads: Vec<PathBuf>,
}

fn container_bytes(item: &Item, container: &ContainerImage) -> Result<Vec<u8>, String> {
    if container.width == 0
        || container.height == 0
        || container.width.max(container.height).max(container.depth) > 16384
        || !matches!(container.faces, 1 | 6)
        || container.payloads.iter().any(|path| !path.is_absolute())
    {
        return Err("Container dimensions, faces or explicit payload paths are invalid".into());
    }
    let format = ktx2::Format::new(container.format).ok_or("Missing Vulkan format")?;
    let (dfd, type_size) = ktx2::dfd::Basic::from_format_with(
        format,
        item.settings.output_alpha_mode == bevy::image::ImageCompressorAlphaMode::Premultiplied,
        None,
        None,
        None,
    )
    .map_err(|e| format!("Invalid container DFD: {e:?}"))?;
    let dfd = ktx2::dfd::Block::Basic(dfd).to_vec();
    let count = container.payloads.len();
    if count == 0
        || count
            != (u32::BITS
                - container
                    .width
                    .max(container.height)
                    .max(container.depth)
                    .leading_zeros()) as usize
    {
        return Err("Container migration requires a complete mip chain".into());
    }
    let dfd_offset = 80 + count * 24;
    let dfd_size = dfd.len() + 4;
    let header = ktx2::Header {
        format: Some(format),
        type_size,
        pixel_width: container.width,
        pixel_height: container.height,
        pixel_depth: container.depth,
        layer_count: container.layers,
        face_count: container.faces,
        level_count: count as u32,
        supercompression_scheme: None,
        index: ktx2::Index {
            dfd_byte_offset: dfd_offset as u32,
            dfd_byte_length: dfd_size as u32,
            kvd_byte_offset: 0,
            kvd_byte_length: 0,
            sgd_byte_offset: 0,
            sgd_byte_length: 0,
        },
    };
    let mut output = header.as_bytes().to_vec();
    output.resize(dfd_offset, 0);
    output.extend_from_slice(&(dfd_size as u32).to_le_bytes());
    output.extend_from_slice(&dfd);
    let mut indices = vec![[0u64; 3]; count];
    for level in (0..count).rev() {
        while output.len() % 16 != 0 {
            output.push(0);
        }
        let payload = std::fs::read(&container.payloads[level]).map_err(|e| e.to_string())?;
        indices[level] = [
            output.len() as u64,
            payload.len() as u64,
            payload.len() as u64,
        ];
        output.extend_from_slice(&payload);
    }
    for (index, level) in indices.iter().enumerate() {
        for (field, value) in level.iter().enumerate() {
            let offset = 80 + index * 24 + field * 8;
            output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    ktx2::Reader::new(&output).map_err(|e| format!("Invalid assembled KTX2: {e:?}"))?;
    Ok(output)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawImage {
    width: u32,
    height: u32,
    channels: u32,
}

#[derive(Serialize)]
struct ItemResult {
    id: String,
    input: PathBuf,
    output: PathBuf,
    role: String,
    is_srgb: bool,
    settings: CompressedImageSaverSettings,
    operation: &'static str,
    width: Option<u32>,
    height: Option<u32>,
    output_bytes: Option<u64>,
    error: Option<String>,
}

#[derive(Serialize)]
struct Report {
    backend: &'static str,
    passed: bool,
    items: Vec<ItemResult>,
}

fn process(item: &Item) -> Result<(u32, u32, u64), String> {
    if !item.input.is_absolute() || !item.output.is_absolute() {
        return Err("Input and output paths must be absolute".into());
    }
    if item.output.extension().and_then(|s| s.to_str()) != Some("ktx2") {
        return Err("Texture preparation outputs must use KTX2".into());
    }
    if item.output.exists() {
        return Err("Output already exists; use a fresh output path".into());
    }
    if item.settings.is_normal_map
        && (item.is_srgb || !matches!(item.role.as_str(), "normal" | "normal_detail"))
    {
        return Err("Normal processing requires a linear normal-map role".into());
    }
    if std::env::var_os("BEVY_COMPRESSED_IMAGE_SAVER_ASTC").is_some() {
        return Err("Implicit ASTC override is forbidden; BCn settings are explicit".into());
    }
    let (width, height, output) = if let Some(container) = &item.container {
        if item.raw.is_some() || item.settings.generate_mipmaps {
            return Err("Container migration cannot encode or regenerate authored mipmaps".into());
        }
        (
            container.width,
            container.height,
            container_bytes(item, container)?,
        )
    } else {
        let data = std::fs::read(&item.input).map_err(|e| e.to_string())?;
        let image = if let Some(raw) = &item.raw {
            if raw.width == 0 || raw.height == 0 || raw.width > 16384 || raw.height > 16384 {
                return Err("Raw image dimensions must be in 1..=16384".into());
            }
            let format = match (raw.channels, item.is_srgb) {
                (1, false) => TextureFormat::R8Unorm,
                (2, false) => TextureFormat::Rg8Unorm,
                (4, false) => TextureFormat::Rgba8Unorm,
                (4, true) => TextureFormat::Rgba8UnormSrgb,
                _ => {
                    return Err(
                        "Raw input requires 1/2 linear channels or 4 linear/sRGB channels".into(),
                    );
                }
            };
            let expected = u64::from(raw.width) * u64::from(raw.height) * u64::from(raw.channels);
            if data.len() as u64 != expected {
                return Err(format!(
                    "Raw payload has {} bytes; expected {expected}",
                    data.len()
                ));
            }
            Image::new(
                Extent3d {
                    width: raw.width,
                    height: raw.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                format,
                RenderAssetUsages::MAIN_WORLD,
            )
        } else {
            let extension = item
                .input
                .extension()
                .and_then(|s| s.to_str())
                .ok_or("Missing input extension")?;
            Image::from_buffer(
                &data,
                ImageType::Extension(extension),
                CompressedImageFormats::all(),
                item.is_srgb,
                Default::default(),
                RenderAssetUsages::MAIN_WORLD,
            )
            .map_err(|e| e.to_string())?
        };
        if image.texture_descriptor.mip_level_count != 1 || image.is_compressed() {
            return Err("Encode one uncompressed semantic mip at a time; authored blocks require lossless container migration".into());
        }
        let (width, height) = (image.width(), image.height());
        let mut writer = Cursor::new(Vec::new());
        block_on(CompressedImageSaver::default().save(
            &mut writer,
            SavedAsset::from_asset(&image),
            &item.settings,
            item.output.clone().into(),
        ))
        .map_err(|e| e.to_string())?;
        (width, height, writer.into_inner())
    };
    let _reloaded = Image::from_buffer(
        &output,
        ImageType::Extension("ktx2"),
        CompressedImageFormats::all(),
        item.is_srgb,
        Default::default(),
        RenderAssetUsages::MAIN_WORLD,
    )
    .map_err(|e| format!("Bevy rejected its saved KTX2: {e}"))?;
    let saved = ktx2::Reader::new(&output).map_err(|e| format!("Invalid saved KTX2: {e:?}"))?;
    if saved.header().pixel_width != width || saved.header().pixel_height != height {
        return Err("Saved KTX2 dimensions differ from the input".into());
    }
    let parent = item.output.parent().ok_or("Missing output directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = item.output.with_extension("ktx2.tmp");
    std::fs::write(&temporary, &output).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &item.output).map_err(|e| e.to_string())?;
    Ok((width, height, output.len() as u64))
}

pub fn run(manifest: &Path, report: &Path) -> Result<bool, String> {
    let batch: Batch = serde_json::from_slice(&std::fs::read(manifest).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for item in batch.items {
        let outcome = process(&item);
        let (width, height, output_bytes, error) = match outcome {
            Ok((width, height, bytes)) => (Some(width), Some(height), Some(bytes), None),
            Err(error) => (None, None, None, Some(error)),
        };
        results.push(ItemResult {
            id: item.id,
            input: item.input,
            output: item.output,
            role: item.role,
            is_srgb: item.is_srgb,
            settings: item.settings,
            operation: if item.container.is_some() {
                "lossless-container"
            } else {
                "CompressedImageSaver"
            },
            width,
            height,
            output_bytes,
            error,
        });
    }
    let passed = results.iter().all(|item| item.error.is_none());
    let result = Report {
        backend: "bevy-0.20.0/CompressedImageSaver/ctt",
        passed,
        items: results,
    };
    let raw = serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?;
    if let Some(parent) = report.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(report, raw).map_err(|e| e.to_string())?;
    Ok(passed)
}
