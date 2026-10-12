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
pub mod achievements;
pub mod actions;
mod animation;
pub mod assets;
pub mod audio;
pub mod barriers;
pub mod buildables;
pub mod chamber;
pub mod cinematics;
pub mod combat_effects;
pub mod crafting;
pub mod damage;
pub mod economy;
pub mod enemies;
pub mod environment;
pub mod equipment;
pub mod finale;
pub mod flythrough;
pub mod frontend;
pub mod gameplay;
pub mod generators;
pub mod hit_regions;
pub mod hud;
pub mod input;
pub mod interactions;
pub mod life;
pub mod lures;
pub mod main_quest;
pub mod mines;
pub mod movement;
mod native_fx;
pub mod navigation;
pub mod origins;
pub mod panzer;
pub mod perk_effects;
pub mod perks;
pub mod player;
pub mod player_health;
pub mod portals;
pub mod powerups;
pub mod presentation;
pub mod projectiles;
pub mod quest_recovery;
pub mod quests;
mod render_backend;
pub mod rendering;
pub mod rituals;
pub mod robots;
pub mod rounds;
mod run_environment;
mod session;
pub mod side_rewards;
pub mod side_secrets;
pub mod solo_revive;
pub mod staff_fields;
pub mod staff_puzzles;
pub mod staff_upgrades;
pub mod stations;
pub mod tablets;
pub mod tank;
#[cfg(feature = "prepare-textures")]
pub mod texture_preparation;
pub mod verification;
pub mod weapons;

use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    PreparingShaders,
    Playing,
    Paused,
    GameOver,
    Ending,
    LoadError,
}

pub fn app(arguments: impl Iterator<Item = std::ffi::OsString>) -> App {
    let options = assets::LaunchOptions::parse(arguments);
    configured_app(options)
}

pub(crate) fn configured_app(options: assets::LaunchOptions) -> App {
    let configuration = assets::AssetPack::discover();
    let mut app = App::new();
    let benchmark = options.benchmark;
    let release_benchmark = options.release_benchmark;
    let gpu_preprocessing = options.mesh_preprocessing != assets::MeshPreprocessing::Cpu;
    let ads_capture = options.verify_ads;
    let fixed_resolution = ads_capture || benchmark || options.validation_remote.is_some();
    let flythrough = options.flythrough;
    app.insert_resource(options);
    let default_plugins = DefaultPlugins.build();
    #[cfg(not(debug_assertions))]
    let default_plugins = default_plugins.disable::<bevy::log::LogPlugin>();
    app.add_plugins(
        default_plugins
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .set(bevy::render::RenderPlugin {
                render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
                    render_backend::settings(),
                )),
                ..default()
            })
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
                    title: if flythrough {
                        "Origins — visual flythrough"
                    } else {
                        "Origins"
                    }
                    .into(),
                    mode: bevy::window::WindowMode::Windowed,
                    decorations: false,
                    resizable: !ads_capture,
                    enabled_buttons: bevy::window::EnabledButtons {
                        maximize: !ads_capture,
                        ..default()
                    },
                    resolution: if fixed_resolution {
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
    .add_plugins(bevy::anti_alias::taa::TemporalAntiAliasPlugin)
    .add_plugins(avian3d::prelude::PhysicsPlugins::default())
    .init_state::<GameState>()
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(assets::LoadConfiguration(configuration))
    .add_plugins(animation::GameAnimationPlugin)
    .add_plugins((
        assets::GameAssetsPlugin,
        input::GameInputPlugin,
        rendering::GameRenderingPlugin,
        player::PlayerPlugin,
        gameplay::GameplayPlugin,
        damage::DamagePlugin,
        (
            (
                session::SessionPlugin,
                frontend::FrontendPlugin,
                quests::QuestPlugin,
                stations::StationPlugin,
                crafting::CraftingPlugin,
                buildables::BuildablePlugin,
                tank::TankPlugin,
                combat_effects::CombatEffectsPlugin,
                panzer::PanzerPlugin,
                life::LifeStatePlugin,
            ),
            (
                solo_revive::SoloRevivePlugin,
                perks::PerkPlugin,
                perk_effects::PerkEffectsPlugin,
                environment::EnvironmentPlugin,
                powerups::PowerupPlugin,
                access::AccessPlugin,
                navigation::NavigationPlugin,
                barriers::BarrierPlugin,
                generators::GeneratorPlugin,
                audio::GameAudioPlugin,
            ),
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
    app.add_plugins((
        native_fx::NativeFxPlugin,
        actions::ActionPlugin,
        side_rewards::SideRewardPlugin,
        side_secrets::SecretPlugin,
        achievements::AchievementPlugin,
    ));
    app.add_plugins((
        portals::PortalPlugin,
        robots::RobotPlugin,
        rituals::RitualPlugin,
        staff_fields::StaffFieldsPlugin,
        chamber::ChamberPlugin,
        staff_puzzles::StaffPuzzlePlugin,
        staff_upgrades::StaffUpgradePlugin,
        tablets::TabletPlugin,
        main_quest::MainQuestPlugin,
        quest_recovery::RecoveryPlugin,
        finale::FinalePlugin,
        cinematics::CinematicPlugin,
    ));
    if cfg!(debug_assertions)
        && !release_benchmark
        && !flythrough
        && (benchmark || cfg!(feature = "profile-tracy") || cfg!(feature = "profile-chrome"))
    {
        if !app.is_plugin_added::<bevy::render::diagnostic::RenderDiagnosticsPlugin>() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
    }
    app.add_plugins(run_environment::UnattendedRunPlugin);
    app
}
