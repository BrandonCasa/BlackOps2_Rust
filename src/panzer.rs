use crate::{
    GameState,
    assets::GameDefinition,
    combat_effects::Armor,
    damage::{DamageApplied, DamageQueue, EnemyDeath, Impact, ImpactSource},
    enemies::Zombie,
    gameplay::{GameStep, Session, SessionEntity},
    life::{LifeState, Vitality},
    origins::content::{ContentId, DamageCategory, WeaponId},
    player::Player,
    rounds::{Gait, PopulationKind},
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::BTreeSet;
mod source;

#[derive(Component, Default)]
pub struct CapturedByPanzer(pub Option<Entity>);
#[derive(Clone, Copy, Debug)]
pub enum PanzerPhase {
    Landing(f32),
    Pursuit,
    Melee(f32),
    FlameWindup(f32),
    Flame(f32),
    Claw {
        tip: Vec3,
        direction: Vec3,
        remaining: f32,
    },
    Pulling(f32),
    HeldBurn(f32),
    Stunned(f32),
    Dead(f32),
}
#[derive(Component)]
#[require(PanzerFeedback)]
pub struct Panzer {
    pub phase: PanzerPhase,
    pub quest: bool,
    pub captured: Option<Entity>,
    flame_cooldown: f32,
    claw_cooldown: f32,
    flame_tick: f32,
    claw_baseline: u32,
    plate_rewarded: bool,
    core_rewarded: bool,
    vocal_timer: f32,
    drop_powerup: bool,
    death_attacker: Option<Entity>,
    pub animation_elapsed: f32,
    exploded: bool,
}
impl Panzer {
    pub fn animation(&self) -> (&'static str, bool) {
        match self.phase {
            PanzerPhase::Landing(_) => ("ai_zombie_mech_intro_jump_in", false),
            PanzerPhase::Pursuit => ("ai_zombie_mech_walk_basic", true),
            PanzerPhase::Melee(_) => ("ai_zombie_mech_melee_a", false),
            PanzerPhase::FlameWindup(_) => ("ai_zombie_mech_ft_fire_start", false),
            PanzerPhase::Flame(_) => ("ai_zombie_mech_ft_fire_loop", true),
            PanzerPhase::Claw { .. } => ("ai_zombie_mech_grapple_intro_sprint_to_aim_5", false),
            PanzerPhase::Pulling(_) => ("ai_zombie_mech_grapple_aim_5", true),
            PanzerPhase::HeldBurn(_) => ("ai_zombie_mech_ft_burn_player", false),
            PanzerPhase::Stunned(_) => ("ai_zombie_mech_stunned", true),
            PanzerPhase::Dead(_) => ("ai_zombie_mech_death", false),
        }
    }
}
#[derive(Message)]
pub struct PanzerClawReleased {
    pub releaser: Entity,
    pub captive: Entity,
}
#[derive(Message)]
pub struct PanzerDestroyed {
    pub entity: Entity,
    pub position: Vec3,
    pub quest: bool,
    pub attacker: Entity,
}
#[derive(Component)]
pub struct PanzerNuke {
    remaining: f32,
    attacker: Entity,
}
pub fn queue_nuke(commands: &mut Commands, target: Entity, attacker: Entity, delay: f32) {
    commands.entity(target).insert(PanzerNuke {
        remaining: delay,
        attacker,
    });
}
#[derive(Component)]
struct FlameIgnited {
    remaining: f32,
    pulse: f32,
    sound: Entity,
}

#[derive(Component)]
struct FlameVolume {
    owner: Option<Entity>,
}
#[derive(Component)]
struct ClawTrigger(Entity);

fn bind_triggers(
    mut commands: Commands,
    game: Res<GameDefinition>,
    panzers: Query<(Entity, &Panzer)>,
    rigs: Query<&crate::enemies::ActorRig>,
    sockets: Query<&GlobalTransform>,
    mut volumes: Query<(
        Entity,
        &crate::environment::triggers::MapTrigger,
        &mut Transform,
        Option<&mut FlameVolume>,
        &mut crate::environment::triggers::TriggerState,
    )>,
    mut claws: Query<
        (
            Entity,
            &ClawTrigger,
            &mut Transform,
            &mut crate::environment::triggers::TriggerState,
        ),
        Without<crate::environment::triggers::MapTrigger>,
    >,
) {
    let mut occupied = BTreeSet::new();
    for (_, _, _, volume, _) in &volumes {
        if let Some(owner) = volume.and_then(|volume| volume.owner)
            && panzers
                .get(owner)
                .is_ok_and(|(_, panzer)| !matches!(panzer.phase, PanzerPhase::Dead(_)))
        {
            occupied.insert(owner);
        }
    }
    for (entity, trigger, mut pose, volume, mut state) in &mut volumes {
        let Some(source) = game.environment.triggers.iter().find(|source| {
            source.id == trigger.0
                && source
                    .source
                    .get("script_noteworthy")
                    .is_some_and(|name| name == "flamethrower_trigger")
        }) else {
            continue;
        };
        let mut owner = volume
            .as_ref()
            .and_then(|volume| volume.owner)
            .filter(|owner| occupied.contains(owner));
        if owner.is_none() {
            owner = panzers
                .iter()
                .find(|(owner, panzer)| {
                    !occupied.contains(owner)
                        && matches!(
                            panzer.phase,
                            PanzerPhase::FlameWindup(_)
                                | PanzerPhase::Flame(_)
                                | PanzerPhase::HeldBurn(_)
                        )
                        && rigs
                            .iter()
                            .any(|rig| rig.socket(*owner, "tag_flamethrower_FX").is_some())
                })
                .map(|(owner, _)| owner);
            if let Some(owner) = owner {
                occupied.insert(owner);
            }
        }
        state.automatic = false;
        state.enabled = owner.is_some();
        if let Some(owner) = owner
            && let Some(socket) = rigs
                .iter()
                .find_map(|rig| rig.socket(owner, "tag_flamethrower_FX"))
            && let Ok(socket) = sockets.get(socket)
        {
            *pose = socket.compute_transform();
        } else {
            *pose = source.transform();
        }
        if let Some(mut volume) = volume {
            volume.owner = owner;
        } else {
            commands.entity(entity).insert(FlameVolume { owner });
        }
    }
    let mut claw_owners: BTreeSet<_> = claws.iter().map(|(_, claw, _, _)| claw.0).collect();
    for (entity, claw, mut pose, mut state) in &mut claws {
        let Ok((_, panzer)) = panzers.get(claw.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        if matches!(panzer.phase, PanzerPhase::Dead(_)) {
            commands.entity(entity).despawn();
            continue;
        }
        state.enabled = true;
        if let Some(socket) = rigs.iter().find_map(|rig| rig.socket(claw.0, "tag_claw"))
            && let Ok(socket) = sockets.get(socket)
        {
            *pose = socket.compute_transform();
        }
    }
    for (owner, panzer) in &panzers {
        if claw_owners.contains(&owner) || matches!(panzer.phase, PanzerPhase::Dead(_)) {
            continue;
        }
        let Some(socket) = rigs.iter().find_map(|rig| rig.socket(owner, "tag_claw")) else {
            continue;
        };
        let Ok(socket) = sockets.get(socket) else {
            continue;
        };
        let definition = crate::environment::triggers::TriggerDefinition {
            id: format!("script.panzer.claw.{}", owner.to_bits()),
            position: socket.translation().to_array(),
            target_position: None,
            source: [
                ("classname".into(), "trigger_damage".into()),
                ("spawnflags".into(), "0".into()),
            ]
            .into(),
            shape: crate::environment::triggers::TriggerShape::Cylinder {
                radius: 3. * 0.0254,
                height: 15. * 0.0254,
            },
        };
        if let Some(entity) = crate::environment::triggers::spawn_owned(
            &mut commands,
            &definition,
            owner,
            crate::environment::support::EnvironmentOwner::Enemy,
        ) {
            commands
                .entity(entity)
                .remove::<crate::environment::triggers::MapTrigger>()
                .insert((ClawTrigger(owner), socket.compute_transform()));
            claw_owners.insert(owner);
        }
    }
}

fn claw_hits(
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    triggers: Query<(&ClawTrigger, &crate::environment::triggers::TriggerState)>,
    panzers: Query<&Panzer>,
    players: Query<&Vitality, With<Player>>,
    mut damage: ResMut<DamageQueue>,
) {
    for hit in hits.read() {
        let Ok((claw, state)) = triggers.get(hit.target) else {
            continue;
        };
        if !state.enabled
            || !players
                .get(hit.attacker)
                .is_ok_and(|health| health.state() == LifeState::Standing)
            || !panzers.get(claw.0).is_ok_and(|panzer| {
                matches!(
                    panzer.phase,
                    PanzerPhase::Claw { .. } | PanzerPhase::Pulling(_)
                )
            })
        {
            continue;
        }
        damage.0.push(Impact {
            attacker: hit.attacker,
            target: claw.0,
            source: ImpactSource::Weapon(hit.weapon.clone()),
            category: hit.category,
            region: HitRegion::LeftArm,
            amount: 1,
        });
    }
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    #[test]
    fn environment_trigger_query_access() {
        let mut world = World::new();
        IntoSystem::into_system(bind_triggers).initialize(&mut world);
        IntoSystem::into_system(claw_hits).initialize(&mut world);
        IntoSystem::into_system(behavior).initialize(&mut world);
    }
}
#[derive(Resource)]
pub struct PanzerWaves {
    pub next_round: u32,
    pub pending: u32,
    pub spawned: u32,
    pub killed: u32,
    pub fire_part_dropped: bool,
    timer: f32,
    activated: bool,
    used: BTreeSet<usize>,
}
impl Default for PanzerWaves {
    fn default() -> Self {
        Self {
            next_round: 8,
            pending: 0,
            spawned: 0,
            killed: 0,
            fire_part_dropped: false,
            timer: 0.,
            activated: false,
            used: BTreeSet::new(),
        }
    }
}
pub fn health_for_wave(waves: &PanzerWaves) -> u32 {
    (5000 + 1000 * waves.spawned).min(22500)
}
pub struct PanzerPlugin;
impl Plugin for PanzerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanzerWaves>()
            .add_message::<PanzerClawReleased>()
            .add_message::<PanzerDestroyed>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                reset.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                schedule
                    .in_set(GameStep::Rounds)
                    .before(crate::rounds::advance)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    bind_triggers,
                    nukes,
                    behavior,
                    burning,
                    feedback,
                    part_failsafe,
                )
                    .chain()
                    .in_set(GameStep::EnemyIntent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                claw_hits
                    .in_set(GameStep::Combat)
                    .after(crate::gameplay::combat)
                    .after(crate::projectiles::flight)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                deaths
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(mut waves: ResMut<PanzerWaves>) {
    *waves = PanzerWaves::default();
}
pub fn spawn(
    commands: &mut Commands,
    _game: &GameDefinition,
    _server: &AssetServer,
    position: Vec3,
    health: u32,
    quest: bool,
) -> Entity {
    let entity = commands
        .spawn((
            Panzer {
                phase: PanzerPhase::Landing(5.8333335),
                quest,
                captured: None,
                flame_cooldown: 2.,
                claw_cooldown: 5.,
                flame_tick: 0.,
                claw_baseline: 0,
                plate_rewarded: false,
                core_rewarded: false,
                vocal_timer: 3.,
                drop_powerup: false,
                death_attacker: None,
                animation_elapsed: 0.,
                exploded: false,
            },
            Zombie::new(
                position + Vec3::Y * 1.2,
                ContentId("enemy.panzer".into()),
                Gait::Run,
                2.5,
            ),
            SessionEntity,
            Collider::capsule(0.5, 1.4),
            ColliderDisabled,
            Transform::from_translation(position + Vec3::Y * 1.2),
            Visibility::Inherited,
            Vitality::ordinary_actor(health, health).expect("source Panzer health"),
            Armor::panzer(health),
            crate::combat_effects::Wounds::default(),
            crate::hit_regions::Hitboxes::panzer(),
        ))
        .id();
    if !quest {
        commands.entity(entity).insert(PopulationKind::Special);
    }
    crate::audio::emit(
        commands,
        "zmb_ai_mechz_incoming_alarm",
        crate::audio::SoundSource::attached(entity),
    );
    entity
}
fn schedule(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut session: ResMut<Session>,
    mut waves: ResMut<PanzerWaves>,
    access: Res<crate::access::ZoneAccess>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    query: SpatialQuery,
    players: Query<&Transform, With<Player>>,
    panzers: Query<&Vitality, With<Panzer>>,
    mut rounds: MessageReader<crate::rounds::RoundStarted>,
) {
    let round_started = rounds.read().count() > 0;
    if round_started {
        waves.used.clear();
    }
    if !access.reachable.iter().any(|z| z.0.starts_with("zone.nml")) {
        return;
    }
    let first_activation = !waves.activated;
    waves.activated = true;
    if (first_activation || round_started)
        && waves.pending == 0
        && session.round >= waves.next_round
    {
        if panzers.iter().any(|v| v.state() == LifeState::ActorAlive) {
            waves.next_round += 1;
        } else {
            waves.pending = 1;
            waves.timer = 10. + session.random() as f32 / u32::MAX as f32 * 5.;
        }
    }
    if waves.pending == 0 {
        return;
    }
    waves.timer -= time.delta_secs();
    if waves.timer > 0. {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let mut choices: Vec<_> = source::SPAWNS
        .iter()
        .enumerate()
        .filter(|(index, _)| !waves.used.contains(index))
        .collect();
    choices.sort_by(|a, b| {
        Vec3::from_array(a.1.1)
            .distance_squared(player.translation)
            .total_cmp(&Vec3::from_array(b.1.1).distance_squared(player.translation))
    });
    let Some((index, (_, point))) = choices.into_iter().find(|(_, (_, point))| {
        query
            .shape_intersections(
                &Collider::capsule(0.5, 1.4),
                Vec3::from_array(*point) + Vec3::Y * 1.2,
                Quat::IDENTITY,
                &crate::environment::solids([]),
            )
            .is_empty()
    }) else {
        waves.timer = 0.5;
        return;
    };
    let entity = spawn(
        &mut commands,
        &game,
        &server,
        Vec3::from_array(*point),
        health_for_wave(&waves),
        false,
    );
    let later = waves.spawned > 0;
    commands.queue(move |world: &mut World| {
        if let Some(mut panzer) = world.get_mut::<Panzer>(entity) {
            panzer.drop_powerup = later;
        }
    });
    waves.used.insert(index);
    waves.spawned += 1;
    waves.pending -= 1;
    waves.next_round = session.round + 4 + session.random() % 2;
}
fn strike(
    damage: &mut DamageQueue,
    attacker: Entity,
    target: Entity,
    amount: u32,
    category: DamageCategory,
) {
    damage.0.push(Impact {
        attacker,
        target,
        source: ImpactSource::Enemy(ContentId("enemy.panzer".into())),
        category,
        region: HitRegion::Torso,
        amount,
    });
}
fn nukes(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut targets: Query<(Entity, &Vitality, &mut PanzerNuke)>,
    mut damage: ResMut<DamageQueue>,
) {
    for (target, health, mut nuke) in &mut targets {
        nuke.remaining -= time.delta_secs();
        if nuke.remaining <= 0. {
            if health.state() == LifeState::ActorAlive {
                damage.0.push(Impact {
                    attacker: nuke.attacker,
                    target,
                    source: ImpactSource::Powerup,
                    category: DamageCategory::Explosive,
                    region: HitRegion::None,
                    amount: (health.current().unwrap_or(0) as f32 * 0.25).round() as u32,
                });
            }
            commands.entity(target).remove::<PanzerNuke>();
        }
    }
}
fn release(panzer: &mut Panzer, captured: &mut CapturedByPanzer) {
    captured.0 = None;
    panzer.captured = None;
    panzer.phase = PanzerPhase::Pursuit;
}
fn behavior(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    query: SpatialQuery,
    slide: MoveAndSlide,
    mut panzers: Query<
        (
            Entity,
            &mut Panzer,
            &mut Transform,
            &Collider,
            &Vitality,
            &mut Armor,
        ),
        Without<Player>,
    >,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &Collider,
            &Vitality,
            &mut CapturedByPanzer,
            &mut crate::crafting::BuildableInventory,
            &crate::solo_revive::PerkInventory,
        ),
        (With<Player>, Without<Panzer>),
    >,
    mut damage: ResMut<DamageQueue>,
    mut received: MessageReader<DamageApplied>,
    mut navigation: ResMut<crate::navigation::Navigation>,
    access: Res<crate::access::ZoneAccess>,
    effects: Res<crate::powerups::PowerupEffects>,
    mut session: ResMut<Session>,
    game: Res<GameDefinition>,
    mut released: MessageWriter<PanzerClawReleased>,
    (mut ignitable, flame_volumes): (
        Query<
            (Entity, &Transform, &Vitality, Has<FlameIgnited>),
            (With<Zombie>, Without<Player>, Without<Panzer>),
        >,
        Query<(&FlameVolume, &Collider, &Transform), (Without<Panzer>, Without<Player>)>,
    ),
) {
    let dt = time.delta_secs();
    for event in received.read() {
        if let Ok((_, mut panzer, _, _, health, armor)) = panzers.get_mut(event.impact.target) {
            let explosive = matches!(event.impact.category, DamageCategory::Explosive)
                || event.impact.source.id().contains("staff_");
            if panzer.captured.is_some()
                && (event.impact.region == HitRegion::LeftArm
                    || explosive
                        && armor.explosive_damage.saturating_sub(panzer.claw_baseline)
                            > health.maximum() / 10)
            {
                if let Some(owner) = panzer.captured
                    && let Ok((_, _, _, _, _, mut captured, _, _)) = players.get_mut(owner)
                {
                    release(&mut panzer, &mut captured);
                    if event.impact.source.player_attributed() {
                        released.write(PanzerClawReleased {
                            releaser: event.impact.attacker,
                            captive: owner,
                        });
                    }
                }
            }
            if armor.broken && !panzer.plate_rewarded {
                panzer.plate_rewarded = true;
                if event.impact.source.player_attributed() {
                    session.award(100);
                }
            }
            if armor.powerplant_destroyed && !panzer.core_rewarded {
                panzer.core_rewarded = true;
                if event.impact.source.player_attributed() {
                    session.award(100);
                }
                if let Some(owner) = panzer.captured
                    && let Ok((_, _, _, _, _, mut captured, _, _)) = players.get_mut(owner)
                {
                    release(&mut panzer, &mut captured);
                }
                panzer.phase = PanzerPhase::Stunned(4.);
            }
        }
    }
    for (entity, mut panzer, mut pose, collider, health, mut armor) in &mut panzers {
        armor.ice_cooldown = (armor.ice_cooldown - dt).max(0.);
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        let previous = std::mem::discriminant(&panzer.phase);
        panzer.flame_cooldown = (panzer.flame_cooldown - dt).max(0.);
        panzer.claw_cooldown = (panzer.claw_cooldown - dt).max(0.);
        panzer.vocal_timer -= dt;
        if panzer.vocal_timer <= 0. {
            panzer.vocal_timer = 3. + session.random() as f32 / u32::MAX as f32 * 3.;
            crate::audio::emit(
                &mut commands,
                "zmb_ai_mechz_vox_ambient",
                crate::audio::SoundSource::attached(entity),
            );
        }
        let independent = match panzer.phase {
            PanzerPhase::Landing(remaining) => {
                panzer.phase = if remaining > dt {
                    PanzerPhase::Landing(remaining - dt)
                } else {
                    commands.entity(entity).remove::<ColliderDisabled>();
                    PanzerPhase::Pursuit
                };
                true
            }
            PanzerPhase::Stunned(remaining) => {
                panzer.phase = if remaining > dt {
                    PanzerPhase::Stunned(remaining - dt)
                } else {
                    PanzerPhase::Pursuit
                };
                true
            }
            _ => false,
        };
        if independent {
            panzer.animation_elapsed = if previous == std::mem::discriminant(&panzer.phase) {
                panzer.animation_elapsed + dt
            } else {
                0.
            };
            continue;
        }
        let Ok((
            owner,
            mut player,
            mut player_pose,
            player_collider,
            vitality,
            mut captured,
            mut shield,
            perks,
        )) = players.single_mut()
        else {
            continue;
        };
        if vitality.state() != LifeState::Standing {
            if panzer.captured == Some(owner) {
                release(&mut panzer, &mut captured);
            }
            continue;
        }
        if effects.zombie_blood > 0. && panzer.captured.is_none() {
            continue;
        }
        let delta = player_pose.translation - pose.translation;
        let distance = delta.length();
        let direction = delta.with_y(0.).normalize_or_zero();
        let forward = pose.rotation * Vec3::NEG_Z;
        let facing = forward.dot(direction);
        let visible = Dir3::new(delta).ok().is_some_and(|d| {
            query
                .cast_ray(
                    pose.translation,
                    d,
                    distance,
                    true,
                    &crate::environment::solids([entity, owner]),
                )
                .is_none()
        });
        let underground = game
            .access
            .volumes
            .iter()
            .filter(|v| v.zone.0 == "zone.ug_bottom_zone")
            .any(|v| {
                Collider::convex_hull(
                    &v.volume
                        .vertices
                        .iter()
                        .copied()
                        .map(Vec3::from_array)
                        .collect::<Vec<_>>(),
                )
                .is_some_and(|shape| {
                    shape.contains_point(
                        Vec3::from_array(v.volume.position),
                        Quat::IDENTITY,
                        pose.translation,
                    )
                })
            });
        match panzer.phase {
            PanzerPhase::Pursuit => {
                if visible && distance < 1.9 {
                    panzer.phase = PanzerPhase::Melee(1.5);
                } else if visible
                    && (2.54..5.68).contains(&distance)
                    && facing > 26_f32.to_radians().cos()
                    && panzer.flame_cooldown == 0.
                {
                    panzer.phase = PanzerPhase::FlameWindup(0.16666667);
                    panzer.flame_cooldown = 5.;
                } else if visible
                    && (7.62..=25.4).contains(&distance)
                    && facing > 45_f32.to_radians().cos()
                    && !underground
                    && !player.prone
                    && !armor.powerplant_destroyed
                    && panzer.claw_cooldown == 0.
                    && captured.0.is_none()
                {
                    panzer.phase = PanzerPhase::Claw {
                        tip: pose.translation + Vec3::Y * 0.4,
                        direction: delta.normalize_or_zero(),
                        remaining: 1.5,
                    };
                    panzer.claw_cooldown = 7.;
                    crate::audio::emit(
                        &mut commands,
                        "zmb_ai_mechz_claw_fire",
                        crate::audio::SoundSource::attached(entity),
                    );
                } else {
                    let feet = pose.translation - Vec3::Y * 1.2;
                    let target = navigation.destination(
                        feet,
                        player_pose.translation - Vec3::Y * player.height * 0.5,
                        &access,
                    ) + Vec3::Y * 1.2;
                    let velocity = (target - pose.translation).with_y(0.).normalize_or_zero()
                        * if distance > 9.144 { 3. } else { 1.8 };
                    let result = crate::player::slide_character(
                        &slide,
                        &query,
                        crate::player::MotionShape {
                            collider,
                            radius: 0.5,
                            step_height: 0.45,
                            grounded: true,
                        },
                        pose.translation,
                        velocity - Vec3::Y * 2.,
                        time.delta(),
                        &crate::environment::solids([entity]),
                    );
                    pose.translation = result.position;
                }
                if direction.length_squared() > 0. {
                    let desired = Quat::from_rotation_y((-direction.x).atan2(-direction.z));
                    pose.rotation = pose
                        .rotation
                        .rotate_towards(desired, 180_f32.to_radians() * dt);
                }
            }
            PanzerPhase::Melee(remaining) => {
                if remaining > 1. && remaining - dt <= 1. && distance < 2.2 && visible {
                    strike(&mut damage, entity, owner, 75, DamageCategory::Melee);
                }
                if remaining > dt {
                    panzer.phase = PanzerPhase::Melee(remaining - dt);
                } else {
                    panzer.phase = PanzerPhase::Pursuit;
                }
            }
            PanzerPhase::FlameWindup(remaining) => {
                panzer.phase = if remaining > dt {
                    PanzerPhase::FlameWindup(remaining - dt)
                } else {
                    panzer.flame_tick = 0.;
                    PanzerPhase::Flame(2.5)
                };
            }
            PanzerPhase::Flame(remaining) | PanzerPhase::HeldBurn(remaining) => {
                let held = matches!(panzer.phase, PanzerPhase::HeldBurn(_));
                let touching = flame_volumes
                    .iter()
                    .find(|(volume, _, _)| volume.owner == Some(entity))
                    .map(|(_, shape, pose)| {
                        query.shape_intersections(
                            shape,
                            pose.translation,
                            pose.rotation,
                            &SpatialQueryFilter::default().with_excluded_entities([entity]),
                        )
                    })
                    .unwrap_or_default();
                panzer.flame_tick -= dt;
                if panzer.flame_tick <= 0. {
                    panzer.flame_tick += 0.5;
                    if touching.contains(&owner) {
                        let juggernog = perks.owned.iter().any(|id| id.0 == "perk.juggernog");
                        strike(
                            &mut damage,
                            entity,
                            owner,
                            if juggernog { 45 } else { 30 },
                            DamageCategory::Fire,
                        );
                    }
                }
                for (target, _, vitality, burning) in &mut ignitable {
                    if !burning
                        && vitality.state() == LifeState::ActorAlive
                        && touching.contains(&target)
                    {
                        let sound = crate::audio::emit_loop(
                            &mut commands,
                            "zmb_fire_loop",
                            crate::audio::SoundSource::attached(target),
                        );
                        commands.entity(target).insert(FlameIgnited {
                            remaining: 12.,
                            pulse: 0.,
                            sound,
                        });
                    }
                }
                if remaining > dt {
                    panzer.phase = if held {
                        PanzerPhase::HeldBurn(remaining - dt)
                    } else {
                        PanzerPhase::Flame(remaining - dt)
                    };
                } else if held {
                    release(&mut panzer, &mut captured);
                } else {
                    panzer.phase = PanzerPhase::Pursuit;
                }
            }
            PanzerPhase::Claw {
                tip,
                direction,
                remaining,
            } => {
                let step = direction * 30.48 * dt;
                let blocker = Dir3::new(step).ok().and_then(|d| {
                    query.cast_shape(
                        &Collider::cuboid(0.762, 1.524, 0.762),
                        tip,
                        Quat::IDENTITY,
                        d,
                        &ShapeCastConfig::from_max_distance(step.length()),
                        &crate::environment::solids([entity]),
                    )
                });
                let contact = blocker.is_some_and(|hit| hit.entity == owner)
                    || blocker.is_none() && (tip + step).distance(player_pose.translation) < 0.65;
                if contact {
                    if shield.shield_held && shield.shield_health > 0 {
                        shield.shield_health = 0;
                        shield.shield_held = false;
                        shield.owned.remove("tomb_shield_zm");
                        panzer.phase = PanzerPhase::Pursuit;
                    } else {
                        captured.0 = Some(entity);
                        panzer.captured = Some(owner);
                        panzer.claw_baseline = armor.explosive_damage;
                        panzer.phase = PanzerPhase::Pulling(8.);
                    }
                } else if blocker.is_some() || remaining <= dt {
                    panzer.phase = PanzerPhase::Pursuit;
                } else {
                    panzer.phase = PanzerPhase::Claw {
                        tip: tip + step,
                        direction,
                        remaining: remaining - dt,
                    };
                }
            }
            PanzerPhase::Pulling(remaining) => {
                if panzer.captured != Some(owner) || captured.0 != Some(entity) || remaining <= dt {
                    release(&mut panzer, &mut captured);
                } else if distance < 1.8 {
                    panzer.flame_tick = 0.;
                    panzer.phase = PanzerPhase::HeldBurn(3.);
                } else {
                    let velocity =
                        (pose.translation - player_pose.translation).normalize_or_zero() * 5.08;
                    let result = slide.move_and_slide(
                        player_collider,
                        player_pose.translation,
                        Quat::IDENTITY,
                        velocity,
                        time.delta(),
                        &MoveAndSlideConfig::default(),
                        &crate::environment::player_solids([owner, entity]),
                        |_| MoveAndSlideHitResponse::Accept,
                    );
                    player_pose.translation = result.position;
                    player.velocity = Vec3::ZERO;
                    panzer.phase = PanzerPhase::Pulling(remaining - dt);
                }
            }
            PanzerPhase::Landing(_) | PanzerPhase::Stunned(_) | PanzerPhase::Dead(_) => {}
        }
        panzer.animation_elapsed = if previous == std::mem::discriminant(&panzer.phase) {
            panzer.animation_elapsed + dt
        } else {
            0.
        };
    }
}
fn deaths(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut deaths: MessageReader<EnemyDeath>,
    mut panzers: Query<(Entity, &mut Panzer, &Transform, &Vitality)>,
    mut players: Query<&mut CapturedByPanzer>,
    mut waves: ResMut<PanzerWaves>,
    mut parts: ResMut<crate::environment::parts::Parts>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    session: Res<crate::session::SessionId>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut rng: ResMut<Session>,
    mut destroyed: MessageWriter<PanzerDestroyed>,
    spatial: SpatialQuery,
    rigs: Query<&crate::enemies::ActorRig>,
    globals: Query<&GlobalTransform>,
) {
    for event in deaths.read() {
        let Ok((entity, mut panzer, pose, _)) = panzers.get_mut(event.0.impact.target) else {
            continue;
        };
        waves.killed += 1;
        panzer.death_attacker = Some(event.0.impact.attacker);
        panzer.phase = PanzerPhase::Dead(2.);
        panzer.animation_elapsed = 0.;
        if let Some(owner) = panzer.captured.take()
            && let Ok(mut captured) = players.get_mut(owner)
        {
            captured.0 = None;
        }
        commands.entity(entity).insert(ColliderDisabled);
        crate::audio::emit(
            &mut commands,
            "zmb_ai_mechz_vox_death",
            crate::audio::SoundSource::World(pose.translation),
        );
    }
    for (entity, mut panzer, pose, health) in &mut panzers {
        if health.state() != LifeState::ActorDead {
            continue;
        }
        let PanzerPhase::Dead(remaining) = panzer.phase else {
            continue;
        };
        if remaining > time.delta_secs() {
            panzer.phase = PanzerPhase::Dead(remaining - time.delta_secs());
            panzer.animation_elapsed += time.delta_secs();
            continue;
        }
        if panzer.exploded {
            commands.entity(entity).despawn();
            continue;
        }
        let attacker = panzer.death_attacker.unwrap_or(entity);
        let position = pose.translation - Vec3::Y * 1.2;
        let explosion = rigs
            .iter()
            .find_map(|r| r.socket(entity, "tag_powersupply"))
            .and_then(|e| globals.get(e).ok())
            .map_or(pose.translation, GlobalTransform::translation);
        crate::native_fx::emit(
            &mut commands,
            "maps/zombie_tomb/fx_tomb_mech_death",
            Transform::from_translation(explosion),
        );
        crate::audio::emit(
            &mut commands,
            "zmb_ai_mechz_death_explode",
            crate::audio::SoundSource::World(explosion),
        );
        blasts.0.push(crate::equipment::BlastRequest {
            session: *session,
            owner: entity,
            definition: WeaponId("weapon.panzer_death".into()),
            center: explosion,
            radius: 3.2512,
            inner_radius: 0.,
            inner_damage: 100,
            outer_damage: 25,
            direction: None,
            category: Some(DamageCategory::Explosive),
            player_damage: None,
            source: Some(ImpactSource::Enemy(ContentId("enemy.panzer".into()))),
        });
        if !panzer.quest && !waves.fire_part_dropped {
            let from = position + Vec3::Y * 1.016;
            let ground = spatial
                .cast_ray(
                    from,
                    Dir3::NEG_Y,
                    100.,
                    true,
                    &crate::environment::solids([entity]),
                )
                .map_or(position, |hit| from - Vec3::Y * hit.distance);
            if let Some(piece) = crate::crafting::reveal_part(
                &mut commands,
                &mut parts,
                "part.elemental_staff_fire.lower_staff",
                ground + Vec3::Y * 0.8128,
            ) {
                commands.entity(piece).insert(PanzerPartFailsafe {
                    remaining: 120.,
                    check: 0.,
                    movement: None,
                });
            }
            waves.fire_part_dropped = true;
        }
        if panzer.drop_powerup && !panzer.quest && players.contains(attacker) {
            let choices: Vec<_> = game
                .content
                .powerups
                .iter()
                .filter(|p| {
                    matches!(
                        p.id.0.as_str(),
                        "powerup.full_ammo"
                            | "powerup.nuke"
                            | "powerup.insta_kill"
                            | "powerup.double_points"
                            | "powerup.zombie_blood"
                    )
                })
                .collect();
            if !choices.is_empty() {
                let choice = choices[rng.random() as usize % choices.len()];
                crate::powerups::spawn_named(
                    &mut commands,
                    &game,
                    &server,
                    choice.id.0.trim_start_matches("powerup."),
                    position,
                );
            }
        }
        destroyed.write(PanzerDestroyed {
            entity,
            position,
            quest: panzer.quest,
            attacker,
        });
        panzer.exploded = true;
        panzer.phase = PanzerPhase::Dead(3.);
        commands.entity(entity).insert(Visibility::Hidden);
    }
}
fn burning(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut burning: Query<(Entity, &Zombie, &Transform, &Vitality, &mut FlameIgnited)>,
    players: Query<(Entity, &Transform, &Vitality), With<Player>>,
    mut damage: ResMut<DamageQueue>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    session: Res<crate::session::SessionId>,
) {
    for (entity, zombie, pose, health, mut burn) in &mut burning {
        burn.remaining -= time.delta_secs();
        if health.state() == LifeState::ActorDead {
            blasts.0.push(crate::equipment::BlastRequest {
                session: *session,
                owner: entity,
                definition: WeaponId("weapon.panzer_ignited_zombie".into()),
                center: pose.translation,
                radius: 3.2512,
                inner_radius: 0.,
                inner_damage: 30,
                outer_damage: 15,
                direction: None,
                category: Some(DamageCategory::Explosive),
                player_damage: None,
                source: Some(ImpactSource::Enemy(zombie.definition.clone())),
            });
            commands.entity(burn.sound).despawn();
            commands.entity(entity).remove::<FlameIgnited>();
            commands.entity(entity).insert(Visibility::Hidden);
            continue;
        }
        if burn.remaining <= 0. {
            commands.entity(burn.sound).despawn();
            commands.entity(entity).remove::<FlameIgnited>();
            continue;
        }
        burn.pulse -= time.delta_secs();
        if burn.pulse <= 0. {
            burn.pulse += 1.;
            for (target, body, vitality) in &players {
                if vitality.state() == LifeState::Standing
                    && body.translation.distance_squared(pose.translation) < 0.635_f32.powi(2)
                {
                    damage.0.push(Impact {
                        attacker: entity,
                        target,
                        source: ImpactSource::Enemy(zombie.definition.clone()),
                        category: DamageCategory::Fire,
                        region: HitRegion::Torso,
                        amount: 2,
                    });
                }
            }
        }
    }
}

#[derive(Component, Default)]
struct PanzerFeedback {
    flame: Option<Entity>,
    sound: Option<Entity>,
    headlamp: Option<Entity>,
    exhaust: Option<Entity>,
    boosters: Vec<Entity>,
    landed: bool,
    helmet: bool,
    powerplant: bool,
}
fn feedback(
    mut commands: Commands,
    mut panzers: Query<(Entity, &Panzer, &Transform, &Armor, &mut PanzerFeedback)>,
    rigs: Query<&crate::enemies::ActorRig>,
) {
    for (entity, panzer, pose, armor, mut state) in &mut panzers {
        let socket = |name: &str| rigs.iter().find_map(|r| r.socket(entity, name));
        let dead = matches!(panzer.phase, PanzerPhase::Dead(_));
        if !dead {
            if state.headlamp.is_none() {
                if let Some(owner) = socket("tag_headlamp_FX") {
                    state.headlamp = Some(crate::native_fx::attach(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_mech_head_light",
                        owner,
                        Transform::IDENTITY,
                    ));
                }
            }
            if state.exhaust.is_none() {
                if let Some(owner) = socket("tag_back_exhaust_FX") {
                    state.exhaust = Some(crate::native_fx::attach(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_mech_exhaust_smoke",
                        owner,
                        Transform::IDENTITY,
                    ));
                }
            }
        } else {
            for emitter in [state.headlamp.take(), state.exhaust.take()]
                .into_iter()
                .flatten()
            {
                commands.entity(emitter).despawn();
            }
        }
        let flame = matches!(
            panzer.phase,
            PanzerPhase::Flame(_) | PanzerPhase::HeldBurn(_)
        );
        if flame && state.flame.is_none() {
            if let Some(owner) = socket("tag_flamethrower_FX") {
                state.flame = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_wpn_flamethrower",
                    owner,
                    Transform::IDENTITY,
                ));
                crate::audio::emit(
                    &mut commands,
                    "zmb_ai_mechz_flame_start",
                    crate::audio::SoundSource::attached(entity),
                );
                state.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_ai_mechz_flame_loop",
                    crate::audio::SoundSource::attached(entity),
                ));
            }
        } else if !flame {
            if let Some(e) = state.flame.take() {
                commands.entity(e).despawn();
                crate::audio::emit(
                    &mut commands,
                    "zmb_ai_mechz_flame_stop",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            if let Some(e) = state.sound.take() {
                commands.entity(e).despawn();
            }
        }
        if matches!(panzer.phase, PanzerPhase::Landing(_)) && state.boosters.is_empty() {
            for (tag, name) in [
                (
                    "tag_booster_LE_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster",
                ),
                (
                    "tag_booster_RI_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster",
                ),
                (
                    "tag_leg_booster_LE_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster_sm",
                ),
                (
                    "tag_leg_booster_RI_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster_sm",
                ),
            ] {
                if let Some(owner) = socket(tag) {
                    state.boosters.push(crate::native_fx::attach(
                        &mut commands,
                        name,
                        owner,
                        Transform::IDENTITY,
                    ));
                }
            }
        } else if !matches!(panzer.phase, PanzerPhase::Landing(_)) {
            for e in state.boosters.drain(..) {
                commands.entity(e).despawn();
            }
            if !state.landed {
                state.landed = true;
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_jump_landing",
                    Transform::from_translation(pose.translation - Vec3::Y * 1.2),
                );
            }
        }
        if armor.broken && !state.helmet {
            state.helmet = true;
            if let Some(owner) = socket("j_helmet") {
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_dmg_armor_face",
                    owner,
                    Transform::IDENTITY,
                );
            }
        }
        if armor.powerplant_destroyed && !state.powerplant {
            state.powerplant = true;
            if let Some(owner) = socket("tag_powersupply") {
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_dmg_sparks",
                    owner,
                    Transform::IDENTITY,
                );
            }
        }
    }
}

#[derive(Component)]
struct PanzerPartFailsafe {
    remaining: f32,
    check: f32,
    movement: Option<(Vec3, Vec3, f32)>,
}
fn part_failsafe(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut parts: Query<(Entity, &mut Transform, &mut PanzerPartFailsafe), Without<Player>>,
    players: Query<&Transform, With<Player>>,
) {
    let dt = time.delta_secs();
    for (entity, mut pose, mut failsafe) in &mut parts {
        if let Some((from, to, elapsed)) = failsafe.movement.as_mut() {
            *elapsed += dt;
            pose.translation = from.lerp(*to, (*elapsed / 3.).min(1.));
            if *elapsed >= 3. {
                commands.entity(entity).remove::<PanzerPartFailsafe>();
            }
            continue;
        }
        failsafe.remaining -= dt;
        if failsafe.remaining > 0. {
            continue;
        }
        failsafe.check -= dt;
        if failsafe.check > 0. {
            continue;
        }
        failsafe.check = 1.;
        if players
            .iter()
            .any(|p| p.translation.xz().distance_squared(pose.translation.xz()) < 25.4_f32.powi(2))
        {
            continue;
        }
        if let Some((_, point)) = source::SPAWNS.iter().min_by(|a, b| {
            Vec3::from_array(a.1)
                .xz()
                .distance_squared(pose.translation.xz())
                .total_cmp(
                    &Vec3::from_array(b.1)
                        .xz()
                        .distance_squared(pose.translation.xz()),
                )
        }) {
            failsafe.movement = Some((
                pose.translation,
                Vec3::from_array(*point) + Vec3::Y * 0.8128,
                0.,
            ));
        }
    }
}
