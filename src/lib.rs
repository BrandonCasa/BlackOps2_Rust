#![cfg(feature = "game")]
#![forbid(unsafe_code)]

#[cfg(all(not(debug_assertions), feature = "dev-tools"))]
compile_error!("Development tools are development-only; omit dev-tools for release builds.");

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
pub mod rendering;
pub mod rituals;
pub mod robots;
pub mod rounds;
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
pub mod weapons;

use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    StartingSession,
    Playing,
    Paused,
    GameOver,
    Ending,
}

pub fn app(arguments: impl Iterator<Item = std::ffi::OsString>) -> App {
    let options = assets::LaunchOptions::parse(arguments);
    configured_app(options)
}

pub(crate) fn configured_app(options: assets::LaunchOptions) -> App {
    let (cinematics, playback) = cinematics::Cinematics::begin(&options.asset_root);
    // Reserve stack space for the debug glTF asset-loading task frames.
    bevy::tasks::IoTaskPool::get_or_init(|| {
        bevy::tasks::TaskPoolBuilder::new()
            .num_threads((bevy::tasks::available_parallelism() / 4).clamp(1, 4))
            .stack_size(64 * 1024 * 1024)
            .thread_name("IO Task Pool".into())
            .build()
    });
    let mut app = App::new();
    app.insert_resource(cinematics)
        .insert_resource(playback)
        .insert_resource(bevy::winit::WinitSettings::continuous());
    let asset_root = options.asset_root.to_string_lossy().into_owned();
    app.insert_resource(options);
    let default_plugins = DefaultPlugins.build();
    #[cfg(all(not(debug_assertions), not(feature = "dev-tools")))]
    let default_plugins = default_plugins.disable::<bevy::log::LogPlugin>();
    app.add_plugins(
        default_plugins
            .set(AssetPlugin {
                file_path: asset_root,
                ..default()
            })
            .set(
                bevy::gltf::GltfPlugin::default()
                    .add_custom_vertex_attribute("SURFACE", assets::ATTRIBUTE_SURFACE),
            )
            .set(bevy::render::RenderPlugin {
                render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
                    bevy::render::settings::WgpuSettings {
                        backends: Some(bevy::render::settings::Backends::VULKAN),
                        ..default()
                    },
                )),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Origins".into(),
                    mode: bevy::window::WindowMode::Windowed,
                    decorations: false,
                    resolution: bevy::window::WindowResolution::new(1920, 1080),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(
        avian3d::prelude::PhysicsPlugins::new(PostUpdate)
            .with_collision_hooks::<crate::enemies::RagdollCollisionHooks>(),
    )
    .init_state::<GameState>()
    .insert_resource(Time::<Fixed>::from_hz(60.0))
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
                combat_effects::CombatEffectsPlugin,
            ),
            (
                solo_revive::SoloRevivePlugin,
                perks::PerkPlugin,
                perk_effects::PerkEffectsPlugin,
                environment::EnvironmentPlugin,
                access::AccessPlugin,
                navigation::NavigationPlugin,
                barriers::BarrierPlugin,
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
    ));
    app.add_plugins((
        native_fx::NativeFxPlugin,
        actions::ActionPlugin,
        generators::GeneratorPlugin,
        powerups::PowerupPlugin,
    ));
    app.add_plugins((
        quests::QuestPlugin,
        stations::StationPlugin,
        crafting::CraftingPlugin,
        buildables::BuildablePlugin,
        tank::TankPlugin,
        panzer::PanzerPlugin,
    ));
    app.add_plugins((
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

    app
}
