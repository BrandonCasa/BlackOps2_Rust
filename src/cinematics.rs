use crate::{
    GameState,
    audio::{PlaybackEvent, PlaybackStatus},
};
use bevy::prelude::*;
use std::collections::BTreeMap;
// Bounded to 48 decoded frames (about 169 MiB at source 1280x720 RGBA).
// A reserve absorbs IO/decode bursts while the full map is loading.
const BUFFER: usize = 48;
const RESUME_BUFFER: usize = 24;

/// Bind an authored ordinary PNG directory to the existing sequence player.
pub(crate) fn bind_frames(
    game: &mut crate::origins::GameDefinition,
    root: &std::path::Path,
) -> Result<(), String> {
    bind_frame_sequences(&mut game.content.cinematics, root)
}
fn bind_frame_sequences(
    sequences: &mut [crate::origins::content::CinematicSequence],
    root: &std::path::Path,
) -> Result<(), String> {
    let root = root.canonicalize().map_err(|error| format!("Cinematic asset root: {error}"))?;
    for sequence in sequences {
        let Some(directory) = &sequence.frame_directory else { continue };
        if !sequence.frames.is_empty() {
            return Err(format!("Cinematic {} defines both frames and frame_directory", sequence.id.0));
        }
        let relative = std::path::Path::new(directory);
        if relative.is_absolute() {
            return Err(format!("Cinematic {} frame_directory must be relative to the asset root", sequence.id.0));
        }
        let directory = root.join(relative).canonicalize()
            .map_err(|error| format!("Cinematic {} frame directory {directory}: {error}", sequence.id.0))?;
        if !directory.starts_with(&root) || !directory.is_dir() {
            return Err(format!("Cinematic {} frame_directory must be an existing directory inside the asset root", sequence.id.0));
        }
        let mut frames = Vec::new();
        for entry in std::fs::read_dir(&directory)
            .map_err(|error| format!("Cinematic {} frame directory: {error}", sequence.id.0))? {
            let entry = entry.map_err(|error| format!("Cinematic {} frame entry: {error}", sequence.id.0))?;
            if entry.path().extension().and_then(|extension| extension.to_str()) != Some("png") { continue; }
            let path = entry.path().canonicalize()
                .map_err(|error| format!("Cinematic {} frame {:?}: {error}", sequence.id.0, entry.path()))?;
            if !path.is_file() || !path.starts_with(&root) {
                return Err(format!("Cinematic {} frame {:?} is not an ordinary file inside the asset root", sequence.id.0, path));
            }
            let path = path.strip_prefix(&root).unwrap().to_str()
                .ok_or_else(|| format!("Cinematic {} frame path is not UTF-8", sequence.id.0))?
                .replace('\\', "/");
            frames.push(path);
        }
        frames.sort();
        if frames.is_empty() {
            return Err(format!("Cinematic {} frame directory {:?} contains no PNG frames", sequence.id.0, directory));
        }
        sequence.frames = frames;
    }
    Ok(())
}

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
    buffering: bool,
    audio_paused: bool,
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
    pub skippable: bool,
    sequence: SequencePlayback,
}
impl LoadingPlayback {
    pub(crate) fn active(&self) -> bool {
        !self.done && !self.sequence.frames.is_empty()
    }
}
#[derive(Component)]
struct SkipPrompt;
pub struct CinematicPlugin;
impl Plugin for CinematicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playback>()
            .init_resource::<LoadingPlayback>()
            .add_systems(
                Update,
                (loading_start, loading_advance)
                    .chain()
                    .before(crate::audio::AudioPlayback)
                    .run_if(|state: Res<State<GameState>>| matches!(**state, GameState::Loading | GameState::StartingSession)),
            )
            .add_systems(OnEnter(GameState::Playing), close)
            .add_systems(OnEnter(GameState::MainMenu), close)
            .add_systems(OnEnter(GameState::LoadError), close)
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
        buffering: true,
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
fn root(commands: &mut Commands, camera: Entity, playback: &SequencePlayback, skippable: bool) {
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
                SkipPrompt,
                crate::hud::native::NativeText::new(if skippable { "ESC / SPACE   Skip" } else { "Loading world..." }, 18.),
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
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
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
        root(&mut commands, camera, &playback.0, true);
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
    // After an underrun, refill a useful reserve before resuming rather than
    // alternating play/pause for each newly decoded PNG.
    let end = (index + if playback.buffering { RESUME_BUFFER } else { 1 })
        .min(playback.frames.len());
    let ready = (index..end).all(|i| {
        playback
            .ready
            .get(&i)
            .is_some_and(|handle| images.contains(handle.id()))
    });
    if !ready {
        playback.buffering = true;
        set_audio_paused(commands, playback, true);
        return false;
    }
    playback.buffering = false;
    if !playback.started {
        playback.started = true;
        if let Some(alias) = &playback.audio {
            let sound = crate::audio::emit(
                commands,
                alias,
                crate::audio::SoundSource::World(Vec3::ZERO),
            );
            commands.entity(sound).remove::<crate::gameplay::SessionEntity>().insert(CinematicAudioPaused(false));
            playback.sound = Some(sound);
            playback.audio_paused = false;
        } else {
            playback.audio_ready = true;
        }
    }
    for mut frame in frames.iter_mut() {
        let image = &playback.ready[&index];
        if frame.image.id() != image.id() {
            frame.image = image.clone();
        }
    }
    set_audio_paused(commands, playback, false);
    if playback.audio_ready {
        let next = playback.elapsed + dt;
        let next_index = ((next * playback.rate) as usize).min(playback.frames.len() - 1);
        if playback
            .ready
            .get(&next_index)
            .is_some_and(|handle| images.contains(handle.id()))
        {
            playback.elapsed = next;
        } else {
            playback.buffering = true;
            set_audio_paused(commands, playback, true);
        }
    }
    playback.elapsed >= playback.duration
}
fn set_audio_paused(commands: &mut Commands, playback: &mut SequencePlayback, paused: bool) {
    if playback.audio_paused == paused {
        return;
    }
    playback.audio_paused = paused;
    if let Some(sound) = playback.sound {
        commands.entity(sound).try_insert(CinematicAudioPaused(paused));
    }
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
        if playback.0.failed {
            panic!("Ending cinematic media could not be read");
        } else {
            next.set(GameState::GameOver);
        }
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
    loading.sequence.ready.clear();
    ending.0.ready.clear();
    for sound in [loading.sequence.sound.take(), ending.0.sound.take()]
        .into_iter()
        .flatten()
    {
        commands.entity(sound).try_despawn();
    }
    loading.sequence = SequencePlayback::default();
    loading.done = true;
    loading.skippable = false;
}
fn loading_start(
    mut commands: Commands,
    mut playback: ResMut<LoadingPlayback>,
    game: Option<Res<crate::assets::GameDefinition>>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
) {
    if playback.active() || !playback.sequence.frames.is_empty() {
        return;
    }
    let Some(game) = game else { return; };
    let Ok(camera) = cameras.single() else { return; };
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
    root(&mut commands, camera, &playback.sequence, false);
}
fn loading_advance(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut playback: ResMut<LoadingPlayback>,
    mut frames: Query<&mut ImageNode, With<CinematicFrame>>,
    mut prompts: Query<&mut crate::hud::native::NativeText, With<SkipPrompt>>,
    roots: Query<Entity, With<CinematicRoot>>,
    mut events: MessageReader<PlaybackEvent>,
) {
    for mut prompt in &mut prompts {
        let text = if playback.skippable { "ESC / SPACE   Skip" } else { "Loading world..." };
        if prompt.value != text { prompt.value = text.into(); }
    }
    if playback.done {
        return;
    }
    if (playback.skippable && (keys.pressed(KeyCode::Escape) || keys.pressed(KeyCode::Space)))
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
        playback.sequence.ready.clear();
        for root in &roots {
            commands.entity(root).try_despawn();
        }
        if let Some(sound) = playback.sequence.sound.take() {
            commands.entity(sound).try_despawn();
        }
        if playback.sequence.failed {
            panic!("Loading cinematic media could not be read");
        }
    }
}

#[cfg(test)]
mod frame_binding_tests {
    use super::*;
    use std::{path::PathBuf, sync::atomic::{AtomicU64, Ordering}};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Root(PathBuf);
    impl Root {
        fn new() -> Self {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/cinematic-binding-tests")
                .join(format!("{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir_all(root.join("frames")).unwrap();
            Self(root)
        }
    }
    impl Drop for Root {
        fn drop(&mut self) {
            for entry in std::fs::read_dir(self.0.join("frames")).unwrap() {
                std::fs::remove_file(entry.unwrap().path()).unwrap();
            }
            std::fs::remove_dir(self.0.join("frames")).unwrap();
            std::fs::remove_dir(&self.0).unwrap();
        }
    }
    fn sequence(directory: &str) -> crate::origins::content::CinematicSequence {
        crate::origins::content::CinematicSequence {
            id: crate::origins::ContentId("cinematic.loading.zm_tomb".into()),
            frame_directory: Some(directory.into()), frames: Vec::new(),
            frames_per_second: 30., audio: None,
        }
    }
    #[test]
    fn ordinary_png_frames_are_sorted_without_changing_source_rate() {
        let root = Root::new();
        for name in ["frame_000002.png", "frame_000001.png"] {
            std::fs::write(root.0.join("frames").join(name), []).unwrap();
        }
        let mut sequences = [sequence("frames")];
        bind_frame_sequences(&mut sequences, &root.0).unwrap();
        assert_eq!(sequences[0].frames, ["frames/frame_000001.png", "frames/frame_000002.png"]);
        assert_eq!(sequences[0].frames_per_second, 30.);
    }
    #[test]
    fn missing_empty_and_escaping_frame_directories_are_errors() {
        let root = Root::new();
        for directory in ["missing", "frames", ".."] {
            let error = bind_frame_sequences(&mut [sequence(directory)], &root.0).unwrap_err();
            assert!(error.contains("cinematic.loading.zm_tomb"));
        }
    }
    #[test]
    fn explicit_frames_and_directory_cannot_conflict() {
        let root = Root::new();
        let mut sequences = [sequence("frames")];
        sequences[0].frames.push("existing.png".into());
        assert!(bind_frame_sequences(&mut sequences, &root.0).unwrap_err().contains("both frames and frame_directory"));
    }
}
