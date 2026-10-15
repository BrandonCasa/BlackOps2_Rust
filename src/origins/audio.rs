//! Authored ordinary audio data; extraction stays in external tools.
use super::{GameDefinition, SoundDefinition};
use crate::audio::AudioDefinition;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceAudio {
    audio: AudioDefinition,
    sounds: Vec<SoundDefinition>,
    sources: Vec<String>,
}

pub(crate) fn bind(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    let source: SourceAudio = serde_json::from_str(include_str!("../../gameplay/origins/audio.json"))
        .map_err(|error| format!("Authored Origins audio data: {error}"))?;
    if source.sources.is_empty() { return Err("Authored audio source provenance is missing".into()); }
    for sound in &source.sounds {
        super::validate_asset_path(&sound.path).map_err(|error| error.to_string())?;
        if !root.join(&sound.path).is_file() {
            return Err(format!("Required source audio {} is missing: {}", sound.id.0, sound.path));
        }
    }
    game.audio = source.audio;
    game.content.sounds = source.sounds;
    Ok(())
}
