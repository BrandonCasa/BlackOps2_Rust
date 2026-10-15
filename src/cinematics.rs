use crate::{GameState, assets::{GameDefinition, LaunchOptions}, audio::AudioSettings};
use bevy::prelude::*;
use std::{fs::{File, OpenOptions}, io::Write, process::{Child, Command, Stdio}};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Resource, Default)]
pub(crate) struct LoadingPlayback {
    pub done: bool,
    pub skippable: bool,
    video: Option<Video>,
}
struct Video {
    player: Child,
    pipe: String,
    connection: Option<File>,
    skip_enabled: bool,
}
impl Drop for Video {
    fn drop(&mut self) {
        let _ = self.player.kill();
        let _ = self.player.wait();
    }
}
pub struct CinematicPlugin;
impl Plugin for CinematicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingPlayback>()
            .add_systems(Update, (start, advance).chain()
                .run_if(|state: Res<State<GameState>>| matches!(**state,
                    GameState::Loading | GameState::StartingSession | GameState::Ending)))
            .add_systems(OnEnter(GameState::Playing), close)
            .add_systems(OnEnter(GameState::MainMenu), close)
            .add_systems(OnExit(GameState::Ending), close);
    }
}
fn start(
    mut commands: Commands,
    state: Res<State<GameState>>,
    options: Res<LaunchOptions>,
    game: Option<Res<GameDefinition>>,
    audio: Res<AudioSettings>,
    mut playback: ResMut<LoadingPlayback>,
    mut cameras: Query<&mut Camera, With<crate::rendering::GameCamera>>,
) {
    if playback.video.is_some() || playback.done { return; }
    let Some(game) = game else { return; };
    let ending = **state == GameState::Ending;
    let sequence = game.content.cinematics.iter().find(|sequence| {
        if ending { sequence.id.0.contains("ending") || sequence.id.0.contains("outro") }
        else { sequence.id.0.contains("loading") || sequence.id.0.contains("intro") }
    }).expect("source cinematic");
    let root = &options.asset_root;
    let tracks = sequence.audio.each_ref().map(|id| {
        let alias = &game.audio.aliases[&id.0];
        let sound = game.content.sounds.iter().find(|sound| sound.id == alias.sounds[0])
            .expect("cinematic audio source");
        (root.join(&sound.path), alias.native[0].volume_max * audio.gain(alias.channel))
    });
    let filter = format!("[aid1]volume={}[stereo];[aid2]volume={}[center];[stereo][center]join=inputs=2:channel_layout=3.0:map=0.0-FL|0.1-FR|1.0-FC[ao]", tracks[0].1, tracks[1].1);
    let pipe = format!(r"\\.\pipe\origins-cinematic-{}", std::process::id());
    let mut command = Command::new(root.parent().unwrap().join("tools/mpv/mpv.exe"));
    command.args(["--no-config", "--no-terminal", "--osc=no", "--fullscreen", "--ontop",
        "--input-default-bindings=no", "--input-terminal=no", "--audio-channels=stereo",
        "--cache=yes", "--cache-pause=yes", "--cache-pause-initial=yes", "--cache-pause-wait=1",
        "--video-sync=audio", "--hwdec=auto-safe", "--title=Origins"])
        .arg(format!("--input-ipc-server={pipe}"))
        .arg(format!("--audio-file={}", tracks[0].0.display()))
        .arg(format!("--audio-file={}", tracks[1].0.display()))
        .arg(format!("--lavfi-complex={filter}"))
        .arg(root.join(&sequence.video)).stdin(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let player = command.spawn().expect("start mpv cinematic player");
    playback.skippable = ending;
    playback.video = Some(Video { player, pipe, connection: None, skip_enabled: false });
    for mut camera in &mut cameras { camera.is_active = false; }
    commands.insert_resource(bevy::winit::WinitSettings::continuous());
}
fn advance(
    state: Res<State<GameState>>,
    mut playback: ResMut<LoadingPlayback>,
    mut next: ResMut<NextState<GameState>>,
) {
    let skippable = playback.skippable;
    let Some(video) = playback.video.as_mut() else { return; };
    if let Some(status) = video.player.try_wait().expect("cinematic player status") {
        assert!(status.success(), "cinematic player failed: {status}");
        playback.video = None;
        playback.done = true;
        if **state == GameState::Ending { next.set(GameState::GameOver); }
        return;
    }
    if video.connection.is_none() {
        if let Ok(mut pipe) = OpenOptions::new().read(true).write(true).open(&video.pipe) {
            writeln!(pipe, "{}", serde_json::json!({"command":["show-text", "Loading world...", 600000]}))
                .expect("cinematic loading prompt");
            video.connection = Some(pipe);
        }
    }
    if skippable && !video.skip_enabled {
        if let Some(pipe) = video.connection.as_mut() {
            for command in [
                serde_json::json!({"command":["define-section", "origins-skip", "ESC quit\nSPACE quit", "force"]}),
                serde_json::json!({"command":["enable-section", "origins-skip"]}),
                serde_json::json!({"command":["show-text", "ESC / SPACE   Skip", 600000]}),
            ] {
                writeln!(pipe, "{command}").expect("cinematic skip binding");
            }
            video.skip_enabled = true;
        }
    }
}
fn close(
    mut commands: Commands,
    mut playback: ResMut<LoadingPlayback>,
    mut cameras: Query<&mut Camera, With<crate::rendering::GameCamera>>,
    mut windows: Query<&mut Window>,
) {
    *playback = LoadingPlayback::default();
    for mut camera in &mut cameras { camera.is_active = true; }
    for mut window in &mut windows { window.focused = true; }
    commands.insert_resource(bevy::winit::WinitSettings::game());
}
