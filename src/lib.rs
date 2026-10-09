#![forbid(unsafe_code)]

#[cfg(all(not(debug_assertions), feature = "dev-dynamic"))]
compile_error!("Bevy dynamic linking is development-only; omit dev-dynamic for release builds.");

#[cfg(all(
    not(debug_assertions),
    any(
        feature = "profile-tracy",
        feature = "profile-tracy-memory",
        feature = "profile-chrome"
    )
))]
compile_error!(
    "Release builds forbid profiling; use the development profile for Tracy or Chrome captures."
);

pub mod access;
pub mod assets;
pub mod audio;
pub mod barriers;
pub mod damage;
pub mod economy;
pub mod enemies;
pub mod environment;
pub mod equipment;
pub mod gameplay;
pub mod generators;
pub mod hit_regions;
pub mod hud;
pub mod input;
pub mod interactions;
pub mod life;
pub mod lures;
pub mod mines;
pub mod movement;
pub mod navigation;
pub mod origins;
pub mod perk_effects;
pub mod perks;
pub mod player;
pub mod player_health;
pub mod presentation;
pub mod rendering;
pub mod rounds;
mod run_environment;
pub mod solo_revive;
pub mod verification;
pub mod weapons;

use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
    Paused,
    GameOver,
    LoadError,
}

pub fn app(arguments: impl Iterator<Item = std::ffi::OsString>) -> App {
    let options = assets::LaunchOptions::parse(arguments);
    let configuration = assets::AssetPack::discover();
    let mut app = App::new();
    let benchmark = options.benchmark;
    let release_benchmark = options.release_benchmark;
    let gpu_preprocessing = options.mesh_preprocessing != assets::MeshPreprocessing::Cpu;
    let ads_capture = options.verify_ads;
    app.insert_resource(options);
    let default_plugins = DefaultPlugins.build();
    #[cfg(not(debug_assertions))]
    let default_plugins = default_plugins.disable::<bevy::log::LogPlugin>();
    app.add_plugins(
        default_plugins
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .set(bevy::log::LogPlugin {
                custom_layer: verification::error_layer,
                ..default()
            })
            .set(bevy::pbr::PbrPlugin {
                use_gpu_instance_buffer_builder: gpu_preprocessing,
                ..default()
            })
            .set(
                bevy::gltf::GltfPlugin::default()
                    .add_custom_vertex_attribute("ORIGINS_LAYER_UV", rendering::LAYER_UV)
                    .add_custom_vertex_attribute("ORIGINS_LAYER3_UV", rendering::LAYER3_UV)
                    .add_custom_vertex_attribute("ORIGINS_ROTATION1", rendering::ROTATION1)
                    .add_custom_vertex_attribute("ORIGINS_ROTATION2", rendering::ROTATION2),
            )
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Origins".into(),
                    resolution: if ads_capture {
                        bevy::window::WindowResolution::new(1920, 1080)
                            .with_scale_factor_override(1.)
                    } else {
                        bevy::window::WindowResolution::new(1920, 1080)
                    },
                    present_mode: if benchmark {
                        bevy::window::PresentMode::AutoNoVsync
                    } else {
                        default()
                    },
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: configuration
                    .as_ref()
                    .map(|c| c.pack_root.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                ..default()
            }),
    )
    .add_plugins(bevy::anti_alias::smaa::SmaaPlugin)
    .add_plugins(avian3d::prelude::PhysicsPlugins::default())
    .init_state::<GameState>()
    .add_systems(Startup, initial_physical_resolution)
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(assets::LoadConfiguration(configuration))
    .add_plugins((
        assets::GameAssetsPlugin,
        input::GameInputPlugin,
        rendering::GameRenderingPlugin,
        player::PlayerPlugin,
        gameplay::GameplayPlugin,
        damage::DamagePlugin,
        (
            life::LifeStatePlugin,
            solo_revive::SoloRevivePlugin,
            perks::PerkPlugin,
            perk_effects::PerkEffectsPlugin,
            environment::EnvironmentPlugin,
            access::AccessPlugin,
            navigation::NavigationPlugin,
            barriers::BarrierPlugin,
            generators::GeneratorPlugin,
            audio::GameAudioPlugin,
        ),
        player_health::PlayerHealthPlugin,
        enemies::EnemyPlugin,
        hit_regions::HitRegionsPlugin,
        equipment::EquipmentPlugin,
        interactions::InteractionPlugin,
        presentation::PresentationPlugin,
        hud::HudPlugin,
        verification::VerificationPlugin,
    ));
    if cfg!(debug_assertions)
        && !release_benchmark
        && (benchmark || cfg!(feature = "profile-tracy") || cfg!(feature = "profile-chrome"))
    {
        if !app.is_plugin_added::<bevy::render::diagnostic::RenderDiagnosticsPlugin>() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
    }
    app.add_plugins(run_environment::UnattendedRunPlugin);
    app
}

fn initial_physical_resolution(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>) {
    // Winit initially interprets the requested size as logical pixels. Resize
    // after creation, when the native DPI is known, without overriding UI scaling.
    if let Ok(mut window) = windows.single_mut()
        && (window.physical_width(), window.physical_height()) != (1920, 1080)
    {
        window.resolution.set_physical_resolution(1920, 1080);
    }
}
