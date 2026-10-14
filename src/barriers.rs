use crate::{
    GameState,
    animation::{AnimationGraphs, AnimationLayer, AnimationPlayback},
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
#[serde(deny_unknown_fields)]
pub struct BarrierDefinition {
    pub id: ContentId,
    pub placement: PlacementId,
    pub entry: ContentId,
    pub exit: ContentId,
    pub boards: Vec<BoardDefinition>,
    pub audio: BarrierAudioDefinition,
    pub repair_origin: [f32; 3],
    pub repair_yaw: f32,
    pub repair_radius: f32,
    pub repair_height: f32,
    pub repair_interval: f32,
    pub repair_points: u32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardDefinition {
    pub presentation: ContentId,
    pub repair: ContentId,
    pub pull: ContentId,
    pub repair_seconds: f32,
    pub pull_seconds: f32,
    pub collision: Vec<crate::environment::brushes::TriangleShape>,
    pub attacks: [BoardAttackDefinition; 3],
    pub audio: BoardAudioDefinition,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarrierAudioDefinition {
    pub repair: [Option<String>; 2],
    pub repair_delay: Option<f32>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardAudioDefinition {
    pub opening: Option<String>,
    pub hover: Option<String>,
    pub repeat_pause: Option<[f32; 2]>,
}
impl BoardDefinition {
    pub fn collider(&self) -> Option<Collider> {
        crate::environment::brushes::compound(&[], &self.collision)
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardAttackDefinition {
    pub grab_seconds: f32,
    pub tear_seconds: f32,
    pub destroy_seconds: f32,
    pub grab_clip: String,
    pub tear_clip: String,
    pub grab_motion: crate::origins::content::AnimationRootMotion,
    pub tear_motion: crate::origins::content::AnimationRootMotion,
}
impl BoardAttackDefinition {
    pub fn valid_motion(&self) -> bool {
        [
            (&self.grab_motion, self.grab_seconds),
            (&self.tear_motion, self.tear_seconds),
        ]
        .into_iter()
        .all(|(motion, seconds)| motion.valid() && (motion.seconds - seconds).abs() < 0.001)
    }
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
    repairers: BTreeMap<(ContentId, usize), (Entity, f32)>,
    repair_cooldowns: BTreeMap<ContentId, f32>,
    audio: BTreeMap<(ContentId, usize), BoardAudioPlayback>,
    audio_elapsed: std::time::Duration,
    audio_random: Option<u32>,
    round: u32,
    use_poll: f32,
    use_ticks: u32,
    use_started: Option<u32>,
    use_active: bool,
}
impl Barriers {
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
        self.windows
            .get(id)
            .is_some_and(|boards| boards.iter().any(|state| *state != BoardState::Missing))
    }
    pub fn claim(&mut self, id: &ContentId, owner: Entity, random: u32) -> Option<(usize, usize)> {
        let slots = self.owners.entry(id.clone()).or_insert([None; 3]);
        let available = slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.is_none().then_some(index))
            .collect::<Vec<_>>();
        let existing = slots.iter().position(|slot| *slot == Some(owner));
        if available.is_empty() && existing.is_none() {
            return None;
        }
        let slot = existing.unwrap_or_else(|| available[random as usize % available.len()]);
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
        slots[slot] = Some(owner);
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
}

#[derive(Default)]
struct BoardAudioPlayback {
    previous: AudioBoardState,
    sequence: Option<BoardSoundSequence>,
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
                None
            }
            waiting => waiting,
        };
        sounds
    }
}
#[derive(Component)]
pub struct BarrierBlocker(pub ContentId, pub usize);
#[derive(Component)]
pub struct BarrierRepairTrigger(pub ContentId);
pub const REPAIR_TRIGGER_LAYER: u32 = 32;
#[derive(Component)]
pub struct BoardVisual {
    id: ContentId,
    index: usize,
    source: Handle<Gltf>,
    graph: Option<(
        Handle<AnimationGraph>,
        AnimationNodeIndex,
        AnimationNodeIndex,
    )>,
}
#[derive(Component)]
struct WoodAudio;
pub struct BarrierPlugin;
impl Plugin for BarrierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Barriers>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
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
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    if !players.is_empty() {
        return;
    }
    *barriers = Barriers::default();
    for d in &definition.content.barriers {
        let Some(p) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        barriers
            .windows
            .insert(d.id.clone(), vec![BoardState::Closed; d.boards.len()]);
        let origin = Vec3::from_array(p.position);
        let rotation = Quat::from_rotation_y(p.yaw);
        let repair_origin = Vec3::from_array(d.repair_origin);
        let Some(floor) = query.cast_ray(
            repair_origin,
            Dir3::NEG_Y,
            100000. * 0.0254,
            false,
            &crate::environment::solids([]),
        ) else {
            failure.0 = format!(
                "Source barrier {} repair origin has no loaded ground collision",
                d.id.0
            );
            next.set(GameState::LoadError);
            return;
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
                    failure.0 = format!(
                        "Source barrier {} board {index} collision is unusable",
                        d.id.0
                    );
                    next.set(GameState::LoadError);
                    return;
                };
                commands.spawn((
                    SessionEntity,
                    BarrierBlocker(d.id.clone(), index),
                    RigidBody::Static,
                    crate::assets::WorldCollider,
                    crate::movement::physics::SurfaceFlags(collision.surface_flags),
                    shape,
                    Transform::from_translation(origin).with_rotation(rotation),
                ));
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
            let base = scene.split('#').next().unwrap_or(scene);
            commands.spawn((
                SessionEntity,
                BoardVisual {
                    id: d.id.clone(),
                    index,
                    source: server.load(base.to_owned()),
                    graph: None,
                },
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_translation(origin).with_rotation(rotation),
                Visibility::Inherited,
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
) {
    for seconds in barriers.repair_cooldowns.values_mut() {
        *seconds = (*seconds - time.delta_secs()).max(0.);
    }
    for (_, seconds) in barriers.repairers.values_mut() {
        *seconds += time.delta_secs();
    }
    if barriers.round != session.round {
        barriers.round = session.round;
        barriers.reward_counter = 0;
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
            if barriers.reward_counter < session.round.saturating_mul(50).min(500) {
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
    time: Res<Time>,
    definition: Res<GameDefinition>,
    mut barriers: ResMut<Barriers>,
) {
    let barriers = &mut *barriers;
    barriers.audio_elapsed += time.delta();
    let now = barriers.audio_elapsed.as_millis() as u64;
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
            let sounds = playback.advance(state, &d.audio, &d.boards[index].audio, now, || {
                let value = random
                    .unwrap_or(1)
                    .wrapping_mul(0x343fd)
                    .wrapping_add(0x269ec3);
                *random = Some(value);
                value >> 16 & 32767
            });
            for alias in sounds {
                let sound = crate::audio::emit(
                    &mut commands,
                    alias,
                    crate::audio::SoundSource::World(Vec3::from_array(placement.position)),
                );
                commands.entity(sound).insert(WoodAudio);
                barriers.sounds += 1;
            }
        }
    }
}
fn animate(
    mut commands: Commands,
    definition: Option<Res<GameDefinition>>,
    mut barriers: ResMut<Barriers>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<AnimationGraphs>,
    mut boards: Query<(Entity, &mut BoardVisual, &mut Visibility, &mut Transform)>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (entity, mut visual, mut visibility, mut pose) in &mut boards {
        let Some(d) = definition
            .content
            .barriers
            .iter()
            .find(|d| d.id == visual.id)
        else {
            continue;
        };
        let Some(state) = barriers
            .windows
            .get(&visual.id)
            .and_then(|b| b.get(visual.index))
            .copied()
        else {
            continue;
        };
        *visibility = if state == BoardState::Missing {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if visual.graph.is_none() {
            let Some(bindings) = cache.scene(&visual.source, &gltfs, &mut graphs) else {
                continue;
            };
            let board = &d.boards[visual.index];
            let Some(repair) = board
                .repair
                .0
                .rsplit('.')
                .next()
                .and_then(|name| bindings.named.get(name))
                .copied()
            else {
                continue;
            };
            let Some(pull) = board
                .pull
                .0
                .rsplit('.')
                .next()
                .and_then(|name| bindings.named.get(name))
                .copied()
            else {
                continue;
            };
            visual.graph = Some((bindings.graph.clone(), repair, pull));
            barriers.animated_boards += 1;
        }
        let Some((graph, repair, pull)) = &visual.graph else {
            continue;
        };
        let board = &d.boards[visual.index];
        let (node, elapsed, animation) = match state {
            BoardState::Opening(age) => (*pull, age, &board.pull),
            BoardState::Closing(age) => (*repair, age, &board.repair),
            _ => (*repair, board.repair_seconds, &board.repair),
        };
        let Some(placement) = definition
            .content
            .placements
            .iter()
            .find(|placement| placement.id == d.placement)
        else {
            continue;
        };
        let base_rotation = Quat::from_rotation_y(placement.yaw);
        pose.translation = Vec3::from_array(placement.position);
        pose.rotation = base_rotation;
        if let Some(root) = definition
            .content
            .animations
            .iter()
            .find(|definition| definition.id == *animation)
            .and_then(|definition| definition.root_motion.as_ref())
        {
            let seconds = elapsed.clamp(0., root.seconds);
            pose.translation += base_rotation * Vec3::from_array(root.translation_at(seconds));
            pose.rotation = base_rotation * Quat::from_array(root.rotation_at(seconds));
        }
        for child in children.iter_descendants(entity) {
            if !players.contains(child) {
                continue;
            }
            commands.entity(child).insert(AnimationPlayback::new(
                graph.clone(),
                vec![AnimationLayer::new(node, Some(elapsed), false)],
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn native_board_audio_preserves_transition_and_delay_order() {
        let general = BarrierAudioDefinition {
            repair: [Some("repair.first".into()), Some("repair.second".into())],
            repair_delay: Some(0.25),
        };
        let board = BoardAudioDefinition {
            opening: Some("tear".into()),
            hover: Some("hover".into()),
            repeat_pause: Some([0.1, 0.1]),
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
        assert!(
            playback
                .advance(BoardState::Closed, &general, &board, 1000, || 0)
                .is_empty()
        );
    }
}
