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
    #[serde(rename = "sources")]
    _sources: Vec<String>,
}

pub(crate) fn bind(game: &mut GameDefinition, root: &Path) {
    let mut source: SourceAudio = serde_json::from_str(include_str!("../../gameplay/origins/audio.json")).expect("Authored audio data");
    // Recover source speaker routing discarded by the old stereo-only data.
    let pan_path = root.join("audio/globals/pan.csv");
    let pan_csv = std::fs::read_to_string(&pan_path)
        .expect("Source speaker pan table");
    let pans = pan_csv.lines().skip(1).filter(|line| !line.trim().is_empty())
        .map(|line| line.split(',').skip(1).take(6).map(str::parse::<f32>).collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>().expect("Source speaker pan values");
    let alias_root = root.join("audio/source");
    let banks = std::fs::read_dir(&alias_root)
        .expect("Source audio aliases");
    for bank in banks {
        let directory = bank.expect("Source audio bank").path().join("sound_aliases");
        if !directory.is_dir() { continue; }
        for file in std::fs::read_dir(directory).expect("Source alias directory") {
            let path = file.expect("Source alias file").path();
            let Some(name) = path.file_stem().and_then(|name| name.to_str()) else { continue; };
            let Some(alias) = source.audio.aliases.get_mut(name) else { continue; };
            if !alias.native.iter().any(|entry| !entry.spatial && !entry.panning) { continue; }
            let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).expect("Source alias data"))
                .expect("Source alias JSON");
            let entries = raw["entries"].as_array().expect("Source alias entries");
            for native in alias.native.iter_mut().filter(|entry| !entry.spatial && !entry.panning) {
                let entry = entries.iter().find(|entry| entry["id"].as_u64() == Some(native.id as u64))
                    .expect("Source alias record");
                let index = entry["pan"].as_u64().expect("Source alias pan index") as usize;
                let pan = &pans[index];
                let surround = pan[1] * std::f32::consts::FRAC_1_SQRT_2;
                let center = pan[2] * std::f32::consts::FRAC_1_SQRT_2;
                // Standard stereo fold-down: front L/R, surround at -3 dB,
                // center at -3 dB into each side; LFE is not a full-range channel.
                let stereo = [(pan[0] + surround) * pan[4] + center,
                              (pan[0] + surround) * pan[5] + center];
                native.pan = [stereo, stereo.map(|gain| gain * 0.5)];
            }
        }
    }
    game.audio = source.audio;
    game.content.sounds = source.sounds;
}
