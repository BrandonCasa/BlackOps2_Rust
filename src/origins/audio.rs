//! Native audio records and ordinary encoded sound assets.
use super::{GameDefinition, SoundDefinition};
use crate::audio::AudioDefinition;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct SourceAudio {
    audio: AudioDefinition,
    sounds: Vec<SoundDefinition>,
}

pub(crate) fn bind(game: &mut GameDefinition, root: &Path) {
    let source: SourceAudio = super::read(&root.join("audio.json"));
    game.audio = source.audio;
    game.content.sounds = source.sounds;
}
