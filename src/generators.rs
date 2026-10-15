use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    interactions::InteractionRejection,
    life::{LifeState, Vitality},
    origins::ContentId,
    player::Player,
};
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::{BTreeMap, BTreeSet};
mod crystals;
mod effects;
mod raids;
mod rewards;
pub use crystals::CrystalProfile;
pub use raids::GeneratorRaids;
pub(crate) use rewards::BoxGlowRamp;

pub(crate) fn reset(world: &mut World) {
    crystals::reset(world);
    world.insert_resource(Generators::default());
    world.insert_resource(GeneratorRaids::default());
    world.insert_resource(rewards::CaptureRewards::default());
}

pub(crate) fn use_point(origin: Vec3, eye: Vec3, box_size: f32) -> Vec3 {
    let center = origin + Vec3::Y * box_size * 0.5;
    let delta = eye - center;
    center + delta.normalize_or_zero() * (box_size * 0.25).min(delta.length() * 0.5)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratorState {
    Inactive,
    Capturing,
    Active,
    Disabled,
}
#[derive(Clone, Debug)]
pub struct Capture {
    pub state: GeneratorState,
    pub progress: f32,
    pub age: f32,
    pub purchaser: Option<Entity>,
    refund: Option<f32>,
    pump_clip: Option<usize>,
    pump_elapsed: f32,
    rotation: Option<(usize, f32)>,
    progress_clock: f32,
    source_progress: f32,
    cleanup_clock: Timer,
}
impl Default for Capture {
    fn default() -> Self {
        Self {
            state: GeneratorState::Inactive,
            progress: 0.,
            age: 0.,
            purchaser: None,
            refund: None,
            pump_clip: None,
            pump_elapsed: 0.,
            rotation: None,
            progress_clock: 0.,
            source_progress: 0.,
            cleanup_clock: Timer::default(),
        }
    }
}
impl Capture {
    fn advance(&mut self, dt: f32, capture_seconds: f32, decay_seconds: f32, inside: bool) {
        self.progress_clock -= dt;
        if self.progress_clock <= 0. {
            self.progress_clock += 0.1;
            self.source_progress = (self.source_progress
                + if inside {
                    100. / capture_seconds * 0.1
                } else {
                    -100. / decay_seconds * 0.1
                })
            .clamp(0., 100.);
            self.progress = self.source_progress / 100.;
        }
    }
}
#[derive(Resource, Default)]
pub struct Generators {
    pub sites: BTreeMap<ContentId, Capture>,
    pub activations: u32,
    pub completions: u32,
    pub abandoned: u32,
    pub rewards: u32,
    pub refunds: u32,
    pub animated: u32,
    pub playing_sounds: u32,
    pub playback: BTreeSet<String>,
    pub defenders_admitted: u32,
    pub defender_failures: u32,
    pub defenders_cleared: u32,
    pub excess_retired: u32,
    pub(crate) raid_active: bool,
    initial_reward: bool,
    retirement_pending: bool,
    retirement_clock: Timer,
    spawn_clock: f32,
    used_emergence: BTreeMap<ContentId, BTreeSet<usize>>,
    capture_points_round: u32,
    capture_points: u32,
}
impl Generators {
    pub(crate) fn capture_kill_points(&mut self, round: u32) -> u32 {
        if self.capture_points_round != round {
            self.capture_points_round = round;
            self.capture_points = 0;
        }
        if self.capture_points < round.saturating_mul(100) {
            self.capture_points += 10;
            10
        } else {
            0
        }
    }

    fn emergence_index(
        &mut self,
        id: &ContentId,
        count: usize,
        session: &mut Session,
    ) -> Option<usize> {
        let used = self.used_emergence.entry(id.clone()).or_default();
        if used.len() >= count {
            used.clear();
        }
        let choices = (0..count)
            .filter(|index| !used.contains(index))
            .collect::<Vec<_>>();
        let index = *choices.get(session.spawn_index(choices.len())?)?;
        used.insert(index);
        Some(index)
    }
    pub fn available(&self, id: &ContentId) -> bool {
        self.sites
            .get(id)
            .is_some_and(|s| matches!(s.state, GeneratorState::Inactive | GeneratorState::Disabled))
    }
    pub fn busy(&self) -> bool {
        self.sites
            .values()
            .any(|s| s.state == GeneratorState::Capturing)
    }
    pub fn reserve(&self) -> usize {
        usize::from(self.busy()) * 4 + usize::from(self.raid_active) * 4
    }
    pub fn activate(
        &mut self,
        id: &ContentId,
        purchaser: Entity,
    ) -> Result<(), InteractionRejection> {
        if self.busy() {
            return Err(InteractionRejection::Busy);
        }
        if !self.available(id) {
            return Err(InteractionRejection::Unavailable);
        }
        let site = self
            .sites
            .get_mut(id)
            .ok_or(InteractionRejection::Unavailable)?;
        *site = Capture {
            state: GeneratorState::Capturing,
            purchaser: Some(purchaser),
            pump_clip: Some(0),
            ..default()
        };
        self.activations += 1;
        self.retirement_pending = true;
        self.retirement_clock = Timer::default();
        Ok(())
    }
    pub fn powered(&self, id: &ContentId) -> bool {
        self.sites
            .get(id)
            .is_some_and(|s| s.state == GeneratorState::Active)
    }
    pub fn active_count(&self) -> usize {
        self.sites
            .values()
            .filter(|s| s.state == GeneratorState::Active)
            .count()
    }
}
#[derive(Component)]
#[require(crate::animation::SceneClips)]
pub struct GeneratorVisual {
    id: ContentId,
    source: Handle<Gltf>,
    nodes: Option<[Option<AnimationNodeIndex>; 8]>,
    fluid_joints: Vec<(Entity, Vec3)>,
    blend_from: [f32; 8],
    blend_target: [f32; 8],
    blend_elapsed: [f32; 8],
    sound_state: Option<GeneratorState>,
    sound_clip: usize,
    sound_time: f32,
    pole: Option<Entity>,
    voice_then: Option<(Entity, String)>,
    capture_voice: Option<(Entity, f32)>,
}
#[derive(Component)]
struct GeneratorAudio(ContentId, ContentId);
#[derive(Component)]
struct CaptureRetirement(Timer);

#[derive(Component)]
pub struct GeneratorDefender {
    pub generator: ContentId,
    pub slot: usize,
    attack_point: Option<usize>,
    scan: f32,
    acquired_player: bool,
}
impl GeneratorDefender {
    pub(crate) fn new(generator: ContentId, slot: usize) -> Self {
        Self {
            generator,
            slot,
            attack_point: None,
            scan: 0.,
            acquired_player: false,
        }
    }
}
pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            rewards::CaptureRewardsPlugin,
            effects::CaptureEffectsPlugin,
            crystals::CrystalPlugin,
        ))
        .init_resource::<Generators>()
        .init_resource::<raids::GeneratorRaids>()
        .add_systems(
            OnEnter(GameState::StartingSession),
            raids::reset.in_set(crate::session::SessionSetup::World),
        )
        .add_systems(
            OnEnter(GameState::StartingSession),
            assemble.in_set(crate::session::SessionSetup::World),
        )
        .add_observer(visual_ready)
        .add_systems(
            PostUpdate,
            fluid_bind_pose.in_set(crate::animation::AnimationPoseModifiers),
        )
        .add_systems(
            FixedUpdate,
            defender_intent
                .in_set(GameStep::EnemyIntent)
                .before(crate::enemies::lifecycle)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            tick.in_set(GameStep::Zombies)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            (retire_excess, clear_defenders, defenders)
                .chain()
                .in_set(GameStep::Zombies)
                .after(tick)
                .before(crate::enemies::EnemyRetirement)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (presentation, audio_evidence).run_if(in_state(GameState::Playing)),
        );
        app.add_systems(
            FixedUpdate,
            raids::attack_state
                .in_set(GameStep::EnemyIntent)
                .after(crate::enemies::lifecycle)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            raids::drain
                .in_set(GameStep::Zombies)
                .before(tick)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            raids::tick
                .in_set(GameStep::EnemyIntent)
                .before(crate::enemies::lifecycle)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            raids::deaths
                .in_set(GameStep::Rewards)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn defender_intent(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    players: Query<(&Player, &Transform, &Vitality)>,
    mut defenders: Query<
        (
            Entity,
            &mut GeneratorDefender,
            &crate::enemies::Zombie,
            &Vitality,
            Option<&crate::enemies::EnemyGoal>,
        ),
        Without<Player>,
    >,
) {
    let mut claimed = BTreeMap::<ContentId, BTreeSet<usize>>::new();
    for (_, guard, _, health, _) in &defenders {
        if health.state() == LifeState::ActorAlive
            && let Some(index) = guard.attack_point
        {
            claimed
                .entry(guard.generator.clone())
                .or_default()
                .insert(index);
        }
    }
    for (entity, mut guard, zombie, health, current_goal) in &mut defenders {
        if health.state() != LifeState::ActorAlive || !zombie.completed_emerging() {
            continue;
        }
        guard.scan -= time.delta_secs();
        if guard.scan > 0. {
            continue;
        }
        let Some(d) = definition
            .content
            .generators
            .iter()
            .find(|d| d.id == guard.generator)
        else {
            continue;
        };
        let Some(p) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        let origin = Vec3::from_array(p.position);
        guard.acquired_player |= players
            .iter()
            .any(|(_, _, health)| health.state() == LifeState::Standing);
        if players.iter().any(|(player, t, v)| {
            v.state() == LifeState::Standing
                && (t.translation - Vec3::Y * (player.height * 0.5)).distance_squared(origin)
                    < (700. * 0.0254_f32).powi(2)
        }) {
            if let Some(index) = guard.attack_point.take() {
                claimed
                    .entry(guard.generator.clone())
                    .or_default()
                    .remove(&index);
            }
            guard.scan = 0.5;
            if current_goal.is_some() {
                commands
                    .entity(entity)
                    .remove::<crate::enemies::EnemyGoal>();
            }
        } else if guard.acquired_player {
            let used = claimed.entry(guard.generator.clone()).or_default();
            let index = if let Some(index) = guard.attack_point {
                index
            } else {
                let group = if used.range(0..3).count() < 3 {
                    0..3
                } else if used.range(4..7).count() < 1 {
                    4..7
                } else if used.range(8..11).count() < 1 {
                    8..11
                } else {
                    0..11
                };
                let valid = |index: &usize| {
                    !used.contains(index)
                        && d.attack_points
                            .get(*index)
                            .is_some_and(|point| !point.inaccessible)
                };
                let mut choices = group.filter(valid).collect::<Vec<_>>();
                if choices.is_empty() {
                    choices = (0..11).filter(valid).collect();
                }
                let Some(selection) = session.spawn_index(choices.len()) else {
                    continue;
                };
                let index = choices[selection];
                used.insert(index);
                guard.attack_point = Some(index);
                index
            };
            let Some(point) = d.attack_points.get(index) else {
                continue;
            };
            let Some(enemy) = definition
                .content
                .enemies
                .iter()
                .find(|enemy| enemy.id == zombie.definition)
            else {
                continue;
            };
            let center = crate::enemies::body_center(enemy, false);
            let goal = crate::enemies::EnemyGoal {
                position: Vec3::from_array(point.position) + center,
                face: Vec3::from_array(point.face) + center,
                radius: 30. * 0.0254,
            };
            guard.scan = 0.2 + 1.3 * (session.random() as f64 / u32::MAX as f64) as f32;
            if current_goal.is_none_or(|current| {
                current.position != goal.position
                    || current.face != goal.face
                    || current.radius != goal.radius
            }) {
                commands.entity(entity).insert(goal);
            }
        }
    }
}
fn retire_excess(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut generators: ResMut<Generators>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    actors: Query<
        (
            Entity,
            &Vitality,
            &crate::rounds::PopulationKind,
            &crate::enemies::Zombie,
            &Transform,
        ),
        With<crate::enemies::Zombie>,
    >,
    mut retiring: Query<(Entity, &mut CaptureRetirement)>,
    mut barriers: ResMut<crate::barriers::Barriers>,
) {
    for (entity, mut retirement) in &mut retiring {
        if retirement.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            generators.excess_retired += 1;
        }
    }
    if !generators.retirement_pending {
        return;
    }
    let candidates: Vec<_> = actors
        .iter()
        .filter_map(|(entity, health, kind, zombie, pose)| {
            (health.state() == LifeState::ActorAlive
                && *kind == crate::rounds::PopulationKind::Ordinary)
                .then_some((entity, zombie, pose))
        })
        .collect();
    let cap = (definition.rounds.ordinary_cap as usize).saturating_sub(generators.reserve());
    if candidates.len() <= cap {
        generators.retirement_pending = false;
        return;
    }
    if !generators.retirement_clock.tick(time.delta()).is_finished() {
        return;
    }
    generators.retirement_clock = Timer::from_seconds(0.1, TimerMode::Once);
    let (entity, zombie, pose) = candidates[session.random() as usize % candidates.len()];
    capture_retirement_fx(&mut commands, &definition, zombie, pose);
    barriers.release(entity);
    commands.entity(entity).remove::<Vitality>().insert((
        CaptureRetirement(Timer::from_seconds(0.1, TimerMode::Once)),
        avian3d::prelude::ColliderDisabled,
    ));
}

fn capture_retirement_fx(
    commands: &mut Commands,
    game: &GameDefinition,
    zombie: &crate::enemies::Zombie,
    pose: &Transform,
) {
    if let Some(profile) = game
        .content
        .enemies
        .iter()
        .find(|profile| profile.id == zombie.definition)
    {
        crate::native_fx::emit(
            commands,
            "maps/zombie_alcatraz/fx_alcatraz_afterlife_zmb_tport",
            Transform::from_translation(
                pose.translation - crate::enemies::body_center(profile, false),
            ),
        );
    }
}

fn clear_defenders(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut generators: ResMut<Generators>,
    mut defenders: Query<
        (
            &mut Vitality,
            &GeneratorDefender,
            &crate::enemies::Zombie,
            &Transform,
        ),
        (With<crate::enemies::Zombie>, With<Transform>),
    >,
) {
    let mut cleared = 0;
    for (id, site) in &mut generators.sites {
        if site.state == GeneratorState::Capturing
            || !site.cleanup_clock.tick(time.delta()).is_finished()
        {
            continue;
        }
        for (mut health, guard, zombie, pose) in &mut defenders {
            if guard.generator != *id || health.state() != LifeState::ActorAlive {
                continue;
            }
            if let Ok(damage) =
                crate::life::ResolvedDamage::new(health.current().unwrap_or(1).saturating_add(100))
            {
                capture_retirement_fx(&mut commands, &definition, zombie, pose);
                let _ = health.apply(damage);
                site.cleanup_clock = Timer::from_seconds(0.1, TimerMode::Once);
                cleared += 1;
            }
            break;
        }
    }
    generators.defenders_cleared += cleared;
}
fn defenders(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut generators: ResMut<Generators>,
    mut session: ResMut<Session>,
    actors: Query<
        (
            &Vitality,
            &Transform,
            Option<&GeneratorDefender>,
            Has<crate::enemies::ActorCorpse>,
        ),
        With<crate::enemies::Zombie>,
    >,
) {
    use avian3d::prelude::ColliderDisabled;
    generators.spawn_clock -= time.delta_secs();
    if generators.spawn_clock > 0. {
        return;
    }
    generators.spawn_clock = 0.5;
    let Some((id, _)) = generators
        .sites
        .iter()
        .find(|(_, s)| s.state == GeneratorState::Capturing)
    else {
        return;
    };
    let id = id.clone();
    let alive: Vec<_> = actors
        .iter()
        .filter(|(v, _, _, _)| v.state() == LifeState::ActorAlive)
        .collect();
    let slots: Vec<_> = alive
        .iter()
        .filter_map(|(_, _, g, _)| g.filter(|g| g.generator == id).map(|g| g.slot))
        .collect();
    if slots.len() >= 4
        || actors.iter().filter(|(_, _, _, corpse)| !*corpse).count()
            >= definition.rounds.actor_cap as usize
    {
        return;
    }
    let Some(d) = definition.content.generators.iter().find(|d| d.id == id) else {
        return;
    };
    let Some(enemy) = definition
        .content
        .enemies
        .iter()
        .find(|enemy| enemy.id == d.capture_enemy)
    else {
        return;
    };
    let Some(slot) = (0..4).find(|s| !slots.contains(s)) else {
        return;
    };
    let Some(index) = generators.emergence_index(&id, d.emergence.len(), &mut session) else {
        return;
    };
    let spot = &d.emergence[index];
    let position = Vec3::from_array(spot.position) + crate::enemies::body_center(enemy, false);
    let shape = crate::enemies::body_shape(enemy, false);
    let rotation = Quat::from_rotation_y(spot.yaw);
    let health = definition.rounds.health(session.round);
    let Ok(health) = Vitality::ordinary_actor(health, health) else {
        return;
    };
    let gait_seed = session.random();
    let gait = session
        .difficulty
        .gait(&definition.rounds, session.round, gait_seed);
    let gait = if gait == crate::rounds::Gait::Walk {
        crate::rounds::Gait::Run
    } else {
        gait
    };
    let mut zombie = crate::enemies::Zombie::new(position, enemy.id.clone(), gait)
        .with_locomotion_seed(session.random());
    let rise = enemy
        .traversals
        .get("rise")
        .and_then(|profiles| profiles.first());
    if let Some(rise) = rise {
        zombie = zombie.dug_rising(rise, rotation);
    }
    let mut actor = commands.spawn((
        SessionEntity,
        zombie,
        shape,
        health,
        crate::rounds::PopulationKind::GeneratorDefender,
        GeneratorDefender::new(id, slot),
        Transform::from_translation(position).with_rotation(rotation),
        Visibility::Inherited,
        Name::new("Generator defender"),
    ));
    if rise.is_some() {
        actor.insert(ColliderDisabled);
    }
    generators.defenders_admitted += 1;
}
fn assemble(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    server: Res<AssetServer>,
    players: Query<(), With<Player>>,
    mut generators: ResMut<Generators>,
) {
    if !players.is_empty() {
        return;
    }
    *generators = default();
    for d in definition.content.generators.iter() {
        generators.sites.insert(d.id.clone(), default());
        let Some(p) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        let Some(scene) = definition
            .content
            .presentations
            .iter()
            .find(|p| p.id == d.presentation)
            .and_then(|p| p.scene.as_ref())
        else {
            continue;
        };
        commands.spawn((
            SessionEntity,
            GeneratorVisual {
                id: d.id.clone(),
                source: server
                    .load_builder()
                    .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                        settings.load_animations = false;
                    })
                    .load(scene.split('#').next().unwrap_or(scene).to_owned()),
                nodes: None,
                fluid_joints: Vec::new(),
                blend_from: [0.; 8],
                blend_target: [0.; 8],
                blend_elapsed: [0.; 8],
                sound_state: None,
                sound_clip: usize::MAX,
                sound_time: -0.001,
                pole: None,
                voice_then: None,
                capture_voice: None,
            },
            WorldAssetRoot(
                server
                    .load_builder()
                    .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                        settings.load_animations = false;
                    })
                    .load(scene.clone()),
            ),
            Transform::from_translation(Vec3::from_array(d.model_position))
                .with_rotation(Quat::from_rotation_y(p.yaw)),
            Visibility::Inherited,
        ));
    }
}
fn visual_ready(
    event: On<WorldInstanceReady>,
    mut visuals: Query<&mut GeneratorVisual>,
    children: Query<&Children>,
    names: Query<&Name>,
    poses: Query<&Transform>,
) {
    let Ok(mut visual) = visuals.get_mut(event.entity) else {
        return;
    };
    visual.fluid_joints = children
        .iter_descendants(event.entity)
        .filter_map(|joint| {
            let name = names.get(joint).ok()?;
            matches!(
                name.as_str(),
                "j_115_vats"
                    | "j_generator_pole"
                    | "vat_01_jnt"
                    | "vat_02_jnt"
                    | "vat_rotate_01_jnt"
                    | "vat_rotate_02_jnt"
            )
            .then(|| {
                (
                    joint,
                    poses
                        .get(joint)
                        .expect("Generator fluid joint bind pose")
                        .translation,
                )
            })
        })
        .collect();
    assert_eq!(
        visual.fluid_joints.len(),
        6,
        "Generator source fluid joints"
    );
    visual.pole = children.iter_descendants(event.entity).find(|child| {
        names
            .get(*child)
            .is_ok_and(|name| name.as_str().eq_ignore_ascii_case("j_generator_pole"))
    });
}
fn tick(
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut generators: ResMut<Generators>,
    mut chests: ResMut<rewards::CaptureRewards>,
    mut session: ResMut<Session>,
    effects: Res<crate::powerups::PowerupEffects>,
    players: Query<(Entity, &Player, &Transform, &Vitality)>,
) {
    let dt = time.delta_secs();
    for d in definition.content.generators.iter() {
        let Some(p) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        let origin = Vec3::from_array(p.position);
        let inside = |entity: Option<Entity>| {
            players.iter().any(|(e, p, t, v)| {
                entity.is_none_or(|owner| owner == e)
                    && v.state() == LifeState::Standing
                    && (t.translation - origin).xz().length_squared() < d.radius * d.radius
                    && t.translation.y - p.height * 0.5 > origin.y - 0.508
            })
        };
        let Some(site) = generators.sites.get_mut(&d.id) else {
            continue;
        };
        site.age += dt;
        site.pump_elapsed += dt;
        if site.pump_clip == Some(3) && site.pump_elapsed >= d.clip_seconds[3] {
            site.pump_clip = None;
            site.pump_elapsed = 0.;
        }
        if let Some((rotation, elapsed)) = site.rotation {
            let elapsed = elapsed + dt;
            site.rotation = (rotation != 7 || elapsed < d.clip_seconds[7])
                .then_some((rotation, elapsed.min(d.clip_seconds[rotation])));
        }
        let previous_progress = site.progress;
        let mut reward = 0;
        let mut refund = 0;
        let mut completed = false;
        let mut abandoned = false;
        if site.state == GeneratorState::Capturing {
            site.advance(dt, d.capture_seconds, d.decay_seconds, inside(None));
            let pump_clip = if site.progress <= 0. {
                Some(3)
            } else if site.progress > previous_progress && site.age > d.clip_seconds[0] {
                Some(1)
            } else if site.pump_clip == Some(1) && site.progress < previous_progress {
                Some(2)
            } else {
                site.pump_clip
            };
            if pump_clip != site.pump_clip {
                site.pump_clip = pump_clip;
                site.pump_elapsed = 0.;
            }
            if site.progress >= 1. {
                site.state = GeneratorState::Active;
                site.age = 0.;
                site.refund = Some(1.);
                reward = 100;
                completed = true;
            } else if site.progress <= 0. && !inside(None) {
                site.state = GeneratorState::Disabled;
                site.age = 0.;
                site.purchaser = None;
                abandoned = true;
            }
        }
        if site.progress >= 1. && previous_progress < 1. {
            site.rotation = Some((6, 0.));
        } else if site.progress < 1. && previous_progress >= 1. {
            site.rotation = Some((7, 0.));
        }
        if !completed && let Some(delay) = site.refund.as_mut() {
            *delay -= dt;
            if *delay <= 0. {
                if inside(site.purchaser) {
                    refund = d.price / effects.multiplier();
                }
                site.refund = None;
                site.purchaser = None;
            }
        }
        if completed && !generators.initial_reward {
            generators.initial_reward = true;
            chests.queue(2.);
        }
        generators.completions += u32::from(completed);
        generators.abandoned += u32::from(abandoned);
        reward *= effects.multiplier();
        generators.rewards += reward;
        generators.refunds += refund;
        session.award(reward.saturating_add(refund));
    }
}
fn audio_evidence(
    sinks: Query<(), (With<GeneratorAudio>, Added<bevy::audio::SpatialAudioSink>)>,
    mut generators: ResMut<Generators>,
) {
    generators.playing_sounds += sinks.iter().count() as u32;
}
fn presentation(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<Session>,
    definition: Res<GameDefinition>,
    generators: Res<Generators>,
    mut assets: crate::animation::ClipAssets,
    mut visuals: Query<(
        Entity,
        &mut GeneratorVisual,
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
    audio: Query<(Entity, &GeneratorAudio)>,
    mut voice_events: MessageReader<crate::audio::PlaybackEvent>,
) {
    let _span = info_span!("generator_presentation").entered();
    let completed = voice_events
        .read()
        .filter(|event| event.status == crate::audio::PlaybackStatus::Completed)
        .map(|event| event.entity)
        .collect::<BTreeSet<_>>();
    for (entity, mut visual, mut clips, mut player, mut graph) in &mut visuals {
        let Some(d) = definition
            .content
            .generators
            .iter()
            .find(|d| d.id == visual.id)
        else {
            continue;
        };
        let Some(site) = generators.sites.get(&visual.id) else {
            continue;
        };
        let state = site.state;
        let progress = site.progress;
        if visual
            .voice_then
            .as_ref()
            .is_some_and(|(voice, _)| completed.contains(voice))
            && let Some((_, alias)) = visual.voice_then.take()
        {
            crate::audio::emit(
                &mut commands,
                alias,
                crate::audio::SoundSource::attached(entity),
            );
        }
        if let Some((speaker, remaining)) = visual.capture_voice.as_mut() {
            *remaining -= time.delta_secs();
            if *remaining <= 0. {
                crate::audio::voice(
                    &mut commands,
                    &definition,
                    &session,
                    "capture_zombies",
                    *speaker,
                    u32::from(d.number),
                );
                visual.capture_voice = None;
            }
        }
        let index = site.pump_clip;
        let elapsed = index.map_or(0., |index| {
            if matches!(index, 1 | 2) {
                site.pump_elapsed.rem_euclid(d.clip_seconds[index])
            } else {
                site.pump_elapsed.min(d.clip_seconds[index])
            }
        });
        let rotation = site.rotation;
        let desired = match state {
            GeneratorState::Capturing => Some(&d.capture_sound),
            GeneratorState::Active => Some(&d.active_sound),
            _ => None,
        };
        let mut found = false;
        for (e, a) in &audio {
            if a.0 != d.id {
                continue;
            }
            if desired == Some(&a.1) {
                found = true;
                commands.entity(e).insert(crate::audio::SoundControl {
                    volume: 1.,
                    pitch: 0.8 + 0.8 * progress,
                });
            } else {
                commands.entity(e).despawn();
            }
        }
        if !found
            && let Some(id) = desired
            && let Some(alias) = definition.audio.bindings.get(&id.0)
            && let Some(pole) = visual.pole
        {
            let sound = crate::audio::emit_loop(
                &mut commands,
                alias.clone(),
                crate::audio::SoundSource::attached(pole),
            );
            commands.entity(sound).insert((
                GeneratorAudio(d.id.clone(), id.clone()),
                crate::audio::SoundControl {
                    volume: 1.,
                    pitch: 0.8 + 0.8 * progress,
                },
            ));
        }
        if visual.sound_state != Some(state) {
            let position = Vec3::from_array(d.model_position);
            match state {
                GeneratorState::Capturing => {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_capturezone_donut_start",
                        crate::audio::SoundSource::attached(entity),
                    );
                    crate::audio::emit(
                        &mut commands,
                        "vox_maxi_generator_initiate_0",
                        crate::audio::SoundSource::attached(entity),
                    );
                    visual.capture_voice = site.purchaser.map(|speaker| (speaker, 2.5));
                }
                GeneratorState::Active => {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_capturezone_success",
                        crate::audio::SoundSource::attached(entity),
                    );
                    let voice = crate::audio::emit(
                        &mut commands,
                        "vox_maxi_generator_process_complete_0",
                        crate::audio::SoundSource::attached(entity),
                    );
                    visual.voice_then = Some((
                        voice,
                        format!("vox_maxi_generator_{}_activated_0", d.number),
                    ));
                    crate::audio::emit(
                        &mut commands,
                        format!("mus_event_generator_{}", d.number),
                        crate::audio::SoundSource::World(position),
                    );
                }
                GeneratorState::Disabled => {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_capturezone_donut_stop",
                        crate::audio::SoundSource::attached(entity),
                    );
                }
                _ => {}
            }
            visual.sound_state = Some(state);
            visual.sound_clip = usize::MAX;
        }
        let fresh = visual.nodes.is_none();
        if fresh {
            // Source pump, fluid height, and fluid rotation layers from
            // zm_tomb_capture_zones.csc; presentation metadata names only the pump clips.
            let names = [
                "fxanim_zom_tomb_generator_start_anim",
                "fxanim_zom_tomb_generator_up_idle_anim",
                "fxanim_zom_tomb_generator_down_idle_anim",
                "fxanim_zom_tomb_generator_end_anim",
                "fxanim_zom_tomb_generator_fluid_up_anim",
                "fxanim_zom_tomb_generator_fluid_down_anim",
                "fxanim_zom_tomb_generator_fluid_rotate_up_anim",
                "fxanim_zom_tomb_generator_fluid_rotate_down_anim",
            ];
            let nodes: Option<Vec<_>> = names
                .into_iter()
                .map(|name| {
                    match crate::animation::clip_binding(&definition, &visual.source, name)
                        .expect("Generator source clip binding")
                    {
                        Some(path) => clips
                            .node(path, &mut graph, &mut assets)
                            .map(|(node, _)| Some(node)),
                        None => Some(None),
                    }
                })
                .collect();
            let Some(nodes) = nodes else { continue };
            visual.nodes = Some(nodes.try_into().unwrap());
        }
        if !clips.ready {
            continue;
        }
        if let Some(index) = index {
            let previous = if visual.sound_clip != index || elapsed < visual.sound_time {
                -0.001
            } else {
                visual.sound_time
            };
            if let Some(presentation) = definition
                .content
                .presentations
                .iter()
                .find(|p| p.id == d.presentation)
                && let Some(animation) = presentation.animations.get(index)
                && let Some(notes) = definition.audio.timelines.get(&animation.0)
            {
                for note in notes
                    .iter()
                    .filter(|n| n.seconds > previous && n.seconds <= elapsed)
                {
                    crate::audio::emit(
                        &mut commands,
                        note.alias.clone(),
                        crate::audio::SoundSource::attached(entity),
                    );
                }
            }
            visual.sound_clip = index;
            visual.sound_time = elapsed;
        }
        let nodes = visual.nodes.unwrap();
        let mut samples = [None; 8];
        samples[4] = Some((d.clip_seconds[4], progress));
        samples[5] = Some((d.clip_seconds[5], 1. - progress));
        if let Some(index) = index {
            samples[index] = Some((elapsed, 1.));
        }
        if let Some((rotation, elapsed)) = rotation {
            samples[rotation] = Some((elapsed, 1.));
        }
        for (index, node) in nodes.into_iter().enumerate() {
            let target = samples[index].map_or(0., |(_, weight)| weight);
            let current = visual.blend_from[index].lerp(
                visual.blend_target[index],
                (visual.blend_elapsed[index] / 0.2).min(1.),
            );
            if fresh {
                visual.blend_from[index] = target;
                visual.blend_target[index] = target;
                visual.blend_elapsed[index] = 0.2;
            } else if target != visual.blend_target[index] {
                visual.blend_from[index] = current;
                visual.blend_target[index] = target;
                visual.blend_elapsed[index] = 0.;
            }
            visual.blend_elapsed[index] += time.delta_secs();
            let fraction = (visual.blend_elapsed[index] / 0.2).min(1.);
            let weight = visual.blend_from[index].lerp(target, fraction);
            // A source pose with no channels contributes its bind frame below.
            let Some(node) = node else { continue };
            if player.animation(node).is_none() && samples[index].is_some() {
                player.start(node);
            }
            let Some(active) = player.animation_mut(node) else {
                continue;
            };
            active
                .set_repeat(if matches!(index, 1 | 2) {
                    bevy::animation::RepeatAnimation::Forever
                } else {
                    bevy::animation::RepeatAnimation::Never
                })
                .set_weight(weight);
            if let Some((seconds, _)) = samples[index] {
                active.set_seek_time(seconds).pause();
            } else if fraction < 1. {
                active.resume();
            } else {
                player.stop(node);
            }
        }
    }
}

// The fluid-down source pose contains no channels. Bevy blends only present curves,
// so add its bind contribution to these six translation channels after evaluation.
fn fluid_bind_pose(
    visuals: Query<(&GeneratorVisual, &AnimationPlayer)>,
    mut joints: Query<&mut Transform>,
) {
    for (visual, player) in &visuals {
        let Some(nodes) = visual.nodes else { continue };
        let bind_weight = visual.blend_from[5].lerp(
            visual.blend_target[5],
            (visual.blend_elapsed[5] / 0.2).min(1.),
        );
        let animated_weight: f32 = [4, 6, 7]
            .into_iter()
            .filter_map(|index| player.animation(nodes[index]?))
            .map(|active| active.weight())
            .sum();
        let total = animated_weight + bind_weight;
        let fraction = if total > 0. {
            animated_weight / total
        } else {
            0.
        };
        for &(joint, bind) in &visual.fluid_joints {
            let mut pose = joints.get_mut(joint).expect("Bound generator fluid joint");
            pose.translation = bind.lerp(pose.translation, fraction);
        }
    }
}

#[cfg(test)]
mod source_capture_tests {
    use super::*;

    #[test]
    fn capture_rewards_preserve_source_round_cap_and_reset() {
        let mut generators = Generators::default();
        assert_eq!(
            (0..15)
                .map(|_| generators.capture_kill_points(1))
                .sum::<u32>(),
            100
        );
        assert_eq!(generators.capture_kill_points(1), 0);
        assert_eq!(
            (0..25)
                .map(|_| generators.capture_kill_points(2))
                .sum::<u32>(),
            200
        );
        assert_eq!(generators.capture_kill_points(2), 0);
    }

    #[test]
    fn source_emergence_selects_every_authored_point_before_reuse() {
        let id = ContentId("generator.generator_start_bunker".into());
        let mut generators = Generators::default();
        let mut session = Session::default();
        for _ in 0..4 {
            let cycle = (0..6)
                .map(|_| generators.emergence_index(&id, 6, &mut session).unwrap())
                .collect::<BTreeSet<_>>();
            assert_eq!(cycle, (0..6).collect());
        }
        assert!(generators.emergence_index(&id, 0, &mut session).is_none());
    }

    #[test]
    fn capture_progress_preserves_immediate_source_poll_and_percentage_rounding() {
        let mut capture = Capture::default();
        capture.advance(0., 12., 20., true);
        assert!((capture.progress - 1. / 120.).abs() < 0.000001);
        let before = capture.progress;
        capture.advance(0.099, 12., 20., true);
        assert_eq!(capture.progress, before);
        capture.advance(0.002, 12., 20., true);
        assert!(capture.progress > before);
        for _ in 2..120 {
            capture.advance(0.1, 12., 20., true);
        }
        assert_eq!(capture.progress, 1.);
        capture.advance(0.1, 12., 20., false);
        assert!((capture.progress - 0.995).abs() < 0.000001);
    }
}
