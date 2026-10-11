use super::ControlSettings;
use bevy::prelude::*;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Resource)]
pub struct SettingsStore {
    pub path: Option<PathBuf>,
    pub error: Option<String>,
    pub loaded: bool,
    pub(crate) force_save: bool,
    saved: ControlSettings,
}
impl SettingsStore {
    pub fn load(path: Option<PathBuf>) -> (ControlSettings, Self) {
        let existed = path.as_deref().is_some_and(Path::is_file);
        let result = path.as_deref().map(read).transpose();
        let (settings, loaded, error) = match result {
            Ok(Some(settings)) => (settings, existed, None),
            Ok(None) => (
                ControlSettings::default(),
                false,
                Some("Local settings directory is unavailable.".into()),
            ),
            Err(error) => (ControlSettings::default(), false, Some(error)),
        };
        let store = Self {
            path,
            error,
            loaded,
            force_save: false,
            saved: settings.clone(),
        };
        (settings, store)
    }
}
pub fn local_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Origins/settings.json"))
}
pub fn read(path: &Path) -> Result<ControlSettings, String> {
    match std::fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ControlSettings::default()),
        Err(e) => return Err(format!("Cannot read settings: {e}")),
        Ok(meta) if meta.len() > 65536 => return Err("Settings exceed 64 KiB.".into()),
        _ => {}
    }
    let mut settings: ControlSettings = serde_json::from_slice(
        &std::fs::read(path).map_err(|e| format!("Cannot read settings: {e}"))?,
    )
    .map_err(|e| format!("Invalid settings: {e}"))?;
    if !settings.bindings.contains_key(&super::Action::Scoreboard) {
        let binding = super::Binding::Key(KeyCode::Tab);
        let available = !settings
            .bindings
            .values()
            .any(|bindings| bindings.contains(&binding));
        settings.bindings.insert(
            super::Action::Scoreboard,
            available.then_some(binding).into_iter().collect(),
        );
    }
    let defaults = ControlSettings::default();
    for action in [super::Action::Buildable, super::Action::StaffSecondary] {
        if settings.bindings.contains_key(&action) { continue; }
        let bindings = defaults.bindings[&action].iter().copied()
            .filter(|binding| !settings.bindings.values().any(|owned| owned.contains(binding)))
            .collect();
        settings.bindings.insert(action, bindings);
    }
    if !settings.valid() {
        return Err("Settings contain unsupported values or conflicting bindings.".into());
    }
    Ok(settings)
}
pub fn write(path: &Path, settings: &ControlSettings) -> Result<(), String> {
    if !settings.valid() {
        return Err("Cannot save invalid controls.".into());
    }
    let parent = path.parent().ok_or("Settings path has no parent.")?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Cannot create settings directory: {e}"))?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("Cannot save settings: {e}"))
}
pub(super) fn save(settings: Res<ControlSettings>, mut store: ResMut<SettingsStore>) {
    if !store.force_save && (!settings.is_changed() || *settings == store.saved) {
        return;
    }
    store.force_save = false;
    let Some(path) = &store.path else {
        return;
    };
    match write(path, &settings) {
        Ok(()) => {
            store.saved = settings.clone();
            store.error = None;
        }
        Err(error) => {
            warn!("{error}");
            store.error = Some(error);
        }
    }
}

pub(super) fn apply_audio(
    settings: Res<ControlSettings>,
    mut audio: ResMut<crate::audio::AudioSettings>,
) {
    *audio = settings.audio.clone();
}
pub(super) fn capture_audio(
    audio: Res<crate::audio::AudioSettings>,
    mut settings: ResMut<ControlSettings>,
) {
    if audio.is_changed() && *audio != settings.audio {
        settings.audio = audio.clone();
    }
}
