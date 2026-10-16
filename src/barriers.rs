use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    origins::{ContentId, PlacementId},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Deserialize)]
pub struct BarrierDefinition {
    pub id: ContentId,
    pub placement: PlacementId,
    pub entry: ContentId,
    pub exit: ContentId,
    pub initially_open: bool,
    pub attack_spacing: f32,
    pub attack_yaw: f32,
    pub boards: Vec<BoardDefinition>,
    pub audio: BarrierAudioDefinition,
    pub repair_quake: Option<RepairQuakeDefinition>,
    pub repair_origin: [f32; 3],
    pub repair_yaw: f32,
    pub repair_radius: f32,
    pub repair_height: f32,
    pub repair_interval: f32,
    pub repair_points: u32,
    pub taunt: Option<BarrierActorAction>,
    pub reach_through: Option<BarrierActorAction>,
    pub enter: Option<BarrierEntryDefinition>,
}
#[derive(Clone, Deserialize)]
pub struct BarrierEntryDefinition {
    pub standing: BTreeMap<String, Vec<BarrierActorClip>>,
    pub crawler: BTreeMap<String, Vec<BarrierActorClip>>,
}
#[derive(Clone, Deserialize)]
pub struct BarrierActorAction {
    pub state: String,
    pub frequency: u32,
    pub range: f32,
    pub clips: Vec<BarrierActorClip>,
}
impl BarrierActorAction {
    pub(crate) fn reach_clip(&self, slot: usize, random: u32) -> Option<&BarrierActorClip> {
        if slot == 0 {
            self.clips.get(random as usize % self.clips.len().max(1))
        } else {
            self.clips.get(slot - 1)
        }
    }
    pub(crate) fn hits_through(&self, player: Vec3, actor: Vec3, trigger: Vec3) -> bool {
        let actor_delta = player - actor;
        let trigger_delta = player - trigger;
        actor_delta.xz().length_squared() < self.range.powi(2)
            && actor_delta.y.abs() < self.range
            && trigger_delta.xz().length_squared() < (51. * 0.0254_f32).powi(2)
            && trigger_delta.y.abs() < 51. * 0.0254
    }
}
#[derive(Clone, Deserialize)]
pub struct BarrierActorClip {
    pub name: String,
    pub seconds: f32,
    pub duration: f32,
    pub hits: Vec<f32>,
    pub motion: Option<crate::origins::content::AnimationRootMotion>,
}
impl BarrierDefinition {
    pub(crate) fn entry_clip(
        &self,
        gait: crate::rounds::Gait,
        slot: usize,
        crawler: bool,
        random: u32,
    ) -> Option<&BarrierActorClip> {
        let entry = self.enter.as_ref()?;
        let gait = match gait {
            crate::rounds::Gait::Walk => "walk",
            crate::rounds::Gait::Run => "run",
            crate::rounds::Gait::Sprint => "sprint",
        };
        let slot = ["m", "r", "l"].get(slot)?;
        let clips = if crawler {
            &entry.crawler
        } else {
            &entry.standing
        };
        let clips = clips.get(&format!("barrier_{gait}_{slot}"))?;
        clips.get(random as usize % clips.len().max(1))
    }
}
#[derive(Clone, Deserialize)]
pub struct BoardDefinition {
    pub presentation: ContentId,
    pub repair: ContentId,
    pub pull: ContentId,
    pub repair_seconds: f32,
    pub pull_seconds: f32,
    pub collision: Vec<crate::environment::brushes::TriangleShape>,
    pub attacks: [BoardAttackDefinition; 3],
    pub audio: BoardAudioDefinition,
    pub repair_effects: [Option<BoardRepairEffect>; 2],
}

#[derive(Clone, Deserialize)]
pub struct BoardRepairEffect {
    pub name: String,
    pub offset: [f32; 3],
}

#[derive(Clone, Deserialize)]
pub struct RepairQuakeDefinition {
    pub scale: [f32; 2],
    pub duration: [f32; 2],
    pub radius: f32,
}

#[derive(Clone, Deserialize)]
pub struct BarrierAudioDefinition {
    pub repair: [Option<String>; 2],
    pub repair_delay: Option<f32>,
}

#[derive(Clone, Deserialize)]
pub struct BoardAudioDefinition {
    pub opening: Option<String>,
    pub hover: Option<String>,
    pub repeat_pause: Option<[f32; 2]>,
    pub effect_delay: [f32; 2],
}
impl BoardDefinition {
    pub(crate) fn attack(&self, slot: usize, crawler: bool) -> &BoardAttackDefinition {
        let attack = &self.attacks[slot];
        if crawler {
            attack.crawler.as_deref().unwrap_or(attack)
        } else {
            attack
        }
    }
    pub fn collider(&self) -> Option<Collider> {
        crate::environment::brushes::compound(&[], &self.collision)
    }
}
#[derive(Clone, Deserialize)]
pub struct BoardAttackDefinition {
    pub grab_seconds: f32,
    pub tear_seconds: f32,
    pub destroy_seconds: f32,
    pub grab_clip: String,
    pub tear_clip: String,
    pub grab_motion: crate::origins::content::AnimationRootMotion,
    pub tear_motion: crate::origins::content::AnimationRootMotion,
    pub crawler: Option<Box<BoardAttackDefinition>>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoardState {
    Closed,
    Reserved(Entity),
    Pulling(Entity, f32),
    Opening(f32),
    Missing,
    Closing(f32),
}
#[derive(Resource, Default)]
pub struct Barriers {
    pub windows: BTreeMap<ContentId, Vec<BoardState>>,
    pub repaired: u32,
    pub destroyed: u32,
    pub entries: u32,
    pub sounds: u32,
    pub playing_sounds: u32,
    pub animated_boards: u32,
    pub awarded: u32,
    pub reward_counter: u32,
    owners: BTreeMap<ContentId, [Option<Entity>; 3]>,
    attack_spots: BTreeMap<ContentId, [Vec3; 3]>,
    repairers: BTreeMap<(ContentId, usize), (Entity, f32)>,
    repair_cooldowns: BTreeMap<ContentId, f32>,
    audio: BTreeMap<(ContentId, usize), BoardAudioPlayback>,
    audio_random: Option<u32>,
    quakes: RepairQuakes,
    round: u32,
    use_poll: f32,
    use_ticks: u32,
    use_started: Option<u32>,
    use_active: bool,
}
impl Barriers {
    pub(crate) fn attack_spot(&self, id: &ContentId, slot: usize) -> Option<Vec3> {
        self.attack_spots.get(id)?.get(slot).copied()
    }
    pub fn update_use(&mut self, held: bool, delta: f32) -> bool {
        self.use_poll += delta;
        while self.use_poll >= 0.05 {
            self.use_poll -= 0.05;
            self.use_ticks = self.use_ticks.wrapping_add(1);
            if self.use_active {
                if !held {
                    self.use_active = false;
                }
            } else if held {
                let started = *self.use_started.get_or_insert(self.use_ticks);
                self.use_active = self.use_ticks.wrapping_sub(started) > 5;
            } else {
                self.use_started = None;
            }
        }
        self.use_active
    }
    pub fn use_progress(&self) -> f32 {
        self.use_started.map_or(0., |started| {
            self.use_ticks.wrapping_sub(started) as f32 * 0.05
        })
    }
    pub fn repairable(&self, id: &ContentId) -> bool {
        self.windows
            .get(id)
            .is_some_and(|b| b.contains(&BoardState::Missing))
    }
    pub fn repair(
        &mut self,
        id: &ContentId,
        repairer: Entity,
        session: &mut Session,
        interval: f32,
    ) -> Option<usize> {
        if self
            .repair_cooldowns
            .get(id)
            .is_some_and(|seconds| *seconds > 0.)
        {
            return None;
        }
        let boards = self.windows.get_mut(id)?;
        let available = boards
            .iter()
            .enumerate()
            .filter_map(|(index, state)| (*state == BoardState::Missing).then_some(index))
            .collect::<Vec<_>>();
        if available.is_empty() {
            return None;
        }
        let index = available[session.random() as usize % available.len()];
        boards[index] = BoardState::Closing(0.);
        self.repairers.insert((id.clone(), index), (repairer, 0.));
        self.repair_cooldowns.insert(id.clone(), interval);
        Some(index)
    }
    pub fn obstructed(&self, id: &ContentId) -> bool {
        self.windows.get(id).is_some_and(|boards| {
            boards
                .iter()
                .any(|state| !matches!(state, BoardState::Missing | BoardState::Opening(_)))
        })
    }
    pub fn slot(&mut self, id: &ContentId, owner: Entity, random: u32) -> Option<usize> {
        let slots = self.owners.entry(id.clone()).or_insert([None; 3]);
        if let Some(slot) = slots.iter().position(|slot| *slot == Some(owner)) {
            return Some(slot);
        }
        let available = slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.is_none().then_some(index))
            .collect::<Vec<_>>();
        let slot = *available.get((random as usize) % available.len().max(1))?;
        slots[slot] = Some(owner);
        Some(slot)
    }
    pub fn claim(&mut self, id: &ContentId, owner: Entity, random: u32) -> Option<(usize, usize)> {
        let slot = self.slot(id, owner, random)?;
        let boards = self.windows.get_mut(id)?;
        let closed = boards
            .iter()
            .enumerate()
            .filter_map(|(index, state)| (*state == BoardState::Closed).then_some(index))
            .collect::<Vec<_>>();
        if closed.is_empty() {
            return None;
        }
        let i = closed[(random as usize / 3) % closed.len()];
        boards[i] = BoardState::Reserved(owner);
        Some((i, slot))
    }
    pub fn destroy(&mut self, id: &ContentId, index: usize, owner: Entity) -> bool {
        let Some(b) = self.windows.get_mut(id).and_then(|b| b.get_mut(index)) else {
            return false;
        };
        if !matches!(*b,BoardState::Reserved(e)|BoardState::Pulling(e,_) if e == owner) {
            return false;
        }
        *b = BoardState::Opening(0.);
        self.destroyed += 1;
        if !self.obstructed(id) {
            self.owners.remove(id);
        }
        true
    }
    pub fn begin_pull(&mut self, id: &ContentId, index: usize, owner: Entity) {
        if let Some(b) = self.windows.get_mut(id).and_then(|b| b.get_mut(index))
            && *b == BoardState::Reserved(owner)
        {
            *b = BoardState::Pulling(owner, 0.);
        }
    }
    pub fn release(&mut self, owner: Entity) {
        for slots in self.owners.values_mut() {
            for slot in slots {
                if *slot == Some(owner) {
                    *slot = None;
                }
            }
        }
        for boards in self.windows.values_mut() {
            for b in boards {
                if matches!(*b,BoardState::Reserved(e)|BoardState::Pulling(e,_) if e==owner) {
                    *b = BoardState::Closed;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
enum AudioBoardState {
    #[default]
    Closed,
    Opening,
    Open,
    Closing,
}

#[derive(Clone, Copy)]
enum BoardSoundSequence {
    Opening,
    Pause,
    WaitingRepeat(u64),
    Repeat,
    Repair,
    WaitingRepair(u64),
    RepairSecond,
    RepairEffectPause,
    WaitingRepairEffect(u64),
    RepairEffectSecond,
}

#[derive(Default)]
struct BoardAudioPlayback {
    previous: AudioBoardState,
    sequence: Option<BoardSoundSequence>,
    effect_phase: Option<usize>,
}

impl BoardAudioPlayback {
    fn advance<'a>(
        &mut self,
        state: BoardState,
        general: &'a BarrierAudioDefinition,
        board: &'a BoardAudioDefinition,
        now: u64,
        random: impl FnOnce() -> u32,
    ) -> Vec<&'a str> {
        self.effect_phase = None;
        let state = match state {
            BoardState::Opening(_) => AudioBoardState::Opening,
            BoardState::Missing => AudioBoardState::Open,
            BoardState::Closing(_) => AudioBoardState::Closing,
            _ => AudioBoardState::Closed,
        };
        let mut sounds = Vec::new();
        if state != self.previous {
            match (state, self.previous) {
                (AudioBoardState::Opening, _) => self.sequence = Some(BoardSoundSequence::Opening),
                (AudioBoardState::Closing, AudioBoardState::Open) => {
                    sounds.extend(board.hover.as_deref());
                }
                (AudioBoardState::Closed, AudioBoardState::Closing) => {
                    self.sequence = Some(BoardSoundSequence::Repair);
                }
                _ => {}
            }
            self.previous = state;
        }
        self.sequence = match self.sequence {
            Some(BoardSoundSequence::Opening) => {
                sounds.extend(board.opening.as_deref());
                board.repeat_pause.map(|_| BoardSoundSequence::Pause)
            }
            Some(BoardSoundSequence::Pause) => board.repeat_pause.map(|[min, max]| {
                let seconds = min as f64
                    + (random() & 32767) as f64
                        * (max as f64 - min as f64)
                        * 3.051851e-05_f32 as f64;
                BoardSoundSequence::WaitingRepeat(now + (seconds * 1000.) as u64)
            }),
            Some(BoardSoundSequence::WaitingRepeat(deadline)) if now > deadline => {
                Some(BoardSoundSequence::Repeat)
            }
            Some(BoardSoundSequence::Repeat) => {
                sounds.extend(board.opening.as_deref());
                None
            }
            Some(BoardSoundSequence::Repair) => {
                sounds.extend(general.repair[0].as_deref());
                Some(
                    general
                        .repair_delay
                        .map_or(BoardSoundSequence::RepairSecond, |seconds| {
                            BoardSoundSequence::WaitingRepair(now + (seconds as f64 * 1000.) as u64)
                        }),
                )
            }
            Some(BoardSoundSequence::WaitingRepair(deadline)) if now > deadline => {
                Some(BoardSoundSequence::RepairSecond)
            }
            Some(BoardSoundSequence::RepairSecond) => {
                sounds.extend(general.repair[1].as_deref());
                self.effect_phase = Some(0);
                Some(BoardSoundSequence::RepairEffectPause)
            }
            Some(BoardSoundSequence::RepairEffectPause) => {
                let [min, max] = board.effect_delay;
                let seconds = min as f64
                    + (random() & 32767) as f64
                        * (max as f64 - min as f64)
                        * 3.051851e-05_f32 as f64;
                Some(BoardSoundSequence::WaitingRepairEffect(
                    now + (seconds * 1000.) as u64,
                ))
            }
            Some(BoardSoundSequence::WaitingRepairEffect(deadline)) if now > deadline => {
                Some(BoardSoundSequence::RepairEffectSecond)
            }
            Some(BoardSoundSequence::RepairEffectSecond) => {
                self.effect_phase = Some(1);
                None
            }
            waiting => waiting,
        };
        sounds
    }
}

fn native_random(seed: &mut Option<u32>) -> u32 {
    let next = seed
        .unwrap_or(1)
        .wrapping_mul(0x343fd)
        .wrapping_add(0x269ec3);
    *seed = Some(next);
    next >> 16 & 32767
}

#[derive(Clone, Copy)]
struct RepairQuake {
    start: i32,
    duration: i32,
    scale: f32,
    radius: f32,
    origin: Vec3,
}

impl RepairQuake {
    fn gains(self, now: i32, eye: Vec3) -> Option<(f32, f32)> {
        let elapsed = now.checked_sub(self.start)?;
        if elapsed < 0 || elapsed >= self.duration {
            return None;
        }
        let life = (1. - elapsed as f32 / self.duration as f32) * self.scale;
        if life <= 0. {
            return None;
        }
        let distance = if self.radius > 0. {
            1. - self.origin.distance(eye) / self.radius
        } else {
            1.
        };
        Some((
            if distance < 0. {
                distance / life
            } else {
                life * distance
            },
            life,
        ))
    }
}

#[derive(Default)]
struct RepairQuakes {
    slots: [Option<RepairQuake>; 4],
    pending: Vec<RepairQuake>,
    phase: f32,
    idle_time: Option<i32>,
}

impl RepairQuakes {
    fn insert(&mut self, quake: RepairQuake, now: i32, eye: Vec3) {
        let free = self.slots.iter().position(|slot| {
            slot.is_none_or(|slot| {
                now < slot.start || now as f32 >= slot.start as f32 + slot.duration as f32
            })
        });
        let replace = free.or_else(|| {
            let mut lowest = 0.;
            let mut replace = None;
            for (index, slot) in self.slots.iter().enumerate() {
                if let Some((gain, _)) = slot.and_then(|slot| slot.gains(now, eye))
                    && gain < lowest
                {
                    lowest = gain;
                    replace = Some(index);
                }
            }
            replace
        });
        if let Some(index) = replace {
            self.slots[index] = Some(quake);
        }
    }

    fn angles(&self, now: i32, eye: Vec3) -> Option<Vec3> {
        let mut strongest = (0., 0.);
        for slot in self.slots.iter().flatten() {
            if let Some((gain, life)) = slot.gains(now, eye)
                && gain > strongest.0
            {
                strongest = (gain, life);
            }
        }
        let (gain, life) = strongest;
        if gain <= 0. {
            return None;
        }
        let time = now as f32 * 0.0016666667;
        let amplitude = gain.min(1.) * life;
        Some(
            Vec3::new(
                (time * 25.132742 + self.phase).sin() * 18. * amplitude,
                (time * 47.12389 + self.phase).sin() * 16. * amplitude,
                (time * 37.699112 + self.phase).sin() * 10. * amplitude,
            ) * (std::f32::consts::PI / 180.),
        )
    }
}

fn repair_quake_camera(
    state: Res<State<GameState>>,
    clock: Res<crate::session::SessionClock>,
    time: Res<Time<Virtual>>,
    mut barriers: ResMut<Barriers>,
    mut cameras: Query<&mut Transform, With<crate::rendering::GameCamera>>,
) {
    if !matches!(
        **state,
        GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let now = clock.milliseconds(&time);
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    let barriers = &mut *barriers;
    let pending = std::mem::take(&mut barriers.quakes.pending);
    for quake in pending {
        barriers.quakes.insert(quake, now, camera.translation);
    }
    if let Some(angles) = barriers.quakes.angles(now, camera.translation) {
        let (yaw, pitch, roll) = camera.rotation.to_euler(EulerRot::YXZ);
        camera.rotation = Quat::from_euler(
            EulerRot::YXZ,
            yaw + angles.y,
            pitch - angles.x,
            roll - angles.z,
        );
    } else if barriers.quakes.idle_time != Some(now) {
        barriers.quakes.phase =
            (native_random(&mut barriers.audio_random) as f32 * 0.000030517578 * 2. - 1.)
                * std::f32::consts::PI;
        barriers.quakes.idle_time = Some(now);
    }
}

#[derive(Component)]
pub struct BarrierBlocker(pub ContentId, pub usize);
#[derive(Component)]
pub struct BarrierRepairTrigger(pub ContentId);
pub const REPAIR_TRIGGER_LAYER: u32 = 32;
#[derive(Component)]
#[require(crate::animation::SceneClips)]
pub struct BoardVisual {
    window: usize,
    index: usize,
    base: Transform,
    clips: [usize; 2],
    _handles: [Handle<AnimationClip>; 2],
}
#[derive(Component)]
struct WoodAudio;
pub struct BarrierPlugin;
impl Plugin for BarrierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Barriers>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                assemble.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                tick.in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, (animate, audio_evidence))
            .add_systems(
                Update,
                audio.after(animate).run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                repair_quake_camera
                    .after(crate::animation::AnimationPoseModifiers)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}
fn assemble(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    players: Query<(), With<Player>>,
    server: Res<AssetServer>,
    mut barriers: ResMut<Barriers>,
    query: SpatialQuery,

    session: Res<Session>,
) {
    if !players.is_empty() {
        return;
    }
    *barriers = Barriers::default();
    barriers.round = session.round;
    for (window, d) in definition.content.barriers.iter().enumerate() {
        let Some(p) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        barriers.windows.insert(
            d.id.clone(),
            vec![
                if d.initially_open {
                    BoardState::Missing
                } else {
                    BoardState::Closed
                };
                d.boards.len()
            ],
        );
        let origin = Vec3::from_array(p.position);
        let rotation = Quat::from_rotation_y(p.yaw);
        let right = Quat::from_rotation_y(d.attack_yaw) * Vec3::Z;
        let mut spots = [Vec3::ZERO; 3];
        for slot in 0..3 {
            let source = if slot == 0 {
                origin
            } else {
                spots[0] + right * d.attack_spacing * if slot == 1 { 1. } else { -1. }
            } + Vec3::Y * (60. * 0.0254);
            let Some(floor) = query.cast_ray(
                source,
                Dir3::NEG_Y,
                100000. * 0.0254,
                false,
                &crate::environment::actor_solids([]),
            ) else {
                panic!(
                    "{}",
                    format!(
                        "Source barrier {} attack slot {slot} has no loaded ground collision",
                        d.id.0
                    )
                );
            };
            spots[slot] = source - Vec3::Y * floor.distance;
        }
        barriers.attack_spots.insert(d.id.clone(), spots);
        let repair_origin = Vec3::from_array(d.repair_origin);
        let Some(floor) = query.cast_ray(
            repair_origin,
            Dir3::NEG_Y,
            100000. * 0.0254,
            false,
            &crate::environment::player_solids([]),
        ) else {
            panic!(
                "{}",
                format!(
                    "Source barrier {} repair origin has no loaded ground collision",
                    d.id.0
                )
            );
        };
        let repair_origin = repair_origin - Vec3::Y * floor.distance + Vec3::Y * (4. * 0.0254)
            - Quat::from_rotation_y(d.repair_yaw) * Vec3::NEG_Z * (11. * 0.0254);
        commands.spawn((
            SessionEntity,
            BarrierRepairTrigger(d.id.clone()),
            RigidBody::Static,
            Sensor,
            CollisionLayers::new(REPAIR_TRIGGER_LAYER, LayerMask::NONE),
            Collider::cylinder(d.repair_radius, d.repair_height),
            Transform::from_translation(repair_origin + Vec3::Y * (d.repair_height * 0.5)),
        ));
        for (index, b) in d.boards.iter().enumerate() {
            for collision in &b.collision {
                let Some(shape) =
                    crate::environment::brushes::compound(&[], std::slice::from_ref(collision))
                else {
                    panic!(
                        "{}",
                        format!(
                            "Source barrier {} board {index} collision is unusable",
                            d.id.0
                        )
                    );
                };
                let mut blocker = commands.spawn((
                    SessionEntity,
                    BarrierBlocker(d.id.clone(), index),
                    RigidBody::Static,
                    crate::assets::WorldCollider,
                    crate::assets::WorldContents(collision.contents),
                    crate::movement::physics::SurfaceFlags(collision.surface_flags),
                    shape,
                    Transform::from_translation(origin).with_rotation(rotation),
                ));
                if d.initially_open {
                    blocker.insert(ColliderDisabled);
                }
            }
            let Some(scene) = definition
                .content
                .presentations
                .iter()
                .find(|p| p.id == b.presentation)
                .and_then(|p| p.scene.as_ref())
            else {
                continue;
            };
            let clips = [&b.repair, &b.pull].map(|id| {
                definition
                    .content
                    .animations
                    .iter()
                    .position(|a| a.id == *id)
                    .expect("Required source board animation")
            });
            commands.spawn((
                SessionEntity,
                BoardVisual {
                    window,
                    index,
                    base: Transform::from_translation(origin).with_rotation(rotation),
                    clips,
                    _handles: clips
                        .map(|clip| server.load(definition.content.animations[clip].path.clone())),
                },
                WorldAssetRoot(
                    server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                        })
                        .load(scene.clone()),
                ),
                Transform::from_translation(origin).with_rotation(rotation),
                if d.initially_open {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
            ));
        }
    }
}
fn audio_evidence(
    sinks: Query<
        (),
        (
            With<WoodAudio>,
            Or<(
                Added<bevy::audio::SpatialAudioSink>,
                Added<bevy::audio::AudioSink>,
            )>,
        ),
    >,
    mut barriers: ResMut<Barriers>,
) {
    barriers.playing_sounds += sinks.iter().count() as u32;
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut barriers: ResMut<Barriers>,
    mut session: ResMut<Session>,
    effects: Res<crate::powerups::PowerupEffects>,
    blockers: Query<(Entity, &BarrierBlocker, Option<&ColliderDisabled>)>,
    enemies: Query<&crate::life::Vitality, With<crate::enemies::Zombie>>,
    players: Query<&crate::life::Vitality, With<Player>>,
    session_id: Res<crate::session::SessionId>,
    mut rounds: MessageReader<crate::rounds::RoundStarted>,
) {
    for seconds in barriers.repair_cooldowns.values_mut() {
        *seconds = (*seconds - time.delta_secs()).max(0.);
    }
    for (_, seconds) in barriers.repairers.values_mut() {
        *seconds += time.delta_secs();
    }
    for event in rounds.read() {
        if event.session == *session_id {
            barriers.round = event.round;
            barriers.reward_counter = 0;
        }
    }
    let retired: Vec<_> = barriers
        .owners
        .values()
        .flat_map(|slots| slots.iter().flatten())
        .copied()
        .filter(|e| {
            !enemies
                .get(*e)
                .is_ok_and(|v| v.state() == crate::life::LifeState::ActorAlive)
        })
        .collect();
    for e in retired {
        barriers.release(e);
    }
    for d in &definition.content.barriers {
        let Some(boards) = barriers.windows.get_mut(&d.id) else {
            continue;
        };
        let mut repaired = Vec::new();
        for (i, b) in boards.iter_mut().enumerate() {
            match *b {
                BoardState::Reserved(owner)
                    if !enemies
                        .get(owner)
                        .is_ok_and(|v| v.state() == crate::life::LifeState::ActorAlive) =>
                {
                    *b = BoardState::Closed
                }
                BoardState::Opening(age) => {
                    let age = age + time.delta_secs();
                    *b = if age >= d.boards[i].pull_seconds {
                        BoardState::Missing
                    } else {
                        BoardState::Opening(age)
                    };
                }
                BoardState::Pulling(owner, age) => {
                    *b = BoardState::Pulling(owner, age + time.delta_secs())
                }
                BoardState::Closing(age) => {
                    let age = age + time.delta_secs();
                    *b = if age >= d.boards[i].repair_seconds {
                        repaired.push(i);
                        BoardState::Closed
                    } else {
                        BoardState::Closing(age)
                    };
                }
                _ => {}
            }
        }
        for (e, b, disabled) in &blockers {
            if b.0 != d.id {
                continue;
            }
            let obstructed = boards.get(b.1).is_some_and(|state| {
                matches!(
                    state,
                    BoardState::Closed | BoardState::Reserved(_) | BoardState::Pulling(_, _)
                )
            });
            if obstructed && disabled.is_some() {
                commands.entity(e).remove::<ColliderDisabled>();
            }
            if !obstructed && disabled.is_none() {
                commands.entity(e).insert(ColliderDisabled);
            }
        }
        barriers.repaired += repaired.len() as u32;
        if !barriers.obstructed(&d.id) {
            barriers.owners.remove(&d.id);
        }
        let rewards = barriers
            .repairers
            .iter()
            .filter_map(|((id, index), (_, seconds))| {
                (*id == d.id && *seconds >= d.repair_interval).then_some(*index)
            })
            .collect::<Vec<_>>();
        for index in rewards {
            let Some((repairer, _)) = barriers.repairers.remove(&(d.id.clone(), index)) else {
                continue;
            };
            if !players
                .get(repairer)
                .is_ok_and(|health| health.state() == crate::life::LifeState::Standing)
            {
                continue;
            }
            barriers.reward_counter = barriers.reward_counter.saturating_add(d.repair_points);
            if barriers.reward_counter < barriers.round.saturating_mul(50).min(500) {
                let points = d.repair_points * effects.multiplier();
                session.award(points);
                barriers.awarded += points;
                let sound = crate::audio::emit(
                    &mut commands,
                    "zmb_cha_ching",
                    crate::audio::SoundSource::attached(repairer),
                );
                commands.entity(sound).insert(WoodAudio);
                barriers.sounds += 1;
            }
        }
    }
}

fn audio(
    mut commands: Commands,
    clock: Res<crate::session::SessionClock>,
    time: Res<Time<Virtual>>,
    definition: Res<GameDefinition>,
    mut barriers: ResMut<Barriers>,
) {
    let barriers = &mut *barriers;
    let now = clock.milliseconds(&time);
    for d in &definition.content.barriers {
        let Some(states) = barriers.windows.get(&d.id).cloned() else {
            continue;
        };
        let Some(placement) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        for (index, state) in states.into_iter().enumerate() {
            let playback = barriers.audio.entry((d.id.clone(), index)).or_default();
            let random = &mut barriers.audio_random;
            let sounds =
                playback.advance(state, &d.audio, &d.boards[index].audio, now as u64, || {
                    native_random(random)
                });
            let effect_phase = playback.effect_phase;
            for alias in sounds {
                let sound = crate::audio::emit(
                    &mut commands,
                    alias,
                    crate::audio::SoundSource::World(Vec3::from_array(placement.position)),
                );
                commands.entity(sound).insert(WoodAudio);
                barriers.sounds += 1;
            }
            if let Some(phase) = effect_phase {
                let pose = Transform::from_translation(Vec3::from_array(placement.position))
                    .with_rotation(Quat::from_rotation_y(placement.yaw));
                if phase == 0
                    && let Some(definition) = &d.repair_quake
                {
                    let range = |range: [f32; 2], random: &mut Option<u32>| {
                        range[0] as f64
                            + native_random(random) as f64
                                * (range[1] as f64 - range[0] as f64)
                                * 3.051851e-05_f32 as f64
                    };
                    let scale = range(definition.scale, &mut barriers.audio_random) as f32;
                    let duration =
                        (range(definition.duration, &mut barriers.audio_random) * 1000.) as i32;
                    barriers.quakes.pending.push(RepairQuake {
                        start: now,
                        duration,
                        scale,
                        radius: definition.radius,
                        origin: pose.translation,
                    });
                }
                if let Some(effect) = &d.boards[index].repair_effects[phase] {
                    crate::native_fx::emit(
                        &mut commands,
                        &effect.name,
                        pose.mul_transform(Transform::from_translation(Vec3::from_array(
                            effect.offset,
                        ))),
                    );
                }
            }
        }
    }
}
fn animate(
    definition: Option<Res<GameDefinition>>,
    barriers: Res<Barriers>,
    mut assets: crate::animation::ClipAssets,
    mut boards: Query<(
        &BoardVisual,
        &mut Visibility,
        &mut Transform,
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (visual, mut visibility, mut pose, mut clips, mut player, mut graph) in &mut boards {
        let d = &definition.content.barriers[visual.window];
        let Some(state) = barriers
            .windows
            .get(&d.id)
            .and_then(|b| b.get(visual.index))
            .copied()
        else {
            continue;
        };
        if state == BoardState::Missing {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;
        let board = &d.boards[visual.index];
        let (elapsed, clip) = match state {
            BoardState::Opening(age) => (age, 1),
            BoardState::Closing(age) => (age, 0),
            _ => (board.repair_seconds, 0),
        };
        let animation = &definition.content.animations[visual.clips[clip]];
        if !clips.sample(
            &animation.path,
            elapsed,
            false,
            &mut player,
            &mut graph,
            &mut assets,
        ) {
            *visibility = Visibility::Hidden;
        }
        let root = animation
            .root_motion
            .as_ref()
            .expect("Required source board root motion");
        let seconds = elapsed.clamp(0., root.seconds);
        *pose = visual.base.mul_transform(
            Transform::from_translation(Vec3::from_array(root.translation_at(seconds)))
                .with_rotation(Quat::from_array(root.rotation_at(seconds))),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_and_tearing_compete_per_board_and_retirement_releases_only_its_claim() {
        let id = ContentId("barrier.test".into());
        let mut world = World::new();
        let zombie = world.spawn_empty().id();
        let player = world.spawn_empty().id();
        let mut session = Session::default();
        let mut barriers = Barriers::default();
        barriers
            .windows
            .insert(id.clone(), vec![BoardState::Closed, BoardState::Missing]);
        let (board, _) = barriers.claim(&id, zombie, 0).unwrap();
        barriers.begin_pull(&id, board, zombie);
        assert_eq!(barriers.repair(&id, player, &mut session, 1.), Some(1));
        assert_eq!(
            barriers.windows[&id][board],
            BoardState::Pulling(zombie, 0.)
        );
        assert_eq!(barriers.windows[&id][1], BoardState::Closing(0.));
        assert!(barriers.claim(&id, player, 0).is_none());
        assert!(barriers.destroy(&id, board, zombie));
        assert!(barriers.obstructed(&id));
        barriers.release(zombie);
        assert_eq!(barriers.windows[&id][board], BoardState::Opening(0.));
        assert_eq!(barriers.windows[&id][1], BoardState::Closing(0.));
        barriers.windows.get_mut(&id).unwrap()[1] = BoardState::Closed;
        assert_eq!(barriers.claim(&id, zombie, 0).unwrap().0, 1);
        barriers.release(zombie);
        assert_eq!(barriers.windows[&id][1], BoardState::Closed);
    }

    #[test]
    fn native_reach_uses_source_slot_minus_one_and_separate_trigger_height_checks() {
        let action = BarrierActorAction {
            state: "zm_window_melee".into(),
            frequency: 50,
            range: 109.8 * 0.0254,
            clips: (0..3)
                .map(|index| BarrierActorClip {
                    name: format!("clip{index}"),
                    seconds: 1.,
                    duration: 1.,
                    hits: vec![0.5],
                    motion: None,
                })
                .collect(),
        };
        assert_eq!(action.reach_clip(0, 2).unwrap().name, "clip2");
        assert_eq!(action.reach_clip(1, 2).unwrap().name, "clip0");
        assert_eq!(action.reach_clip(2, 2).unwrap().name, "clip1");
        let near = Vec3::new(50. * 0.0254, 50. * 0.0254, 0.);
        assert!(action.hits_through(near, Vec3::ZERO, Vec3::ZERO));
        assert!(!action.hits_through(near.with_y(51. * 0.0254), Vec3::ZERO, Vec3::ZERO));
        assert!(!action.hits_through(near.with_x(51. * 0.0254), Vec3::ZERO, Vec3::ZERO));
    }

    #[test]
    fn source_opening_last_board_clears_attack_slots_before_visual_finishes() {
        let id = ContentId("barrier.test".into());
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let mut barriers = Barriers::default();
        barriers.windows.insert(
            id.clone(),
            vec![BoardState::Missing, BoardState::Reserved(actor)],
        );
        barriers.slot(&id, actor, 0);
        assert!(barriers.destroy(&id, 1, actor));
        assert!(!barriers.obstructed(&id));
        assert!(!barriers.owners.contains_key(&id));
    }

    #[test]
    fn native_attack_slots_survive_other_actor_retirement() {
        let mut world = World::new();
        let actors = std::array::from_fn::<_, 4, _>(|_| world.spawn_empty().id());
        let id = ContentId("barrier.test".into());
        let mut barriers = Barriers::default();
        barriers
            .windows
            .insert(id.clone(), vec![BoardState::Closed; 6]);
        let first = barriers.claim(&id, actors[0], 0).unwrap();
        let second = barriers.claim(&id, actors[1], 0).unwrap();
        let third = barriers.claim(&id, actors[2], 0).unwrap();
        assert_eq!([first.1, second.1, third.1], [0, 1, 2]);
        assert!(barriers.claim(&id, actors[3], 0).is_none());
        barriers.begin_pull(&id, first.0, actors[0]);
        barriers.windows.get_mut(&id).unwrap()[first.0] = BoardState::Pulling(actors[0], 0.6);
        assert!(barriers.destroy(&id, first.0, actors[0]));
        assert_eq!(barriers.windows[&id][first.0], BoardState::Opening(0.));
        barriers.release(actors[0]);
        assert_eq!(
            barriers.windows[&id][second.0],
            BoardState::Reserved(actors[1])
        );
        assert_eq!(barriers.claim(&id, actors[3], 0).unwrap().1, first.1);
        assert!(barriers.obstructed(&id));
    }

    #[test]
    fn source_held_use_requires_strict_quarter_second_and_clears_on_release() {
        let mut barriers = Barriers::default();
        for _ in 0..6 {
            assert!(!barriers.update_use(true, 0.05));
        }
        assert!(barriers.update_use(true, 0.05));
        assert!(!barriers.update_use(false, 0.05));
        assert!(!barriers.update_use(false, 0.05));
        assert!(!barriers.update_use(true, 0.05));
    }

    #[test]
    fn native_repair_quakes_keep_four_slots_and_use_source_lifetime_distance_gain() {
        let quake = RepairQuake {
            start: 0,
            duration: 1000,
            scale: 0.4,
            radius: 10.,
            origin: Vec3::ZERO,
        };
        assert_eq!(quake.gains(0, Vec3::ZERO), Some((0.4, 0.4)));
        let (gain, life) = quake.gains(500, Vec3::X * 5.).unwrap();
        assert!((gain - 0.1).abs() < 1e-6);
        assert!((life - 0.2).abs() < 1e-6);
        assert!(quake.gains(1000, Vec3::ZERO).is_none());
        assert!(quake.gains(-1, Vec3::ZERO).is_none());
        let mut quakes = RepairQuakes::default();
        for _ in 0..4 {
            quakes.insert(quake, 0, Vec3::ZERO);
        }
        quakes.insert(RepairQuake { scale: 1., ..quake }, 0, Vec3::ZERO);
        assert!(quakes.slots.iter().flatten().all(|slot| slot.scale == 0.4));
        quakes.slots[2] = Some(RepairQuake {
            origin: Vec3::X * 20.,
            ..quake
        });
        quakes.insert(RepairQuake { scale: 1., ..quake }, 0, Vec3::ZERO);
        assert_eq!(quakes.slots[2].unwrap().scale, 1.);
        let angles = quakes.angles(500, Vec3::ZERO).unwrap();
        let time = 500. * 0.0016666667;
        assert!(
            (angles.x - (time * 25.132742_f32).sin() * 18_f32.to_radians() * 0.25).abs() < 1e-6
        );
    }

    #[test]
    fn native_board_audio_preserves_transition_and_delay_order() {
        let general = BarrierAudioDefinition {
            repair: [Some("repair.first".into()), Some("repair.second".into())],
            repair_delay: Some(0.25),
        };
        let board = BoardAudioDefinition {
            opening: Some("tear".into()),
            hover: Some("hover".into()),
            repeat_pause: Some([0.1, 0.1]),
            effect_delay: [0.1, 0.1],
        };
        let mut playback = BoardAudioPlayback::default();
        assert!(
            playback
                .advance(BoardState::Closed, &general, &board, 0, || 0)
                .is_empty()
        );
        assert_eq!(
            playback.advance(BoardState::Opening(0.), &general, &board, 10, || 0),
            ["tear"]
        );
        assert!(
            playback
                .advance(BoardState::Missing, &general, &board, 20, || 0)
                .is_empty()
        );
        assert!(
            playback
                .advance(BoardState::Missing, &general, &board, 120, || 0)
                .is_empty()
        );
        assert!(
            playback
                .advance(BoardState::Missing, &general, &board, 121, || 0)
                .is_empty()
        );
        assert_eq!(
            playback.advance(BoardState::Closing(0.), &general, &board, 122, || 0),
            ["hover", "tear"]
        );
        assert_eq!(
            playback.advance(BoardState::Closed, &general, &board, 500, || 0),
            ["repair.first"]
        );
        assert!(
            playback
                .advance(BoardState::Closed, &general, &board, 750, || 0)
                .is_empty()
        );
        assert!(
            playback
                .advance(BoardState::Closed, &general, &board, 751, || 0)
                .is_empty()
        );
        assert_eq!(
            playback.advance(BoardState::Closed, &general, &board, 752, || 0),
            ["repair.second"]
        );
        assert_eq!(playback.effect_phase, Some(0));
        assert!(
            playback
                .advance(BoardState::Closed, &general, &board, 1000, || 0)
                .is_empty()
        );
        assert_eq!(playback.effect_phase, None);
        playback.advance(BoardState::Closed, &general, &board, 1100, || 0);
        assert_eq!(playback.effect_phase, None);
        playback.advance(BoardState::Closed, &general, &board, 1101, || 0);
        assert_eq!(playback.effect_phase, None);
        playback.advance(BoardState::Closed, &general, &board, 1102, || 0);
        assert_eq!(playback.effect_phase, Some(1));
    }
}
