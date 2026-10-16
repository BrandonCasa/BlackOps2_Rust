use crate::{
    GameState,
    assets::LaunchOptions,
    audio::{AudioSettings, Channel},
};
use bevy::prelude::*;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    process::{Child, Command, Stdio},
};

#[derive(Resource, Default)]
pub(crate) struct LoadingPlayback {
    pub done: bool,
    pub skippable: bool,
    video: Option<Video>,
}
#[derive(Resource, serde::Deserialize)]
pub(crate) struct Cinematics {
    intro: Sequence,
    ending: Sequence,
}
#[derive(serde::Deserialize)]
struct Sequence {
    video: String,
    /// Original stereo bed followed by the center narration stem.
    stems: [Stem; 2],
}
#[derive(serde::Deserialize)]
struct Stem {
    path: String,
    volume: f32,
    channel: Channel,
}
impl Cinematics {
    pub(crate) fn load(root: &std::path::Path) -> Self {
        let path = root.join("cinematics.json");
        serde_json::from_slice(
            &std::fs::read(&path)
                .unwrap_or_else(|error| panic!("Read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("Decode {}: {error}", path.display()))
    }
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
            .add_systems(
                Update,
                (start, advance)
                    .chain()
                    .run_if(|state: Res<State<GameState>>| {
                        matches!(
                            **state,
                            GameState::Loading | GameState::StartingSession | GameState::Ending
                        )
                    }),
            )
            .add_systems(OnEnter(GameState::Playing), close)
            .add_systems(OnEnter(GameState::MainMenu), close)
            .add_systems(OnExit(GameState::Ending), close);
    }
}
fn start(
    mut commands: Commands,
    state: Res<State<GameState>>,
    options: Res<LaunchOptions>,
    sequences: Res<Cinematics>,
    audio: Res<AudioSettings>,
    mut playback: ResMut<LoadingPlayback>,
    mut cameras: Query<&mut Camera, With<crate::rendering::GameCamera>>,
    windows: Query<&bevy::window::RawHandleWrapper, With<bevy::window::PrimaryWindow>>,
) {
    if playback.video.is_some() || playback.done {
        return;
    }
    // Winit owns the parent window. Do not start a separate player before it exists.
    let Ok(window) = windows.single() else {
        return;
    };
    let parent = match window.get_window_handle() {
        raw_window_handle::RawWindowHandle::Win32(handle) => handle.hwnd.get() as u32 as u64,
        raw_window_handle::RawWindowHandle::Xlib(handle) => handle.window as u64,
        raw_window_handle::RawWindowHandle::Xcb(handle) => handle.window.get() as u64,
        _ => panic!("cinematic embedding requires a Win32 or X11 game window"),
    };
    let ending = **state == GameState::Ending;
    let sequence = if ending {
        &sequences.ending
    } else {
        &sequences.intro
    };
    playback.skippable = ending;
    playback.video = Some(sequence.play(&options.asset_root, &audio, parent));
    for mut camera in &mut cameras {
        camera.is_active = false;
    }
    commands.insert_resource(bevy::winit::WinitSettings::continuous());
}
impl Sequence {
    fn play(&self, root: &std::path::Path, audio: &AudioSettings, parent: u64) -> Video {
        let tracks = self.stems.each_ref().map(|stem| {
            (
                root.join(&stem.path),
                stem.volume * audio.gain(stem.channel),
            )
        });
        let filter = format!(
            "[aid1]volume={}[stereo];[aid2]volume={}[center];[stereo][center]join=inputs=2:channel_layout=3.0:map=0.0-FL|0.1-FR|1.0-FC[ao]",
            tracks[0].1, tracks[1].1
        );
        let pipe = format!(r"\\.\pipe\origins-cinematic-{}", std::process::id());
        let mut command = Command::new(root.parent().unwrap().join("tools/mpv/mpv.exe"));
        command
            .args([
                "--no-config",
                "--no-terminal",
                "--osc=no",
                "--input-default-bindings=no",
                "--input-terminal=no",
                "--input-vo-keyboard=yes",
                "--audio-channels=stereo",
                "--cache=yes",
                "--cache-pause=yes",
                "--cache-pause-initial=yes",
                "--cache-pause-wait=1",
                "--video-sync=audio",
                "--hwdec=auto-safe",
                "--title=Origins",
            ])
            .arg(format!("--wid={parent}"))
            .arg(format!("--input-ipc-server={pipe}"))
            .arg(format!("--audio-file={}", tracks[0].0.display()))
            .arg(format!("--audio-file={}", tracks[1].0.display()))
            .arg(format!("--lavfi-complex={filter}"))
            .arg(root.join(&self.video))
            .stdin(Stdio::null());
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let player = command.spawn().expect("start mpv cinematic player");
        Video {
            player,
            pipe,
            connection: None,
            skip_enabled: false,
        }
    }
}

fn advance(
    state: Res<State<GameState>>,
    mut playback: ResMut<LoadingPlayback>,
    mut next: ResMut<NextState<GameState>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let skippable = playback.skippable;
    let Some(video) = playback.video.as_mut() else {
        return;
    };
    if let Some(status) = video.player.try_wait().expect("cinematic player status") {
        assert!(status.success(), "cinematic player failed: {status}");
        playback.video = None;
        playback.done = true;
        if **state == GameState::Ending {
            next.set(GameState::GameOver);
        }
        return;
    }
    if video.connection.is_none() {
        if let Ok(mut pipe) = OpenOptions::new().read(true).write(true).open(&video.pipe) {
            writeln!(
                pipe,
                "{}",
                serde_json::json!({"command":["show-text", "Loading world...", 600000]})
            )
            .expect("cinematic loading prompt");
            video.connection = Some(pipe);
        }
    }
    if skippable != video.skip_enabled {
        if let Some(pipe) = video.connection.as_mut() {
            let binding = if skippable { "quit" } else { "ignore" };
            for key in ["ESC", "SPACE"] {
                writeln!(
                    pipe,
                    "{}",
                    serde_json::json!({"command":["keybind", key, binding]})
                )
                .expect("cinematic skip binding");
            }
            let prompt = if skippable {
                "ESC / SPACE   Skip"
            } else {
                "Loading world..."
            };
            writeln!(
                pipe,
                "{}",
                serde_json::json!({"command":["show-text", prompt, 600000]})
            )
            .expect("cinematic skip prompt");
            video.skip_enabled = skippable;
        }
    }
    if skippable
        && windows.single().is_ok_and(|window| window.focused)
        && (keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Space))
        && let Some(pipe) = video.connection.as_mut()
    {
        writeln!(pipe, "{}", serde_json::json!({"command":["quit"]}))
            .expect("skip cinematic from game window");
    }
}
fn close(
    mut commands: Commands,
    mut playback: ResMut<LoadingPlayback>,
    mut cameras: Query<
        (&mut Camera, Option<&mut bevy::anti_alias::dlss::Dlss>),
        With<crate::rendering::GameCamera>,
    >,
    mut windows: Query<&mut Window>,
) {
    *playback = LoadingPlayback::default();
    for (mut camera, dlss) in &mut cameras {
        camera.is_active = true;
        if let Some(mut dlss) = dlss {
            dlss.reset = true;
        }
    }
    for mut window in &mut windows {
        window.focused = true;
    }
    commands.insert_resource(bevy::winit::WinitSettings::game());
}
