#![forbid(unsafe_code)]

pub mod assets;
pub mod gameplay;
pub mod hud;
pub mod input;
pub mod life;
pub mod origins;
pub mod player;
pub mod rendering;
pub mod verification;

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
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Origins".into(),
                    resolution: (1280, 720).into(),
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
        hud::HudPlugin,
        verification::VerificationPlugin,
    ));
    app
}
