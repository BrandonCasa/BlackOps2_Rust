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
    pub tangent_bounce: f32,
    pub normal_bounce: f32,
    pub collision_radius: f32,
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
            && [self.normal_bounce, self.tangent_bounce]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
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
enum GrenadeMotion {
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
#[derive(Component)]
struct Blast {
    remaining: f32,
}
pub struct BlastRequest {
    pub owner: Entity,
    pub definition: WeaponId,
    pub center: Vec3,
    pub radius: f32,
    pub inner_radius: f32,
    pub inner_damage: u32,
    pub outer_damage: u32,
    pub direction: Option<(Vec3, f32)>,
}
#[derive(Resource, Default)]
pub struct BlastQueue(pub Vec<BlastRequest>);
#[derive(Resource, Default)]
pub struct EquipmentEvidence {
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
                (throw_input, flight, resolve_blasts, blast_lifetime)
                    .chain()
                    .in_set(GameStep::Combat)
                    .before(crate::gameplay::combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                replenish
                    .in_set(GameStep::Rounds)
                    .after(crate::gameplay::rounds)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn replenish(
    mut started: MessageReader<crate::rounds::RoundStarted>,
    definition: Res<GameDefinition>,
    mut players: Query<(&mut GrenadeInventory, &Vitality), With<Player>>,
    mut evidence: ResMut<EquipmentEvidence>,
) {
    for event in started.read() {
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
    ) in &mut players
    {
        if health.state() != LifeState::Standing || drinking {
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
            let pressed = held && !inventory.held;
            inventory.held = held;
            let dt = time.delta_secs();
            let mut launch = None;
            match inventory.state {
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
                        launch = Some((0., true));
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
                        launch = Some((0., true));
                        evidence.cooked_off += 1;
                    } else if remaining <= dt {
                        launch = Some((fuse, false));
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
            if let Some((fuse, cooked)) = launch {
                let rotation =
                    Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch);
                let forward = rotation * Vec3::NEG_Z;
                let eye =
                    transform.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
                let position = eye + forward * if cooked { 0. } else { 0.25 };
                let velocity = if cooked {
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
                        let mut child = parent.spawn(WorldAssetRoot(server.load(scene.to_owned())));
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
                inventory.state = ThrowState::Recovery {
                    remaining: tuning.throw_seconds
                        - if cooked { 0. } else { tuning.release_delay },
                };
                evidence.thrown += u32::from(!cooked);
                evidence.record(serde_json::json!({"event":"launch","weapon":weapon.id.0,"cooked":cooked,"fuse":fuse,"release_delay":tuning.release_delay,"position":position.to_array(),"velocity":velocity.to_array(),"count":inventory.count}));
            }
        }
    }
}
fn flight(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut grenades: Query<(Entity, &mut Grenade, &mut Transform)>,
    targets: Query<(), (With<Vitality>, With<Transform>, Without<Grenade>)>,
    poses: Query<(&Transform, &GlobalTransform, Has<ChildOf>), Without<Grenade>>,
    surfaces: Query<&crate::audio::SurfaceSound>,
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
            velocity.y -= definition.tuning.gravity * dt;
            let delta = velocity * dt;
            if let Ok(direction) = Dir3::new(delta) {
                let hit = query.cast_shape(
                    &Collider::sphere(grenade.radius),
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
                        |s| s.0.as_str(),
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
                        velocity = (velocity - perpendicular) * tuning.tangent_bounce
                            - perpendicular * tuning.normal_bounce;
                        grenade.motion = if normal.y > 0.6 && velocity.length() < 0.5 {
                            GrenadeMotion::Resting
                        } else {
                            GrenadeMotion::Flying(velocity)
                        };
                        evidence.bounces += 1;
                        evidence.record(serde_json::json!({"event":"bounce","normal":normal.to_array(),"position":transform.translation.to_array(),"speed":velocity.length()}));
                    }
                } else {
                    transform.translation += delta;
                    grenade.motion = GrenadeMotion::Flying(velocity);
                }
            }
        }
        if weapon.lure.is_some()
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
            owner: grenade.owner,
            definition: grenade.definition.clone(),
            center,
            radius: tuning.radius,
            inner_radius: tuning.inner_radius,
            inner_damage: tuning.inner_damage,
            outer_damage: tuning.outer_damage,
            direction: None,
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
    game: Res<GameDefinition>,
    mut queue: ResMut<BlastQueue>,
    query: SpatialQuery,
    targets: Query<(Entity, &Vitality, &Transform, Has<Player>)>,
    mut impacts: ResMut<crate::damage::DamageQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut evidence: ResMut<EquipmentEvidence>,
) {
    for blast in queue.0.drain(..) {
        let center = blast.center;
        crate::audio::binding(
            &mut commands,
            &game,
            &format!("{}.explode", blast.definition.0),
            crate::audio::SoundSource::World(center),
        );
        for (target, vitality, pose, player) in &targets {
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
            let damage = (blast.inner_damage as f32
                + (blast.outer_damage as f32 - blast.inner_damage as f32) * ratio)
                .round() as u32;
            impacts.0.push(crate::damage::Impact {
                attacker: blast.owner,
                target,
                source: crate::damage::ImpactSource::Projectile(blast.definition.clone()),
                category: DamageCategory::Explosive,
                region: HitRegion::Torso,
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
