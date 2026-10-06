#![forbid(unsafe_code)]

pub mod assets;
pub mod damage;
pub mod economy;
pub mod enemies;
pub mod equipment;
pub mod gameplay;
pub mod hit_regions;
pub mod hud;
pub mod input;
pub mod interactions;
pub mod life;
pub mod lures;
pub mod mines;
pub mod movement;
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
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .set(bevy::pbr::PbrPlugin {
                use_gpu_instance_buffer_builder: false,
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
                    resolution: (1280, 720).into(),
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
    app
}
