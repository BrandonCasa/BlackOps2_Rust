//! One durable log per game process, including failures before Bevy starts.
use bevy::{log::tracing_subscriber, prelude::*};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Resource)]
pub(crate) struct RunLog {
    file: Arc<File>,
}

impl RunLog {
    pub(crate) fn open() -> std::io::Result<Self> {
        let directory = std::env::current_exe()?
            .parent()
            .expect("Executable directory")
            .join("logs");
        std::fs::create_dir_all(&directory)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path: PathBuf = directory.join(format!("origins-{stamp}-{}.log", std::process::id()));
        let mut file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)?;
        writeln!(
            file,
            "Origins {} | pid={} | executable={} | cwd={}",
            env!("CARGO_PKG_VERSION"),
            std::process::id(),
            std::env::current_exe()?.display(),
            std::env::current_dir()?.display()
        )?;
        let file = Arc::new(file);
        let panic_file = file.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            let mut file = &*panic_file;
            let _ = writeln!(
                file,
                "\nPANIC on thread {:?}: {panic}\n{}",
                std::thread::current().name(),
                std::backtrace::Backtrace::force_capture()
            );
            let _ = file.sync_data();
            previous(panic);
        }));
        eprintln!("Run log: {}", path.display());
        Ok(Self { file })
    }
}

pub(crate) fn layer(app: &mut App) -> Option<bevy::log::BoxedLayer> {
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(app.world().resource::<RunLog>().file.clone()),
    ))
}
