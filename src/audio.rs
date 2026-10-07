//! Bounded Bevy playback of prepared source aliases; no gameplay RNG consumption.
use crate::{GameState, assets::GameDefinition, gameplay::SessionEntity, origins::ContentId};
use bevy::{
    audio::{AudioSinkPlayback, PlaybackMode, Volume},
    prelude::*,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
mod gameplay;
pub(crate) use gameplay::voice;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Effects,
    Ambience,
    Music,
    Dialogue,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alias {
    pub sounds: Vec<ContentId>,
    pub spatial: bool,
    pub channel: Channel,
    pub subtitle: String,
    pub limit: usize,
    pub priority: u32,
    pub seconds: f32,
    pub secondary: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundNote {
    pub seconds: f32,
    pub alias: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmitterDefinition {
    pub alias: String,
    pub position: [f32; 3],
}
#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioDefinition {
    pub aliases: BTreeMap<String, Alias>,
    pub bindings: BTreeMap<String, String>,
    pub timelines: BTreeMap<String, Vec<SoundNote>>,
    pub emitters: Vec<EmitterDefinition>,
    pub surfaces: BTreeMap<String, String>,
    pub character: usize,
}
impl AudioDefinition {
    pub fn valid(&self, game: &GameDefinition) -> bool {
        self.character < 4
            && self.aliases.len() <= 8192
            && self.emitters.len() <= 4096
            && self.aliases.iter().all(|(name, a)| {
                !name.is_empty()
                    && !a.sounds.is_empty()
                    && a.limit > 0
                    && a.limit <= 32
                    && a.seconds.is_finite()
                    && a.seconds > 0.
                    && a.secondary
                        .as_ref()
                        .is_none_or(|n| self.aliases.contains_key(n))
                    && a.sounds
                        .iter()
                        .all(|id| game.content.sounds.iter().any(|s| s.id == *id))
            })
            && self.aliases.keys().all(|name| {
                let mut seen = BTreeSet::new();
                let mut current = Some(name.as_str());
                while let Some(name) = current {
                    if !seen.insert(name) {
                        return false;
                    }
                    current = self.aliases.get(name).and_then(|a| a.secondary.as_deref());
                }
                true
            })
            && self
                .bindings
                .values()
                .all(|name| self.aliases.contains_key(name))
            && self.timelines.iter().all(|(id, notes)| {
                game.content.animations.iter().any(|a| a.id.0 == *id)
                    && notes.iter().all(|n| {
                        n.seconds.is_finite()
                            && n.seconds >= 0.
                            && self.aliases.contains_key(&n.alias)
                    })
            })
            && self.emitters.iter().all(|e| {
                e.position.iter().all(|x| x.is_finite()) && self.aliases.contains_key(&e.alias)
            })
            && self.surfaces.values().all(|s| {
                matches!(
                    s.as_str(),
                    "mud"
                        | "dirt"
                        | "wood"
                        | "metal"
                        | "brick"
                        | "concrete"
                        | "rock"
                        | "gravel"
                        | "grass"
                        | "water"
                        | "default"
                )
            })
    }
}
#[derive(Resource)]
pub struct AudioSettings {
    pub master: f32,
    pub effects: f32,
    pub music: f32,
    pub dialogue: f32,
    pub subtitles: bool,
}
impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master: 1.,
            effects: 1.,
            music: 0.7,
            dialogue: 1.,
            subtitles: true,
        }
    }
}
impl AudioSettings {
    fn gain(&self, channel: Channel) -> f32 {
        self.master
            * match channel {
                Channel::Music => self.music,
                Channel::Dialogue => self.dialogue,
                _ => self.effects,
            }
    }
}
#[derive(Resource, Default)]
pub struct AudioEvidence {
    pub requested: u32,
    pub started: u32,
    pub rejected: u32,
    pub peak: usize,
    pub paused: u32,
    pub resumed: u32,
    pub owner_stops: u32,
    pub events: BTreeMap<String, u32>,
    pub sinks: BTreeMap<String, u32>,
}
#[derive(Resource, Default)]
pub struct Subtitle {
    pub text: String,
    remaining: f32,
}
#[derive(Component)]
pub struct SurfaceSound(pub String);
#[derive(Resource, Default)]
struct Mixer {
    sequence: u64,
    clock: f32,
    voice_cooldown: f32,
    last_voice: BTreeMap<String, f32>,
}
#[derive(Component)]
pub struct SoundRequest {
    alias: String,
    position: Vec3,
    owner: Option<Entity>,
    looping: bool,
}
/// Event requests are admitted together by the mixer, including pending command spawns.
pub fn emit(
    commands: &mut Commands,
    alias: impl Into<String>,
    position: Vec3,
    owner: Option<Entity>,
) -> Entity {
    commands
        .spawn((
            SessionEntity,
            SoundRequest {
                alias: alias.into(),
                position,
                owner,
                looping: false,
            },
        ))
        .id()
}
pub fn emit_loop(
    commands: &mut Commands,
    alias: impl Into<String>,
    position: Vec3,
    owner: Option<Entity>,
) -> Entity {
    commands
        .spawn((
            SessionEntity,
            SoundRequest {
                alias: alias.into(),
                position,
                owner,
                looping: true,
            },
        ))
        .id()
}
pub fn binding(
    commands: &mut Commands,
    game: &GameDefinition,
    key: &str,
    position: Vec3,
    owner: Option<Entity>,
) {
    if let Some(alias) = game.audio.bindings.get(key) {
        emit(commands, alias.clone(), position, owner);
    }
}
#[derive(Component)]
struct PlayingSound {
    alias: String,
    channel: Channel,
    owner: Option<Entity>,
    looping: bool,
    base: f32,
    distance: f32,
    priority: u32,
    gain: f32,
    started: f32,
}
#[derive(Component)]
struct RetiringSound;
fn speech(alias: &str) -> bool {
    alias.starts_with("vox_plr_") || alias.starts_with("vox_sam_")
}
pub struct GameAudioPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AudioPlayback;
impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioSettings>()
            .init_resource::<AudioEvidence>()
            .init_resource::<Subtitle>()
            .init_resource::<Mixer>()
            .add_systems(
                Update,
                (settings, play, maintain, subtitles)
                    .chain()
                    .in_set(AudioPlayback),
            )
            .add_systems(OnEnter(GameState::GameOver), (stop, game_over).chain())
            .add_systems(OnEnter(GameState::Loading), stop);
        gameplay::install(app);
    }
}
fn settings(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut settings: ResMut<AudioSettings>,
) {
    if **state != GameState::Paused {
        return;
    }
    let step = if keys.pressed(KeyCode::ShiftLeft) {
        -0.1
    } else {
        0.1
    };
    if keys.just_pressed(KeyCode::F6) {
        settings.master = (settings.master + step).clamp(0., 1.);
    }
    if keys.just_pressed(KeyCode::F7) {
        settings.music = (settings.music + step).clamp(0., 1.);
    }
    if keys.just_pressed(KeyCode::F8) {
        settings.effects = (settings.effects + step).clamp(0., 1.);
    }
    if keys.just_pressed(KeyCode::F9) {
        settings.dialogue = (settings.dialogue + step).clamp(0., 1.);
    }
    if keys.just_pressed(KeyCode::F10) {
        settings.subtitles = !settings.subtitles;
    }
}
fn play(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    state: Res<State<GameState>>,
    settings: Res<AudioSettings>,
    time: Res<Time>,
    requests: Query<(Entity, &SoundRequest)>,
    playing: Query<(Entity, &PlayingSound)>,
    camera: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    mut mixer: ResMut<Mixer>,
    mut evidence: ResMut<AudioEvidence>,
    mut subtitle: ResMut<Subtitle>,
) {
    let Some(game) = game else {
        return;
    };
    if **state != GameState::Playing && **state != GameState::GameOver {
        for (e, _) in &requests {
            commands.entity(e).despawn();
        }
        return;
    }
    mixer.voice_cooldown = (mixer.voice_cooldown - time.delta_secs()).max(0.);
    mixer.clock += time.delta_secs();
    let listener = camera
        .single()
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut live = BTreeMap::<String, Vec<(Entity, f32)>>::new();
    let mut count = 0;
    let mut voice = Vec::new();
    for (e, s) in &playing {
        count += 1;
        *counts.entry(s.alias.clone()).or_default() += 1;
        live.entry(s.alias.clone())
            .or_default()
            .push((e, s.started));
        if s.channel == Channel::Dialogue && speech(&s.alias) {
            voice.push((e, s.priority));
        }
    }
    for (entity, request) in &requests {
        if **state == GameState::GameOver && request.alias != "mus_zombie_game_over" {
            commands.entity(entity).despawn();
            continue;
        }
        evidence.requested += 1;
        let Some(alias) = game.audio.aliases.get(&request.alias) else {
            commands.entity(entity).despawn();
            evidence.rejected += 1;
            continue;
        };
        if alias.channel == Channel::Music && request.alias.starts_with("mus_zombie_") {
            for (e, s) in &playing {
                if s.alias.starts_with("mus_zombie_") {
                    commands.entity(e).insert(RetiringSound);
                }
            }
        }
        let concurrent = *counts.get(&request.alias).unwrap_or(&0);
        let character_speech = alias.channel == Channel::Dialogue && speech(&request.alias);
        if count >= 64 || (concurrent >= alias.limit && character_speech) {
            commands.entity(entity).despawn();
            evidence.rejected += 1;
            continue;
        }
        if concurrent >= alias.limit {
            // Native same-alias limits replace older tails; rejecting new requests
            // loses bullets during sustained fire. Include this frame's admissions.
            if let Some(entries) = live.get_mut(&request.alias)
                && let Some((index, _)) = entries
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.1.total_cmp(&b.1))
            {
                let (old, _) = entries.remove(index);
                commands.entity(old).despawn();
                count -= 1;
                *counts.get_mut(&request.alias).unwrap() -= 1;
            }
        }
        if character_speech {
            let now = mixer.clock;
            let repeated = mixer
                .last_voice
                .get(&request.alias)
                .is_some_and(|last| now - last < 45.);
            if repeated
                || (mixer.voice_cooldown > 0. && alias.priority < 100)
                || voice.iter().any(|(_, p)| *p >= alias.priority)
            {
                commands.entity(entity).despawn();
                evidence.rejected += 1;
                continue;
            }
            for (e, _) in voice.drain(..) {
                commands.entity(e).despawn();
            }
            mixer.last_voice.insert(request.alias.clone(), now);
            mixer.voice_cooldown = 8.;
            subtitle.text = game
                .content
                .localized_strings
                .iter()
                .find(|s| s.key == alias.subtitle)
                .map_or_else(|| alias.subtitle.clone(), |s| s.text.clone());
            if subtitle.text.is_empty() {
                // Native aliases contain no speech transcripts. Caption the event honestly.
                subtitle.text = if request.alias.starts_with("vox_plr_") {
                    "[Player dialogue]".into()
                } else {
                    "[Voice]".into()
                };
            }
            subtitle.remaining = alias.seconds;
        }
        // Independent deterministic variation cannot alter spawn selection/combat spread.
        mixer.sequence = mixer
            .sequence
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let choice = (mixer.sequence >> 32) as usize % alias.sounds.len();
        let Some(sound) = game
            .content
            .sounds
            .iter()
            .find(|s| s.id == alias.sounds[choice])
        else {
            commands.entity(entity).despawn();
            continue;
        };
        if alias.spatial && listener.distance(request.position) > sound.distance {
            commands.entity(entity).despawn();
            evidence.rejected += 1;
            continue;
        }
        let fraction = ((mixer.sequence >> 16) & 65535) as f32 / 65535.;
        let speed = sound.pitch[0] + (sound.pitch[1] - sound.pitch[0]) * fraction;
        let looping = request.looping || sound.looping;
        let gain = settings.gain(alias.channel);
        commands.entity(entity).remove::<SoundRequest>().insert((
            PlayingSound {
                alias: request.alias.clone(),
                channel: alias.channel,
                owner: request.owner,
                looping,
                base: sound.volume,
                distance: sound.distance,
                priority: alias.priority,
                gain,
                started: mixer.clock,
            },
            AudioPlayer::new(server.load(sound.path.clone())),
            PlaybackSettings {
                mode: if looping {
                    PlaybackMode::Loop
                } else {
                    PlaybackMode::Despawn
                },
                spatial: alias.spatial,
                speed,
                volume: Volume::Linear(sound.volume * gain),
                ..default()
            },
            Transform::from_translation(request.position),
        ));
        if character_speech {
            voice.push((entity, alias.priority));
        }
        *counts.entry(request.alias.clone()).or_default() += 1;
        live.entry(request.alias.clone())
            .or_default()
            .push((entity, mixer.clock));
        count += 1;
        evidence.started += 1;
        *evidence.events.entry(request.alias.clone()).or_default() += 1;
        if let Some(secondary) = &alias.secondary {
            emit(
                &mut commands,
                secondary.clone(),
                request.position,
                request.owner,
            );
        }
    }
    evidence.peak = evidence.peak.max(count);
}
fn maintain(
    mut commands: Commands,
    state: Res<State<GameState>>,
    settings: Res<AudioSettings>,
    time: Res<Time>,
    owners: Query<&GlobalTransform, Without<PlayingSound>>,
    camera: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    mut sounds: Query<(
        Entity,
        &mut PlayingSound,
        &mut Transform,
        Option<&mut AudioSink>,
        Option<&mut SpatialAudioSink>,
        &mut PlaybackSettings,
        Option<&RetiringSound>,
    )>,
    mut evidence: ResMut<AudioEvidence>,
    mut heard: Local<BTreeSet<Entity>>,
) {
    let listener = camera
        .single()
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    heard.retain(|e| sounds.contains(*e));
    for (entity, mut sound, mut pose, regular, spatial, mut playback, retiring) in &mut sounds {
        playback.paused = **state == GameState::Paused;
        if let Some(owner) = sound.owner {
            if let Ok(owner) = owners.get(owner) {
                pose.translation = owner.translation();
            } else if sound.looping {
                commands.entity(entity).despawn();
                evidence.owner_stops += 1;
                continue;
            }
        }
        if regular.is_some() || spatial.is_some() {
            if heard.insert(entity) {
                *evidence.sinks.entry(sound.alias.clone()).or_default() += 1;
            }
        }
        let distance = listener.distance(pose.translation);
        // Explicit bounded linear distance falloff complements Bevy stereo localization.
        let attenuation = if spatial.is_some() {
            (1. - distance / sound.distance).clamp(0., 1.)
        } else {
            1.
        };
        let target = if retiring.is_some() {
            0.
        } else {
            settings.gain(sound.channel) * attenuation
        };
        if **state == GameState::Paused {
            if retiring.is_none() {
                sound.gain = target;
            }
        } else {
            sound.gain += (target - sound.gain) * (time.delta_secs() * 5.).min(1.);
        }
        if retiring.is_some() && sound.gain < 0.002 {
            commands.entity(entity).despawn();
            continue;
        }
        let volume = Volume::Linear(sound.base * sound.gain);
        for sink in regular
            .into_iter()
            .map(|s| s.into_inner() as &mut dyn AudioSinkPlayback)
            .chain(
                spatial
                    .into_iter()
                    .map(|s| s.into_inner() as &mut dyn AudioSinkPlayback),
            )
        {
            sink.set_volume(volume);
            if **state == GameState::Paused && !sink.is_paused() {
                sink.pause();
                evidence.paused += 1;
            }
            if **state == GameState::Playing && sink.is_paused() {
                sink.play();
                evidence.resumed += 1;
            }
        }
    }
}
fn stop(
    mut commands: Commands,
    sounds: Query<Entity, Or<(With<PlayingSound>, With<SoundRequest>)>>,
    mut mixer: ResMut<Mixer>,
    mut subtitle: ResMut<Subtitle>,
) {
    for e in &sounds {
        commands.entity(e).despawn();
    }
    *mixer = Mixer::default();
    *subtitle = Subtitle::default();
}
fn game_over(mut commands: Commands, game: Option<Res<GameDefinition>>) {
    if game.is_some_and(|g| g.audio.aliases.contains_key("mus_zombie_game_over")) {
        emit(&mut commands, "mus_zombie_game_over", Vec3::ZERO, None);
    }
}
fn subtitles(state: Res<State<GameState>>, time: Res<Time>, mut subtitle: ResMut<Subtitle>) {
    if **state == GameState::Playing {
        subtitle.remaining = (subtitle.remaining - time.delta_secs()).max(0.);
    }
    if subtitle.remaining == 0. {
        subtitle.text.clear();
    }
}
