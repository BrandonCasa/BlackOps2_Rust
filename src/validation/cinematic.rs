//! Observe the existing movie player. A clean process exit alone is not an ending.
use super::Controller;
use bevy::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Resource, Serialize)]
pub(crate) struct Playback {
    pub file: PathBuf,
    pub position: f64,
    pub duration: Option<f64>,
    pub speed: Option<f64>,
    pub paused: Option<bool>,
    pub captures: [Option<PathBuf>; 3],
    pub terminal: Option<Value>,
    pub failure: Option<String>,
    pub complete: bool,
    #[serde(skip)]
    report: PathBuf,
    #[serde(skip)]
    buffer: Vec<u8>,
    #[cfg(windows)]
    #[serde(skip)]
    pipe: Option<
        interprocess::os::windows::named_pipe::DuplexPipeStream<
            interprocess::os::windows::named_pipe::pipe_mode::Bytes,
        >,
    >,
    #[serde(skip)]
    attached: bool,
    #[serde(skip)]
    released: bool,
    #[serde(skip)]
    observed_file: bool,
    #[serde(skip)]
    desired_pause: bool,
    #[serde(skip)]
    pending_captures: [bool; 3],
    #[serde(skip)]
    last_message: Instant,
    #[serde(skip)]
    ping: Instant,
    #[serde(skip)]
    progressed: bool,
    #[serde(skip)]
    started: Instant,
    #[serde(skip)]
    failure_saved: bool,
}
impl Playback {
    pub(crate) fn new(file: PathBuf, controller: &Controller) -> Self {
        Self {
            file,
            position: 0.,
            duration: None,
            speed: None,
            paused: None,
            captures: default(),
            terminal: None,
            failure: None,
            complete: false,
            report: controller.report.clone(),
            buffer: Vec::with_capacity(8192),
            #[cfg(windows)]
            pipe: None,
            attached: false,
            released: false,
            observed_file: false,
            desired_pause: false,
            pending_captures: [false; 3],
            last_message: Instant::now(),
            ping: Instant::now(),
            progressed: false,
            started: Instant::now(),
            failure_saved: false,
        }
    }
    pub(crate) fn pause(&mut self, paused: bool) {
        self.desired_pause = paused;
    }
    #[cfg(windows)]
    fn send(&mut self, command: Value, id: u32) -> Result<(), String> {
        writeln!(
            self.pipe.as_mut().unwrap(),
            "{}",
            json!({"command":command,"request_id":id})
        )
        .map_err(|e| format!("Cinematic IPC write: {e}"))
    }
    #[cfg(windows)]
    pub(crate) fn poll(&mut self, pipe_name: &str) -> Result<Vec<String>, String> {
        if !self.attached {
            use interprocess::os::windows::named_pipe::{DuplexPipeStream, pipe_mode::Bytes};
            let stream = match DuplexPipeStream::<Bytes>::connect_by_path_with_wait_mode(
                pipe_name,
                interprocess::ConnectWaitMode::Timeout(Duration::ZERO),
            ) {
                Ok(stream) => stream,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::WouldBlock
                    ) && self.started.elapsed() < Duration::from_secs(5) =>
                {
                    return Ok(Vec::new());
                }
                Err(error) => return Err(format!("Attach ending observation: {error}")),
            };
            stream
                .set_nonblocking(true)
                .map_err(|e| format!("Cinematic nonblocking IPC: {e}"))?;
            self.pipe = Some(stream);
            for (id, name) in ["path", "time-pos", "duration", "speed", "pause"]
                .into_iter()
                .enumerate()
            {
                self.send(json!(["observe_property", id, name]), id as u32)?;
            }
            for (key, message) in [
                ("ESC", "validator-toggle-pause"),
                ("F12", "validator-abort"),
                ("SPACE", "validator-skip"),
            ] {
                self.send(
                    json!(["keybind", key, format!("script-message {message}")]),
                    20,
                )?;
            }
            self.send(
                json!([
                    "show-text",
                    "Origins Easter egg ending | ESC pause | F12 abort",
                    600000
                ]),
                21,
            )?;
            self.attached = true;
        }
        let mut controls = Vec::new();
        let mut disconnected = false;
        let mut bytes = [0u8; 8192];
        loop {
            match self.pipe.as_mut().unwrap().read(&mut bytes) {
                Ok(0) => {
                    // An empty nonblocking Windows pipe also reports zero.
                    // Process status and telemetry expiry establish closure.
                    break;
                }
                Ok(count) => self.buffer.extend_from_slice(&bytes[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                    disconnected = true;
                    break;
                }
                Err(error) => return Err(format!("Cinematic IPC read: {error}")),
            }
        }
        while let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let message: Value = serde_json::from_slice(&self.buffer[..end])
                .map_err(|e| format!("Cinematic IPC message: {e}"))?;
            self.buffer.drain(..=end);
            self.last_message = Instant::now();
            if message["event"] == "client-message" {
                if let Some(control) = message["args"][0].as_str() {
                    if control == "validator-skip" {
                        self.failure = Some("Ending playback was skipped".into());
                    } else {
                        controls.push(control.to_owned());
                    }
                }
            }
            self.observe(message);
        }
        if disconnected && self.terminal.is_none() {
            return Err("Cinematic IPC closed before terminal evidence".into());
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.terminal.is_some() {
            return Ok(controls);
        }
        if self.last_message.elapsed() > Duration::from_secs(5) {
            return Err("Cinematic telemetry lost".into());
        }
        if !self.released && self.started.elapsed() > Duration::from_secs(5) {
            return Err("Ending properties were unavailable before playback".into());
        }
        if self.ping.elapsed() > Duration::from_millis(500) {
            self.send(json!(["get_property", "pause"]), 30)?;
            self.ping = Instant::now();
        }
        if !self.released
            && self.observed_file
            && self.duration.is_some()
            && self.speed == Some(1.)
            && self.paused == Some(true)
        {
            self.send(json!(["set_property", "pause", self.desired_pause]), 31)?;
            self.released = true;
        } else if self.released && self.paused.is_some_and(|p| p != self.desired_pause) {
            self.send(json!(["set_property", "pause", self.desired_pause]), 31)?;
        }
        if let Some(duration) = self.duration {
            for (index, at) in [1., duration * 0.5, (duration - 2.).max(1.)]
                .into_iter()
                .enumerate()
            {
                if self.position >= at && !self.pending_captures[index] {
                    let path = self
                        .report
                        .join(format!("ending-{}.png", ["early", "middle", "late"][index]));
                    self.send(
                        json!(["screenshot-to-file", path, "window"]),
                        100 + index as u32,
                    )?;
                    self.pending_captures[index] = true;
                }
            }
        }
        Ok(controls)
    }
    #[cfg(not(windows))]
    pub(crate) fn poll(&mut self, _: &str) -> Result<Vec<String>, String> {
        Err("Validator movie observation requires Windows named pipes".into())
    }
    fn observe(&mut self, message: Value) {
        match message["event"].as_str() {
            Some("property-change") => match message["name"].as_str() {
                Some("path") => {
                    if let Some(path) = message["data"].as_str() {
                        self.observed_file = path
                            .replace('\\', "/")
                            .eq_ignore_ascii_case(&self.file.to_string_lossy().replace('\\', "/"));
                        if !self.observed_file {
                            self.failure = Some(format!("Unexpected ending movie: {path}"));
                        }
                    }
                }
                Some("time-pos") => {
                    if let Some(position) = message["data"].as_f64() {
                        if position + 0.05 < self.position {
                            self.failure = Some("Ending playback moved backwards".into());
                        }
                        self.progressed |= position > self.position + 0.01;
                        self.position = self.position.max(position);
                    }
                }
                Some("duration") => self.duration = message["data"].as_f64().filter(|d| *d > 0.),
                Some("speed") => {
                    self.speed = message["data"].as_f64();
                    if self.speed.is_some_and(|speed| (speed - 1.).abs() > 1e-6) {
                        self.failure = Some("Ending playback speed changed".into());
                    }
                }
                Some("pause") => self.paused = message["data"].as_bool(),
                _ => {}
            },
            Some("seek") => self.failure = Some("Ending playback was sought or skipped".into()),
            Some("end-file") => {
                self.complete = self.failure.is_none()
                    && self.released
                    && self.observed_file
                    && self.progressed
                    && self.speed == Some(1.)
                    && message["reason"] == "eof"
                    && message.get("file_error").is_none_or(Value::is_null)
                    && self
                        .duration
                        .is_some_and(|d| (d - self.position).abs() <= 1.)
                    && self.captures.iter().all(Option::is_some);
                if !self.complete {
                    self.failure = Some("Ending stopped without complete natural playback".into());
                }
                self.terminal = Some(message);
                let path = self.report.join("ending.json");
                if let Err(e) = std::fs::write(path, serde_json::to_vec_pretty(self).unwrap()) {
                    self.failure = Some(format!("Save ending result: {e}"));
                    self.complete = false;
                }
                return;
            }
            _ => {}
        }
        if let Some(id) = message["request_id"]
            .as_u64()
            .filter(|id| (100..103).contains(id))
        {
            let index = id as usize - 100;
            let path = self
                .report
                .join(format!("ending-{}.png", ["early", "middle", "late"][index]));
            if message["error"] == "success" && path.is_file() {
                self.captures[index] = Some(path);
            } else {
                self.failure = Some(format!("Ending screenshot {} failed", index + 1));
            }
        }
    }
    pub(crate) fn exited(&mut self) -> Result<(), String> {
        if self.complete {
            Ok(())
        } else {
            Err(self
                .failure
                .clone()
                .unwrap_or_else(|| "Movie exited before natural EOF was observed".into()))
        }
    }
    pub(crate) fn failed(&mut self, reason: String) -> bool {
        if self.failure_saved {
            return false;
        }
        self.complete = false;
        self.failure = Some(reason);
        self.desired_pause = true;
        #[cfg(windows)]
        if self.pipe.is_some() {
            let _ = self.send(json!(["set_property", "pause", true]), 32);
        }
        if let Err(error) = std::fs::write(
            self.report.join("ending.json"),
            serde_json::to_vec_pretty(self).unwrap(),
        ) {
            error!("Save cinematic failure: {error}");
        }
        self.failure_saved = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed_movie() -> Playback {
        let report = std::env::temp_dir().join(format!(
            "origins-ending-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&report).unwrap();
        let mut movie = Playback::new(report.join("ending.webm"), &Controller::new(2818, report));
        movie.observe(json!({"event":"property-change","name":"path","data":movie.file}));
        movie.observe(json!({"event":"property-change","name":"duration","data":120.}));
        movie.observe(json!({"event":"property-change","name":"speed","data":1.}));
        movie.released = true;
        movie.observe(json!({"event":"property-change","name":"time-pos","data":119.5}));
        movie.captures = [
            Some("early.png".into()),
            Some("middle.png".into()),
            Some("late.png".into()),
        ];
        movie
    }

    #[test]
    fn ending_requires_natural_eof_full_position_and_all_three_captures() {
        let mut movie = observed_movie();
        assert!(
            movie.exited().is_err(),
            "Process exit alone is insufficient"
        );
        movie.observe(json!({"event":"end-file","reason":"eof"}));
        assert!(movie.complete);
        assert!(movie.exited().is_ok());
        let saved: Value =
            serde_json::from_slice(&std::fs::read(movie.report.join("ending.json")).unwrap())
                .unwrap();
        assert_eq!(saved["terminal"]["reason"], "eof");
        for case in 0..6 {
            let mut movie = observed_movie();
            match case {
                0 => movie.observe(json!({"event":"seek"})),
                1 => movie.observe(json!({"event":"property-change","name":"speed","data":2.})),
                2 => movie.position = 100.,
                3 => movie.captures[1] = None,
                4 => movie.released = false,
                _ => movie.observe(
                    json!({"event":"property-change","name":"path","data":"another.webm"}),
                ),
            }
            movie.observe(json!({"event":"end-file","reason":"eof"}));
            assert!(!movie.complete, "Invalid ending case {case} passed");
            assert!(movie.failure.is_some());
        }
        let mut movie = observed_movie();
        movie.observe(json!({"event":"end-file","reason":"quit"}));
        assert!(!movie.complete);
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "requires the authorized local mpv and Origins ending export"]
    fn mpv_observation_is_nonblocking_and_can_pause_and_resume() {
        use std::{
            os::windows::process::CommandExt,
            process::{Command, Stdio},
        };
        struct Player(std::process::Child);
        impl Drop for Player {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        let mut movie = observed_movie();
        movie = Playback::new(
            root.join("assets/cinematics/zm_tomb_ee.webm"),
            &Controller::new(2818, movie.report),
        );
        movie.pause(true);
        // This fixture tests transport and controls, with no output window.
        // The route separately requires actual early/middle/late screenshots.
        movie.pending_captures = [true; 3];
        let name = format!(r"\\.\pipe\origins-ending-test-{}", std::process::id());
        let _player = Player(
            Command::new(root.join("tools/mpv/mpv.exe"))
                .args([
                    "--no-config",
                    "--vo=null",
                    "--ao=null",
                    "--pause=yes",
                    "--no-terminal",
                ])
                .arg(format!("--input-ipc-server={name}"))
                .arg(&movie.file)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x08000000)
                .spawn()
                .unwrap(),
        );
        let started = Instant::now();
        while !movie.released {
            movie.poll(&name).unwrap();
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(movie.paused, Some(true));
        assert_eq!(movie.speed, Some(1.));
        movie.pause(false);
        while movie.position < 0.1 {
            movie.poll(&name).unwrap();
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
        movie.pause(true);
        while movie.paused != Some(true) {
            movie.poll(&name).unwrap();
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
        let position = movie.position;
        for _ in 0..20 {
            movie.poll(&name).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!((movie.position - position).abs() < 0.05);
        assert!(movie.exited().is_err());
    }
}
