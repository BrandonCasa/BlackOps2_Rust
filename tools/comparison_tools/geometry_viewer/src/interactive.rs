use super::*;
use bevy::{
    input::{
        ButtonState, InputSystems,
        keyboard::{Key, KeyboardInput},
    },
    render::{Render, RenderSystems},
    time::TimeUpdateStrategy,
    window::PrimaryWindow,
};
use bo2_geometry_viewer::camera::{
    InspectionCamera, InspectionCameraPlugin, InspectionControlSystems,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource)]
struct Scene(Option<PreparedGeometry>);
#[derive(Resource)]
struct RetainedSurfaces {
    _records: Vec<bo2_assets::geometry_snapshot::SnapshotSurface>,
}
#[derive(Resource, Clone)]
struct GpuProbe {
    ready: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    positions: usize,
    indices: usize,
}
#[derive(Resource)]
struct Session {
    frames: u32,
    limit: Option<u32>,
    smoke: bool,
    started: Instant,
    home: Transform,
    moved: bool,
    looked: bool,
    reset: bool,
    requested: bool,
    completed: Arc<AtomicBool>,
}

pub fn run(
    geometry: PreparedGeometry,
    frames: Option<u32>,
    smoke: bool,
) -> Result<(), DiagnosticError> {
    run_with_image(geometry, frames, smoke, None)
}
pub fn run_with_image(
    geometry: PreparedGeometry,
    frames: Option<u32>,
    smoke: bool,
    image: Option<Image>,
) -> Result<(), DiagnosticError> {
    let completed = Arc::new(AtomicBool::new(false));
    let gpu = GpuProbe {
        ready: Arc::new(AtomicBool::new(false)),
        failed: Arc::new(AtomicBool::new(false)),
        positions: geometry.positions,
        indices: geometry.indices,
    };
    let mut app = configured_app(true, Some(Window {
        title: "Raw geometry inspection | W/S A/D E/Q arrows Shift/Ctrl R reset Esc exit | fidelity UNKNOWN".into(),
        resolution: bevy::window::WindowResolution::new(960, 720).with_scale_factor_override(1.0),
        resizable: !smoke,
        resize_constraints: bevy::window::WindowResizeConstraints { min_width: 320.0, min_height: 240.0, max_width: 4096.0, max_height: 4096.0 },
        ..default()
    }));
    if let Some(image) = image {
        app.insert_resource(ColorPreview(image));
    }
    app.insert_resource(Scene(Some(geometry)))
        .insert_resource(Session {
            frames: 0,
            limit: frames,
            smoke,
            started: Instant::now(),
            home: Transform::IDENTITY,
            moved: false,
            looked: false,
            reset: false,
            requested: false,
            completed: completed.clone(),
        })
        .insert_resource(gpu.clone())
        .add_plugins(InspectionCameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(First, tick)
        .add_systems(PreUpdate, scripted_input.before(InputSystems))
        .add_systems(Update, observe.after(InspectionControlSystems));
    if smoke {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
    }
    app.get_sub_app_mut(RenderApp)
        .ok_or(DiagnosticError::EngineUnavailable)?
        .insert_resource(gpu)
        .add_systems(Render, pipeline_probe.in_set(RenderSystems::Cleanup));
    let result = app.run();
    if result.is_error() || (frames.is_some() && !completed.load(Ordering::Relaxed)) {
        return Err(DiagnosticError::InteractiveValidation);
    }
    Ok(())
}

fn setup(world: &mut World) {
    let admitted = world
        .get_resource::<RenderAdapter>()
        .is_some_and(|a| a.features().contains(OVERLAY_FEATURES))
        && world
            .get_resource::<RenderDevice>()
            .is_some_and(|d| d.features().contains(OVERLAY_FEATURES));
    if !admitted {
        world.write_message(AppExit::error());
        return;
    }
    let Some(PreparedGeometry {
        mesh,
        surfaces,
        views,
        ..
    }) = world.resource_mut::<Scene>().0.take()
    else {
        world.write_message(AppExit::error());
        return;
    };
    let radius = views[2].projection.near;
    let center = views[2].transform.translation + *views[2].transform.forward() * (3.0 * radius);
    let Some(mut controller) = InspectionCamera::new(center, radius) else {
        world.write_message(AppExit::error());
        return;
    };
    let transform = controller.home(4.0 / 3.0);
    let projection = controller.projection();
    world.resource_mut::<Session>().home = transform;
    spawn_mesh(world, mesh);
    world.insert_resource(RetainedSurfaces { _records: surfaces });
    world.spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::srgb(0.04, 0.04, 0.04).into(),
            ..default()
        },
        Projection::Perspective(projection),
        transform,
        controller,
        Tonemapping::None,
        DebandDither::Disabled,
        Msaa::Off,
    ));
}

fn pipeline_probe(
    cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    probe: Res<GpuProbe>,
) {
    if cache.pipelines().any(|p| matches!(&p.state, CachedPipelineState::Err(error)
        if !matches!(error, ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable))) {
        probe.failed.store(true, Ordering::Relaxed);
    }
    let prepared = meshes.iter().any(|(_, mesh)| mesh.vertex_count as usize == probe.positions
        && matches!(mesh.buffer_info, RenderMeshBufferInfo::Indexed { count, index_format: bevy::render::render_resource::IndexFormat::Uint32 }
            if count as usize == probe.indices));
    if prepared && cache.waiting_pipelines().next().is_none() && cache.pipelines().next().is_some()
    {
        probe.ready.store(true, Ordering::Relaxed);
    }
}

fn tick(mut session: ResMut<Session>, gpu: Res<GpuProbe>, mut exit: MessageWriter<AppExit>) {
    session.frames = session.frames.saturating_add(1);
    if gpu.failed.load(Ordering::Relaxed)
        || (session.limit.is_some() && session.started.elapsed() > Duration::from_secs(60))
    {
        exit.write(AppExit::error());
    } else if session.limit.is_some_and(|limit| session.frames >= limit) {
        if session.smoke {
            exit.write(AppExit::error());
        } else {
            session.completed.store(true, Ordering::Relaxed);
            exit.write(AppExit::Success);
        }
    }
}

fn scripted_input(
    session: Res<Session>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut messages: MessageWriter<KeyboardInput>,
) {
    if !session.smoke {
        return;
    }
    let Some(window) = window.iter().next() else {
        return;
    };
    let key = match session.frames {
        16 => Some((KeyCode::KeyW, ButtonState::Pressed)),
        18 => Some((KeyCode::KeyW, ButtonState::Released)),
        19 => Some((KeyCode::ArrowLeft, ButtonState::Pressed)),
        21 => Some((KeyCode::ArrowLeft, ButtonState::Released)),
        22 => Some((KeyCode::KeyR, ButtonState::Pressed)),
        23 => Some((KeyCode::KeyR, ButtonState::Released)),
        _ => None,
    };
    if let Some((key_code, state)) = key {
        messages.write(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    }
}

fn observe(
    mut commands: Commands,
    mut session: ResMut<Session>,
    gpu: Res<GpuProbe>,
    camera: Query<&Transform, With<InspectionCamera>>,
) {
    if !session.smoke {
        return;
    }
    let Some(transform) = camera.iter().next() else {
        return;
    };
    if session.frames == 17 {
        session.moved = transform.translation.distance(session.home.translation) > 0.0;
    }
    if session.frames == 20 {
        session.looked = transform.rotation.angle_between(session.home.rotation) > 0.001;
    }
    if session.frames == 22 {
        session.reset = transform.translation == session.home.translation
            && transform.rotation == session.home.rotation;
    }
    if session.frames >= 32
        && session.moved
        && session.looked
        && session.reset
        && !session.requested
        && gpu.ready.load(Ordering::Relaxed)
    {
        session.requested = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(captured);
    }
}

fn captured(
    event: On<ScreenshotCaptured>,
    session: Res<Session>,
    gpu: Res<GpuProbe>,
    mut exit: MessageWriter<AppExit>,
) {
    let image = &event.image;
    let pixels = image.data.as_deref().unwrap_or_default();
    let layout = image.width() == 960
        && image.height() == 720
        && pixels.len() == 960 * 720 * 4
        && matches!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
        );
    let foreground = if layout {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] == 255 && p[0] >= 24 && p[0] == p[1] && p[1] == p[2])
            .count()
    } else {
        0
    };
    if foreground > 0
        && session.moved
        && session.looked
        && session.reset
        && !gpu.failed.load(Ordering::Relaxed)
    {
        let result = writeln!(
            io::stdout().lock(),
            "Interactive smoke: PASS (updates={}, GPU mesh/pipelines ready, move/look/reset=true, window readback=960x720, foreground={}). Fidelity UNKNOWN; keyboard messages scripted.",
            session.frames,
            foreground
        );
        if result.is_ok() {
            session.completed.store(true, Ordering::Relaxed);
            exit.write(AppExit::Success);
            return;
        }
    }
    exit.write(AppExit::error());
}
