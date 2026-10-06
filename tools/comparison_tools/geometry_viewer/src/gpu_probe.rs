//! Bounded offscreen readback for material integration checks.
use super::*;
use bevy::render::texture::GpuImage;

#[derive(Resource, Default)]
struct ProbeCapture {
    pixels: Option<Vec<u8>>,
    failed: bool,
}

pub struct ProbeScene {
    pub meshes: Vec<Handle<Mesh>>,
    pub images: Vec<Handle<Image>>,
    pub camera: Transform,
    pub projection: Projection,
    pub output: ProbeOutput,
}

#[derive(Clone, Copy)]
pub struct ProbeOutput {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub msaa: Msaa,
}
impl Default for ProbeOutput {
    fn default() -> Self {
        Self {
            width: IMAGE_SIDE,
            height: IMAGE_SIDE,
            format: TextureFormat::Rgba8UnormSrgb,
            msaa: Msaa::Off,
        }
    }
}

pub struct ProbeResult {
    pub pixels: Vec<u8>,
    pub updates: u32,
}

/// Configure plugins before calling; setup runs after plugin initialization.
/// A successful readback proves GPU execution, not agreement with the original.
pub fn render(
    mut app: App,
    setup: impl FnOnce(&mut World) -> Result<ProbeScene, DiagnosticError>,
) -> Result<ProbeResult, DiagnosticError> {
    let started = Instant::now();
    while app.plugins_state() == PluginsState::Adding {
        if started.elapsed() > Duration::from_secs(60) {
            return Err(DiagnosticError::Deadline);
        }
        bevy::tasks::tick_global_task_pools_on_main_thread();
        std::thread::yield_now();
    }
    app.finish();
    app.cleanup();
    let gpu_failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let gpu_error = gpu_failed.clone();
    app.get_sub_app(RenderApp)
        .ok_or(DiagnosticError::EngineUnavailable)?
        .world()
        .resource::<bevy::render::renderer::RenderDevice>()
        .wgpu_device()
        .on_uncaptured_error(std::sync::Arc::new(move |error| {
            eprintln!("probe GPU error: {error}");
            gpu_error.store(true, std::sync::atomic::Ordering::Relaxed);
        }));
    let scene = setup(app.world_mut())?;
    if scene.meshes.is_empty() {
        return Err(DiagnosticError::EmptyGeometry);
    }
    let output = scene.output;
    if output.width == 0
        || output.height == 0
        || output.width > 4096
        || output.height > 4096
        || !matches!(
            output.format,
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
        )
    {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let expected_bytes = output.width as usize * output.height as usize * 4;
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            output.width,
            output.height,
            output.format,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        scene.projection,
        scene.camera,
        Tonemapping::None,
        DebandDither::Disabled,
        output.msaa,
    ));
    app.init_resource::<ProbeCapture>();
    let mut requested = false;
    for updates in 1..=600 {
        if started.elapsed() > Duration::from_secs(60) {
            return Err(DiagnosticError::Deadline);
        }
        app.update();
        if gpu_failed.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(DiagnosticError::PipelineFailure);
        }
        bevy::tasks::tick_global_task_pools_on_main_thread();
        // Manual offscreen execution must also advance pending pipeline tasks.
        app.get_sub_app_mut(RenderApp)
            .ok_or(DiagnosticError::EngineUnavailable)?
            .world_mut()
            .resource_mut::<PipelineCache>()
            .process_queue();
        app.get_sub_app(RenderApp)
            .ok_or(DiagnosticError::EngineUnavailable)?
            .world()
            .resource::<bevy::render::renderer::RenderDevice>()
            .poll(bevy::render::render_resource::PollType::Poll)
            .map_err(|_| DiagnosticError::ReadbackLayout)?;
        let render_world = app
            .get_sub_app(RenderApp)
            .ok_or(DiagnosticError::EngineUnavailable)?
            .world();
        let cache = render_world
            .get_resource::<PipelineCache>()
            .ok_or(DiagnosticError::EngineUnavailable)?;
        if cache.pipelines().any(|p| matches!(&p.state,
            CachedPipelineState::Err(error) if !matches!(error,
                ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable))) {
            return Err(DiagnosticError::PipelineFailure);
        }
        let meshes_ready = render_world
            .get_resource::<RenderAssets<RenderMesh>>()
            .is_some_and(|assets| scene.meshes.iter().all(|h| assets.get(h.id()).is_some()));
        let images_ready = scene.images.is_empty()
            || render_world
                .get_resource::<RenderAssets<GpuImage>>()
                .is_some_and(|assets| scene.images.iter().all(|h| assets.get(h.id()).is_some()));
        if updates == 100 || updates == 600 {
            if updates == 100 {
                for pipeline in cache.pipelines() {
                    let (label, state) = match &pipeline.state {
                        CachedPipelineState::Queued => ("queued", None),
                        CachedPipelineState::Creating(_) => ("creating", None),
                        CachedPipelineState::Err(ShaderCacheError::ShaderNotLoaded(id)) => {
                            ("missing shader", Some(*id))
                        }
                        CachedPipelineState::Err(ShaderCacheError::ShaderImportNotYetAvailable) => {
                            ("missing import", None)
                        }
                        _ => continue,
                    };
                    eprintln!("probe pending pipeline: {label}, shader={state:?}");
                    if let Some(id) = state {
                        eprintln!(
                            "probe shader asset: {:?}",
                            app.world().resource::<AssetServer>().get_path(id)
                        );
                    }
                }
            }
            let ready_images = render_world
                .get_resource::<RenderAssets<GpuImage>>()
                .map_or(0, |assets| {
                    scene
                        .images
                        .iter()
                        .filter(|h| assets.get(h.id()).is_some())
                        .count()
                });
            eprintln!(
                "probe readiness: updates={updates}, meshes={meshes_ready}, images={ready_images}/{}, pending pipelines={}, pipelines={}, screenshot_requested={requested}",
                scene.images.len(),
                cache.waiting_pipelines().count(),
                cache.pipelines().count()
            );
        }
        if updates >= 20
            && meshes_ready
            && images_ready
            && !requested
            && cache.waiting_pipelines().next().is_none()
            && cache.pipelines().next().is_some()
        {
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(
                    move |event: On<ScreenshotCaptured>, mut capture: ResMut<ProbeCapture>| {
                        let image = &event.image;
                        if image.width() != output.width
                            || image.height() != output.height
                            || image.texture_descriptor.format != output.format
                            || image
                                .data
                                .as_ref()
                                .is_none_or(|data| data.len() != expected_bytes)
                        {
                            capture.failed = true;
                        } else {
                            capture.pixels = image.data.clone();
                        }
                    },
                );
            requested = true;
        }
        let mut capture = app.world_mut().resource_mut::<ProbeCapture>();
        if capture.failed {
            return Err(DiagnosticError::ReadbackLayout);
        }
        if let Some(pixels) = capture.pixels.take() {
            return Ok(ProbeResult { pixels, updates });
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(DiagnosticError::Deadline)
}
