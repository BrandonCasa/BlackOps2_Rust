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
        spawn_world(world, &frame, &scene, &bank, &secondary, true)
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
pub(super) fn spawn_world(
    world: &mut World,
    frame: &bo2_assets::captured_frame::CapturedFrameInput,
    scene: &bo2_assets::full_scene::FullSceneInput,
    bank: &bo2_assets::shader_bank::ShaderBankInput,
    secondary: &[bo2_assets::secondary_stream::SourceSecondaryCorner],
    initialize: bool,
) -> Result<gpu_probe::ProbeScene, DiagnosticError> {
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
    if initialize {
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
    }
    let handles = CapturedHandles {
        shaders: &shaders,
        images: &images,
    };
    let limits = world.resource::<RenderDevice>().limits();
    let draws = if initialize {
        prepare_captured_world(frame, scene, secondary, bank, handles, &limits)
            .map_err(|e| {
                eprintln!("captured world admission: {e:?}");
                DiagnosticError::Preservation
            })?
            .into_iter()
            .map(|d| super::full_map_materials::MapDraw {
                template_event: d.dispatch.event,
                mesh: d.mesh,
                material: d.material,
                dispatch: d.dispatch,
                uniforms: BTreeMap::new(),
            })
            .collect()
    } else {
        let mut scene_images = BTreeMap::new();
        for (index, source) in scene.images.iter().enumerate() {
            let image = Image::from_buffer(
                &source.dds,
                bevy::image::ImageType::Extension("dds"),
                bevy::image::CompressedImageFormats::BC,
                false,
                bevy::image::ImageSampler::Default,
                RenderAssetUsages::default(),
            )
            .map_err(|_| DiagnosticError::Preservation)?;
            let view = match image.texture_descriptor.format {
                TextureFormat::Bc1RgbaUnorm => "BC1_UNORM",
                TextureFormat::Bc2RgbaUnorm => "BC2_UNORM",
                TextureFormat::Bc3RgbaUnorm => "BC3_UNORM",
                TextureFormat::Bc5RgUnorm => "BC5_UNORM",
                TextureFormat::Rgba8Unorm => "R8G8B8A8_UNORM",
                TextureFormat::R32Float => "R32_FLOAT",
                _ => "other source view",
            };
            let key = (
                format!("{:x}", Sha256::digest(&source.dds)),
                view.to_string(),
            );
            let handle = if let Some(existing) = images.get(&key)
                && !view_classes.get(&key).copied().unwrap_or(false)
            {
                existing.clone()
            } else {
                world.resource_mut::<Assets<Image>>().add(image)
            };
            image_proof.push(handle.clone());
            scene_images.insert(index, handle);
        }
        let prepared = super::full_map_materials::prepare(
            frame,
            scene,
            secondary,
            bank,
            handles,
            &scene_images,
            &limits,
        )?;
        eprintln!(
            "Native full-map preview: {}/{} surfaces, {} grouped draws; omissions: {:?}",
            prepared.admitted,
            scene.surfaces.len(),
            prepared.draws.len(),
            prepared.omitted
        );
        prepared.draws
    };
    let mut targets = Vec::new();
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
        targets.push(UniformTarget {
            event: draw.template_event,
            material: material.clone(),
            baseline: draw.uniforms,
        });
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
    world.insert_resource(UniformTargets(targets));
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
}

struct UniformTarget {
    event: u32,

    material: Handle<OriginalMaterial>,

    baseline: BTreeMap<u32, Vec<u8>>,
}

#[derive(Resource)]

struct UniformTargets(Vec<UniformTarget>);

#[derive(Resource)]

struct FlightAdapter(bo2_rendering::native_camera::NativeCameraAdapter);

pub fn fly(
    scene_path: &Path,
    frame_path: &Path,
    bank_path: &Path,
    secondary_path: &Path,
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

    let adapter = bo2_rendering::native_camera::NativeCameraAdapter::from_frame(&frame, &bank)
        .map_err(|e| {
            eprintln!("native camera: {e:?}");
            DiagnosticError::Preservation
        })?;

    let pose = adapter.initial_pose();

    let mut app = configured_app(
        false,
        Some(Window {
            title: "Origins native shader preview â€” WASD / Q E / arrows / Shift / R reset".into(),

            resolution: bevy::window::WindowResolution::new(1920, 1080)
                .with_scale_factor_override(1.0),

            resizable: false,
            ..default()
        }),
    );

    app.add_plugins((
        OriginalMaterialPlugin,
        CapturedWorldPlugin,
        depth_atlas::DepthAtlasPlugin,
        bo2_geometry_viewer::camera::InspectionCameraPlugin,
    ));

    app.insert_resource(FlightAdapter(adapter));

    app.add_systems(Startup, move |world: &mut World| {
        if let Err(error) = spawn_world(world, &frame, &scene, &bank, &secondary, false) {
            eprintln!("native scene startup: {error:?}");

            world.write_message(AppExit::error());

            return;
        }

        world.spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            bevy::camera::CompositingSpace::Srgb,
            bevy::render::view::NoIndirectDrawing,
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            bevy::core_pipeline::tonemapping::DebandDither::Disabled,
            Msaa::Sample4,
            pose,
            bo2_geometry_viewer::camera::InspectionCamera::at_pose(pose, 350.0)
                .expect("fixed valid inspection pose"),
        ));
    });

    app.add_systems(
        Update,
        update_flight.after(bo2_geometry_viewer::camera::InspectionControlSystems),
    );

    app.run();

    Ok(())
}

fn update_flight(
    adapter: Res<FlightAdapter>,

    targets: Option<Res<UniformTargets>>,

    cameras: Query<&Transform, (With<Camera3d>, Changed<Transform>)>,

    mut materials: ResMut<Assets<OriginalMaterial>>,

    mut exit: MessageWriter<AppExit>,
    mut previous: Local<Option<Transform>>,
) {
    let Some(targets) = targets else {
        return;
    };

    for pose in &cameras {
        if previous.as_ref() == Some(pose) {
            continue;
        }
        *previous = Some(*pose);
        let patches = match adapter.0.uniform_patches(pose) {
            Ok(patches) => patches,

            Err(error) => {
                eprintln!("native camera update: {error:?}");
                exit.write(AppExit::error());
                return;
            }
        };

        for target in &targets.0 {
            let Some(update) = patches.iter().find(|p| p.event == target.event) else {
                continue;
            };

            let Some(mut material) = materials.get_mut(&target.material) else {
                continue;
            };

            for (&binding, baseline) in &target.baseline {
                if !update.patches.iter().any(|p| p.binding == binding) {
                    continue;
                }
                let mut bytes = baseline.clone();

                for patch in update.patches.iter().filter(|p| p.binding == binding) {
                    let start = patch.offset;

                    let Some(destination) = bytes.get_mut(start..start + patch.bytes.len()) else {
                        eprintln!("native camera patch exceeds uniform");
                        exit.write(AppExit::error());
                        return;
                    };

                    destination.copy_from_slice(&patch.bytes);
                }

                if let Err(error) = material.update_uniform(binding, bytes) {
                    eprintln!("native camera material: {error:?}");
                    exit.write(AppExit::error());
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod flight_tests {
    use super::*;
    #[test]
    #[ignore = "requires licensed external scene and GPU"]
    fn full_map_camera_gpu() {
        let scene_path = std::env::var("BO2_FULL_SCENE").unwrap();
        let frame_path = std::env::var("BO2_CAPTURED_FRAME").unwrap();
        let bank_path = std::env::var("BO2_SHADER_BANK").unwrap();
        let secondary_path = std::env::var("BO2_SECONDARY").unwrap();
        let scene = bo2_assets::full_scene::load_full_scene(Path::new(&scene_path)).unwrap();
        let bank = bo2_assets::shader_bank::load_shader_bank(Path::new(&bank_path)).unwrap();
        let hash = format!("{:x}", Sha256::digest(fs::read(&bank_path).unwrap()));
        let frame = load_captured_frame(Path::new(&frame_path), &scene, &bank, &hash).unwrap();
        let secondary = bo2_assets::secondary_stream::load_secondary_corners(
            Path::new(&secondary_path),
            &scene.geometry,
            &bank.geometry_sha256,
        )
        .unwrap();
        let adapter =
            bo2_rendering::native_camera::NativeCameraAdapter::from_frame(&frame, &bank).unwrap();
        let mut original = None;
        for moved in [false, true, false] {
            let mut pose = adapter.initial_pose();
            if moved {
                pose.translation += *pose.forward() * 120.0;
                pose.rotation *= Quat::from_rotation_y(0.2);
            }
            let mut app = configured_app(false, None);
            app.add_plugins((
                OriginalMaterialPlugin,
                CapturedWorldPlugin,
                depth_atlas::DepthAtlasPlugin,
            ));
            let report = gpu_probe::render(app, |world| {
                let mut proof = spawn_world(world, &frame, &scene, &bank, &secondary, false)?;
                let patches = adapter.uniform_patches(&pose).unwrap();
                let targets = world.remove_resource::<UniformTargets>().unwrap();
                for target in &targets.0 {
                    let update = patches.iter().find(|p| p.event == target.event).unwrap();
                    let mut materials = world.resource_mut::<Assets<OriginalMaterial>>();
                    let mut material = materials.get_mut(&target.material).unwrap();
                    for (&binding, baseline) in &target.baseline {
                        let mut bytes = baseline.clone();
                        for patch in update.patches.iter().filter(|p| p.binding == binding) {
                            let start = patch.offset;
                            bytes[start..start + patch.bytes.len()].copy_from_slice(&patch.bytes);
                        }
                        material.update_uniform(binding, bytes).unwrap();
                    }
                }
                proof.camera = pose;
                Ok(proof)
            })
            .unwrap();
            assert!(
                report
                    .pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[..3] != [0, 0, 0])
                    .count()
                    > 100_000
            );
            if let Some(original) = &original {
                if moved {
                    assert_ne!(original, &report.pixels);
                } else {
                    assert_eq!(original, &report.pixels);
                }
            } else {
                original = Some(report.pixels);
            }
        }
    }
}
