use crate::{GameState, assets::GameDefinition, gameplay::SessionEntity, origins::ContentId};
use bevy::{
    audio::{AddAudioSource, AudioSinkPlayback, PlaybackMode, SpatialScale, Volume},
    prelude::*,
    transform::helper::TransformHelper,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};
mod dialogue;
mod gameplay;
mod master;
mod narrative;
mod obstruction;
mod reverb;
pub(crate) use dialogue::voice;
pub(crate) use narrative::{NarrativeEvent, NarrativeProgress, RobotPurgeReady, samantha};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Effects,
    Ambience,
    Music,
    Dialogue,
}
#[derive(Clone, Deserialize)]
pub struct Alias {
    pub native: Vec<NativeSound>,
}

#[derive(Clone, Deserialize)]
pub struct NativeSound {
    pub id: u32,
    pub sound: Option<ContentId>,
    pub seconds: Option<f32>,
    pub volume_max: f32,
    pub spatial: bool,
    pub channel: Channel,
    pub subtitle: String,
    pub secondary: Option<String>,
    pub context: [u32; 2],
    pub probability: u8,
    pub randomize: u8,
    pub never_play_twice: bool,
    pub ignore_max_distance: bool,
    pub stop_on_death: bool,
    pub loaded: bool,
    pub voice_limit: bool,
    pub announcer: bool,
    pub pauseable: bool,
    pub motion: u8,
    pub motion_seconds: f32,
    pub bus: u8,
    pub distance_low_pass: bool,
    pub obstruction: f32,
    pub large: bool,
    pub panning: bool,
    pub pan: [[f32; 2]; 2],
    pub duck: Option<u32>,
    pub duck_group: u8,
    pub priority: [u8; 2],
    pub priority_threshold: [f32; 2],
    pub limit: NativeLimit,
    pub entity_limit: NativeLimit,
    pub delay: f32,
    pub fades: [f32; 2],
    pub curves: [SoundCurve; 2],
    pub reverb_send: f32,
    pub wet_distance: [f32; 2],
    pub reverb_curves: [SoundCurve; 2],
}

#[derive(Clone, Copy, Deserialize)]
pub struct NativeLimit {
    pub count: u8,
    pub policy: u8,
}

#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub struct SoundCurve(pub [[f32; 2]; 8]);

impl SoundCurve {
    fn sample(&self, value: f32) -> f32 {
        let value = if value.is_nan() { 0. } else { value };
        if value <= self.0[0][0] {
            return self.0[0][1];
        }
        if value >= self.0[7][0] {
            return self.0[7][1];
        }
        for points in self.0.windows(2) {
            if value >= points[0][0] && value <= points[1][0] {
                let fraction = (value - points[0][0]) / (points[1][0] - points[0][0]);
                return points[0][1] * (1. - fraction) + points[1][1] * fraction;
            }
        }
        0.
    }
}

impl NativeSound {
    fn velocity(&self, listener: Vec3, emitter: Vec3) -> Vec3 {
        if !self.spatial || !matches!(self.motion, 1 | 3) || self.motion_seconds == 0. {
            return Vec3::ZERO;
        }
        let direction = (listener - emitter).normalize_or_zero();
        let sign = if self.motion == 1 { -1. } else { 1. };
        (direction * 0.7 + direction.cross(Vec3::Y) * 0.3 * sign).normalize_or_zero()
            * (self.wet_distance[1] / self.motion_seconds)
    }
    fn fade_in(&self, age: f32) -> f32 {
        if self.fades[0] > 0. {
            (age / self.fades[0]).clamp(0., 1.)
        } else {
            1.
        }
    }
    fn distance_filter(&self, distance: f32) -> f32 {
        if !self.spatial || !self.distance_low_pass {
            return 1.;
        }
        let gain = ((distance / 0.0254 - 800.) / 2200.).clamp(0., 1.) * -30.;
        1. + gain / 35.
    }
    fn attenuation(&self, distance: f32, minimum: f32, maximum: f32) -> f32 {
        if distance < minimum {
            self.curves[1].sample((distance / minimum).clamp(0., 1.))
        } else {
            self.curves[0].sample(((distance - minimum) / (maximum - minimum)).clamp(0., 1.))
        }
    }
    fn priority_distance(&self, distance: f32, minimum: f32) -> f32 {
        if !self.spatial {
            return self.priority_at(0.);
        }
        let zero_curve =
            |curve: &SoundCurve| curve.0.iter().map(|point| point[1]).sum::<f32>() < 1.52879e-05;
        let radius = if zero_curve(&self.curves[0]) || zero_curve(&self.reverb_curves[0]) {
            minimum
        } else {
            self.wet_distance[1]
        };
        self.priority_at(1. - (distance / radius).clamp(0., 1.))
    }
    fn priority_at(&self, gain: f32) -> f32 {
        if gain <= self.priority_threshold[0] {
            return self.priority[0] as f32;
        }
        if gain >= self.priority_threshold[1] {
            return self.priority[1] as f32;
        }
        let fraction = (gain - self.priority_threshold[0])
            / (self.priority_threshold[1] - self.priority_threshold[0]);
        self.priority[0] as f32 * (1. - fraction) + self.priority[1] as f32 * fraction
    }
}
#[derive(Clone, Deserialize)]
pub struct SoundNote {
    pub seconds: f32,
    pub alias: String,
}
#[derive(Clone, Deserialize)]
pub struct EmitterDefinition {
    pub alias: String,
    pub position: [f32; 3],
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct WaterDefinition {
    pub volumes: Vec<Vec<[f32; 4]>>,
    pub triangles: Vec<[[f32; 3]; 3]>,
}
impl WaterDefinition {
    fn touches(&self, position: Vec3) -> bool {
        let start = position + Vec3::Y * 200. * 0.0254;
        let movement = Vec3::NEG_Y * 400. * 0.0254;
        let epsilon = 0.125 * 0.0254;
        for planes in &self.volumes {
            let mut first = 0_f32;
            let mut last = 1_f32;
            let mut hit = true;
            for plane in planes {
                let normal = Vec3::new(plane[0], plane[1], plane[2]);
                let distance = normal.dot(start) - plane[3];
                let delta = normal.dot(movement);
                if delta == 0. {
                    if distance > 0. {
                        hit = false;
                        break;
                    }
                } else if delta < 0. {
                    first = first.max((distance - epsilon) / -delta);
                } else {
                    last = last.min(distance / -delta);
                }
                if first > last {
                    hit = false;
                    break;
                }
            }
            if hit && first <= 1. && start.y + movement.y * first >= position.y {
                return true;
            }
        }
        for triangle in &self.triangles {
            let [a, b, c] = triangle.map(Vec3::from_array);
            let normal = (b - a).cross(c - a).normalize_or_zero();
            if normal.y == 0. {
                continue;
            }
            let fraction = (normal.dot(a - start) + epsilon) / normal.dot(movement);
            if !(0. ..=1.).contains(&fraction) {
                continue;
            }
            let point = start + movement * fraction;
            let inside = [(a, b), (b, c), (c, a)]
                .iter()
                .all(|(a, b)| (b - a).cross(point - a).dot(normal) >= -0.000001);
            if inside && point.y >= position.y {
                return true;
            }
        }
        false
    }
}
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct AudioDefinition {
    pub aliases: BTreeMap<String, Alias>,
    pub bindings: BTreeMap<String, String>,
    pub timelines: BTreeMap<String, Vec<SoundNote>>,
    pub emitters: Vec<EmitterDefinition>,
    pub surfaces: BTreeMap<String, String>,
    pub character: usize,
    pub absent: BTreeSet<String>,
    pub contexts: BTreeMap<u32, u32>,
    pub footsteps: Vec<[Option<String>; 7]>,
    pub water: WaterDefinition,
    pub room_contexts: BTreeMap<String, BTreeMap<u32, u32>>,
    pub ducks: BTreeMap<u32, DuckProfile>,
    pub rooms: BTreeMap<String, RoomDefinition>,
    pub master: Option<MasterDefinition>,
}
impl AudioDefinition {
    fn named_alias(&self, name: &str) -> Option<(&str, &Alias)> {
        self.aliases
            .get_key_value(name)
            .or_else(|| {
                self.aliases
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(name))
            })
            .map(|(key, alias)| (key.as_str(), alias))
    }
    fn absent_alias(&self, name: &str) -> Option<&str> {
        self.absent
            .get(name)
            .or_else(|| {
                self.absent
                    .iter()
                    .find(|key| key.eq_ignore_ascii_case(name))
            })
            .map(String::as_str)
    }
}
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
    pub(crate) fn gain(&self, channel: Channel) -> f32 {
        self.master
            * match channel {
                Channel::Music => self.music,
                Channel::Dialogue => self.dialogue,
                _ => self.effects,
            }
    }
}
#[derive(Resource, Default)]
pub struct Subtitle {
    pub text: String,
    remaining: f32,
    pauseable: bool,
}
#[derive(Component)]
pub struct SurfaceSound(pub String);
#[derive(Resource, Default)]
pub(crate) struct Mixer {
    clock: f32,
    native_random: Option<u32>,
    native_previous: BTreeMap<String, usize>,
    native_played: BTreeSet<(String, usize)>,
    native_frame: u64,
    entity_random: Vec<Option<(u32, f32)>>,
}
impl Mixer {
    fn random(&mut self) -> u32 {
        let next = self
            .native_random
            .unwrap_or(1)
            .wrapping_mul(0x343fd)
            .wrapping_add(0x269ec3);
        self.native_random = Some(next);
        next >> 16 & 32767
    }
    fn entity_fraction(&mut self, key: u32) -> f32 {
        if self.entity_random.is_empty() {
            self.entity_random.resize(136, None);
        }
        for entry in &mut self.entity_random {
            if entry.is_some_and(|(_, time)| self.clock - time > 30.) {
                *entry = None;
            }
        }
        let index = if let Some(index) = self
            .entity_random
            .iter()
            .position(|entry| entry.is_some_and(|(id, _)| id == key))
        {
            index
        } else {
            let mut largest = (0, 0);
            let mut index = 0;
            while index < self.entity_random.len() {
                let start = index;
                while index < self.entity_random.len() && self.entity_random[index].is_none() {
                    index += 1;
                }
                if index - start > largest.1 {
                    largest = (start, index - start);
                }
                index += 1;
            }
            if largest.1 > 0 {
                largest.0 + largest.1 / 2
            } else {
                self.entity_random
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.unwrap().1.total_cmp(&b.unwrap().1))
                    .map(|(index, _)| index)
                    .unwrap_or(0)
            }
        };
        self.entity_random[index] = Some((key, self.clock));
        index as f32 * 0.0074074073
    }
    fn select(
        &mut self,
        name: &str,
        alias: &Alias,
        source: SoundSource,
        contexts: &BTreeMap<u32, u32>,
    ) -> Option<usize> {
        let available = alias
            .native
            .iter()
            .take(64)
            .enumerate()
            .filter_map(|(index, sound)| {
                (sound.context[0] == 0
                    || sound.context[1] == 0
                    || contexts.get(&sound.context[0]).copied().unwrap_or(0) == sound.context[1])
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        if available.is_empty() {
            return None;
        }
        let mut choice = 0;
        if available.len() > 1 {
            if alias.native[available[0]].randomize & 4 != 0 {
                let mut seed = (std::ptr::from_ref(alias).addr() as u32).wrapping_add(source.key());
                seed = seed.wrapping_mul(0x41c64e6d).wrapping_add(0x3039);
                choice = ((seed as i32).wrapping_add(((seed as i32) >> 31) as u32 as i32 & 65535)
                    >> 16) as usize
                    & 32767;
                choice %= available.len();
            } else {
                for _ in 0..100 {
                    choice = self.random() as usize % available.len();
                    if available.len() <= 2 || self.native_previous.get(name) != Some(&choice) {
                        break;
                    }
                }
            }
        }
        self.native_previous.insert(name.into(), choice);
        Some(available[choice])
    }
}
#[derive(Clone, Copy, Debug)]
pub enum SoundSource {
    World(Vec3),
    Entity { entity: Entity, offset: Vec3 },
}
impl SoundSource {
    fn key(self) -> u32 {
        match self {
            Self::World(_) => 0xfff,
            Self::Entity { entity, .. } => entity.to_bits() as u32 & 0xfff,
        }
    }
    pub fn attached(entity: Entity) -> Self {
        Self::Entity {
            entity,
            offset: Vec3::ZERO,
        }
    }
    fn position(self, transforms: &TransformHelper) -> Option<Vec3> {
        match self {
            Self::World(position) => Some(position),
            Self::Entity { entity, offset } => transforms
                .compute_global_transform(entity)
                .ok()
                .map(|pose| pose.transform_point(offset)),
        }
    }
}
#[derive(Component, Clone)]
pub struct SoundRequest {
    alias: String,
    source: SoundSource,
    looping: bool,
    native: Option<usize>,
    secondary_sent: bool,
}
#[cfg(test)]
pub(crate) fn requested_sounds(world: &mut World) -> Vec<(String, SoundSource)> {
    world
        .query::<&SoundRequest>()
        .iter(world)
        .map(|request| (request.alias.clone(), request.source))
        .collect()
}
#[derive(Component, Clone, Copy)]
pub struct SoundControl {
    pub volume: f32,
    pub pitch: f32,
}
impl Default for SoundControl {
    fn default() -> Self {
        Self {
            volume: 1.,
            pitch: 1.,
        }
    }
}
pub fn emit(commands: &mut Commands, alias: impl Into<String>, source: SoundSource) -> Entity {
    commands
        .spawn((
            SessionEntity,
            SoundRequest {
                alias: alias.into(),
                source,
                looping: false,
                native: None,
                secondary_sent: false,
            },
        ))
        .id()
}
pub fn emit_loop(commands: &mut Commands, alias: impl Into<String>, source: SoundSource) -> Entity {
    commands
        .spawn((
            SessionEntity,
            SoundRequest {
                alias: alias.into(),
                source,
                looping: true,
                native: None,
                secondary_sent: false,
            },
        ))
        .id()
}
#[derive(Resource, Default)]
pub(crate) struct StoryReservation(pub Option<Entity>);
#[derive(Resource, Default)]
pub(crate) struct MusicOverride(pub Option<Entity>);
#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct PlaybackEvent {
    pub entity: Entity,
    pub status: PlaybackStatus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaybackStatus {
    Accepted,
    Completed,
    Rejected,
    Cancelled,
}
#[derive(Resource, Default)]
pub(crate) struct Lifetimes(BTreeMap<Entity, bool>);

#[derive(Clone, Deserialize)]
pub struct MasterDefinition {
    pub buses: [f32; 9],
    pub gain: f32,
    pub equalizers: [EqualizerDefinition; 4],
    pub compressor: DynamicsDefinition,
    pub limiter: DynamicsDefinition,
}

#[derive(Clone, Deserialize)]
pub struct EqualizerDefinition {
    pub enabled: bool,
    pub gain: f32,
    pub frequency: f32,
    pub quality: f32,
}

#[derive(Clone, Deserialize)]
pub struct DynamicsDefinition {
    pub enabled: bool,
    pub pre_gain: f32,
    pub post_gain: f32,
    pub threshold: f32,
    pub ratio: f32,
    pub attack: f32,
    pub release: f32,
}

#[derive(Clone, Deserialize)]
pub struct RoomDefinition {
    pub parameters: [f32; 16],
}

#[derive(Clone, Deserialize)]
pub struct DuckProfile {
    pub name: String,
    pub fades: [f32; 2],
    pub delay: f32,
    pub distance: f32,
    pub length: f32,
    pub attenuation: [f32; 32],
    pub filter: [f32; 32],
    pub update_while_paused: bool,
}
impl DuckProfile {
    fn amount(&self, age: f32, stopped: Option<f32>, distance: f32) -> Option<f32> {
        let start = age - self.delay;
        let fade = if let Some(stopped) = stopped.filter(|stopped| age > *stopped) {
            let elapsed = age - stopped;
            if start < 0. || elapsed > self.fades[1] {
                return None;
            }
            if start >= self.fades[0] {
                1. - elapsed / self.fades[1]
            } else {
                (start / self.fades[0] - elapsed / self.fades[1]).max(0.)
            }
        } else if start < 0. {
            0.
        } else if start < self.fades[0] {
            start / self.fades[0]
        } else {
            1.
        };
        let spatial = if self.distance > 1.52879e-05 {
            (1. - distance / self.distance).max(0.)
        } else {
            1.
        };
        Some(fade.min(spatial))
    }
}
struct DuckVoice {
    entity: Entity,
    profile: u32,
    position: Vec3,
    seconds: f32,
    looping: bool,
    playing: bool,
    paused: bool,
}
struct DuckInstance {
    profile: u32,
    position: Vec3,
    age: f32,
    stopped: Option<f32>,
    amount: f32,
}
#[derive(Resource)]
pub(crate) struct NativeDucks {
    active: BTreeMap<Entity, DuckInstance>,
    seen: BTreeSet<Entity>,
    attenuation: [f32; 32],
    filter: [f32; 32],
}
impl Default for NativeDucks {
    fn default() -> Self {
        Self {
            active: BTreeMap::new(),
            seen: BTreeSet::new(),
            attenuation: [1.; 32],
            filter: [1.; 32],
        }
    }
}
impl NativeDucks {
    fn update(
        &mut self,
        profiles: &BTreeMap<u32, DuckProfile>,
        voices: &[DuckVoice],
        listener: Vec3,
        delta: f32,
        paused: bool,
    ) {
        self.seen
            .retain(|entity| voices.iter().any(|voice| voice.entity == *entity));
        for voice in voices {
            if !voice.playing || voice.paused || self.seen.contains(&voice.entity) {
                continue;
            }
            self.seen.insert(voice.entity);
            if self.active.len() >= 128 {
                continue;
            }
            let profile = &profiles[&voice.profile];
            self.active.insert(
                voice.entity,
                DuckInstance {
                    profile: voice.profile,
                    position: voice.position,
                    age: 0.,
                    amount: 0.,
                    stopped: if profile.length > 1.52879e-05 {
                        Some(profile.length)
                    } else if !voice.looping {
                        Some(voice.seconds - profile.fades[1])
                    } else {
                        None
                    },
                },
            );
        }
        self.active.retain(|entity, instance| {
            let profile = &profiles[&instance.profile];
            if let Some(voice) = voices.iter().find(|voice| voice.entity == *entity) {
                instance.position = voice.position;
                if !voice.playing && instance.stopped.is_none() {
                    instance.stopped = Some(instance.age);
                }
            } else if instance
                .stopped
                .is_none_or(|stopped| stopped > instance.age)
            {
                if paused || instance.amount < 1.52879e-05 {
                    return false;
                }
                instance.stopped = Some(instance.age);
            }
            if paused && !profile.update_while_paused {
                return true;
            }
            instance.age += delta;
            let Some(amount) = profile.amount(
                instance.age,
                instance.stopped,
                listener.distance(instance.position),
            ) else {
                return false;
            };
            instance.amount = amount;
            true
        });
        let mut target = [1_f32; 32];
        let mut filter = [1_f32; 32];
        for instance in self.active.values() {
            let profile = &profiles[&instance.profile];
            for (target, attenuation) in target.iter_mut().zip(profile.attenuation) {
                *target = target.min(1. - instance.amount * (1. - attenuation));
            }
            for (target, value) in filter.iter_mut().zip(profile.filter) {
                *target = target.min(1. - instance.amount * (1. - value));
            }
        }
        let step = (delta * 10.).min(0.1);
        for (current, target) in self.attenuation.iter_mut().zip(target) {
            *current += (target - *current).clamp(-step, step);
        }
        for (current, target) in self.filter.iter_mut().zip(filter) {
            *current += (target - *current).clamp(-step, step);
        }
    }
    fn gain(&self, group: u8) -> f32 {
        self.attenuation[group as usize].powi(2)
    }
}
#[derive(Component, Default)]
pub(crate) struct SemanticPlayback {
    accepted: bool,
    completed: bool,
    dry_done: Option<Arc<AtomicBool>>,
    wet: Option<Arc<AtomicU32>>,
    filter: Option<Arc<AtomicU32>>,
    voice: Option<Arc<reverb::VoiceControl>>,
    obstruction: f32,
    obstruction_target: f32,
}

impl Drop for SemanticPlayback {
    fn drop(&mut self) {
        if let Some(voice) = &self.voice {
            voice.stopped.store(true, Ordering::Release);
        }
    }
}
#[derive(Component)]
struct PendingRoom(Handle<AudioSource>);
pub(crate) fn resolve_alias<'a>(game: &'a GameDefinition, name: &str) -> Option<&'a str> {
    let name = match name {
        "purchase" => "zmb_cha_ching",
        "music_chest" => "zmb_music_box",
        other => other,
    };
    if let Some((key, _)) = game.audio.named_alias(name) {
        return Some(key);
    }
    if let Some(alias) = game.audio.bindings.get(name) {
        return game.audio.named_alias(alias).map(|(key, _)| key);
    }
    if let Some(alias) = game.audio.absent_alias(name) {
        return Some(alias);
    }
    game.audio
        .aliases
        .iter()
        .find(|(_, alias)| {
            alias
                .native
                .iter()
                .any(|sound| sound.sound.as_ref().is_some_and(|id| id.0 == name))
        })
        .map(|(name, _)| name.as_str())
}
#[derive(Component, Clone)]
pub struct SoundFade {
    pub remaining: f32,
    pub duration: f32,
    pub out: bool,
}
pub fn binding(commands: &mut Commands, game: &GameDefinition, key: &str, source: SoundSource) {
    if let Some(alias) = game.audio.bindings.get(key) {
        emit(commands, alias.clone(), source);
    }
}
#[derive(Component)]
pub(crate) struct PlayingSound {
    pub(crate) alias: String,
    pub(crate) channel: Channel,
    pub(crate) source: SoundSource,
    pub(crate) looping: bool,
    pub(crate) base: f32,
    pub(crate) distance_min: f32,
    pub(crate) distance: f32,
    priority: f32,
    gain: f32,
    started: f32,
    native: usize,
    frame: u64,
    delay: f32,
    age: f32,
    stop_remaining: Option<f32>,
    speed: f32,
    velocity: Vec3,
}

fn pitch_speed(value: f32) -> f32 {
    if (value - 1.).abs() < 1.52879e-05 {
        1.
    } else {
        value.clamp(0.01, 1.9)
    }
}
#[derive(Component)]
pub(crate) struct RetiringSound;
struct NativeVoice {
    entity: Entity,
    alias: String,
    source: u32,
    priority: f32,
    started: f32,
    frame: u64,
    loaded: bool,
    voice_limit: bool,
    announcer: bool,
}
fn native_limit(
    limit: NativeLimit,
    voices: &[&NativeVoice],
    priority: f32,
    frame: Option<u64>,
) -> Result<Option<Entity>, ()> {
    if limit.policy == 0 {
        return Ok(None);
    }
    if frame.is_some_and(|frame| voices.iter().any(|voice| voice.frame == frame)) {
        return Err(());
    }
    if voices.len() < limit.count as usize {
        return Ok(None);
    }
    match limit.policy {
        1 => Ok(voices
            .iter()
            .min_by(|a, b| a.started.total_cmp(&b.started))
            .map(|voice| voice.entity)),
        2 => Err(()),
        3 => voices
            .iter()
            .min_by(|a, b| a.priority.total_cmp(&b.priority))
            .filter(|voice| priority > voice.priority + 3.)
            .map(|voice| Some(voice.entity))
            .ok_or(()),
        _ => Err(()),
    }
}
pub struct GameAudioPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AudioPlayback;
impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioSettings>()
            .init_resource::<Subtitle>()
            .init_resource::<Mixer>()
            .init_resource::<StoryReservation>()
            .init_resource::<MusicOverride>()
            .init_resource::<Lifetimes>()
            .init_resource::<NativeDucks>()
            .init_resource::<reverb::ListenerRoom>()
            .add_message::<PlaybackEvent>()
            .add_audio_source::<reverb::RoomClip>()
            .add_audio_source::<reverb::RoomBus>()
            .add_systems(
                Update,
                (settings, reverb::update)
                    .chain()
                    .after(crate::input::pause_and_restart),
            )
            .add_systems(
                PostUpdate,
                (play, route, lifecycle, subtitles, maintain)
                    .chain()
                    .after(bevy::app::AnimationSystems)
                    .after(crate::presentation::CameraPose)
                    .before(bevy::transform::TransformSystems::Propagate)
                    .in_set(AudioPlayback),
            )
            .add_systems(OnEnter(GameState::GameOver), (stop, game_over).chain())
            .add_systems(OnEnter(GameState::Loading), stop);
        app.add_systems(OnEnter(GameState::StartingSession), stop);
        app.add_systems(OnEnter(GameState::MainMenu), stop);
        app.add_systems(OnEnter(GameState::Ending), stop);
        gameplay::install(app);
        dialogue::install(app);
        narrative::install(app);
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
    time: Res<Time<Real>>,
    requests: Query<(
        Entity,
        &SoundRequest,
        Option<&SoundFade>,
        Option<&SoundControl>,
    )>,
    playing: Query<(Entity, &PlayingSound, &SemanticPlayback)>,
    camera: Query<Entity, With<crate::rendering::GameCamera>>,
    transforms: TransformHelper,
    mut mixer: ResMut<Mixer>,
    mut subtitle: ResMut<Subtitle>,
    (music, generators, weather): (
        Res<MusicOverride>,
        Res<crate::generators::Generators>,
        Res<crate::environment::weather::Weather>,
    ),
    mut events: MessageWriter<PlaybackEvent>,
    mut lifetimes: ResMut<Lifetimes>,
) {
    let Some(game) = game else { return };
    mixer.clock += time.delta_secs();
    if **state == GameState::Paused {
        return;
    }
    if !matches!(
        **state,
        GameState::Playing | GameState::GameOver | GameState::Ending | GameState::StartingSession
    ) {
        for (entity, ..) in &requests {
            commands.entity(entity).try_despawn();
        }
        return;
    }
    mixer.native_frame = mixer.native_frame.wrapping_add(1);
    if requests.is_empty() {
        return;
    }
    let mut contexts = game.audio.contexts.clone();
    if let Some(room) = game
        .audio
        .room_contexts
        .get(&weather.ambient_room)
        .or_else(|| game.audio.room_contexts.get("tomb_outdoor"))
    {
        contexts.extend(room.iter().map(|(kind, value)| (*kind, *value)));
    }
    let listener = camera
        .single()
        .ok()
        .and_then(|e| transforms.compute_global_transform(e).ok())
        .map_or(Vec3::ZERO, |t| t.translation());
    let mut native_voices = Vec::new();
    for (entity, sound, semantic) in &playing {
        if semantic.completed {
            continue;
        }
        let native = &game.audio.aliases[&sound.alias].native[sound.native];
        native_voices.push(NativeVoice {
            entity,
            alias: sound.alias.clone(),
            source: sound.source.key(),
            priority: sound.priority,
            started: sound.started,
            frame: sound.frame,
            loaded: native.loaded,
            voice_limit: native.voice_limit,
            announcer: native.announcer,
        });
    }
    let mut pending: VecDeque<_> = requests
        .iter()
        .map(|(entity, request, fade, control)| {
            (entity, request.clone(), fade.cloned(), control.cloned())
        })
        .collect();
    while let Some((entity, mut request, fade, control)) = pending.pop_front() {
        if let Some(resolved) = resolve_alias(&game, &request.alias) {
            request.alias = resolved.to_owned();
        }
        if **state == GameState::GameOver && request.alias != "mus_zombie_game_over" {
            commands.entity(entity).try_despawn();
            continue;
        }
        let Some(alias) = game.audio.aliases.get(&request.alias) else {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: if game.audio.absent.contains(&request.alias) {
                    PlaybackStatus::Completed
                } else {
                    PlaybackStatus::Rejected
                },
            });
            continue;
        };
        let Some(native_index) = request
            .native
            .or_else(|| mixer.select(&request.alias, alias, request.source, &contexts))
        else {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Completed,
            });
            continue;
        };
        request.native = Some(native_index);
        let native = &alias.native[native_index];
        if native.never_play_twice
            && mixer
                .native_played
                .contains(&(request.alias.clone(), native_index))
        {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        if !request.secondary_sent {
            request.secondary_sent = true;
            if let Some(secondary) = &native.secondary {
                let child = emit(&mut commands, secondary.clone(), request.source);
                let secondary = SoundRequest {
                    alias: secondary.clone(),
                    source: request.source,
                    looping: false,
                    native: None,
                    secondary_sent: false,
                };
                pending.push_front((entity, request, fade, control.clone()));
                pending.push_front((child, secondary, None, control));
                continue;
            }
        }
        let Some(sound_id) = &native.sound else {
            if !native.subtitle.is_empty() {
                subtitle.text = game
                    .content
                    .localized_strings
                    .iter()
                    .find(|s| s.key == native.subtitle)
                    .map_or_else(|| native.subtitle.clone(), |s| s.text.clone());
                subtitle.remaining = 5.;
                subtitle.pauseable = native.pauseable;
            }
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Completed,
            });
            continue;
        };
        if native.channel == Channel::Music && request.alias.starts_with("mus_zombie_") {
            if music.0.is_some()
                || generators.raid_active
                    && !matches!(
                        request.alias.as_str(),
                        "mus_zombie_dog_start" | "mus_zombie_dog_end"
                    )
            {
                commands.entity(entity).try_despawn();
                events.write(PlaybackEvent {
                    entity,
                    status: PlaybackStatus::Rejected,
                });
                continue;
            }
            for (old, sound, _) in &playing {
                if sound.alias.starts_with("mus_zombie_") {
                    commands.entity(old).try_insert(RetiringSound);
                }
            }
        }
        let sound = game
            .content
            .sounds
            .iter()
            .find(|sound| sound.id == *sound_id)
            .unwrap_or_else(|| {
                panic!(
                    "native alias {} references missing sound {}",
                    request.alias, sound_id.0
                )
            });
        let Some(position) = request.source.position(&transforms) else {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        };
        let looping = request.looping || sound.looping;
        let distance = listener.distance(position);
        if native.spatial && !native.ignore_max_distance && distance > sound.distance {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        let attenuation = if native.spatial {
            native.attenuation(distance, sound.distance_min, sound.distance)
        } else {
            1.
        };
        let priority = native.priority_distance(distance, sound.distance_min);
        if sound.volume < 1.52879e-05 && native.volume_max < 1.52879e-05 {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Completed,
            });
            continue;
        }
        if native.probability != 0
            && mixer.random() as f32 * 3.0517578e-05 > native.probability as f32 * 0.003921569
        {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        let mut victims = BTreeSet::new();
        let mut rejected = false;
        for (limit, per_entity) in [(native.limit, false), (native.entity_limit, true)] {
            let candidates = native_voices
                .iter()
                .filter(|voice| {
                    voice.alias == request.alias
                        && (!per_entity || voice.source == request.source.key())
                        && !victims.contains(&voice.entity)
                })
                .collect::<Vec<_>>();
            match native_limit(
                limit,
                &candidates,
                priority,
                per_entity.then_some(mixer.native_frame),
            ) {
                Ok(Some(victim)) => {
                    victims.insert(victim);
                }
                Ok(None) => {}
                Err(()) => {
                    rejected = true;
                    break;
                }
            }
        }
        if !rejected {
            rejected = native.announcer
                && native_voices
                    .iter()
                    .any(|voice| voice.announcer && !victims.contains(&voice.entity));
        }
        if !rejected && native.voice_limit {
            victims.extend(
                native_voices
                    .iter()
                    .filter(|voice| voice.voice_limit && voice.source == request.source.key())
                    .map(|voice| voice.entity),
            );
        }
        if !rejected {
            let concurrent = native_voices
                .iter()
                .filter(|voice| voice.loaded == native.loaded && !victims.contains(&voice.entity))
                .count();
            rejected = concurrent >= if native.loaded { 58 } else { 10 };
        }
        if rejected {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        for victim in victims {
            commands.entity(victim).try_despawn();
            native_voices.retain(|voice| voice.entity != victim);
        }

        let key = native
            .id
            .wrapping_shl(16)
            .wrapping_add(request.source.key());
        let fraction = if native.randomize & 1 != 0 {
            mixer.entity_fraction(key | 0x80000000)
        } else {
            mixer.random() as f32 * 3.0517578e-05
        };
        let volume = sound.volume + (native.volume_max - sound.volume) * fraction;
        let fraction = if native.randomize & 2 != 0 {
            mixer.entity_fraction(key & 0x7fffffff)
        } else {
            mixer.random() as f32 * 3.0517578e-05
        };
        let speed = sound.pitch[0] + (sound.pitch[1] - sound.pitch[0]) * fraction;
        let gain = if fade.as_ref().is_some_and(|fade| !fade.out) {
            0.
        } else {
            settings.gain(native.channel).powi(2) * attenuation
        };
        let semantic = SemanticPlayback {
            accepted: false,
            completed: false,
            dry_done: Some(Arc::new(AtomicBool::new(false))),
            wet: Some(Arc::new(AtomicU32::new(
                (native.reverb_send / 100.).to_bits(),
            ))),
            filter: Some(Arc::new(AtomicU32::new(1_f32.to_bits()))),
            voice: None,
            obstruction: 0.,
            obstruction_target: 0.,
        };
        commands.entity(entity).remove::<SoundRequest>().insert((
            PlayingSound {
                alias: request.alias.clone(),
                channel: native.channel,
                source: request.source,
                looping,
                base: volume,
                distance_min: sound.distance_min,
                distance: sound.distance,
                priority,
                gain,
                started: mixer.clock,
                native: native_index,
                frame: mixer.native_frame,
                delay: native.delay,
                age: 0.,
                stop_remaining: None,
                speed,
                velocity: native.velocity(listener, position),
            },
            PlaybackSettings {
                mode: PlaybackMode::Once,
                spatial: native.spatial && native.panning,
                spatial_scale: Some(SpatialScale::new(1. / (sound.distance + 0.2))),
                speed: pitch_speed(speed * control.map_or(1., |control| control.pitch)),
                volume: Volume::Linear(
                    volume * gain * control.map_or(1., |control| control.volume),
                ),
                paused: native.delay > 0.,
                ..default()
            },
            Transform::from_translation(position),
            semantic,
            PendingRoom(server.load(sound.path.clone())),
        ));
        lifetimes.0.insert(entity, false);
        mixer
            .native_played
            .insert((request.alias.clone(), native_index));
        native_voices.push(NativeVoice {
            entity,
            alias: request.alias.clone(),
            source: request.source.key(),
            priority,
            started: mixer.clock,
            frame: mixer.native_frame,
            loaded: native.loaded,
            voice_limit: native.voice_limit,
            announcer: native.announcer,
        });
        if !native.subtitle.is_empty() {
            subtitle.text = game
                .content
                .localized_strings
                .iter()
                .find(|s| s.key == native.subtitle)
                .map_or_else(|| native.subtitle.clone(), |s| s.text.clone());
            subtitle.remaining = native.seconds.unwrap_or(5.);
            subtitle.pauseable = native.pauseable;
        }
    }
}
fn maintain(
    mut commands: Commands,
    state: Res<State<GameState>>,
    settings: Res<AudioSettings>,
    time: Res<Time<Real>>,
    game: Option<Res<GameDefinition>>,
    camera: Query<(Entity, &SpatialListener), With<crate::rendering::GameCamera>>,
    mut queries: ParamSet<(
        TransformHelper,
        Query<(
            Entity,
            &mut PlayingSound,
            &mut SemanticPlayback,
            &mut Transform,
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
            &mut PlaybackSettings,
            Option<&RetiringSound>,
            Option<&mut SoundFade>,
            Option<&SoundControl>,
        )>,
    )>,
    mut events: MessageWriter<PlaybackEvent>,
    mut lifetimes: ResMut<Lifetimes>,
    mut ducks: ResMut<NativeDucks>,
    query: avian3d::prelude::SpatialQuery,
    contents: Query<&crate::assets::WorldContents>,
    mut obstruction_frame: Local<u32>,
) {
    let Some(game) = game else { return };
    *obstruction_frame = obstruction_frame.wrapping_add(1);
    let sources: Vec<_> = queries
        .p1()
        .iter()
        .map(|(e, s, ..)| (e, s.source))
        .collect();
    let transforms = queries.p0();
    let listener_pose = camera
        .single()
        .ok()
        .and_then(|(e, _)| transforms.compute_global_transform(e).ok())
        .unwrap_or_default();
    let listener = listener_pose.translation();
    let ears = camera
        .single()
        .map_or_else(|_| SpatialListener::default(), |(_, ears)| ears.clone());
    let left_ear = listener_pose.transform_point(ears.left_ear_offset);
    let right_ear = listener_pose.transform_point(ears.right_ear_offset);
    let positions: BTreeMap<_, _> = sources
        .into_iter()
        .map(|(e, source)| (e, source.position(&transforms)))
        .collect();
    let voices = queries
        .p1()
        .iter()
        .filter_map(
            |(entity, sound, semantic, pose, regular, spatial, playback, _, _, _)| {
                let native = &game.audio.aliases[&sound.alias].native[sound.native];
                let profile = native.duck?;
                if !native.spatial && game.audio.ducks[&profile].distance > 1.52879e-05 {
                    return None;
                }
                Some(DuckVoice {
                    entity,
                    profile,
                    position: positions[&entity].unwrap_or(pose.translation),
                    seconds: native.seconds.unwrap_or(0.) / pitch_speed(sound.speed),
                    looping: sound.looping,
                    playing: (regular.is_some() || spatial.is_some())
                        && !semantic.completed
                        && sound.delay <= time.delta_secs(),
                    paused: (**state == GameState::Paused && native.pauseable)
                        || playback.paused && sound.delay > time.delta_secs(),
                })
            },
        )
        .collect::<Vec<_>>();
    ducks.update(
        &game.audio.ducks,
        &voices,
        listener,
        time.delta_secs(),
        **state == GameState::Paused,
    );
    let mut sounds = queries.p1();
    for (
        entity,
        mut sound,
        mut semantic,
        mut pose,
        regular,
        spatial,
        mut playback,
        retiring,
        fade,
        control,
    ) in &mut sounds
    {
        let native = &game.audio.aliases[&sound.alias].native[sound.native];
        let paused = **state == GameState::Paused && native.pauseable;
        if !paused {
            sound.delay = (sound.delay - time.delta_secs()).max(0.);
        }
        playback.paused = paused || sound.delay > 0.;
        if let Some(position) = positions[&entity] {
            pose.translation = position;
            if !paused && matches!(sound.source, SoundSource::World(_)) {
                pose.translation += sound.velocity * time.delta_secs();
                sound.source = SoundSource::World(pose.translation);
            }
        } else {
            sound.source = SoundSource::World(pose.translation);
            if sound.looping || native.stop_on_death {
                let duration = native.fades[1].max(0.) * native.fade_in(sound.age);
                sound.stop_remaining.get_or_insert(duration);
            }
        }
        let falloff = if native.spatial {
            native.attenuation(
                listener.distance(pose.translation),
                sound.distance_min,
                sound.distance,
            )
        } else {
            1.
        };
        sound.gain = settings.gain(sound.channel).powi(2);
        sound.gain *= ducks.gain(native.duck_group);
        if native.spatial && native.obstruction > 0. && !playback.paused {
            if !semantic.accepted
                || (entity.to_bits() as u32).wrapping_add(*obstruction_frame) & 3 == 0
            {
                semantic.obstruction_target =
                    obstruction::amount(listener, pose.translation, |start, end| {
                        let delta = end - start;
                        let Ok(direction) = Dir3::new(delta) else {
                            return None;
                        };
                        query
                            .cast_ray_predicate(
                                start,
                                direction,
                                delta.length(),
                                true,
                                &avian3d::prelude::SpatialQueryFilter::default(),
                                &|entity| {
                                    contents
                                        .get(entity)
                                        .is_ok_and(|contents| contents.0 & 0x31 != 0)
                                },
                            )
                            .map(|hit| hit.distance / delta.length())
                    });
                if !semantic.accepted {
                    semantic.obstruction = semantic.obstruction_target;
                }
            }
            semantic.obstruction += (semantic.obstruction_target - semantic.obstruction)
                .clamp(-time.delta_secs() * 6., time.delta_secs() * 6.);
        }
        if let Some(filter) = &semantic.filter {
            let mut spatial_filter = native.distance_filter(listener.distance(pose.translation));
            if native.spatial {
                let mut gain = (spatial_filter - 1.) * 35.;
                gain = gain.min(-35. * native.obstruction * semantic.obstruction);
                if native.large && semantic.obstruction < 1. {
                    gain *= (listener.distance(pose.translation) / (1000. * 0.0254)).clamp(0., 1.);
                }
                spatial_filter = 1. + gain / 35.;
            }
            filter.store(
                ducks.filter[native.duck_group as usize]
                    .min(spatial_filter)
                    .to_bits(),
                Ordering::Relaxed,
            );
        }
        sound.priority =
            native.priority_distance(listener.distance(pose.translation), sound.distance_min);
        if (regular.is_some() || spatial.is_some())
            && !playback.paused
            && sound.stop_remaining.is_none()
        {
            sound.age += time.delta_secs();
        }
        if retiring.is_some() {
            let duration = native.fades[1].max(0.) * native.fade_in(sound.age);
            sound.stop_remaining.get_or_insert(duration);
        }
        if let Some(mut remaining) = sound.stop_remaining {
            if !paused {
                remaining = (remaining - time.delta_secs()).max(0.);
            }
            sound.stop_remaining = Some(remaining);
            if remaining == 0. {
                commands.entity(entity).try_despawn();
                continue;
            }
            sound.gain *= (remaining / native.fades[1]).clamp(0., 1.).powi(2);
        } else {
            sound.gain *= native.fade_in(sound.age).powi(2);
        }
        if let Some(mut fade) = fade {
            if !paused {
                fade.remaining = (fade.remaining - time.delta_secs()).max(0.);
            }
            let fraction = (fade.remaining / fade.duration.max(0.001)).clamp(0., 1.);
            let gain = if fade.out { fraction } else { 1. - fraction };
            sound.gain *= gain.powi(2);
            if fade.remaining == 0. {
                if fade.out {
                    commands.entity(entity).try_despawn();
                    continue;
                }
                commands.entity(entity).remove::<SoundFade>();
            }
        }
        if let Some(wet) = &semantic.wet {
            let distance = if native.spatial {
                listener.distance(pose.translation)
            } else {
                0.
            };
            let [minimum, maximum] = native.wet_distance;
            let gain = if distance < minimum {
                native.reverb_curves[1].sample((distance / minimum).clamp(0., 1.))
            } else {
                native.reverb_curves[0]
                    .sample(((distance - minimum) / (maximum - minimum)).clamp(0., 1.))
            };
            let source_gain =
                (sound.base * sound.gain * control.map_or(1., |control| control.volume))
                    .clamp(0., 1.);
            wet.store(
                (source_gain * native.reverb_send / 100.
                    * gain
                    * (1.
                        - semantic.obstruction
                            * (1. - (0.75 + native.obstruction * 0.5).clamp(0., 1.))))
                .to_bits(),
                Ordering::Relaxed,
            );
        }
        let has_sink = regular.is_some() || spatial.is_some();
        if has_sink && !playback.paused && !semantic.accepted {
            semantic.accepted = true;
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Accepted,
            });
        }
        let empty = regular.as_ref().is_some_and(|s| s.empty())
            || spatial.as_ref().is_some_and(|s| s.empty());
        let dry_done = semantic
            .dry_done
            .as_ref()
            .is_some_and(|done| done.load(Ordering::Acquire));
        if !sound.looping && !semantic.completed && (dry_done || empty) {
            semantic.completed = true;
            lifetimes.0.insert(entity, true);
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Completed,
            });
        }
        if empty && !sound.looping {
            commands.entity(entity).try_despawn();
            continue;
        }
        let source_volume = sound.base * sound.gain * control.map_or(1., |control| control.volume);
        let volume = Volume::Linear(
            source_volume.clamp(0., 1.)
                * falloff
                * ((1. - semantic.obstruction) * (1.25 - native.obstruction * 0.5).clamp(0., 1.)
                    + semantic.obstruction)
                * game
                    .audio
                    .master
                    .as_ref()
                    .map_or(1., |master| master.buses[native.bus as usize]),
        );
        playback.volume = volume;
        playback.speed = pitch_speed(sound.speed * control.map_or(1., |control| control.pitch));
        if let Some(voice) = &semantic.voice {
            voice
                .dry
                .store(volume.to_linear().to_bits(), Ordering::Relaxed);
            if voice.positioned {
                let scale = 1. / (sound.distance + 0.2);
                let left_squared = left_ear.distance_squared(pose.translation) * scale * scale;
                let right_squared = right_ear.distance_squared(pose.translation) * scale * scale;
                let maximum_difference = left_ear.distance(right_ear) * scale;
                for (channel, (near, far)) in
                    [(left_squared, right_squared), (right_squared, left_squared)]
                        .into_iter()
                        .enumerate()
                {
                    let pan =
                        (((near.sqrt() - far.sqrt()) / maximum_difference + 1.) / 4. + 0.5).min(1.);
                    voice.spatial[channel]
                        .store((pan * (1. / near).min(1.)).to_bits(), Ordering::Relaxed);
                }
            }
            voice
                .speed
                .store(playback.speed.to_bits(), Ordering::Relaxed);
            voice.paused.store(playback.paused, Ordering::Relaxed);
            playback.speed = 1.;
        }
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
            sink.set_speed(playback.speed);
            if playback.paused && !sink.is_paused() {
                sink.pause();
            }
            if !playback.paused && sink.is_paused() {
                sink.play();
            }
        }
    }
}
pub(crate) fn stop(
    mut commands: Commands,
    sounds: Query<Entity, Or<(With<PlayingSound>, With<SoundRequest>)>>,
    mut mixer: ResMut<Mixer>,
    mut subtitle: ResMut<Subtitle>,
    mut reservation: ResMut<StoryReservation>,
    mut music: ResMut<MusicOverride>,
    mut ducks: ResMut<NativeDucks>,
    mut room: ResMut<reverb::ListenerRoom>,
    buses: Query<Entity, With<AudioPlayer<reverb::RoomBus>>>,
    mut lifetimes: ResMut<Lifetimes>,
    mut events: ResMut<Messages<PlaybackEvent>>,
    mut narrative: ResMut<Messages<NarrativeProgress>>,
    mut robots: ResMut<Messages<RobotPurgeReady>>,
) {
    lifetimes.0.clear();
    events.clear();
    narrative.clear();
    robots.clear();
    reservation.0 = None;
    music.0 = None;
    for e in &sounds {
        commands.entity(e).try_despawn();
    }
    for entity in &buses {
        commands.entity(entity).try_despawn();
    }
    *room = reverb::ListenerRoom::default();
    *mixer = Mixer::default();
    *ducks = NativeDucks::default();
    *subtitle = Subtitle::default();
}
fn game_over(mut commands: Commands, game: Option<Res<GameDefinition>>) {
    if game.is_some_and(|g| g.audio.aliases.contains_key("mus_zombie_game_over")) {
        emit(
            &mut commands,
            "mus_zombie_game_over",
            SoundSource::World(Vec3::ZERO),
        );
    }
}
fn subtitles(state: Res<State<GameState>>, time: Res<Time<Real>>, mut subtitle: ResMut<Subtitle>) {
    if **state != GameState::Paused || !subtitle.pauseable {
        subtitle.remaining = (subtitle.remaining - time.delta_secs()).max(0.);
    }
    if subtitle.remaining == 0. {
        subtitle.text.clear();
    }
}

fn route(
    mut commands: Commands,
    server: Res<AssetServer>,
    streams: Res<Assets<AudioSource>>,
    mut assets: ResMut<Assets<reverb::RoomClip>>,
    mut room: ResMut<reverb::ListenerRoom>,
    mut buses: ResMut<Assets<reverb::RoomBus>>,
    mut pending: Query<(
        Entity,
        &PendingRoom,
        &mut SemanticPlayback,
        &PlayingSound,
        &mut PlaybackSettings,
    )>,
    mut events: MessageWriter<PlaybackEvent>,
    game: Option<Res<GameDefinition>>,
) {
    let Some(game) = game else { return };
    for (entity, pending, mut semantic, sound, mut playback) in &mut pending {
        if matches!(
            server.get_load_state(pending.0.id()),
            Some(bevy::asset::LoadState::Failed(_))
        ) {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Cancelled,
            });
            continue;
        }
        let input = streams.get(&pending.0);
        if let Some(input) = input {
            let voice = Arc::new(reverb::VoiceControl::new(
                semantic.wet.as_ref().unwrap().clone(),
                semantic.filter.as_ref().unwrap().clone(),
                playback.speed,
                playback.paused,
                {
                    let native = &game.audio.aliases[&sound.alias].native[sound.native];
                    if native.panning {
                        [[1.; 2]; 2]
                    } else {
                        native.pan
                    }
                },
                0.,
                semantic.dry_done.as_ref().unwrap().clone(),
                playback.spatial,
            ));
            room.enqueue(input, sound.looping, voice.clone());
            if !room.started {
                let bus = buses.add(room.bus());
                commands.spawn((
                    SessionEntity,
                    AudioPlayer::<reverb::RoomBus>(bus),
                    PlaybackSettings::ONCE,
                ));
                room.started = true;
            }
            let clip = assets.add(reverb::RoomClip::new(
                semantic.dry_done.as_ref().unwrap().clone(),
                voice.clone(),
            ));
            semantic.voice = Some(voice);
            playback.speed = 1.;
            playback.spatial = false;
            commands
                .entity(entity)
                .remove::<PendingRoom>()
                .insert(AudioPlayer::<reverb::RoomClip>(clip));
        }
    }
}
fn lifecycle(
    playing: Query<Entity, With<PlayingSound>>,
    mut lifetimes: ResMut<Lifetimes>,
    mut events: MessageWriter<PlaybackEvent>,
) {
    lifetimes.0.retain(|entity, completed| {
        if playing.contains(*entity) {
            true
        } else {
            if !*completed {
                events.write(PlaybackEvent {
                    entity: *entity,
                    status: PlaybackStatus::Cancelled,
                });
            }
            false
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_duck_envelope_respects_delay_distance_partial_stop_and_pause() {
        let profile = DuckProfile {
            name: "duck".into(),
            fades: [1., 2.],
            delay: 0.5,
            distance: 10.,
            length: 0.,
            attenuation: [0.5; 32],
            filter: [1.; 32],
            update_while_paused: false,
        };
        assert_eq!(profile.amount(0.4, None, 0.), Some(0.));
        assert_eq!(profile.amount(1., None, 0.), Some(0.5));
        assert!((profile.amount(2., None, 8.).unwrap() - 0.2).abs() < 0.00001);
        assert_eq!(profile.amount(1., Some(0.75), 0.), Some(0.375));
        assert_eq!(profile.amount(3., Some(0.75), 0.), None);
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let profiles = BTreeMap::from([(1, profile)]);
        let voice = DuckVoice {
            entity,
            profile: 1,
            position: Vec3::ZERO,
            seconds: 3.,
            looping: true,
            playing: true,
            paused: false,
        };
        let mut ducks = NativeDucks::default();
        for _ in 0..20 {
            ducks.update(
                &profiles,
                std::slice::from_ref(&voice),
                Vec3::ZERO,
                0.1,
                false,
            );
        }
        assert!((ducks.gain(0) - 0.25).abs() < 0.00001);
        let age = ducks.active[&entity].age;
        ducks.update(&profiles, &[voice], Vec3::ZERO, 1., true);
        assert_eq!(ducks.active[&entity].age, age);
        ducks.update(&profiles, &[], Vec3::ZERO, 0.1, false);
        assert!(ducks.active[&entity].stopped.is_some());
    }

    #[test]
    fn native_curve_interpolates_and_handles_zero_distance_range() {
        let curve = SoundCurve([
            [0., 1.],
            [0.2, 0.8],
            [0.4, 0.6],
            [0.6, 0.4],
            [0.8, 0.2],
            [1., 0.],
            [1., 0.],
            [1., 0.],
        ]);
        assert!((curve.sample(0.3) - 0.7).abs() < 0.00001);
        assert_eq!(curve.sample(f32::NAN), 1.);
        assert_eq!(curve.sample(2.), 0.);
    }

    #[test]
    fn native_voice_priority_requires_more_than_three_and_rejects_same_frame() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let voice = NativeVoice {
            entity,
            alias: "alias".into(),
            source: 1,
            priority: 17.,
            started: 0.,
            frame: 4,
            loaded: true,
            voice_limit: false,
            announcer: false,
        };
        let voices = [&voice];
        let priority = NativeLimit {
            count: 1,
            policy: 3,
        };
        assert_eq!(native_limit(priority, &voices, 20., None), Err(()));
        assert_eq!(
            native_limit(priority, &voices, 20.1, None),
            Ok(Some(entity))
        );
        assert_eq!(
            native_limit(
                NativeLimit {
                    count: 8,
                    policy: 1
                },
                &voices,
                99.,
                Some(4)
            ),
            Err(())
        );
        assert_eq!(
            native_limit(
                NativeLimit {
                    count: 0,
                    policy: 0
                },
                &voices,
                0.,
                Some(4)
            ),
            Ok(None)
        );
    }

    #[test]
    fn native_entity_random_cache_reuses_keys_and_expires_after_thirty_seconds() {
        let mut mixer = Mixer::default();
        let first = mixer.entity_fraction(1);
        let second = mixer.entity_fraction(2);
        assert_ne!(first, second);
        assert_eq!(mixer.entity_fraction(1), first);
        mixer.clock = 30.;
        assert_eq!(mixer.entity_fraction(1), first);
        mixer.clock = 61.;
        mixer.entity_fraction(3);
        assert_eq!(mixer.entity_random.iter().flatten().count(), 1);
    }

    #[test]
    fn native_pitch_clamps_the_combined_speed_and_snaps_near_one() {
        assert_eq!(pitch_speed(0.), 0.01);
        assert_eq!(pitch_speed(1.5 * 1.6), 1.9);
        assert_eq!(pitch_speed(1.000001), 1.);
    }

    #[test]
    fn native_water_contact_traces_convex_volumes_and_surface_triangles() {
        let mut water = WaterDefinition {
            volumes: vec![vec![
                [-1., 0., 0., 1.],
                [1., 0., 0., 1.],
                [0., -1., 0., 1.],
                [0., 1., 0., 0.],
                [0., 0., -1., 1.],
                [0., 0., 1., 1.],
            ]],
            triangles: Vec::new(),
        };
        assert!(water.touches(Vec3::new(0., -0.5, 0.)));
        assert!(!water.touches(Vec3::new(0., 0.1, 0.)));
        assert!(!water.touches(Vec3::new(2., -0.5, 0.)));
        water.volumes.clear();
        water
            .triangles
            .push([[-1., 0., -1.], [0., 0., 1.], [1., 0., -1.]]);
        assert!(water.touches(Vec3::new(0., -0.5, 0.)));
        assert!(!water.touches(Vec3::new(2., -0.5, 0.)));
    }
}
