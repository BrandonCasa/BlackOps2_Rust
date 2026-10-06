//! Scene-wide frozen native world dispatch; excluded capture operations remain explicit.
use super::*;
use bevy::{camera::visibility::NoFrustumCulling, render::render_resource::*};
use bo2_assets::{
    captured_frame::{Dispatch, load_captured_frame},
    shader_bank::{ResourceKind, ShaderStage, TextureSample},
};
use bo2_rendering::{
    captured_world::{CapturedHandles, CapturedWorldPlugin, prepare_captured_world},
    original_material::{OriginalMaterial, OriginalMaterialPlugin},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Resource)]
struct NativePrograms {
    _handles: Vec<Handle<bevy::shader::Shader>>,
}

pub fn run(
    scene_path: &Path,
    frame_path: &Path,
    bank_path: &Path,
    secondary_path: &Path,
    output: &Path,
) -> Result<(), DiagnosticError> {
    for path in [scene_path, frame_path, bank_path, secondary_path] {
        outside_workspace(
            &path
                .canonicalize()
                .map_err(|e| io_error("world input", e))?,
        )?;
    }
    let scene =
        bo2_assets::full_scene::load_full_scene(scene_path).map_err(DiagnosticError::Appearance)?;
    let bank = bo2_assets::shader_bank::load_shader_bank(bank_path)
        .map_err(DiagnosticError::Appearance)?;
    let bank_hash = format!(
        "{:x}",
        Sha256::digest(fs::read(bank_path).map_err(|e| io_error("bank identity", e))?)
    );
    let frame = load_captured_frame(frame_path, &scene, &bank, &bank_hash)
        .map_err(DiagnosticError::Appearance)?;
    let secondary = bo2_assets::secondary_stream::load_secondary_corners(
        secondary_path,
        &scene.geometry,
        &bank.geometry_sha256,
    )
    .map_err(DiagnosticError::Appearance)?;
    let admitted = frame.draws.len();
    let receipts = frame.manifest.draws.len();
    let producer_count = frame.manifest.producer_dependencies.len();
    let mut app = configured_app(false, None);
    app.add_plugins((
        OriginalMaterialPlugin,
        CapturedWorldPlugin,
        depth_atlas::DepthAtlasPlugin,
        frame_initialization::FrameInitializationPlugin,
    ));
    let report = gpu_probe::render(app, |world| {
        let shaders: Vec<_> = bank
            .shaders
            .iter()
            .map(|s| {
                world.resource_mut::<Assets<bevy::shader::Shader>>().add(
                    bevy::shader::Shader::from_wgsl(
                        s.wgsl.clone(),
                        format!("external shader {}", s.declaration.source_sha256),
                    ),
                )
            })
            .collect();
        world.insert_resource(NativePrograms {
            _handles: shaders.clone(),
        });
        let mut images = BTreeMap::new();
        let mut image_proof = Vec::new();
        let mut depth_copies = Vec::new();
        let mut view_classes = BTreeMap::new();
        for receipt in &frame.manifest.draws {
            let Dispatch::World { draw } = &receipt.dispatch else {
                continue;
            };
            let loaded = frame
                .draws
                .iter()
                .find(|d| d.event == receipt.event)
                .ok_or(DiagnosticError::Preservation)?;
            for (binding, bytes) in draw.textures.iter().zip(&loaded.textures) {
                let source_shader = if binding.stage == ShaderStage::Vertex {
                    &draw.vertex_shader_sha256
                } else {
                    &draw.fragment_shader_sha256
                };
                let declaration = &bank
                    .shaders
                    .iter()
                    .find(|s| &s.declaration.source_sha256 == source_shader)
                    .ok_or(DiagnosticError::Preservation)?
                    .declaration;
                let Some(reflected) = declaration
                    .resources
                    .iter()
                    .find(|r| r.binding == 32 + u32::from(binding.register))
                else {
                    continue;
                };
                let is_depth = matches!(
                    reflected.kind,
                    ResourceKind::Texture {
                        sample: TextureSample::Depth,
                        ..
                    }
                );
                let key = (binding.bytes.sha256.clone(), binding.view_format.clone());
                if let Some(prior) = view_classes.insert(key.clone(), is_depth)
                    && prior != is_depth
                {
                    return Err(DiagnosticError::Preservation);
                }
                if images.contains_key(&key) {
                    continue;
                }
                let mut image = Image::from_buffer(
                    bytes,
                    bevy::image::ImageType::Extension("dds"),
                    bevy::image::CompressedImageFormats::BC,
                    false,
                    bevy::image::ImageSampler::Default,
                    RenderAssetUsages::default(),
                )
                .map_err(|error| {
                    eprintln!("captured texture decode: {error:?}");
                    DiagnosticError::Preservation
                })?;
                let expected = match binding.view_format.as_str() {
                    "BC1_UNORM" => TextureFormat::Bc1RgbaUnorm,
                    "BC2_UNORM" => TextureFormat::Bc2RgbaUnorm,
                    "BC3_UNORM" => TextureFormat::Bc3RgbaUnorm,
                    "BC5_UNORM" => TextureFormat::Bc5RgUnorm,
                    "R8G8B8A8_UNORM" => TextureFormat::Rgba8Unorm,
                    "R32_FLOAT" => TextureFormat::R32Float,
                    _ => return Err(DiagnosticError::Preservation),
                };
                if image.texture_descriptor.format != expected {
                    eprintln!(
                        "captured typed view: expected {expected:?}, decoded {:?}",
                        image.texture_descriptor.format
                    );
                    return Err(DiagnosticError::Preservation);
                }
                if is_depth {
                    if expected != TextureFormat::R32Float
                        || image.texture_descriptor.mip_level_count != 1
                    {
                        return Err(DiagnosticError::Preservation);
                    }
                    let data = image.data.as_ref().ok_or(DiagnosticError::Preservation)?;
                    if data.as_chunks::<4>().0.iter().any(|b| {
                        let d = f32::from_le_bytes(*b);
                        !d.is_finite() || !(0.0..=1.0).contains(&d)
                    }) {
                        return Err(DiagnosticError::Preservation);
                    }
                    let source = world.resource_mut::<Assets<Image>>().add(image.clone());
                    image_proof.push(source.clone());
                    image.data = None;
                    image.texture_descriptor.format = TextureFormat::Depth32Float;
                    image.texture_descriptor.usage =
                        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
                    let target = world.resource_mut::<Assets<Image>>().add(image);
                    depth_copies.push((source, target.clone()));
                    image_proof.push(target.clone());
                    images.insert(key, target);
                } else {
                    let handle = world.resource_mut::<Assets<Image>>().add(image);
                    image_proof.push(handle.clone());
                    images.insert(key, handle);
                }
            }
        }
        depth_atlas::register(world, depth_copies);
        let color = frame
            .manifest
            .initial_attachments
            .iter()
            .position(|a| a.view_format == "R8G8B8A8_UNORM")
            .ok_or(DiagnosticError::Preservation)?;
        let depth = frame
            .manifest
            .initial_attachments
            .iter()
            .position(|a| a.view_format == "D32S8")
            .ok_or(DiagnosticError::Preservation)?;
        image_proof.extend(frame_initialization::register(
            world,
            1920,
            1080,
            &frame.initial_attachments[color],
            &frame.initial_attachments[depth],
        )?);
        let draws = prepare_captured_world(
            &frame,
            &scene,
            &secondary,
            &bank,
            CapturedHandles {
                shaders: &shaders,
                images: &images,
            },
            &world.resource::<RenderDevice>().limits(),
        )
        .map_err(|e| {
            eprintln!("captured world admission: {e:?}");
            DiagnosticError::Preservation
        })?;
        let mut meshes = Vec::new();
        for draw in draws {
            draw.material
                .validate_images_with_features(
                    world.resource::<Assets<Image>>(),
                    world.resource::<RenderDevice>().features(),
                )
                .map_err(|error| {
                    eprintln!(
                        "captured typed resources event {}: {error:?}",
                        draw.dispatch.event
                    );
                    DiagnosticError::Preservation
                })?;
            let mesh = world.resource_mut::<Assets<Mesh>>().add(draw.mesh);
            let material = world
                .resource_mut::<Assets<OriginalMaterial>>()
                .add(draw.material);
            world.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                draw.dispatch,
                Transform::IDENTITY,
                NoFrustumCulling,
                bevy::render::batching::NoAutomaticBatching,
            ));
            meshes.push(mesh);
        }
        let camera = Vec3::from_array(bo2_rendering::coordinates::source_to_bevy(
            frame.manifest.camera_origin,
        ));
        Ok(gpu_probe::ProbeScene {
            meshes,
            images: image_proof,
            camera: Transform::from_translation(camera),
            projection: Projection::Perspective(PerspectiveProjection {
                near: 0.1,
                far: 100_000.0,
                ..default()
            }),
            output: gpu_probe::ProbeOutput {
                width: 1920,
                height: 1080,
                format: TextureFormat::Rgba8UnormSrgb,
                msaa: Msaa::Sample4,
            },
        })
    })?;
    let initial_color = frame
        .manifest
        .initial_attachments
        .iter()
        .position(|a| a.view_format == "R8G8B8A8_UNORM")
        .ok_or(DiagnosticError::Preservation)?;
    let samples = &frame.initial_attachments[initial_color];
    let changed_pixels = report
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(pixel, actual)| {
            (0..4).any(|channel| {
                let value = samples
                    .iter()
                    .map(|s| u32::from(s[pixel * 4 + channel]))
                    .sum::<u32>();
                actual[channel].abs_diff(((value + 2) / 4) as u8) > 2
            })
        })
        .count();
    if changed_pixels == 0 {
        return Err(DiagnosticError::Preservation);
    }
    let surfaces: BTreeSet<_> = frame
        .manifest
        .draws
        .iter()
        .filter_map(|r| {
            if let Dispatch::World { draw } = &r.dispatch {
                Some(&draw.slices)
            } else {
                None
            }
        })
        .flatten()
        .map(|s| s.surface)
        .collect();
    let output = new_output(output)?;
    PngEncoder::new(BufWriter::new(exclusive_file(
        &output.join("captured-world.png"),
    )?))
    .write_image(&report.pixels, 1920, 1080, image::ExtendedColorType::Rgba8)
    .map_err(|_| DiagnosticError::Encoding)?;
    let receipt = format!(
        "Captured native world dispatch: draws={admitted}/{receipts}, surfaces={}, shaders={}, updates={}, changed pixels={changed_pixels}, frozen producer dependencies={producer_count}. Original uniform/image versions, ordered native states and four-sample initial attachments. Excluded interleaved draws and postprocessing prevent complete-frame equivalence.\n",
        surfaces.len(),
        bank.shaders.len(),
        report.updates
    );
    exclusive_file(&output.join("render.txt"))?
        .write_all(receipt.as_bytes())
        .map_err(|e| io_error("world receipt", e))?;
    print!("{receipt}");
    Ok(())
}
