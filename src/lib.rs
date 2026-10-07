#![forbid(unsafe_code)]

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
    let gpu_preprocessing = options.mesh_preprocessing != assets::MeshPreprocessing::Cpu;
    let ads_capture = options.verify_ads;
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
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
                        (1280, 720).into()
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
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(assets::LoadConfiguration(configuration))
    .insert_resource(options)
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
    if benchmark && !app.is_plugin_added::<bevy::render::diagnostic::RenderDiagnosticsPlugin>() {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
    }
    if benchmark {
        // Unfocused reactive updates otherwise throttle the native benchmark,
        // independently of VSync and the renderer's actual throughput.
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    app
}
