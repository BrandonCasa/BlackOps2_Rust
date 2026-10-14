use crate::{
    GameState,
    animation::{AnimationGraphs, AnimationLayer, AnimationPlayback},
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
mod raids;
mod rewards;
pub use raids::GeneratorRaids;
pub(crate) use rewards::BoxGlowRamp;

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
    fire_reward: bool,
    retirement_pending: bool,
    spawn_clock: f32,
    spawn_cursor: usize,
}
impl Generators {
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
pub struct GeneratorVisual {
    id: ContentId,
    source: Handle<Gltf>,
    graph: Option<(Handle<AnimationGraph>, Vec<AnimationNodeIndex>)>,
    sound_state: Option<GeneratorState>,
    sound_clip: usize,
    sound_time: f32,
    players: Vec<Entity>,
    recorded_clips: u8,
}
#[derive(Component)]
struct GeneratorAudio(ContentId, ContentId);
#[derive(Component)]
struct CaptureRetirement;

#[derive(Component)]
pub struct GeneratorDefender {
    pub generator: ContentId,
    pub slot: usize,
}
pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(rewards::CaptureRewardsPlugin)
            .init_resource::<Generators>()
            .add_systems(
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
            .init_resource::<raids::GeneratorRaids>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                raids::reset.in_set(crate::session::SessionSetup::World),
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
            )
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                assemble.in_set(crate::session::SessionSetup::World),
            )
            .add_observer(visual_ready)
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
    }
}
fn defender_intent(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    players: Query<(&Transform, &Vitality), With<Player>>,
    defenders: Query<
        (
            Entity,
            &GeneratorDefender,
            Option<&crate::enemies::EnemyGoal>,
        ),
        Without<Player>,
    >,
) {
    for (entity, guard, current_goal) in &defenders {
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
        if players.iter().any(|(t, v)| {
            v.state() == LifeState::Standing
                && t.translation.distance_squared(origin) < 17.78 * 17.78
        }) {
            if current_goal.is_some() {
                commands
                    .entity(entity)
                    .remove::<crate::enemies::EnemyGoal>();
            }
        } else {
            let rotation = Quat::from_rotation_y(p.yaw);
            let offset =
                rotation * [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z][guard.slot % 4] * 1.3208;
            let goal = crate::enemies::EnemyGoal {
                position: origin + offset + Vec3::Y * 0.85,
                face: origin + Vec3::Y * 0.85,
                radius: 0.762,
            };
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
    mut generators: ResMut<Generators>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    actors: Query<
        (Entity, &Vitality, &crate::rounds::PopulationKind),
        With<crate::enemies::Zombie>,
    >,
    retiring: Query<Entity, With<CaptureRetirement>>,
    mut barriers: ResMut<crate::barriers::Barriers>,
) {
    for entity in &retiring {
        commands.entity(entity).despawn();
        generators.excess_retired += 1;
    }
    if !generators.retirement_pending {
        return;
    }
    let candidates: Vec<_> = actors
        .iter()
        .filter_map(|(entity, health, kind)| {
            (health.state() == LifeState::ActorAlive
                && *kind == crate::rounds::PopulationKind::Ordinary)
                .then_some(entity)
        })
        .collect();
    let cap = (definition.rounds.ordinary_cap as usize).saturating_sub(generators.reserve());
    if candidates.len() <= cap {
        generators.retirement_pending = false;
        return;
    }
    let entity = candidates[session.random() as usize % candidates.len()];
    barriers.release(entity);
    commands
        .entity(entity)
        .remove::<Vitality>()
        .insert((CaptureRetirement, avian3d::prelude::ColliderDisabled));
}

fn clear_defenders(
    mut generators: ResMut<Generators>,
    mut defenders: Query<
        (&mut Vitality, &GeneratorDefender),
        (With<crate::enemies::Zombie>, With<Transform>),
    >,
) {
    for (mut health, guard) in &mut defenders {
        if generators
            .sites
            .get(&guard.generator)
            .is_none_or(|s| s.state != GeneratorState::Capturing)
            && health.state() == LifeState::ActorAlive
            && let Ok(damage) = crate::life::ResolvedDamage::new(health.current().unwrap_or(1))
        {
            let _ = health.apply(damage);
            generators.defenders_cleared += 1;
        }
    }
}
fn defenders(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut generators: ResMut<Generators>,
    session: Res<Session>,
    navigation: Res<crate::navigation::Navigation>,
    access: Res<crate::access::ZoneAccess>,
    query: avian3d::prelude::SpatialQuery,
    actors: Query<
        (&Vitality, &Transform, Option<&GeneratorDefender>),
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
        .filter(|(v, _, _)| v.state() == LifeState::ActorAlive)
        .collect();
    let slots: Vec<_> = alive
        .iter()
        .filter_map(|(_, _, g)| g.filter(|g| g.generator == id).map(|g| g.slot))
        .collect();
    if slots.len() >= 4 || alive.len() >= definition.rounds.actor_cap as usize {
        return;
    }
    let Some(d) = definition.content.generators.iter().find(|d| d.id == id) else {
        return;
    };
    let variants: Vec<_> = definition
        .content
        .enemies
        .iter()
        .filter(|e| e.family == "crusader")
        .collect();
    let Some(enemy) = variants.get(generators.spawn_cursor % variants.len().max(1)) else {
        return;
    };
    let Some(slot) = (0..4).find(|s| !slots.contains(s)) else {
        return;
    };
    let native = Vec3::from_array(d.emergence[slot]);
    let Some(base) = navigation.project_emergence(native, &access) else {
        generators.defender_failures += 1;
        return;
    };
    let position = base + Vec3::Y * 0.85;
    let shape = crate::player::capsule(0.3, 1.7);
    if alive.iter().any(|(_, t, _)| {
        (t.translation - position).xz().length_squared() < 0.5625
            && (t.translation.y - position.y).abs() < 1.
    }) || !query
        .shape_intersections(
            &shape,
            position,
            Quat::IDENTITY,
            &crate::environment::solids([]),
        )
        .is_empty()
    {
        generators.defender_failures += 1;
        return;
    }
    let health = definition.rounds.health(session.round);
    let Ok(health) = Vitality::ordinary_actor(health, health) else {
        return;
    };
    let mut zombie =
        crate::enemies::Zombie::new(position, enemy.id.clone(), crate::rounds::Gait::Run);
    let rise = enemy
        .traversals
        .get("rise")
        .and_then(|profiles| profiles.first());
    if let Some(rise) = rise {
        zombie = zombie.rising(rise);
    }
    let mut actor = commands.spawn((
        SessionEntity,
        zombie,
        shape,
        health,
        crate::rounds::PopulationKind::GeneratorDefender,
        GeneratorDefender {
            generator: id,
            slot,
        },
        Transform::from_translation(position),
        Visibility::Inherited,
        Name::new("Generator defender"),
    ));
    if rise.is_some() {
        actor.insert(ColliderDisabled);
    }
    generators.defenders_admitted += 1;
    generators.spawn_cursor += 1;
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
    for d in &definition.content.generators {
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
                source: server.load(scene.split('#').next().unwrap_or(scene).to_owned()),
                graph: None,
                sound_state: None,
                sound_clip: usize::MAX,
                sound_time: -0.001,
                players: Vec::new(),
                recorded_clips: 0,
            },
            WorldAssetRoot(server.load(scene.clone())),
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
    players: Query<(), With<AnimationPlayer>>,
) {
    let Ok(mut visual) = visuals.get_mut(event.entity) else {
        return;
    };
    visual.players = children
        .iter_descendants(event.entity)
        .filter(|child| players.contains(*child))
        .collect();
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
    for d in &definition.content.generators {
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
            site.progress = (site.progress
                + if inside(None) {
                    dt / d.capture_seconds
                } else {
                    -dt / d.decay_seconds
                })
            .clamp(0., 1.);
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
        if let Some(delay) = site.refund.as_mut() {
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
            chests.queue(0, 2.);
        }
        if completed && d.number == 6 && !generators.fire_reward {
            generators.fire_reward = true;
            chests.queue(1, 0.);
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
    session: Res<Session>,
    definition: Res<GameDefinition>,
    mut generators: ResMut<Generators>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut graph_cache: ResMut<AnimationGraphs>,
    mut visuals: Query<(Entity, &mut GeneratorVisual)>,
    audio: Query<(Entity, &GeneratorAudio)>,
) {
    let _span = info_span!("generator_presentation").entered();
    for (entity, mut visual) in &mut visuals {
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
            } else {
                commands.entity(e).despawn();
            }
        }
        if !found
            && let Some(id) = desired
            && let Some(alias) = definition.audio.bindings.get(&id.0)
        {
            let sound = crate::audio::emit_loop(
                &mut commands,
                alias.clone(),
                crate::audio::SoundSource::attached(entity),
            );
            commands
                .entity(sound)
                .insert(GeneratorAudio(d.id.clone(), id.clone()));
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
                    if let Some(speaker) = site.purchaser {
                        crate::audio::voice(
                            &mut commands,
                            &definition,
                            &session,
                            "capture_zones",
                            speaker,
                            u32::from(d.number),
                        );
                    }
                }
                GeneratorState::Active => {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_capturezone_success",
                        crate::audio::SoundSource::attached(entity),
                    );
                    crate::audio::emit(
                        &mut commands,
                        format!("mus_event_generator_{}", d.number),
                        crate::audio::SoundSource::World(position),
                    );
                    crate::audio::emit(
                        &mut commands,
                        "zmb_capturezone_donut_stop",
                        crate::audio::SoundSource::attached(entity),
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
        if visual.graph.is_none() {
            let Some(bindings) = graph_cache.scene(&visual.source, &gltfs, &mut graphs) else {
                continue;
            };
            if bindings.ordered.len() != 8 {
                continue;
            }
            visual.graph = Some((bindings.graph.clone(), bindings.ordered.clone()));
            generators.animated += 1;
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
        let Some((graph, nodes)) = &visual.graph else {
            continue;
        };
        let mut layers = vec![
            AnimationLayer::new(nodes[4], Some(d.clip_seconds[4]), false).weighted(progress),
            AnimationLayer::new(nodes[5], Some(d.clip_seconds[5]), false).weighted(1. - progress),
        ];
        let mut recorded_clips = visual.recorded_clips | (1 << 4) | (1 << 5);
        if let Some(index) = index {
            layers.push(AnimationLayer::new(
                nodes[index],
                Some(elapsed),
                matches!(index, 1 | 2),
            ));
            recorded_clips |= 1 << index;
        }
        if let Some((rotation, elapsed)) = rotation {
            layers.push(AnimationLayer::new(nodes[rotation], Some(elapsed), false));
            recorded_clips |= 1 << rotation;
        }
        let mut playback = AnimationPlayback::new(graph.clone(), layers);
        playback.blend_seconds = 0.2;
        for &child in &visual.players {
            commands.entity(child).insert(playback.clone());
        }
        for clip in 0..8 {
            if recorded_clips & (1 << clip) != 0 && visual.recorded_clips & (1 << clip) == 0 {
                generators.playback.insert(format!("{}:{clip}", d.id.0));
            }
        }
        visual.recorded_clips = recorded_clips;
    }
}
