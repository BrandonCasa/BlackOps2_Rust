use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Sample {
    pub interval_seconds: f64,
    pub cpu_percent: f64,
    pub working_set_mib: f64,
    pub private_mib: f64,
    pub gpu_percent: Option<f64>,
    pub dedicated_gpu_mib: Option<f64>,
    pub system_cpu_percent: Option<f64>,
}

#[derive(Resource)]
pub(super) struct Telemetry {
    receiver: Mutex<mpsc::Receiver<Sample>>,
    stop: Arc<AtomicBool>,
}

impl Default for Telemetry {
    fn default() -> Self {
        let (sender, receiver) = mpsc::sync_channel(4);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        std::thread::spawn(move || collect(sender, stopping));
        Self {
            receiver: Mutex::new(receiver),
            stop,
        }
    }
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Telemetry {
    pub fn drain(&self) -> Vec<Sample> {
        self.receiver
            .lock()
            .map(|receiver| receiver.try_iter().collect())
            .unwrap_or_default()
    }
}

#[cfg(windows)]
fn collect(sender: mpsc::SyncSender<Sample>, stop: Arc<AtomicBool>) {
    use std::{
        io::{BufRead, BufReader},
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    let script = include_str!("telemetry.ps1");
    let command = format!(
        "& {{ {script} }} -TargetProcess {} -LogicalCpus {cores}",
        std::process::id()
    );
    let mut process = match Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .creation_flags(0x08000000)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(process) => process,
        Err(_) => return,
    };
    if let Some(stdout) = process.stdout.take() {
        for line in BufReader::new(stdout).lines() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let Ok(line) = line else {
                break;
            };
            if let Ok(sample) = serde_json::from_str::<Sample>(&line) {
                match sender.try_send(sample) {
                    Err(mpsc::TrySendError::Disconnected(_)) => break,
                    _ => {}
                }
            }
        }
    }
    let _ = process.kill();
    let _ = process.wait();
}

#[cfg(not(windows))]
fn collect(_: mpsc::SyncSender<Sample>, _: Arc<AtomicBool>) {}

pub(super) fn average(samples: &[Sample], field: impl Fn(&Sample) -> Option<f64>) -> Option<f64> {
    let (total, seconds) = samples
        .iter()
        .filter_map(|s| field(s).map(|value| (value, s.interval_seconds)))
        .fold((0., 0.), |(total, seconds), (value, weight)| {
            (total + value * weight, seconds + weight)
        });
    (seconds > 0.).then_some(total / seconds)
}
