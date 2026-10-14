use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    life::{LifeState, Vitality},
    origins::content::{DamageCategory, WeaponId},
    player::Player,
    weapons::{HitRegion, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldAssetRoot};

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrowableDefinition {
    pub slot: ThrowableSlot,
    pub initial: u32,
    pub capacity: u32,
    pub round_award: u32,
    pub prime_seconds: f32,
    pub throw_seconds: f32,
    pub release_delay: f32,
    pub fuse_seconds: f32,
    pub speed: f32,
    pub up_speed: f32,
    pub radius: f32,
    pub inner_radius: f32,
    pub inner_damage: u32,
    pub outer_damage: u32,
    pub tangent_bounce: [f32; 32],
    pub normal_bounce: [f32; 32],
    pub collision_radius: f32,
    #[serde(default)]
    pub collision_half_extents: Option<[f32; 3]>,
    pub cook_in_hand: bool,
    pub sticky: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThrowableSlot {
    Lethal,
    Tactical,
}
impl ThrowableDefinition {
    pub fn valid(&self) -> bool {
        self.capacity > 0
            && self.round_award <= self.capacity
            && self.initial <= self.capacity
            && self.inner_damage >= self.outer_damage
            && self.outer_damage > 0
            && [
                self.prime_seconds,
                self.throw_seconds,
                self.fuse_seconds,
                self.speed,
                self.radius,
                self.collision_radius,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            && self.up_speed.is_finite()
            && self.release_delay.is_finite()
            && self.release_delay >= 0.
            && self.release_delay < self.throw_seconds
            && self.inner_radius.is_finite()
            && self.inner_radius >= 0.
            && self.inner_radius < self.radius
            && self
                .normal_bounce
                .iter()
                .chain(self.tangent_bounce.iter())
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            && self
                .collision_half_extents
                .is_none_or(|v| v.into_iter().all(|v| v.is_finite() && v > 0.))
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub enum ThrowState {
    #[default]
    Ready,
    Priming {
        remaining: f32,
    },
    Cooking {
        remaining: f32,
    },
    Releasing {
        remaining: f32,
        fuse: f32,
    },
    Recovery {
        remaining: f32,
    },
}
#[derive(Component)]
pub struct GrenadeInventory {
    pub definition: WeaponId,
    pub count: u32,
    pub capacity: u32,
    pub state: ThrowState,
    held: bool,
}
impl GrenadeInventory {
    pub fn new(id: WeaponId, tuning: &ThrowableDefinition) -> Self {
        Self {
            definition: id,
            count: tuning.initial,
            capacity: tuning.capacity,
            state: default(),
            held: false,
        }
    }
    pub fn busy(&self) -> bool {
        !matches!(self.state, ThrowState::Ready)
    }
    pub fn cancel(&mut self) {
        if matches!(
            self.state,
            ThrowState::Priming { .. } | ThrowState::Cooking { .. } | ThrowState::Releasing { .. }
        ) {
            self.count = self.count.saturating_add(1).min(self.capacity);
        }
        self.state = ThrowState::Ready;
    }
    pub fn receive(
        &mut self,
        weapon: &crate::origins::content::WeaponDefinition,
    ) -> Result<(), EquipmentError> {
        let tuning = weapon
            .throwable
            .as_ref()
            .filter(|t| t.slot == ThrowableSlot::Lethal)
            .ok_or(EquipmentError::Unsupported)?;
        if self.busy() {
            return Err(EquipmentError::Busy);
        }
        if self.definition == weapon.id && self.count == self.capacity {
            return Err(EquipmentError::Full);
        }
        self.definition = weapon.id.clone();
        self.capacity = tuning.capacity;
        self.count = tuning.initial;
        Ok(())
    }
}
#[derive(Component, Default)]
pub struct TacticalInventory(pub Option<GrenadeInventory>);
impl TacticalInventory {
    pub fn busy(&self) -> bool {
        self.0.as_ref().is_some_and(GrenadeInventory::busy)
    }
    pub fn receive(
        &mut self,
        weapon: &crate::origins::content::WeaponDefinition,
    ) -> Result<(), EquipmentError> {
        let tuning = weapon
            .throwable
            .as_ref()
            .filter(|t| t.slot == ThrowableSlot::Tactical)
            .ok_or(EquipmentError::Unsupported)?;
        if self.busy() {
            return Err(EquipmentError::Busy);
        }
        if self
            .0
            .as_ref()
            .is_some_and(|i| i.definition == weapon.id && i.count == i.capacity)
        {
            return Err(EquipmentError::Full);
        }
        self.0 = Some(GrenadeInventory::new(weapon.id.clone(), tuning));
        Ok(())
    }
}
#[derive(Debug)]
pub enum EquipmentError {
    Busy,
    Full,
    Unsupported,
}
#[derive(Clone, Copy)]
pub(crate) enum GrenadeMotion {
    Flying(Vec3),
    Resting,
    Attached {
        target: Entity,
        offset: Vec3,
        rotation: Quat,
    },
}
#[derive(Component)]
pub struct Grenade {
    pub owner: Entity,
    pub definition: WeaponId,
    motion: GrenadeMotion,
    pub remaining: f32,
    elapsed: f32,
    radius: f32,
    pub lure_active: bool,
}
impl Grenade {
    pub fn settled(&self) -> bool {
        matches!(
            self.motion,
            GrenadeMotion::Resting | GrenadeMotion::Attached { .. }
        )
    }
}
pub(crate) fn dig_grenade(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    owner: Entity,
    position: Vec3,
    velocity: Vec3,
) {
    let Some(weapon) = game
        .content
        .weapons
        .iter()
        .find(|w| w.id.0 == "weapon.frag_grenade_zm")
    else {
        return;
    };
    let Some(tuning) = weapon.throwable.as_ref() else {
        return;
    };
    let mut entity = commands.spawn((
        SessionEntity,
        Grenade {
            owner,
            definition: weapon.id.clone(),
            motion: GrenadeMotion::Flying(velocity),
            remaining: 3.,
            elapsed: 0.,
            radius: tuning.collision_radius,
            lure_active: false,
        },
        Transform::from_translation(position),
        Visibility::Inherited,
    ));
    if let Some(scene) = weapon
        .pickup
        .as_ref()
        .and_then(|id| game.content.presentations.iter().find(|p| p.id == *id))
        .and_then(|p| p.scene.as_ref())
    {
        entity.insert(WorldAssetRoot(server.load(scene.clone())));
    }
}
#[derive(Component)]
struct Blast {
    remaining: f32,
}
pub struct BlastRequest {
    pub(crate) session: crate::session::SessionId,
    pub owner: Entity,
    pub definition: WeaponId,
    pub center: Vec3,
    pub radius: f32,
    pub inner_radius: f32,
    pub inner_damage: u32,
    pub outer_damage: u32,
    pub direction: Option<(Vec3, f32)>,
    pub category: Option<DamageCategory>,
    pub player_damage: Option<[u32; 2]>,
    pub source: Option<crate::damage::ImpactSource>,
}
#[derive(Resource, Default)]
pub struct BlastQueue(pub Vec<BlastRequest>);
#[derive(Resource, Default)]
pub struct EquipmentEvidence {
    pub retired_blasts: u32,
    pub round_awards: u32,
    pub primed: u32,
    pub thrown: u32,
    pub cooked_off: u32,
    pub bounces: u32,
    pub detonations: u32,
    pub obstructed: u32,
    pub self_impacts: u32,
    pub enemy_impacts: u32,
    pub surface_attachments: u32,
    pub actor_attachments: u32,
    pub attached_motion: f32,
    pub events: Vec<serde_json::Value>,
}
impl EquipmentEvidence {
    fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event);
        }
    }
}
pub struct EquipmentPlugin;
impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        crate::mines::install(app);
        crate::lures::install(app);
        app.init_resource::<BlastQueue>()
            .init_resource::<EquipmentEvidence>()
            .add_systems(
                FixedUpdate,
                (
                    (throw_input, flight)
                        .chain()
                        .before(crate::gameplay::combat),
                    (resolve_blasts, blast_lifetime)
                        .chain()
                        .after(crate::gameplay::combat),
                )
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                replenish
                    .in_set(GameStep::Rounds)
                    .after(crate::rounds::advance)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn replenish(
    session: Res<crate::session::SessionId>,
    mut started: MessageReader<crate::rounds::RoundStarted>,
    definition: Res<GameDefinition>,
    mut players: Query<(&mut GrenadeInventory, &Vitality), With<Player>>,
    mut evidence: ResMut<EquipmentEvidence>,
) {
    for event in started.read() {
        if event.session != *session {
            continue;
        }
        for (mut inventory, vitality) in &mut players {
            if vitality.state() != LifeState::Standing {
                continue;
            }
            let Some(profile) = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id == inventory.definition)
                .and_then(|w| w.throwable.as_ref())
            else {
                continue;
            };
            if profile.round_award == 0 {
                continue;
            }
            let before = inventory.count;
            inventory.count = inventory
                .count
                .saturating_add(profile.round_award)
                .min(profile.capacity);
            evidence.round_awards += 1;
            evidence.record(
                serde_json::json!({"event":"round_award","round":event.round,
                "weapon":inventory.definition.0,"before":before,"after":inventory.count}),
            );
        }
    }
}
pub(crate) fn throw_input(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    input: Res<GameInput>,
    definition: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut GrenadeInventory,
        &mut WeaponInventory,
        &crate::mines::MineInventory,
        &mut TacticalInventory,
        Has<crate::perks::PerkDrinking>,
        Has<crate::actions::HandReservation>,
        Has<crate::robots::StompReaction>,
        Has<crate::robots::RobotTransit>,
        Has<crate::tank::TankRescue>,
    )>,
    mut evidence: ResMut<EquipmentEvidence>,
) {
    for (
        owner,
        player,
        transform,
        health,
        mut lethal,
        mut weapons,
        mines,
        mut tactical,
        drinking,
        reserved,
        stomp,
        robot_transit,
        tank_rescue,
    ) in &mut players
    {
        let standing = health.state() == LifeState::Standing;
        if reserved || stomp || robot_transit || tank_rescue {
            lethal.cancel();
            if let Some(tactical) = tactical.0.as_mut() {
                tactical.cancel();
            }
            continue;
        }
        if standing && drinking {
            continue;
        }
        for tactical_slot in [false, true] {
            let other_busy = if tactical_slot {
                lethal.busy()
            } else {
                tactical.busy()
            };
            let held = if tactical_slot {
                input.tactical_held
            } else {
                input.grenade_held
            };
            let inventory = if tactical_slot {
                let Some(inventory) = tactical.0.as_mut() else {
                    continue;
                };
                inventory
            } else {
                &mut *lethal
            };
            let pressed = held && !inventory.held;
            inventory.held = held;
            let Some(weapon) = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id == inventory.definition)
            else {
                continue;
            };
            let Some(tuning) = &weapon.throwable else {
                continue;
            };
            let dt = time.delta_secs();
            let mut launch = None;
            match inventory.state {
                state if !standing => {
                    let fuse = match state {
                        ThrowState::Priming { .. } => Some(tuning.fuse_seconds),
                        ThrowState::Cooking { remaining } => Some(remaining),
                        ThrowState::Releasing { fuse, .. } => Some(fuse),
                        ThrowState::Ready | ThrowState::Recovery { .. } => None,
                    };
                    if !matches!(state, ThrowState::Ready) {
                        evidence.record(
                            serde_json::json!({"event":"interrupted","weapon":weapon.id.0,
                            "state":format!("{state:?}"),"fuse":fuse}),
                        );
                    }
                    inventory.state = ThrowState::Ready;
                    launch = fuse.map(|fuse| (fuse, false, true));
                }
                ThrowState::Ready
                    if pressed && inventory.count > 0 && !mines.busy() && !other_busy =>
                {
                    inventory.count -= 1;
                    weapons.cancel_action();
                    inventory.state = ThrowState::Priming {
                        remaining: tuning.prime_seconds,
                    };
                    evidence.primed += 1;
                }
                ThrowState::Priming { remaining } => {
                    if remaining <= dt {
                        inventory.state = ThrowState::Cooking {
                            remaining: tuning.fuse_seconds,
                        };
                    } else {
                        inventory.state = ThrowState::Priming {
                            remaining: remaining - dt,
                        };
                    }
                }
                ThrowState::Cooking { remaining } => {
                    if tuning.cook_in_hand && remaining <= dt {
                        launch = Some((0., true, false));
                        evidence.cooked_off += 1;
                    } else if !held {
                        inventory.state = ThrowState::Releasing {
                            remaining: tuning.release_delay,
                            fuse: if tuning.cook_in_hand {
                                remaining - dt
                            } else {
                                remaining
                            },
                        };
                        evidence.record(serde_json::json!({"event":"release_started",
                            "weapon":weapon.id.0,"release_delay":tuning.release_delay,
                            "throw_seconds":tuning.throw_seconds}));
                    } else {
                        inventory.state = ThrowState::Cooking {
                            remaining: if tuning.cook_in_hand {
                                remaining - dt
                            } else {
                                remaining
                            },
                        };
                    }
                }
                ThrowState::Releasing { remaining, fuse } => {
                    let fuse = if tuning.cook_in_hand { fuse - dt } else { fuse };
                    if tuning.cook_in_hand && fuse <= 0. {
                        launch = Some((0., true, false));
                        evidence.cooked_off += 1;
                    } else if remaining <= dt {
                        launch = Some((fuse, false, false));
                    } else {
                        inventory.state = ThrowState::Releasing {
                            remaining: remaining - dt,
                            fuse,
                        };
                    }
                }
                ThrowState::Recovery { remaining } => {
                    inventory.state = if remaining <= dt {
                        ThrowState::Ready
                    } else {
                        ThrowState::Recovery {
                            remaining: remaining - dt,
                        }
                    };
                }
                _ => {}
            }
            if let Some((fuse, cooked, dropped)) = launch {
                let rotation =
                    Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch);
                let forward = rotation * Vec3::NEG_Z;
                let eye =
                    transform.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
                let position = eye + forward * if cooked || dropped { 0. } else { 0.25 };
                let velocity = if cooked || dropped {
                    Vec3::ZERO
                } else {
                    forward * tuning.speed + Vec3::Y * tuning.up_speed + player.velocity
                };
                let entity = commands
                    .spawn((
                        Grenade {
                            owner,
                            definition: weapon.id.clone(),
                            motion: if cooked {
                                GrenadeMotion::Resting
                            } else {
                                GrenadeMotion::Flying(velocity)
                            },
                            remaining: fuse,
                            elapsed: 0.,
                            radius: tuning.collision_radius,
                            lure_active: false,
                        },
                        SessionEntity,
                        Transform::from_translation(position),
                        Visibility::default(),
                    ))
                    .id();
                if let Some(scene) = weapon
                    .pickup
                    .as_ref()
                    .and_then(|id| {
                        definition
                            .content
                            .presentations
                            .iter()
                            .find(|p| p.id == *id)
                    })
                    .and_then(|p| p.scene.as_ref())
                {
                    commands.entity(entity).with_children(|parent| {
                        let mut child = parent.spawn((
                            WorldAssetRoot(server.load(scene.to_owned())),
                            crate::presentation::camo::WeaponMaterialContext(weapon.id.clone()),
                        ));
                        if let Some(lure) = &weapon.lure
                            && let Some(clip) = definition
                                .content
                                .animations
                                .iter()
                                .find(|a| a.id == lure.animation)
                        {
                            child.insert(crate::lures::LureRig::new(
                                entity,
                                server.load(clip.path.clone()),
                            ));
                        }
                    });
                }
                inventory.state = if dropped {
                    ThrowState::Ready
                } else {
                    ThrowState::Recovery {
                        remaining: tuning.throw_seconds
                            - if cooked { 0. } else { tuning.release_delay },
                    }
                };
                evidence.thrown += u32::from(!cooked && !dropped);
                evidence.record(serde_json::json!({"event":"launch","weapon":weapon.id.0,"cooked":cooked,"dropped":dropped,"fuse":fuse,"release_delay":tuning.release_delay,"position":position.to_array(),"velocity":velocity.to_array(),"count":inventory.count}));
            }
        }
    }
}
pub(crate) fn flight(
    mut commands: Commands,
    session: Res<crate::session::SessionId>,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    options: Res<crate::assets::LaunchOptions>,
    mut physics_evidence: ResMut<crate::movement::physics::PhysicsEvidence>,
    mut grenades: Query<(Entity, &mut Grenade, &mut Transform)>,
    targets: Query<(), (With<Vitality>, With<Transform>, Without<Grenade>)>,
    poses: Query<(&Transform, &GlobalTransform, Has<ChildOf>), Without<Grenade>>,
    surfaces: Query<(
        &crate::audio::SurfaceSound,
        Option<&crate::movement::physics::SurfaceFlags>,
    )>,
    query: SpatialQuery,
    mut blasts: ResMut<BlastQueue>,
    mut lures: ResMut<crate::lures::LureEvidence>,
    mut evidence: ResMut<EquipmentEvidence>,
) {
    let dt = time.delta_secs();
    for (entity, mut grenade, mut transform) in &mut grenades {
        let Some(weapon) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == grenade.definition)
        else {
            continue;
        };
        let Some(tuning) = weapon.throwable.as_ref() else {
            continue;
        };
        grenade.remaining -= dt;
        grenade.elapsed += dt;
        if let GrenadeMotion::Attached {
            target,
            offset,
            rotation,
        } = grenade.motion
        {
            if let Ok((local, global, parent)) = poses.get(target) {
                let pose = if !parent {
                    *local
                } else {
                    global.compute_transform()
                };
                let next = pose.compute_affine().transform_point3(offset);
                evidence.attached_motion += transform.translation.distance(next);
                transform.translation = next;
                transform.rotation = pose.rotation * rotation;
            } else {
                grenade.motion = GrenadeMotion::Flying(Vec3::ZERO);
            }
        }
        if let GrenadeMotion::Flying(mut velocity) = grenade.motion
            && grenade.remaining > 0.
        {
            let gravity_step = definition.tuning.gravity * dt;
            let delta = (velocity - Vec3::Y * gravity_step * 0.5) * dt;
            velocity.y -= gravity_step;
            if let Ok(direction) = Dir3::new(delta) {
                let shape = tuning.collision_half_extents.map_or_else(
                    || Collider::sphere(grenade.radius),
                    |half| Collider::cuboid(half[0] * 2., half[1] * 2., half[2] * 2.),
                );
                let hit = query.cast_shape(
                    &shape,
                    transform.translation,
                    Quat::IDENTITY,
                    direction,
                    &ShapeCastConfig::from_max_distance(delta.length()),
                    &crate::environment::solids([grenade.owner]),
                );
                if let Some(hit) = hit {
                    let normal = hit.normal1;
                    transform.translation += direction.as_vec3() * hit.distance.max(0.);
                    transform.translation += normal * 0.003;
                    let surface = surfaces.get(hit.entity).map_or(
                        if targets.contains(hit.entity) {
                            "flesh"
                        } else {
                            "default"
                        },
                        |(s, _)| s.0.as_str(),
                    );
                    let alias = format!(
                        "{}_{}",
                        if tuning.sticky {
                            "wpn_sticky_gren_bounce"
                        } else if weapon.lure.is_some() {
                            "zmb_monkey_land"
                        } else {
                            "wpn_grenade_bounce"
                        },
                        surface
                    );
                    crate::audio::emit(
                        &mut commands,
                        alias,
                        crate::audio::SoundSource::World(transform.translation),
                    );
                    if tuning.sticky {
                        transform.rotation = Quat::from_rotation_arc(Vec3::Y, normal);
                        grenade.motion = if let Ok((local, global, parent)) = poses.get(hit.entity)
                        {
                            let pose = if !parent {
                                *local
                            } else {
                                global.compute_transform()
                            };
                            GrenadeMotion::Attached {
                                target: hit.entity,
                                offset: pose
                                    .compute_affine()
                                    .inverse()
                                    .transform_point3(transform.translation),
                                rotation: pose.rotation.inverse() * transform.rotation,
                            }
                        } else {
                            GrenadeMotion::Resting
                        };
                        let actor = targets.contains(hit.entity);
                        evidence.actor_attachments += u32::from(actor);
                        evidence.surface_attachments += u32::from(!actor);
                        evidence.record(serde_json::json!({"event":"attach","target":hit.entity.to_bits(),"actor":actor,"position":transform.translation.to_array()}));
                    } else {
                        let perpendicular = normal * velocity.dot(normal);
                        let surface_index = surfaces
                            .get(hit.entity)
                            .ok()
                            .and_then(|(_, flags)| flags)
                            .map_or(if targets.contains(hit.entity) { 7 } else { 0 }, |flags| {
                                ((flags.0 >> 20) & 0x3f) as usize
                            });
                        let tangent = tuning
                            .tangent_bounce
                            .get(surface_index)
                            .copied()
                            .unwrap_or(0.);
                        let restitution = tuning
                            .normal_bounce
                            .get(surface_index)
                            .copied()
                            .unwrap_or(0.);
                        velocity =
                            (velocity - perpendicular) * tangent - perpendicular * restitution;
                        grenade.motion = if normal.y > 0.6 && velocity.length() < 0.5 {
                            GrenadeMotion::Resting
                        } else {
                            GrenadeMotion::Flying(velocity)
                        };
                        evidence.bounces += 1;
                        evidence.record(serde_json::json!({"event":"bounce","normal":normal.to_array(),"position":transform.translation.to_array(),"speed":velocity.length()}));
                    }
                } else {
                    if cfg!(debug_assertions)
                        && options.validation_remote.is_some()
                        && physics_evidence.projectiles.len() < 512
                    {
                        physics_evidence.projectiles.push(serde_json::json!({
                            "kind":"throwable","entity":entity.to_bits(),"weapon":weapon.id.0,"dt":dt,
                            "gravity":definition.tuning.gravity,
                            "position_before":transform.translation.to_array(),"position_after":(transform.translation+delta).to_array(),
                            "velocity_after":velocity.to_array()
                        }));
                    }
                    transform.translation += delta;
                    grenade.motion = GrenadeMotion::Flying(velocity);
                }
            }
        }
        if (weapon.lure.is_some() || weapon.id.0 == "weapon.beacon_zm")
            && !grenade.lure_active
            && matches!(grenade.motion, GrenadeMotion::Resting)
        {
            grenade.lure_active = true;
            grenade.remaining = tuning.fuse_seconds;
            lures.activated += 1;
            lures.record(serde_json::json!({"event":"activate","bomb":entity.to_bits(),"flight_seconds":grenade.elapsed,"fuse":grenade.remaining,"position":transform.translation.to_array()}));
        }
        if grenade.remaining > 0. {
            continue;
        }
        if weapon.lure.is_some() {
            lures.detonation += 1;
            lures.record(serde_json::json!({"event":"detonate","bomb":entity.to_bits(),"elapsed":grenade.elapsed,"position":transform.translation.to_array()}));
        }
        evidence.detonations += 1;
        let center = transform.translation;
        blasts.0.push(BlastRequest {
            session: *session,
            owner: grenade.owner,
            definition: grenade.definition.clone(),
            center,
            radius: tuning.radius,
            inner_radius: tuning.inner_radius,
            inner_damage: tuning.inner_damage,
            outer_damage: tuning.outer_damage,
            direction: None,
            category: None,
            player_damage: None,
            source: None,
        });
        evidence.record(serde_json::json!({"event":"detonate","position":center.to_array(),"flight_seconds":grenade.elapsed}));
        commands.entity(entity).despawn();
    }
}
fn blast_lifetime(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut blasts: Query<(Entity, &mut Blast, &mut Transform), Without<Grenade>>,
) {
    for (entity, mut blast, mut transform) in &mut blasts {
        blast.remaining -= time.delta_secs();
        transform.scale = Vec3::splat(1. + (0.25 - blast.remaining) * 16.);
        if blast.remaining <= 0. {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn resolve_blasts(
    mut commands: Commands,
    session: Res<crate::session::SessionId>,
    game: Res<GameDefinition>,
    mut queue: ResMut<BlastQueue>,
    query: SpatialQuery,
    targets: Query<(
        Entity,
        &Vitality,
        &Transform,
        Has<Player>,
        Option<&crate::hit_regions::Hitboxes>,
    )>,
    mut impacts: ResMut<crate::damage::DamageQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut evidence: ResMut<EquipmentEvidence>,
    environmental: crate::environment::support::DamageTriggerTargets,
    mut environment_hits: MessageWriter<crate::environment::support::EnvironmentalHit>,
) {
    for blast in queue.0.drain(..) {
        if blast.session != *session {
            evidence.retired_blasts += 1;
            continue;
        }
        let center = blast.center;
        for (target, shape, pose, state, trigger) in &environmental {
            if !state.enabled || state.spawnflags & 0x40 != 0 {
                continue;
            }
            let bounds = shape.aabb(pose.translation, pose.rotation, 0.);
            let point = if trigger.brush {
                center.clamp(bounds.min, bounds.max)
            } else {
                pose.translation
            };
            let delta = point - center;
            let distance = delta.length();
            if distance >= blast.radius {
                continue;
            }
            if let Some((forward, cosine)) = blast.direction
                && distance > 0.001
                && forward.dot(delta / distance) < cosine
            {
                continue;
            }
            let midpoint = (bounds.min + bounds.max) * 0.5;
            let half = (bounds.max - bounds.min) * 0.5;
            let mut toward = (center - midpoint).normalize_or_zero();
            if toward.xz().length_squared() < 0.001 {
                toward = Vec3::X;
            }
            let right = Vec3::Y.cross(toward).normalize_or_zero();
            let up = toward.cross(right);
            let right = right * right.abs().dot(half);
            let up = up * up.abs().dot(half);
            if ![
                midpoint,
                midpoint + right + up,
                midpoint + right - up,
                midpoint - right + up,
                midpoint - right - up,
            ]
            .into_iter()
            .any(|sample| {
                let delta = sample - center;
                if let Some((forward, cosine)) = blast.direction
                    && delta.length_squared() > 0.
                    && forward.dot(delta.normalize()) < cosine
                {
                    return false;
                }
                Dir3::new(delta).map_or(true, |direction| {
                    query
                        .cast_ray(
                            center,
                            direction,
                            delta.length(),
                            true,
                            &crate::environment::solids([blast.owner, target]),
                        )
                        .is_none()
                })
            }) {
                continue;
            }
            let ratio = ((distance - blast.inner_radius) / (blast.radius - blast.inner_radius))
                .clamp(0., 1.);
            let amount = (blast.inner_damage as f32
                + (blast.outer_damage as f32 - blast.inner_damage as f32) * ratio)
                .max(1.) as u32;
            let native_mod = game
                .content
                .weapons
                .iter()
                .find(|weapon| weapon.id == blast.definition)
                .map(|weapon| if weapon.throwable.is_some() { 4 } else { 6 })
                .or(Some(15));
            environment_hits.write(crate::environment::support::EnvironmentalHit {
                target,
                attacker: blast.owner,
                weapon: blast.definition.clone(),
                category: blast.category.unwrap_or(DamageCategory::Explosive),
                native_mod,
                amount,
                point: center,
                direction: pose.translation - center + Vec3::Y * (24. * 0.0254),
            });
        }
        crate::audio::binding(
            &mut commands,
            &game,
            &format!("{}.explode", blast.definition.0),
            crate::audio::SoundSource::World(center),
        );
        for (target, vitality, pose, player, _hitboxes) in &targets {
            if !matches!(
                vitality.state(),
                LifeState::Standing | LifeState::ActorAlive
            ) {
                continue;
            }
            let delta = pose.translation - center;
            let distance = delta.length();
            if distance > blast.radius {
                continue;
            }
            if let Ok(direction) = Dir3::new(delta)
                && query
                    .cast_ray_predicate(
                        center,
                        direction,
                        distance,
                        true,
                        &crate::environment::solids([target, blast.owner]),
                        &|entity| !targets.contains(entity),
                    )
                    .is_some()
            {
                evidence.obstructed += 1;
                evidence.record(serde_json::json!({"event":"obstructed","target":target.to_bits(),"center":center.to_array(),"position":pose.translation.to_array()}));
                continue;
            }
            if let Some((forward, cosine)) = blast.direction
                && distance > 0.001
                && forward.dot(delta / distance) < cosine
            {
                continue;
            }
            let ratio = ((distance - blast.inner_radius) / (blast.radius - blast.inner_radius))
                .clamp(0., 1.);
            let [inner, outer] = if player {
                blast
                    .player_damage
                    .unwrap_or([blast.inner_damage, blast.outer_damage])
            } else {
                [blast.inner_damage, blast.outer_damage]
            };
            let raw_damage = inner as f32 + (outer as f32 - inner as f32) * ratio;
            let weapon_damage = !player
                && blast.source.as_ref().is_none_or(|source| {
                    matches!(
                        source,
                        crate::damage::ImpactSource::Weapon(_)
                            | crate::damage::ImpactSource::Projectile(_)
                    )
                });
            let damage = if weapon_damage
                && let Some(weapon) = game
                    .content
                    .weapons
                    .iter()
                    .find(|weapon| weapon.id == blast.definition)
            {
                (raw_damage as f64 * weapon.mechanics.hit_multipliers[0] as f64)
                    .trunc()
                    .max(1.) as u32
            } else {
                raw_damage.round() as u32
            };
            impacts.0.push(crate::damage::Impact {
                point: Some(center),
                attacker: blast.owner,
                target,
                source: blast.source.clone().unwrap_or_else(|| {
                    crate::damage::ImpactSource::Projectile(blast.definition.clone())
                }),
                category: blast.category.unwrap_or(DamageCategory::Explosive),
                region: HitRegion::None,
                amount: damage,
            });
            if player {
                evidence.self_impacts += 1;
            } else {
                evidence.enemy_impacts += 1;
            }
            evidence.record(serde_json::json!({"event":"blast_impact","target":target.to_bits(),"self":player,"distance":distance,"damage":damage}));
        }
        commands.spawn((
            SessionEntity,
            Blast { remaining: 0.25 },
            Mesh3d(meshes.add(Sphere::new(0.15).mesh().ico(2).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(1., 0.45, 0.08, 0.75),
                emissive: LinearRgba::new(20., 5., 0.5, 1.),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(center),
        ));
    }
}
