#![forbid(unsafe_code)]
use bevy::{
    app::PluginsState,
    asset::{
        RenderAssetUsages,
        io::{AssetSourceBuilder, AssetSourceId, memory::MemoryAssetReader},
    },
    camera::RenderTarget,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    mesh::{Indices, VertexAttributeValues},
    pbr::wireframe::{Wireframe, WireframeColor, WireframePlugin},
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        mesh::{RenderMesh, RenderMeshBufferInfo},
        render_asset::RenderAssets,
        render_resource::{CachedPipelineState, PipelineCache, TextureFormat, WgpuFeatures},
        renderer::{RenderAdapter, RenderAdapterInfo, RenderDevice},
        settings::WgpuSettings,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    shader::ShaderCacheError,
    window::{ExitCondition, WindowPlugin},
};
use bo2_assets::geometry_snapshot::read_geometry_snapshot;
use bo2_geometry_viewer::{
    DiagnosticError, IMAGE_BYTES, IMAGE_SIDE, PreparedGeometry, VIEW_NAMES, foreground_pixels,
    io_error, prepare, synthetic_snapshot,
};
use image::{ImageEncoder, codecs::png::PngEncoder};
use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

mod captured_material;
mod captured_world;
mod depth_atlas;
mod frame_initialization;
mod full_map_materials;
mod gpu_probe;
mod interactive;
mod material_probe;
mod scene;
mod texture_scene;

#[derive(Resource)]
struct ColorPreview(Image);
#[derive(Resource)]
struct PreviewTextureProof {
    handle: Handle<Image>,
    mips: u32,
    size: bevy::render::render_resource::Extent3d,
    format: TextureFormat,
    bytes: usize,
}

const HELP: &str = "Usage: bo2-geometry-viewer --scene <external-scene.json> (--output <new-external-directory> | --interactive [--frames <64..600>])
Appearance scene v1 currently isolates one surface: unlit base texture and original corner UVs, linear diagnostic transfer. Full original materials/lighting unsupported; fidelity INVESTIGATING.
       bo2-geometry-viewer --texture-scene <external-v2-scene.json> (--output <new-external-directory> | --interactive [--frames <64..600>])
Texture scene is explicitly an unlit first-color-layer diagnostic, not original layered/shaded materials.
Usage: bo2-geometry-viewer --synthetic --output <new-external-directory>
       bo2-geometry-viewer --snapshot <single-external-snapshot> --output <new-external-directory>
       bo2-geometry-viewer (--synthetic | --snapshot <single-external-snapshot>) --interactive [--frames <64..600>]
       bo2-geometry-viewer (--synthetic | --snapshot <single-external-snapshot>) --interactive-smoke [--frames <64..600>]
       bo2-geometry-viewer --help
       bo2-geometry-viewer --check-gpu
       bo2-geometry-viewer --material-smoke
       bo2-geometry-viewer --captured-material <external-v2-scene.json> <external-fixture4462-directory> --output <new-external-directory>
       bo2-geometry-viewer --captured-translated <external-v2-scene.json> <external-fixture4462-directory> <external-shader-bank.json> --output <new-external-directory>
       bo2-geometry-viewer --captured-shadow <external-v2-scene.json> <external-fixture4633-directory> <external-shader-bank.json> --output <new-external-directory>
       bo2-geometry-viewer --native-fly <scene.json> <frame.json> <shader-bank.json> <secondary.json>
       bo2-geometry-viewer --captured-world <scene.json> <frame.json> <shader-bank.json> <secondary.json> --output <new-external-directory>

Finite offscreen GPU diagnostic of project-owned geometry snapshot v1.
Source Z-up positions adapt to canonical Bevy Y-up: (x,y,z) -> (x,z,-y). Reads one
explicit snapshot or an asymmetric synthetic tetrahedron; no discovery/configuration.
Writes raw-z.png, raw-x.png, raw-oblique.png and diagnostic.txt exclusively to a new
directory whose existing parent is outside this workspace. Never overwrites files.
Three fixed 512x512 orthographic Bevy views, identity mesh transform, unchanged
indices/source surface metadata, adapted positions, position-only neutral unlit solid display, culling disabled.
Built-in 1-pixel triangle-edge overlay retains Bevy depth/bias/backface rules;
hidden/back-facing edges are not guaranteed visible. Requires POLYGON_MODE_LINE
and IMMEDIATES; --check-gpu reports supported/enabled adapter features without images.
Legacy raw-* filenames now show canonical Bevy axes, not original front/profile views.
Original orientation, units, visible side, camera, materials and appearance unresolved.
Overall/gameplay/visual fidelity UNKNOWN. Parser limits apply separately from GPU
allocations; camera admission rejects extreme/poorly resolved/empty data. At most
600 app updates and 60 seconds checked between engine calls; use an independent
120-second supervisor because engine/driver/file calls can block. At most three
2-MiB PNG files plus <=2-KiB metadata; failed task-owned outputs may remain.
Interactive mode opens a 960x720 perspective inspection window: W/S forward/back,
A/D strafe, E/Q Bevy world +Y/-Y, arrows look (no roll), Shift 5x speed, Ctrl 0.1x,
R reset/reframe, Escape or window close exit. Speed = bounds radius/second;
axes/speed/projection are diagnostic choices. No collision or original movement.
No output files in interactive mode. --frames bounds app updates and 60 seconds;
--interactive-smoke defaults to 180 updates, exercises move/look/reset through
Bevy keyboard messages, waits for prepared GPU mesh/pipelines and validates one
in-memory window readback before clean exit. External 120-second supervision
still applies. Smoke does not qualify physical keyboard routing or fidelity.
No original game, original decoder or project AssetLoader.
Errors report type/stage only. Exit: 0 success/help, 1 diagnostic failure, 2 arguments.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum SceneInput {
    Raw,
    Single,
    TextureDiagnostic,
}

enum Action {
    Help,
    CheckGpu,
    MaterialSmoke,
    NativeFly {
        scene: OsString,
        frame: OsString,
        bank: OsString,
        secondary: OsString,
    },
    CapturedWorld {
        scene: OsString,
        frame: OsString,
        bank: OsString,
        secondary: OsString,
        output: OsString,
    },
    CapturedMaterial {
        scene: OsString,
        fixture: OsString,
        bank: Option<OsString>,
        shadow: bool,
        output: OsString,
    },
    Render {
        input: Option<OsString>,
        scene: SceneInput,
        output: OsString,
    },
    Interactive {
        input: Option<OsString>,
        scene: SceneInput,
        frames: Option<u32>,
        smoke: bool,
    },
}
fn arguments(mut args: impl Iterator<Item = OsString>) -> Result<Action, ()> {
    let Some(first) = args.next() else {
        return Err(());
    };
    if (first == "--help" || first == "-h") && args.next().is_none() {
        return Ok(Action::Help);
    }
    if first == "--check-gpu" && args.next().is_none() {
        return Ok(Action::CheckGpu);
    }
    if first == "--material-smoke" && args.next().is_none() {
        return Ok(Action::MaterialSmoke);
    }
    if first == "--native-fly" {
        let result = Action::NativeFly {
            scene: args.next().ok_or(())?,
            frame: args.next().ok_or(())?,
            bank: args.next().ok_or(())?,
            secondary: args.next().ok_or(())?,
        };
        if args.next().is_some() {
            return Err(());
        }
        return Ok(result);
    }
    if first == "--captured-world" {
        let scene = args.next().ok_or(())?;
        let frame = args.next().ok_or(())?;
        let bank = args.next().ok_or(())?;
        let secondary = args.next().ok_or(())?;
        if args.next().ok_or(())? != "--output" {
            return Err(());
        }
        let output = args.next().ok_or(())?;
        if args.next().is_some() {
            return Err(());
        }
        return Ok(Action::CapturedWorld {
            scene,
            frame,
            bank,
            secondary,
            output,
        });
    }
    if first == "--captured-material"
        || first == "--captured-translated"
        || first == "--captured-shadow"
    {
        let scene = args.next().ok_or(())?;
        let fixture = args.next().ok_or(())?;
        let bank = if first == "--captured-translated" || first == "--captured-shadow" {
            Some(args.next().ok_or(())?)
        } else {
            None
        };
        if args.next().ok_or(())? != "--output" {
            return Err(());
        }
        let output = args.next().ok_or(())?;
        if args.next().is_some() {
            return Err(());
        }
        return Ok(Action::CapturedMaterial {
            scene,
            fixture,
            bank,
            shadow: first == "--captured-shadow",
            output,
        });
    }
    let input = if first == "--synthetic" {
        None
    } else if first == "--snapshot" || first == "--scene" || first == "--texture-scene" {
        Some(args.next().ok_or(())?)
    } else {
        return Err(());
    };
    let scene = if first == "--scene" {
        SceneInput::Single
    } else if first == "--texture-scene" {
        SceneInput::TextureDiagnostic
    } else {
        SceneInput::Raw
    };
    let mode = args.next().ok_or(())?;
    if mode == "--interactive" || mode == "--interactive-smoke" {
        let smoke = mode == "--interactive-smoke";
        if scene != SceneInput::Raw && smoke {
            return Err(());
        }
        let frames = match args.next() {
            None => smoke.then_some(180),
            Some(flag) if flag == "--frames" => {
                let value = args.next().ok_or(())?;
                let frames = value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?;
                if !(64..=600).contains(&frames) || args.next().is_some() {
                    return Err(());
                }
                Some(frames)
            }
            _ => return Err(()),
        };
        return Ok(Action::Interactive {
            input,
            scene,
            frames,
            smoke,
        });
    }
    if mode != "--output" {
        return Err(());
    }
    let output = args.next().ok_or(())?;
    if args.next().is_some() {
        return Err(());
    }
    Ok(Action::Render {
        input,
        scene,
        output,
    })
}
fn main() -> ExitCode {
    let action = match arguments(env::args_os().skip(1)) {
        Ok(action) => action,
        Err(()) => {
            let _ = writeln!(io::stderr().lock(), "invalid arguments; use --help");
            return ExitCode::from(2);
        }
    };
    // Redact recoverable engine/driver panics. Aborts and blocking calls remain external-supervisor limits.
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(|| match action {
        Action::Help => writeln!(io::stdout().lock(), "{HELP}").map_err(|e| io_error("stdout", e)),
        Action::CheckGpu => check_gpu(),
        Action::MaterialSmoke => material_probe::run(),
        Action::NativeFly {
            scene,
            frame,
            bank,
            secondary,
        } => captured_world::fly(
            Path::new(&scene),
            Path::new(&frame),
            Path::new(&bank),
            Path::new(&secondary),
        ),
        Action::CapturedWorld {
            scene,
            frame,
            bank,
            secondary,
            output,
        } => captured_world::run(
            Path::new(&scene),
            Path::new(&frame),
            Path::new(&bank),
            Path::new(&secondary),
            Path::new(&output),
        ),
        Action::CapturedMaterial {
            scene,
            fixture,
            bank,
            shadow,
            output,
        } if shadow => captured_material::run_shadow(
            Path::new(&scene),
            Path::new(&fixture),
            Path::new(bank.as_ref().ok_or(DiagnosticError::Preservation)?),
            Path::new(&output),
        ),
        Action::CapturedMaterial {
            scene,
            fixture,
            bank,
            output,
            ..
        } => captured_material::run(
            Path::new(&scene),
            Path::new(&fixture),
            bank.as_deref().map(Path::new),
            Path::new(&output),
        ),
        Action::Render {
            input,
            scene,
            output,
        } => match scene {
            SceneInput::Single => scene::execute(
                Path::new(input.as_ref().ok_or(DiagnosticError::Encoding)?),
                Path::new(&output),
            ),
            SceneInput::TextureDiagnostic => texture_scene::offscreen(
                Path::new(input.as_ref().ok_or(DiagnosticError::Encoding)?),
                Path::new(&output),
            ),
            SceneInput::Raw => execute(input.as_deref(), Path::new(&output)),
        },
        Action::Interactive {
            input,
            scene,
            frames,
            smoke,
        } => match scene {
            SceneInput::Single => scene::interactive(
                Path::new(input.as_ref().ok_or(DiagnosticError::Encoding)?),
                frames,
                smoke,
            ),
            SceneInput::TextureDiagnostic => texture_scene::interactive(
                Path::new(input.as_ref().ok_or(DiagnosticError::Encoding)?),
                frames,
            ),
            SceneInput::Raw => interactive::run(load_geometry(input.as_deref())?, frames, smoke),
        },
    })
    .unwrap_or(Err(DiagnosticError::EnginePanic));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "diagnostic failed: {error}");
            ExitCode::FAILURE
        }
    }
}
fn workspace_root() -> Result<PathBuf, DiagnosticError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .map_err(|e| io_error("workspace identity", e))
}
fn outside_workspace(path: &Path) -> Result<(), DiagnosticError> {
    let root = workspace_root()?;
    let candidate = path.to_string_lossy();
    let root = root.to_string_lossy();
    let (candidate, root) = if cfg!(windows) {
        (candidate.to_lowercase(), root.to_lowercase())
    } else {
        (candidate.into_owned(), root.into_owned())
    };
    if candidate == root || candidate.starts_with(&(root + std::path::MAIN_SEPARATOR_STR)) {
        return Err(DiagnosticError::WorkspacePath);
    }
    Ok(())
}
fn new_output(path: &Path) -> Result<PathBuf, DiagnosticError> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(DiagnosticError::WorkspacePath);
    }
    let parent = path
        .parent()
        .ok_or(DiagnosticError::WorkspacePath)?
        .canonicalize()
        .map_err(|e| io_error("output parent", e))?;
    outside_workspace(&parent)?;
    let output = parent.join(path.file_name().ok_or(DiagnosticError::WorkspacePath)?);
    fs::create_dir(&output).map_err(|e| io_error("output reservation", e))?;
    Ok(output)
}
fn load_geometry(input: Option<&std::ffi::OsStr>) -> Result<PreparedGeometry, DiagnosticError> {
    let snapshot = match input {
        None => synthetic_snapshot()?,
        Some(input) => {
            let path = Path::new(input)
                .canonicalize()
                .map_err(|e| io_error("input identity", e))?;
            outside_workspace(&path)?;
            let mut file = File::open(path).map_err(|e| io_error("input open", e))?;
            let metadata = file.metadata().map_err(|e| io_error("input metadata", e))?;
            if !metadata.is_file() {
                return Err(DiagnosticError::NotRegularFile);
            }
            read_geometry_snapshot(&mut file, metadata.len()).map_err(DiagnosticError::Snapshot)?
        }
    };
    prepare(snapshot)
}
fn execute(input: Option<&std::ffi::OsStr>, output: &Path) -> Result<(), DiagnosticError> {
    let geometry = load_geometry(input)?;
    let output = new_output(output)?;
    let report = render(geometry, None)?;
    let mut coverage = [0; 3];
    for (i, bytes) in report.images.iter().enumerate() {
        coverage[i] = foreground_pixels(bytes)?;
        let file = exclusive_file(&output.join(format!("{}.png", VIEW_NAMES[i])))?;
        let mut writer = LimitedWriter {
            inner: BufWriter::new(file),
            remaining: 2 * 1024 * 1024,
        };
        PngEncoder::new(&mut writer)
            .write_image(
                bytes,
                IMAGE_SIDE,
                IMAGE_SIDE,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|_| DiagnosticError::Encoding)?;
        writer.flush().map_err(|e| io_error("PNG flush", e))?;
    }
    let summary = format!(
        "diagnostic: canonical Bevy geometry; source Z-up -> Y-up\nmode: {}\nimage_dimensions: 512x512 RGBA8 sRGB\nviews: raw-z raw-x raw-oblique\nmesh_transform: identity\nmaterial: neutral unlit solid; culling disabled\noverlay: Bevy indexed triangles; width=1; depth-tested; slope bias=1; backface culling\nGPU_features_supported_enabled: POLYGON_MODE_LINE IMMEDIATES\nframing: canonical bounds sphere; orthographic; positions=(x,z,-y), source metadata retained\noriginal_orientation_units_visible_side_camera_material: unresolved\noverall_gameplay_visual_fidelity: UNKNOWN\nbevy: 0.19.1\nbackend: {}\nvendor_device: {} {}\napp_updates: {}\nrender_world_mesh: prepared indexed U32 position-only\nCPU_geometry_retained: true\npositions: {}\nindices: {}\nsurfaces_retained: {}\nforeground_pixels_raw_z_x_oblique: {} {} {}\n",
        if input.is_some() {
            "single snapshot"
        } else {
            "synthetic fixture"
        },
        report.backend,
        report.vendor,
        report.device,
        report.frames,
        report.positions,
        report.indices,
        report.surfaces,
        coverage[0],
        coverage[1],
        coverage[2]
    );
    let mut writer = LimitedWriter {
        inner: exclusive_file(&output.join("diagnostic.txt"))?,
        remaining: 2048,
    };
    writer
        .write_all(summary.as_bytes())
        .map_err(|e| io_error("metadata write", e))?;
    writer.flush().map_err(|e| io_error("metadata flush", e))?;
    writeln!(
        io::stdout().lock(),
        "{summary}output: three exclusive diagnostic PNGs and metadata"
    )
    .map_err(|e| io_error("stdout", e))
}
fn exclusive_file(path: &Path) -> Result<File, DiagnosticError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| io_error("output create", e))
}
struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::ErrorKind::FileTooLarge.into());
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
#[derive(Resource, Default)]
struct Captures {
    images: [Option<Vec<u8>>; 3],
    failed: bool,
}
struct Report {
    images: [Vec<u8>; 3],
    frames: u32,
    backend: String,
    vendor: u32,
    device: u32,
    positions: usize,
    indices: usize,
    surfaces: usize,
}
const OVERLAY_FEATURES: WgpuFeatures =
    WgpuFeatures::POLYGON_MODE_LINE.union(WgpuFeatures::IMMEDIATES);

fn configured_app(overlay: bool, window: Option<Window>) -> App {
    let mut app = App::new();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(|| Box::new(MemoryAssetReader::default())),
    );
    let mut settings = WgpuSettings::default();
    if overlay {
        settings.features |= OVERLAY_FEATURES;
    }
    let interactive = window.is_some();
    let plugins = DefaultPlugins
        .set(RenderPlugin {
            render_creation: settings.into(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: window,
            exit_condition: if interactive {
                ExitCondition::OnPrimaryClosed
            } else {
                ExitCondition::DontExit
            },
            ..default()
        });
    // Enabling the window dependency must not create an event loop in legacy offscreen paths.
    if interactive {
        app.add_plugins(plugins);
    } else {
        app.add_plugins(plugins.disable::<bevy::winit::WinitPlugin>());
    }
    if overlay {
        app.add_plugins(WireframePlugin::default());
    }
    app
}
fn render_app(overlay: bool, started: Instant) -> Result<App, DiagnosticError> {
    let deadline = Duration::from_secs(60);
    let mut app = configured_app(overlay, None);
    while app.plugins_state() == PluginsState::Adding {
        if started.elapsed() > deadline {
            return Err(DiagnosticError::Deadline);
        }
        bevy::tasks::tick_global_task_pools_on_main_thread();
        std::thread::yield_now();
    }
    app.finish();
    app.cleanup();
    Ok(app)
}

fn spawn_mesh(world: &mut World, mesh: Mesh) -> Handle<Mesh> {
    let handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
    if let Some(preview) = world.remove_resource::<ColorPreview>() {
        let mips = preview.0.texture_descriptor.mip_level_count;
        let size = preview.0.texture_descriptor.size;
        let format = preview.0.texture_descriptor.format;
        let bytes = preview.0.data.as_ref().map_or(0, Vec::len);
        let image = world.resource_mut::<Assets<Image>>().add(preview.0);
        world.insert_resource(PreviewTextureProof {
            handle: image.clone(),
            mips,
            size,
            format,
            bytes,
        });
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color_texture: Some(image),
                unlit: true,
                cull_mode: None,
                ..default()
            });
        world.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
        ));
        return handle;
    }
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.72, 0.72),
            unlit: true,
            cull_mode: None,
            ..default()
        });
    world.spawn((
        Mesh3d(handle.clone()),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        Wireframe,
        WireframeColor {
            color: Color::srgb(0.12, 0.12, 0.12),
        },
    ));
    handle
}

fn check_gpu() -> Result<(), DiagnosticError> {
    let app = render_app(false, Instant::now())?;
    feature_check(&app)?;
    let adapter = app.world().resource::<RenderAdapterInfo>();
    writeln!(io::stdout().lock(), "GPU qualification: supported/enabled POLYGON_MODE_LINE IMMEDIATES\nbackend: {:?}\nvendor_device: {} {}\nno images; diagnostic features only; fidelity UNKNOWN", adapter.backend, adapter.vendor, adapter.device).map_err(|e| io_error("stdout", e))
}

fn feature_check(app: &App) -> Result<(), DiagnosticError> {
    let adapter = app
        .world()
        .get_resource::<RenderAdapter>()
        .ok_or(DiagnosticError::EngineUnavailable)?;
    let device = app
        .world()
        .get_resource::<RenderDevice>()
        .ok_or(DiagnosticError::EngineUnavailable)?;
    if !adapter.features().contains(OVERLAY_FEATURES)
        || !device.features().contains(OVERLAY_FEATURES)
    {
        return Err(DiagnosticError::OverlayFeaturesUnavailable);
    }
    Ok(())
}

fn render(geometry: PreparedGeometry, image: Option<Image>) -> Result<Report, DiagnosticError> {
    let appearance = image.is_some();
    let started = Instant::now();
    let deadline = Duration::from_secs(60);
    let mut app = render_app(true, started)?;
    if let Some(image) = image {
        app.insert_resource(ColorPreview(image));
    }
    feature_check(&app)?;
    let adapter = app
        .world()
        .get_resource::<RenderAdapterInfo>()
        .ok_or(DiagnosticError::EngineUnavailable)?;
    let (backend, vendor, device) = (
        format!("{:?}", adapter.backend),
        adapter.vendor,
        adapter.device,
    );
    let PreparedGeometry {
        mesh,
        surfaces,
        views,
        positions,
        indices,
    } = geometry;
    let position_pointer = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.as_ptr(),
        _ => return Err(DiagnosticError::Preservation),
    };
    let index_pointer = match mesh.indices() {
        Some(Indices::U32(v)) => v.as_ptr(),
        _ => return Err(DiagnosticError::Preservation),
    };
    let handle = spawn_mesh(app.world_mut(), mesh);
    let targets = views.map(|view| {
        let target =
            app.world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::new_target_texture(
                    IMAGE_SIDE,
                    IMAGE_SIDE,
                    TextureFormat::Rgba8UnormSrgb,
                    None,
                ));
        app.world_mut().spawn((
            Camera3d::default(),
            Camera {
                clear_color: Color::srgb(0.04, 0.04, 0.04).into(),
                ..default()
            },
            RenderTarget::Image(target.clone().into()),
            Projection::Orthographic(view.projection),
            view.transform,
            Tonemapping::None,
            DebandDither::Disabled,
            Msaa::Off,
        ));
        target
    });
    app.init_resource::<Captures>();
    let mut requested = false;
    for frames in 1..=600 {
        if started.elapsed() > deadline {
            return Err(DiagnosticError::Deadline);
        }
        app.update();
        let render_world = app
            .get_sub_app(RenderApp)
            .ok_or(DiagnosticError::EngineUnavailable)?
            .world();
        let cache = render_world
            .get_resource::<PipelineCache>()
            .ok_or(DiagnosticError::EngineUnavailable)?;
        if cache
            .pipelines()
            .any(|p| matches!(&p.state, CachedPipelineState::Err(error) if !matches!(error, ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable)))
        {
            return Err(DiagnosticError::PipelineFailure);
        }
        let gpu_mesh = render_world
            .get_resource::<RenderAssets<RenderMesh>>()
            .and_then(|assets| assets.get(handle.id()));
        let prepared = gpu_mesh.is_some_and(|m| m.vertex_count as usize == positions && matches!(m.buffer_info, RenderMeshBufferInfo::Indexed { count, index_format: bevy::render::render_resource::IndexFormat::Uint32 } if count as usize == indices));
        let texture_ready = app
            .world()
            .get_resource::<PreviewTextureProof>()
            .is_none_or(|proof| {
                render_world
                    .get_resource::<RenderAssets<bevy::render::texture::GpuImage>>()
                    .and_then(|assets| assets.get(proof.handle.id()))
                    .is_some_and(|image| {
                        image.had_data
                            && image.texture_descriptor.mip_level_count == proof.mips
                            && image.texture_descriptor.size == proof.size
                            && image.texture_descriptor.format == proof.format
                    })
            });
        if !requested
            && texture_ready
            && frames >= 16
            && prepared
            && cache.waiting_pipelines().next().is_none()
            && cache.pipelines().next().is_some()
        {
            if let Some(proof) = app.world().get_resource::<PreviewTextureProof>() {
                println!(
                    "Appearance GPU image prepared: {}x{}, {} mips, {:?}, {} source chain bytes",
                    proof.size.width, proof.size.height, proof.mips, proof.format, proof.bytes
                );
            }
            for (i, target) in targets.iter().enumerate() {
                app.world_mut()
                    .spawn(Screenshot::image(target.clone()))
                    .observe(
                        move |event: On<ScreenshotCaptured>, mut captures: ResMut<Captures>| {
                            let image = &event.image;
                            if image.width() != IMAGE_SIDE
                                || image.height() != IMAGE_SIDE
                                || image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
                                || image
                                    .data
                                    .as_ref()
                                    .is_none_or(|data| data.len() != IMAGE_BYTES)
                            {
                                captures.failed = true;
                            } else {
                                captures.images[i] = image.data.clone();
                            }
                        },
                    );
            }
            requested = true;
        }
        let captures = app.world().resource::<Captures>();
        if captures.failed {
            return Err(DiagnosticError::ReadbackLayout);
        }
        if captures.images.iter().all(Option::is_some) {
            let cpu = app
                .world()
                .resource::<Assets<Mesh>>()
                .get(&handle)
                .ok_or(DiagnosticError::Preservation)?;
            if !cpu
                .asset_usage
                .contains(RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD)
                || cpu.attributes().count() != if appearance { 2 } else { 1 }
                || !matches!(cpu.attribute(Mesh::ATTRIBUTE_POSITION), Some(VertexAttributeValues::Float32x3(v)) if v.len() == positions && v.as_ptr() == position_pointer)
                || !matches!(cpu.indices(), Some(Indices::U32(v)) if v.len() == indices && v.as_ptr() == index_pointer)
            {
                return Err(DiagnosticError::Preservation);
            }
            let images = app
                .world_mut()
                .resource_mut::<Captures>()
                .images
                .each_mut()
                .map(|image| image.take().ok_or(DiagnosticError::ReadbackLayout));
            let [z, x, oblique] = images;
            return Ok(Report {
                images: [z?, x?, oblique?],
                frames,
                backend,
                vendor,
                device,
                positions,
                indices,
                surfaces: surfaces.len(),
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(DiagnosticError::Deadline)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_modes_require_a_single_input_and_output() {
        let parse = |args: &[&str]| arguments(args.iter().map(OsString::from));
        assert!(matches!(
            parse(&["--synthetic", "--output", "out"]),
            Ok(Action::Render { input: None, .. })
        ));
        assert!(matches!(
            parse(&["--snapshot", "-input", "--output", "out"]),
            Ok(Action::Render { input: Some(_), .. })
        ));
        for invalid in [
            &[][..],
            &["--synthetic"][..],
            &["--snapshot", "in", "--output", "out", "extra"][..],
            &["in", "out"][..],
        ] {
            assert!(parse(invalid).is_err());
        }
    }
    #[test]
    fn interactive_modes_admit_one_input_and_bounded_frames_only() {
        let parse = |args: &[&str]| arguments(args.iter().map(OsString::from));
        assert!(matches!(
            parse(&["--synthetic", "--interactive"]),
            Ok(Action::Interactive {
                input: None,
                frames: None,
                smoke: false,
                scene: SceneInput::Raw
            })
        ));
        assert!(matches!(
            parse(&["--snapshot", "in", "--interactive-smoke"]),
            Ok(Action::Interactive {
                input: Some(_),
                frames: Some(180),
                smoke: true,
                scene: SceneInput::Raw
            })
        ));
        for frames in ["64", "600"] {
            assert!(matches!(
                parse(&["--synthetic", "--interactive", "--frames", frames]),
                Ok(Action::Interactive {
                    frames: Some(_),
                    ..
                })
            ));
        }
        for invalid in [
            &["--synthetic", "--interactive", "--frames"][..],
            &["--synthetic", "--interactive", "--frames", "0"][..],
            &["--synthetic", "--interactive", "--frames", "63"][..],
            &["--synthetic", "--interactive-smoke", "--frames", "601"][..],
            &["--synthetic", "--interactive-smoke", "--frames", "NaN"][..],
            &["--synthetic", "--interactive", "--output", "out"][..],
            &["--synthetic", "--interactive", "--frames", "180", "extra"][..],
            &["--snapshot", "--interactive"][..],
        ] {
            assert!(parse(invalid).is_err());
        }
    }
    #[test]
    fn scene_modes_keep_diagnostic_smoke_separate() {
        let parse = |args: &[&str]| arguments(args.iter().map(OsString::from));
        assert!(matches!(
            parse(&["--scene", "scene.json", "--output", "new"]),
            Ok(Action::Render {
                scene: SceneInput::Single,
                ..
            })
        ));
        assert!(matches!(
            parse(&["--scene", "scene.json", "--interactive"]),
            Ok(Action::Interactive {
                scene: SceneInput::Single,
                ..
            })
        ));
        assert!(parse(&["--scene", "scene.json", "--interactive-smoke"]).is_err());
    }
    #[test]
    fn bounded_writer_fails_without_writing_past_the_cap() {
        let mut writer = LimitedWriter {
            inner: Vec::new(),
            remaining: 4,
        };
        writer.write_all(b"abc").unwrap();
        assert_eq!(
            writer.write_all(b"de").unwrap_err().kind(),
            io::ErrorKind::FileTooLarge
        );
        assert_eq!(writer.inner, b"abc");
    }
    #[test]
    fn texture_diagnostic_is_explicit_and_material_probe_takes_no_original_input() {
        let parse = |args: &[&str]| arguments(args.iter().map(OsString::from));
        assert!(matches!(
            parse(&["--texture-scene", "scene.json", "--interactive"]),
            Ok(Action::Interactive {
                scene: SceneInput::TextureDiagnostic,
                ..
            })
        ));
        assert!(matches!(
            parse(&["--texture-scene", "scene.json", "--output", "new"]),
            Ok(Action::Render {
                scene: SceneInput::TextureDiagnostic,
                ..
            })
        ));
        assert!(matches!(
            parse(&["--material-smoke"]),
            Ok(Action::MaterialSmoke)
        ));
        assert!(parse(&["--material-smoke", "scene.json"]).is_err());
        assert!(parse(&["--texture-scene", "scene.json", "--interactive-smoke"]).is_err());
    }
    #[test]
    fn error_discards_private_paths_and_raw_messages() {
        let error = io_error(
            "input open",
            io::Error::new(io::ErrorKind::PermissionDenied, "PRIVATE_PATH_MESSAGE"),
        );
        assert_eq!(error.to_string(), "input open I/O (PermissionDenied)");
        assert!(!format!("{error:?}").contains("PRIVATE"));
    }
}
