//! Explicit full-map first-color diagnostic. This does not admit original shaders.
use super::*;
use bevy::render::texture::GpuImage;
use bevy::render::{
    Render, RenderSystems,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
};
use bo2_assets::full_scene::load_full_scene;
use bo2_geometry_viewer::camera::{InspectionCamera, InspectionCameraPlugin};
use bo2_rendering::full_scene::{DiagnosticTextureScene, diagnostic_texture_scene};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource, Clone, ExtractResource)]
struct Proof {
    meshes: Vec<Handle<Mesh>>,
    images: Vec<Handle<Image>>,
    ready: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}

#[derive(Resource)]
struct Loaded {
    scene: Option<DiagnosticTextureScene>,
    center: Vec3,
    radius: f32,
    view: Option<bo2_geometry_viewer::DiagnosticView>,
}

struct SceneHandles {
    meshes: Vec<Handle<Mesh>>,
    images: Vec<Handle<Image>>,
}

fn load(path: &Path) -> Result<Loaded, DiagnosticError> {
    let path = path
        .canonicalize()
        .map_err(|e| io_error("scene identity", e))?;
    outside_workspace(&path)?;
    let input = load_full_scene(&path).map_err(DiagnosticError::Appearance)?;
    let positions = input
        .geometry
        .positions()
        .iter()
        .copied()
        .map(bo2_rendering::coordinates::source_to_bevy)
        .collect::<Vec<_>>();
    let [_, _, view] = bo2_geometry_viewer::views(&positions)?;
    let radius = view.projection.near;
    let center = view.transform.translation + *view.transform.forward() * (3.0 * radius);
    let scene = diagnostic_texture_scene(input, bevy::image::CompressedImageFormats::BC)
        .map_err(DiagnosticError::Appearance)?;
    println!(
        "Texture diagnostic: {} source surfaces, {} material meshes, {} shared image/sampler pairs, {} multi-color surfaces show first layer only, {} no-color surfaces marked magenta; no original lighting/layering/shader claim",
        scene.source_surface_count,
        scene.meshes.len(),
        scene.images.len(),
        scene.multi_color_surfaces,
        scene.missing_color_surfaces
    );
    Ok(Loaded {
        scene: Some(scene),
        center,
        radius,
        view: Some(view),
    })
}

fn spawn(
    world: &mut World,
    scene: DiagnosticTextureScene,
) -> Result<SceneHandles, DiagnosticError> {
    let images = scene
        .images
        .into_iter()
        .map(|image| world.resource_mut::<Assets<Image>>().add(image))
        .collect::<Vec<_>>();
    let materials = scene
        .materials
        .into_iter()
        .map(|material| {
            let image = material
                .image
                .map(|i| images.get(i).cloned().ok_or(DiagnosticError::Preservation))
                .transpose()?;
            Ok(world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: if image.is_some() {
                        Color::WHITE
                    } else {
                        Color::srgb(1.0, 0.0, 1.0)
                    },
                    base_color_texture: image,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                }))
        })
        .collect::<Result<Vec<_>, DiagnosticError>>()?;
    let mut meshes = Vec::new();
    for surface in scene.meshes {
        let material = materials
            .get(surface.material)
            .cloned()
            .ok_or(DiagnosticError::Preservation)?;
        let mesh = world.resource_mut::<Assets<Mesh>>().add(surface.mesh);
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
        ));
        meshes.push(mesh);
    }
    Ok(SceneHandles { meshes, images })
}

pub fn offscreen(path: &Path, output: &Path) -> Result<(), DiagnosticError> {
    let mut loaded = load(path)?;
    let view = loaded.view.take().ok_or(DiagnosticError::Preservation)?;
    let scene = loaded.scene.take().ok_or(DiagnosticError::Preservation)?;
    let report = gpu_probe::render(configured_app(false, None), |world| {
        let SceneHandles { meshes, images } = spawn(world, scene)?;
        Ok(gpu_probe::ProbeScene {
            output: gpu_probe::ProbeOutput::default(),
            meshes,
            images,
            camera: view.transform,
            projection: Projection::Orthographic(view.projection),
        })
    })?;
    let coverage = report
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count();
    if coverage == 0 {
        return Err(DiagnosticError::BackgroundOnly);
    }
    let output = new_output(output)?;
    PngEncoder::new(BufWriter::new(exclusive_file(
        &output.join("texture-diagnostic.png"),
    )?))
    .write_image(
        &report.pixels,
        IMAGE_SIDE,
        IMAGE_SIDE,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|_| DiagnosticError::Encoding)?;
    println!(
        "Full-map texture diagnostic GPU: updates={}, foreground={coverage}, prepared meshes/images and 512x512 readback; original materials remain unsupported",
        report.updates
    );
    Ok(())
}

#[derive(Resource)]
struct Session {
    frames: u32,
    limit: Option<u32>,
    started: Instant,
    requested: bool,
    captured: bool,
    failed: bool,
    completed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

pub fn interactive(path: &Path, frames: Option<u32>) -> Result<(), DiagnosticError> {
    let loaded = load(path)?;
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut app = configured_app(false, Some(Window {
        title: "Full-map texture DIAGNOSTIC | first color layer; magenta=no-color | W/S A/D E/Q arrows R Esc".into(),
        resolution: bevy::window::WindowResolution::new(960, 720).with_scale_factor_override(1.0),
        ..default()
    }));
    app.insert_resource(Proof {
        meshes: Vec::new(),
        images: Vec::new(),
        ready: Arc::new(AtomicBool::new(false)),
        failed: Arc::new(AtomicBool::new(false)),
    });
    app.insert_resource(loaded)
        .insert_resource(Session {
            frames: 0,
            limit: frames,
            started: Instant::now(),
            requested: false,
            captured: false,
            failed: false,
            completed: completed.clone(),
        })
        .add_plugins((
            InspectionCameraPlugin,
            ExtractResourcePlugin::<Proof>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, monitor);
    app.get_sub_app_mut(RenderApp)
        .ok_or(DiagnosticError::EngineUnavailable)?
        .add_systems(Render, pipeline_probe.in_set(RenderSystems::Cleanup));
    let result = app.run();
    if result.is_error()
        || (frames.is_some() && !completed.load(std::sync::atomic::Ordering::Relaxed))
    {
        return Err(DiagnosticError::InteractiveValidation);
    }
    Ok(())
}

fn setup(world: &mut World) {
    let (scene, center, radius) = {
        let mut loaded = world.resource_mut::<Loaded>();
        (loaded.scene.take(), loaded.center, loaded.radius)
    };
    let result = (|| {
        let SceneHandles { meshes, images } =
            spawn(world, scene.ok_or(DiagnosticError::Preservation)?)?;
        let mut controller =
            InspectionCamera::new(center, radius).ok_or(DiagnosticError::UnframeableGeometry)?;
        let transform = controller.home(4.0 / 3.0);
        let projection = controller.projection();
        world.spawn((
            Camera3d::default(),
            Camera {
                clear_color: Color::BLACK.into(),
                ..default()
            },
            Projection::Perspective(projection),
            transform,
            controller,
            Tonemapping::None,
            DebandDither::Disabled,
            Msaa::Off,
        ));
        let mut proof = world.resource_mut::<Proof>();
        proof.meshes = meshes;
        proof.images = images;
        Ok::<_, DiagnosticError>(())
    })();
    if result.is_err() {
        world.write_message(AppExit::error());
    }
}

fn monitor(world: &mut World) {
    let ready = world.resource::<Proof>().ready.load(Ordering::Relaxed);
    let gpu_failed = world.resource::<Proof>().failed.load(Ordering::Relaxed);
    let (request, finish, failed) = {
        let mut session = world.resource_mut::<Session>();
        session.frames += 1;
        let request = session.frames >= 32 && ready && !session.requested;
        if request {
            session.requested = true;
        }
        let finish = session.limit.is_some_and(|limit| session.frames >= limit);
        let failed = gpu_failed
            || session.failed
            || (session.limit.is_some() && session.started.elapsed() > Duration::from_secs(60));
        (request, finish, failed)
    };
    if failed {
        world.write_message(AppExit::error());
        return;
    }
    if request {
        world.spawn(Screenshot::primary_window()).observe(
            |event: On<ScreenshotCaptured>, mut session: ResMut<Session>| {
                let pixels = event.image.data.as_deref().unwrap_or_default();
                let foreground = pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[0] > 24 || p[1] > 24 || p[2] > 24)
                    .count();
                session.captured = event.image.width() == 960
                    && event.image.height() == 720
                    && pixels.len() == 960 * 720 * 4
                    && foreground > 0
                    && matches!(
                        event.image.texture_descriptor.format,
                        TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
                    );
                session.failed = !session.captured;
                println!(
                    "Texture diagnostic window readback: foreground={foreground}, captured={}",
                    session.captured
                );
            },
        );
    }
    if finish {
        let session = world.resource::<Session>();
        let success = session.captured && !session.failed;
        session
            .completed
            .store(success, std::sync::atomic::Ordering::Relaxed);
        world.write_message(if success {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}

fn pipeline_probe(
    cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    images: Res<RenderAssets<GpuImage>>,
    proof: Res<Proof>,
) {
    if cache.pipelines().any(|p| matches!(&p.state, CachedPipelineState::Err(error)
        if !matches!(error, ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable))) {
        proof.failed.store(true, Ordering::Relaxed);
    }
    if !proof.meshes.is_empty()
        && proof.meshes.iter().all(|h| meshes.get(h.id()).is_some())
        && proof.images.iter().all(|h| images.get(h.id()).is_some())
        && cache.waiting_pipelines().next().is_none()
        && cache.pipelines().next().is_some()
    {
        proof.ready.store(true, Ordering::Relaxed);
    }
}
