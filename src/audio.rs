use crate::{GameState, assets::GameDefinition, gameplay::SessionEntity, origins::ContentId};
use bevy::{
    audio::{AddAudioSource, AudioSinkPlayback, PlaybackMode, SpatialScale, Volume},
    prelude::*,
    transform::helper::TransformHelper,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};
mod decoded;
mod gameplay;
mod narrative;
mod reverb;
pub(crate) use decoded::AudioDecodeStats;
pub(crate) use gameplay::voice;
pub(crate) use narrative::{NarrativeProgress, RobotPurgeReady};

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
    #[serde(default)]
    pub reverb_send: f32,
    #[serde(default)]
    pub wet_distance_min: f32,
    #[serde(default)]
    pub wet_distance_max: f32,
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
                    && a.reverb_send.is_finite()
                    && (0. ..=100.).contains(&a.reverb_send)
                    && a.wet_distance_min.is_finite()
                    && a.wet_distance_min >= 0.
                    && a.wet_distance_max.is_finite()
                    && a.wet_distance_max >= a.wet_distance_min
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
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
    pub frozen_tails: u32,
    pub events: BTreeMap<String, u32>,
    pub sinks: BTreeMap<String, u32>,
}
#[cfg(all(debug_assertions, feature = "dev-tools"))]
#[derive(Resource, Default)]
pub(crate) struct RequestedAliases(pub BTreeMap<String, u32>);
#[derive(Resource, Default)]
pub struct Subtitle {
    pub text: String,
    remaining: f32,
}
#[derive(Component)]
pub struct SurfaceSound(pub String);
#[derive(Resource, Default)]
pub(crate) struct Mixer {
    sequence: u64,
    clock: f32,
    voice_cooldown: f32,
    last_voice: BTreeMap<String, f32>,
}
#[derive(Clone, Copy, Debug)]
pub enum SoundSource {
    World(Vec3),
    Entity { entity: Entity, offset: Vec3 },
}
impl SoundSource {
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
#[derive(Component)]
pub struct SoundRequest {
    alias: String,
    source: SoundSource,
    looping: bool,
}
pub fn emit(commands: &mut Commands, alias: impl Into<String>, source: SoundSource) -> Entity {
    commands
        .spawn((
            SessionEntity,
            SoundRequest {
                alias: alias.into(),
                source,
                looping: false,
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
            },
        ))
        .id()
}
#[derive(Component)]
pub(crate) struct ConversationSound(pub Entity);
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
struct Lifetimes(BTreeMap<Entity, bool>);
#[derive(Component, Default)]
struct SemanticPlayback {
    accepted: bool,
    completed: bool,
    dry_done: Option<Arc<AtomicBool>>,
    wet: Option<Arc<AtomicU32>>,
    send: f32,
    wet_min: f32,
    wet_max: f32,
}
#[derive(Component)]
enum PendingRoom {
    Cached(Handle<decoded::DecodedClip>),
    Stream(Handle<AudioSource>),
}
pub(crate) fn emit_owned(
    commands: &mut Commands,
    alias: impl Into<String>,
    source: SoundSource,
    owner: Entity,
) -> Entity {
    let entity = emit(commands, alias, source);
    commands.entity(entity).insert(ConversationSound(owner));
    entity
}
pub(crate) fn resolve_alias<'a>(game: &'a GameDefinition, name: &str) -> Option<&'a str> {
    let name = match name {
        "purchase" => "zmb_cha_ching",
        "music_chest" => "zmb_music_box",
        other => other,
    };
    if let Some((key, _)) = game.audio.aliases.get_key_value(name) {
        return Some(key);
    }
    if let Some(alias) = game.audio.bindings.get(name) {
        return Some(alias);
    }
    game.audio
        .aliases
        .iter()
        .find(|(_, a)| a.sounds.iter().any(|id| id.0 == name))
        .map(|(name, _)| name.as_str())
}
#[derive(Component)]
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
    priority: u32,
    gain: f32,
    started: f32,
}
pub(crate) fn attenuation(distance: f32, minimum: f32, maximum: f32) -> f32 {
    if distance >= maximum {
        0.
    } else if distance <= minimum {
        1.
    } else {
        (maximum - distance) / (maximum - minimum)
    }
}
#[derive(Component)]
pub(crate) struct RetiringSound;
fn speech(alias: &str) -> bool {
    if alias.starts_with("vox_maxi_audio_log_") {
        return false;
    }
    alias.starts_with("vox_plr_") || alias.starts_with("vox_sam_") || alias.starts_with("vox_maxi_")
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
            .init_resource::<StoryReservation>()
            .init_resource::<MusicOverride>()
            .init_resource::<Lifetimes>()
            .init_resource::<reverb::ListenerRoom>()
            .add_message::<PlaybackEvent>()
            .init_resource::<decoded::ClipCache>()
            .init_resource::<AudioDecodeStats>()
            .add_audio_source::<decoded::DecodedClip>()
            .add_audio_source::<reverb::RoomClip>()
            .init_asset_loader::<decoded::DecodedClipLoader>()
            .add_systems(
                Update,
                (
                    settings,
                    reverb::update,
                    decoded::maintain,
                    play,
                    route,
                    lifecycle,
                    subtitles,
                )
                    .chain()
                    .after(crate::input::pause_and_restart)
                    .in_set(AudioPlayback),
            )
            .add_systems(
                PostUpdate,
                maintain
                    .after(bevy::app::AnimationSystems)
                    .after(crate::presentation::CameraPose)
                    .before(bevy::transform::TransformSystems::Propagate)
                    .in_set(AudioPlayback),
            )
            .add_systems(OnEnter(GameState::GameOver), (stop, game_over).chain())
            .add_systems(OnEnter(GameState::Loading), stop);
        app.add_systems(OnEnter(GameState::MainMenu), stop);
        app.add_systems(OnEnter(GameState::Ending), stop);
        gameplay::install(app);
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
    time: Res<Time>,
    requests: Query<(
        Entity,
        &SoundRequest,
        Option<&SoundFade>,
        Option<&ConversationSound>,
    )>,
    playing: Query<(Entity, &PlayingSound, &SemanticPlayback)>,
    camera: Query<Entity, With<crate::rendering::GameCamera>>,
    transforms: TransformHelper,
    mut mixer: ResMut<Mixer>,
    mut evidence: ResMut<AudioEvidence>,
    mut subtitle: ResMut<Subtitle>,
    (mut clips, mut decode_stats): (ResMut<decoded::ClipCache>, ResMut<AudioDecodeStats>),
    (reservation, music, generators, mut events, mut lifetimes): (
        Res<StoryReservation>,
        Res<MusicOverride>,
        Res<crate::generators::Generators>,
        MessageWriter<PlaybackEvent>,
        ResMut<Lifetimes>,
    ),
    #[cfg(all(debug_assertions, feature = "dev-tools"))] mut validation_requests: Option<
        ResMut<RequestedAliases>,
    >,
) {
    let Some(game) = game else {
        return;
    };
    if **state == GameState::Paused {
        return;
    }
    if !matches!(
        **state,
        GameState::Playing | GameState::GameOver | GameState::Ending | GameState::PreparingShaders
    ) {
        for (e, ..) in &requests {
            commands.entity(e).try_despawn();
        }
        return;
    }
    mixer.voice_cooldown = (mixer.voice_cooldown - time.delta_secs()).max(0.);
    mixer.clock += time.delta_secs();
    let listener = camera
        .single()
        .ok()
        .and_then(|e| transforms.compute_global_transform(e).ok())
        .map_or(Vec3::ZERO, |t| t.translation());
    let mut counts = BTreeMap::<String, usize>::new();
    let mut live = BTreeMap::<String, Vec<(Entity, f32)>>::new();
    let mut count = 0;
    let mut voice = Vec::new();
    for (e, s, semantic) in &playing {
        count += 1;
        *counts.entry(s.alias.clone()).or_default() += 1;
        live.entry(s.alias.clone())
            .or_default()
            .push((e, s.started));
        if s.channel == Channel::Dialogue && speech(&s.alias) && !semantic.completed {
            voice.push((e, s.priority));
        }
    }
    for (entity, request, fade, owner) in &requests {
        let resolved = resolve_alias(&game, &request.alias).map(str::to_owned);
        let request = SoundRequest {
            alias: resolved.unwrap_or_else(|| request.alias.clone()),
            source: request.source,
            looping: request.looping,
        };
        if **state == GameState::GameOver && request.alias != "mus_zombie_game_over" {
            commands.entity(entity).try_despawn();
            continue;
        }
        evidence.requested += 1;
        #[cfg(all(debug_assertions, feature = "dev-tools"))]
        if let Some(aliases) = validation_requests.as_mut() {
            *aliases.0.entry(request.alias.clone()).or_default() += 1;
        }
        let Some(alias) = game.audio.aliases.get(&request.alias) else {
            commands.entity(entity).try_despawn();
            evidence.rejected += 1;
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        };
        if alias.channel == Channel::Dialogue
            && !request.alias.starts_with("vox_maxi_audio_log_")
            && reservation.0.is_some()
            && reservation.0 != owner.map(|o| o.0)
        {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            evidence.rejected += 1;
            continue;
        }
        if alias.channel == Channel::Music
            && request.alias.starts_with("mus_zombie_")
            && (music.0.is_some()
                || (generators.raid_active
                    && !matches!(
                        request.alias.as_str(),
                        "mus_zombie_dog_start" | "mus_zombie_dog_end"
                    )))
        {
            commands.entity(entity).try_despawn();
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            evidence.rejected += 1;
            continue;
        }
        if alias.channel == Channel::Music && request.alias.starts_with("mus_zombie_") {
            for (e, s, _) in &playing {
                if s.alias.starts_with("mus_zombie_") {
                    commands.entity(e).try_insert(RetiringSound);
                }
            }
        }
        let concurrent = *counts.get(&request.alias).unwrap_or(&0);
        let character_speech = alias.channel == Channel::Dialogue
            && speech(&request.alias)
            && !request.alias.starts_with("vox_maxi_audio_log_");
        if concurrent >= alias.limit && character_speech {
            commands.entity(entity).try_despawn();
            evidence.rejected += 1;
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        if concurrent >= alias.limit {
            if let Some(entries) = live.get_mut(&request.alias)
                && let Some((index, _)) = entries
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.1.total_cmp(&b.1))
            {
                let (old, _) = entries.remove(index);
                commands.entity(old).try_despawn();
                count -= 1;
                *counts.get_mut(&request.alias).unwrap() -= 1;
            }
        }
        if count >= 64 {
            commands.entity(entity).try_despawn();
            evidence.rejected += 1;
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        if character_speech {
            let now = mixer.clock;
            let repeated = mixer
                .last_voice
                .get(&request.alias)
                .is_some_and(|last| now - last < 45.);
            if (owner.is_none() && repeated)
                || (owner.is_none() && mixer.voice_cooldown > 0. && alias.priority < 100)
                || voice.iter().any(|(_, p)| *p >= alias.priority)
            {
                commands.entity(entity).try_despawn();
                evidence.rejected += 1;
                events.write(PlaybackEvent {
                    entity,
                    status: PlaybackStatus::Rejected,
                });
                continue;
            }
            for (e, _) in voice.drain(..) {
                commands.entity(e).try_despawn();
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
                subtitle.text = if request.alias.starts_with("vox_plr_") {
                    "[Player dialogue]".into()
                } else {
                    "[Voice]".into()
                };
            }
            subtitle.remaining = alias.seconds;
        }
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
            commands.entity(entity).try_despawn();
            continue;
        };
        let Some(position) = request.source.position(&transforms) else {
            commands.entity(entity).try_despawn();
            evidence.rejected += 1;
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        };
        let looping = request.looping || sound.looping;
        if alias.spatial && !looping && listener.distance(position) > sound.distance {
            commands.entity(entity).try_despawn();
            evidence.rejected += 1;
            events.write(PlaybackEvent {
                entity,
                status: PlaybackStatus::Rejected,
            });
            continue;
        }
        let fraction = ((mixer.sequence >> 16) & 65535) as f32 / 65535.;
        let speed = sound.pitch[0] + (sound.pitch[1] - sound.pitch[0]) * fraction;
        let gain = if fade.is_some_and(|f| !f.out) { 0. } else { 1. }
            * settings.gain(alias.channel)
            * if alias.spatial {
                attenuation(
                    listener.distance(position),
                    sound.distance_min,
                    sound.distance,
                )
            } else {
                1.
            };
        commands.entity(entity).remove::<SoundRequest>().insert((
            PlayingSound {
                alias: request.alias.clone(),
                channel: alias.channel,
                source: request.source,
                looping,
                base: sound.volume,
                distance_min: sound.distance_min,
                distance: sound.distance,
                priority: alias.priority,
                gain,
                started: mixer.clock,
            },
            PlaybackSettings {
                mode: if looping {
                    PlaybackMode::Loop
                } else {
                    PlaybackMode::Once
                },
                spatial: alias.spatial,
                spatial_scale: Some(SpatialScale::new(1. / (sound.distance + 0.2))),
                speed,
                volume: Volume::Linear(sound.volume * gain),
                ..default()
            },
            Transform::from_translation(position),
        ));
        let mut semantic = SemanticPlayback::default();
        if alias.reverb_send > 0. {
            semantic.dry_done = Some(Arc::new(AtomicBool::new(false)));
            semantic.wet = Some(Arc::new(AtomicU32::new(
                (alias.reverb_send / 100.).to_bits(),
            )));
            semantic.send = alias.reverb_send / 100.;
            semantic.wet_min = alias.wet_distance_min;
            semantic.wet_max = alias.wet_distance_max;
            let pending = if let Some(clip) =
                clips.clip(&server, &sound.path, alias.seconds, &mut decode_stats)
            {
                PendingRoom::Cached(clip)
            } else {
                PendingRoom::Stream(server.load(sound.path.clone()))
            };
            commands.entity(entity).insert(pending);
            if looping {
                commands.entity(entity).insert(PlaybackSettings {
                    mode: PlaybackMode::Once,
                    spatial: alias.spatial,
                    spatial_scale: Some(SpatialScale::new(1. / (sound.distance + 0.2))),
                    speed,
                    volume: Volume::Linear(sound.volume * gain),
                    ..default()
                });
            }
        } else if let Some(clip) =
            clips.clip(&server, &sound.path, alias.seconds, &mut decode_stats)
        {
            commands
                .entity(entity)
                .insert(AudioPlayer::<decoded::DecodedClip>(clip));
        } else {
            commands
                .entity(entity)
                .insert(AudioPlayer::<AudioSource>::new(
                    server.load(sound.path.clone()),
                ));
        }
        commands.entity(entity).insert(semantic);
        lifetimes.0.insert(entity, false);
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
            emit(&mut commands, secondary.clone(), request.source);
        }
    }
    evidence.peak = evidence.peak.max(count);
}
fn maintain(
    mut commands: Commands,
    state: Res<State<GameState>>,
    settings: Res<AudioSettings>,
    time: Res<Time>,
    camera: Query<Entity, With<crate::rendering::GameCamera>>,
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
            Option<&crate::cinematics::CinematicAudioPaused>,
        )>,
    )>,
    mut evidence: ResMut<AudioEvidence>,
    mut heard: Local<BTreeSet<Entity>>,
    mut events: MessageWriter<PlaybackEvent>,
    mut lifetimes: ResMut<Lifetimes>,
) {
    let sources: Vec<_> = queries
        .p1()
        .iter()
        .map(|(e, s, ..)| (e, s.source))
        .collect();
    let transforms = queries.p0();
    let listener = camera
        .single()
        .ok()
        .and_then(|e| transforms.compute_global_transform(e).ok())
        .map_or(Vec3::ZERO, |t| t.translation());
    let positions: BTreeMap<_, _> = sources
        .into_iter()
        .map(|(e, source)| (e, source.position(&transforms)))
        .collect();
    let mut sounds = queries.p1();
    heard.retain(|e| sounds.contains(*e));
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
        cinematic_paused,
    ) in &mut sounds
    {
        playback.paused = **state == GameState::Paused || cinematic_paused.is_some_and(|p| p.0);
        if let Some(position) = positions[&entity] {
            pose.translation = position;
        } else if sound.looping {
            commands.entity(entity).try_despawn();
            evidence.owner_stops += 1;
            continue;
        } else {
            sound.source = SoundSource::World(pose.translation);
            evidence.frozen_tails += 1;
        }
        if (regular.is_some() || spatial.is_some()) && heard.insert(entity) {
            *evidence.sinks.entry(sound.alias.clone()).or_default() += 1;
        }
        let falloff = if playback.spatial {
            attenuation(
                listener.distance(pose.translation),
                sound.distance_min,
                sound.distance,
            )
        } else {
            1.
        };
        let target = settings.gain(sound.channel) * falloff;
        if retiring.is_some() {
            if **state != GameState::Paused {
                sound.gain *= (1. - time.delta_secs() * 5.).max(0.);
            }
        } else {
            sound.gain = target;
        }
        if let Some(mut fade) = fade {
            if **state != GameState::Paused {
                fade.remaining = (fade.remaining - time.delta_secs()).max(0.);
            }
            let fraction = (fade.remaining / fade.duration.max(0.001)).clamp(0., 1.);
            sound.gain *= if fade.out { fraction } else { 1. - fraction };
            if fade.remaining == 0. {
                if fade.out {
                    commands.entity(entity).try_despawn();
                    continue;
                }
                commands.entity(entity).remove::<SoundFade>();
            }
        }
        if retiring.is_some() && sound.gain < 0.002 {
            commands.entity(entity).try_despawn();
            continue;
        }
        if let Some(wet) = &semantic.wet {
            let distance = if playback.spatial {
                listener.distance(pose.translation)
            } else {
                0.
            };
            let gain = if semantic.wet_max > semantic.wet_min {
                attenuation(distance, semantic.wet_min, semantic.wet_max)
            } else {
                1.
            };
            wet.store((semantic.send * gain).to_bits(), Ordering::Relaxed);
        }
        let has_sink = regular.is_some() || spatial.is_some();
        if has_sink && !semantic.accepted {
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
        let volume = Volume::Linear(sound.base * sound.gain);
        playback.volume = volume;
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
            if playback.paused && !sink.is_paused() {
                sink.pause();
                evidence.paused += 1;
            }
            if !playback.paused && sink.is_paused() {
                sink.play();
                evidence.resumed += 1;
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
) {
    reservation.0 = None;
    music.0 = None;
    for e in &sounds {
        commands.entity(e).try_despawn();
    }
    *mixer = Mixer::default();
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
fn subtitles(state: Res<State<GameState>>, time: Res<Time>, mut subtitle: ResMut<Subtitle>) {
    if **state == GameState::Playing {
        subtitle.remaining = (subtitle.remaining - time.delta_secs()).max(0.);
    }
    if subtitle.remaining == 0. {
        subtitle.text.clear();
    }
}

fn route(
    mut commands: Commands,
    server: Res<AssetServer>,
    decoded: Res<Assets<decoded::DecodedClip>>,
    streams: Res<Assets<AudioSource>>,
    mut assets: ResMut<Assets<reverb::RoomClip>>,
    room: Res<reverb::ListenerRoom>,
    pending: Query<(Entity, &PendingRoom, &SemanticPlayback, &PlayingSound)>,
    mut events: MessageWriter<PlaybackEvent>,
) {
    for (entity, pending, semantic, sound) in &pending {
        let input = match pending {
            PendingRoom::Cached(handle) => {
                if let Some(clip) = decoded.get(handle) {
                    Some(reverb::Input::Cached(clip.clone()))
                } else {
                    if matches!(
                        server.get_load_state(handle.id()),
                        Some(bevy::asset::LoadState::Failed(_))
                    ) && let Some(path) = server.get_path(handle.id())
                    {
                        commands
                            .entity(entity)
                            .insert(PendingRoom::Stream(server.load(path.to_string())));
                    }
                    None
                }
            }
            PendingRoom::Stream(handle) => {
                if matches!(
                    server.get_load_state(handle.id()),
                    Some(bevy::asset::LoadState::Failed(_))
                ) {
                    commands.entity(entity).try_despawn();
                    events.write(PlaybackEvent {
                        entity,
                        status: PlaybackStatus::Cancelled,
                    });
                }
                streams
                    .get(handle)
                    .map(|source| reverb::Input::Stream(source.clone()))
            }
        };
        if let Some(input) = input {
            let clip = assets.add(reverb::RoomClip::new(
                input,
                &room,
                semantic.wet.as_ref().unwrap().clone(),
                semantic.dry_done.as_ref().unwrap().clone(),
                sound.looping,
            ));
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
