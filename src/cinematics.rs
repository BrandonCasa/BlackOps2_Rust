use crate::{
    GameState,
    audio::{PlaybackEvent, PlaybackStatus},
};
use bevy::prelude::*;
use std::collections::BTreeMap;
const BUFFER: usize = 8;
#[derive(Resource, Default)]
struct Playback(SequencePlayback);
#[derive(Default)]
struct SequencePlayback {
    elapsed: f32,
    frames: Vec<String>,
    ready: BTreeMap<usize, Handle<Image>>,
    rate: f32,
    duration: f32,
    audio: Option<String>,
    sound: Option<Entity>,
    started: bool,
    audio_ready: bool,
    failed: bool,
}
#[derive(Component)]
struct CinematicRoot;
#[derive(Component)]
struct CinematicFrame;
#[derive(Component)]
pub(crate) struct CinematicAudioPaused(pub bool);
#[derive(Resource, Default)]
pub(crate) struct LoadingPlayback {
    pub done: bool,
    sequence: SequencePlayback,
}
pub struct CinematicPlugin;
impl Plugin for CinematicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playback>()
            .init_resource::<LoadingPlayback>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                loading_start.after(crate::session::SessionSetup::Player),
            )
            .add_systems(
                Update,
                loading_advance
                    .before(crate::audio::AudioPlayback)
                    .run_if(in_state(GameState::PreparingShaders)),
            )
            .add_systems(OnExit(GameState::PreparingShaders), close)
            .add_systems(OnEnter(GameState::Ending), start.after(crate::audio::stop))
            .add_systems(
                Update,
                advance
                    .before(crate::audio::AudioPlayback)
                    .run_if(in_state(GameState::Ending)),
            )
            .add_systems(OnExit(GameState::Ending), close);
    }
}
fn prepare(
    sequence: &crate::origins::content::CinematicSequence,
    game: &crate::assets::GameDefinition,
    server: &AssetServer,
) -> SequencePlayback {
    let frames = sequence.frames.clone();
    let rate = sequence.frames_per_second;
    let mut playback = SequencePlayback {
        duration: frames.len() as f32 / rate,
        frames,
        rate,
        audio: sequence
            .audio
            .as_ref()
            .and_then(|id| crate::audio::resolve_alias(game, &id.0))
            .map(str::to_owned),
        ..default()
    };
    playback.failed = sequence.audio.is_some() && playback.audio.is_none();
    preload(&mut playback, server, 0);
    playback
}
fn preload(playback: &mut SequencePlayback, server: &AssetServer, index: usize) {
    playback.ready.retain(|i, _| *i + 1 >= index);
    for i in index..(index + BUFFER).min(playback.frames.len()) {
        playback
            .ready
            .entry(i)
            .or_insert_with(|| server.load(playback.frames[i].clone()));
    }
}
fn root(commands: &mut Commands, camera: Entity, playback: &SequencePlayback) {
    commands
        .spawn((
            CinematicRoot,
            UiTargetCamera(camera),
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            if let Some(frame) = playback.ready.get(&0) {
                parent.spawn((
                    CinematicFrame,
                    ImageNode::new(frame.clone()),
                    Node {
                        width: percent(100.),
                        height: percent(100.),
                        ..default()
                    },
                ));
            } else {
                parent.spawn((
                    crate::hud::native::NativeText::new("LITTLE GIRL LOST", 48.),
                    Node {
                        width: px(700.),
                        height: px(70.),
                        ..default()
                    },
                ));
            }
            parent.spawn((
                crate::hud::native::NativeText::new("ESC / SPACE   Skip", 18.),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(20.),
                    left: px(20.),
                    width: px(400.),
                    height: px(26.),
                    ..default()
                },
            ));
        });
}
fn start(
    mut commands: Commands,
    mut playback: ResMut<Playback>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
) {
    playback.0 = game
        .content
        .cinematics
        .iter()
        .find(|c| c.id.0.contains("ending") || c.id.0.contains("outro"))
        .map(|sequence| prepare(sequence, &game, &server))
        .unwrap_or(SequencePlayback {
            duration: 12.,
            started: true,
            audio_ready: true,
            ..default()
        });
    if let Ok(camera) = cameras.single() {
        root(&mut commands, camera, &playback.0);
    }
}
fn step(
    commands: &mut Commands,
    dt: f32,
    playback: &mut SequencePlayback,
    server: &AssetServer,
    images: &Assets<Image>,
    frames: &mut Query<&mut ImageNode, With<CinematicFrame>>,
    events: &mut MessageReader<PlaybackEvent>,
) -> bool {
    for event in events.read() {
        if playback.sound == Some(event.entity) {
            match event.status {
                PlaybackStatus::Accepted => playback.audio_ready = true,
                PlaybackStatus::Rejected | PlaybackStatus::Cancelled => {
                    playback.failed = true;
                }
                _ => {}
            }
        }
    }
    if playback.failed {
        return true;
    }
    if playback.frames.is_empty() {
        playback.elapsed += dt;
        return playback.elapsed >= playback.duration;
    }
    let index = ((playback.elapsed * playback.rate) as usize).min(playback.frames.len() - 1);
    preload(playback, server, index);
    if playback.ready.values().any(|handle| {
        matches!(
            server.get_load_state(handle.id()),
            Some(bevy::asset::LoadState::Failed(_))
        )
    }) {
        playback.failed = true;
        return true;
    }
    let end = (index + 2).min(playback.frames.len());
    let ready = (index..end).all(|i| {
        playback
            .ready
            .get(&i)
            .is_some_and(|handle| images.contains(handle.id()))
    });
    if !ready {
        if let Some(sound) = playback.sound {
            commands
                .entity(sound)
                .try_insert(CinematicAudioPaused(true));
        }
        return false;
    }
    if !playback.started {
        playback.started = true;
        if let Some(alias) = &playback.audio {
            let sound = crate::audio::emit(
                commands,
                alias,
                crate::audio::SoundSource::World(Vec3::ZERO),
            );
            commands.entity(sound).insert(CinematicAudioPaused(false));
            playback.sound = Some(sound);
        } else {
            playback.audio_ready = true;
        }
    }
    for mut frame in frames.iter_mut() {
        frame.image = playback.ready[&index].clone();
    }
    if let Some(sound) = playback.sound {
        commands
            .entity(sound)
            .try_insert(CinematicAudioPaused(false));
    }
    if playback.audio_ready {
        let next = playback.elapsed + dt;
        let next_index = ((next * playback.rate) as usize).min(playback.frames.len() - 1);
        preload(playback, server, next_index);
        if playback
            .ready
            .get(&next_index)
            .is_some_and(|handle| images.contains(handle.id()))
        {
            playback.elapsed = next;
        } else if let Some(sound) = playback.sound {
            commands
                .entity(sound)
                .try_insert(CinematicAudioPaused(true));
        }
    }
    playback.elapsed >= playback.duration
}
fn advance(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut playback: ResMut<Playback>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut frames: Query<&mut ImageNode, With<CinematicFrame>>,
    mut events: MessageReader<PlaybackEvent>,
    mut next: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Space)
        || step(
            &mut commands,
            time.delta_secs(),
            &mut playback.0,
            &server,
            &images,
            &mut frames,
            &mut events,
        )
    {
        if let Some(sound) = playback.0.sound.take() {
            commands.entity(sound).try_despawn();
        }
        next.set(GameState::GameOver);
    }
}
fn close(
    mut commands: Commands,
    mut loading: ResMut<LoadingPlayback>,
    mut ending: ResMut<Playback>,
    roots: Query<Entity, With<CinematicRoot>>,
) {
    for entity in &roots {
        commands.entity(entity).try_despawn();
    }
    for sound in [loading.sequence.sound.take(), ending.0.sound.take()]
        .into_iter()
        .flatten()
    {
        commands.entity(sound).try_despawn();
    }
}
fn loading_start(
    mut commands: Commands,
    mut playback: ResMut<LoadingPlayback>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
) {
    *playback = LoadingPlayback {
        done: true,
        ..default()
    };
    let Some(sequence) = game
        .content
        .cinematics
        .iter()
        .find(|c| c.id.0.contains("loading") || c.id.0.contains("intro"))
    else {
        return;
    };
    if sequence.frames.is_empty() {
        return;
    }
    playback.done = false;
    playback.sequence = prepare(sequence, &game, &server);
    if let Ok(camera) = cameras.single() {
        root(&mut commands, camera, &playback.sequence);
    }
}
fn loading_advance(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut playback: ResMut<LoadingPlayback>,
    mut frames: Query<&mut ImageNode, With<CinematicFrame>>,
    roots: Query<Entity, With<CinematicRoot>>,
    mut events: MessageReader<PlaybackEvent>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    if playback.done {
        return;
    }
    if keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Space)
        || step(
            &mut commands,
            time.delta_secs(),
            &mut playback.sequence,
            &server,
            &images,
            &mut frames,
            &mut events,
        )
    {
        playback.done = true;
        for root in &roots {
            commands.entity(root).try_despawn();
        }
        if let Some(sound) = playback.sequence.sound.take() {
            commands.entity(sound).try_despawn();
        }
        if playback.sequence.failed {
            failure.0 = "Loading cinematic media could not be read".into();
            next.set(GameState::LoadError);
        }
    }
}
