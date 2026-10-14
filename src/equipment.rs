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
    #[serde(default)]
    pub native_launch: Option<GrenadeLaunch>,
    pub cook_in_hand: bool,
    pub sticky: bool,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrenadeLaunch {
    pub right_speed: f32,
    pub horizontal_speed: f32,
    pub rolling: bool,
    pub rotating: bool,
    pub rotation_mode: u32,
    pub preserve_rotation: bool,
    pub rest_speed: f32,
    pub friction: [f32; 2],
    pub friction_speed: f32,
    pub bump_frequency: f32,
    pub bump_magnitude: f32,
    pub bump_speed: f32,
    pub wobble_frequency: f32,
    pub wobble_forward: f32,
    pub wobble_side: f32,
    pub wobble_damping: f32,
    pub curve: f32,
    pub restitution_max: f32,
    pub view_kick: [f32; 3],
    pub restore_seconds: f32,
}

fn native_launch_velocity(
    tuning: &ThrowableDefinition,
    rotation: Quat,
    player_velocity: Vec3,
) -> Vec3 {
    let Some(native) = &tuning.native_launch else {
        return rotation * Vec3::NEG_Z * tuning.speed + Vec3::Y * tuning.up_speed + player_velocity;
    };
    let forward = rotation * Vec3::NEG_Z;
    let initial = forward * (tuning.speed / 0.0254).round()
        + rotation * Vec3::X * (native.right_speed / 0.0254).round()
        + Vec3::Y * (tuning.up_speed / 0.0254).round()
        + forward.with_y(0.).normalize_or_zero() * (native.horizontal_speed / 0.0254).round();
    let direction = initial.normalize_or_zero();
    let snapped = Vec3::from_array(initial.to_array().map(f32::trunc)) * 0.0254;
    snapped + direction * direction.dot(player_velocity)
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
            && self.release_delay <= self.throw_seconds
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
            && self.native_launch.as_ref().is_none_or(|native| {
                native.right_speed.is_finite()
                    && native.horizontal_speed.is_finite()
                    && native.rotation_mode <= 2
                    && [
                        native.rest_speed,
                        native.friction_speed,
                        native.bump_speed,
                        native.wobble_frequency,
                        native.wobble_forward,
                        native.wobble_side,
                        native.wobble_damping,
                        native.curve,
                        native.restore_seconds,
                    ]
                    .into_iter()
                    .all(|value| value.is_finite() && value >= 0.)
                    && native
                        .friction
                        .into_iter()
                        .chain([
                            native.bump_frequency,
                            native.bump_magnitude,
                            native.restitution_max,
                        ])
                        .all(|value| value.is_finite() && (0. ..=1.).contains(&value))
                    && native
                        .view_kick
                        .into_iter()
                        .all(|value| value.is_finite() && value >= 0.)
                    && native.view_kick[1] <= native.view_kick[2]
            })
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
    recovery: f32,
}
impl GrenadeInventory {
    pub fn new(id: WeaponId, tuning: &ThrowableDefinition) -> Self {
        Self {
            definition: id,
            count: tuning.initial,
            capacity: tuning.capacity,
            state: default(),
            held: false,
            recovery: 0.,
        }
    }
    pub fn busy(&self) -> bool {
        !matches!(self.state, ThrowState::Ready)
    }
    pub fn cancel(&mut self) {
        self.state = ThrowState::Ready;
        self.held = false;
        self.recovery = 0.;
    }
    fn advance(
        &mut self,
        held: bool,
        tuning: &ThrowableDefinition,
        dt: f32,
    ) -> Option<(f32, bool)> {
        match self.state {
            ThrowState::Priming { remaining } => {
                self.state = if remaining <= dt + 0.000001 {
                    ThrowState::Cooking {
                        remaining: tuning.fuse_seconds,
                    }
                } else {
                    ThrowState::Priming {
                        remaining: remaining - dt,
                    }
                };
            }
            ThrowState::Cooking { remaining } => {
                if tuning.cook_in_hand && remaining <= dt + 0.000001 {
                    return Some((0., true));
                }
                let fuse = if tuning.cook_in_hand {
                    remaining - dt
                } else {
                    remaining
                };
                self.state = if !held {
                    self.recovery = tuning.throw_seconds;
                    ThrowState::Releasing {
                        remaining: tuning.release_delay,
                        fuse,
                    }
                } else {
                    ThrowState::Cooking { remaining: fuse }
                };
            }
            ThrowState::Releasing { remaining, fuse } => {
                self.recovery = (self.recovery - dt).max(0.);
                let fuse = if tuning.cook_in_hand { fuse - dt } else { fuse };
                if tuning.cook_in_hand && fuse <= 0.000001 {
                    return Some((0., true));
                }
                if remaining <= dt + 0.000001 {
                    return Some((fuse, false));
                }
                self.state = ThrowState::Releasing {
                    remaining: remaining - dt,
                    fuse,
                };
            }
            ThrowState::Recovery { remaining } => {
                self.state = if remaining <= dt + 0.000001 {
                    ThrowState::Ready
                } else {
                    ThrowState::Recovery {
                        remaining: remaining - dt,
                    }
                };
            }
            ThrowState::Ready => {}
        }
        None
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
    fresh: bool,
    step_age: f32,
    angular_velocity: Vec3,
    angles: Vec3,
    wobble: f32,
    curve: f32,
    random: u64,
    contacts: u32,
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
#[derive(Component)]
pub struct WeaponViewKick {
    started: f64,
    angles: Vec2,
}
impl WeaponViewKick {
    pub fn rotation(&self, now: f64) -> Quat {
        let age = (now - self.started) as f32;
        let ease = |value: f32| (2. - value.abs()) * value;
        let scale = if age < 0. {
            0.
        } else if age < 0.1 {
            ease(age * 10.)
        } else if age < 0.5 {
            1. - ease((age - 0.1) * 2.5)
        } else {
            0.
        };
        Quat::from_euler(
            EulerRot::XYZ,
            self.angles.x * scale,
            0.,
            self.angles.y * scale,
        )
    }
}
fn damage_view_kick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut damage: MessageReader<crate::player_health::PlayerDamage>,
    players: Query<(&Player, &Transform, &Vitality), With<GrenadeInventory>>,
    poses: Query<&Transform>,
) {
    let Some(native) = game
        .content
        .weapons
        .iter()
        .find(|weapon| weapon.id.0 == "weapon.frag_grenade_zm")
        .and_then(|weapon| weapon.throwable.as_ref())
        .and_then(|profile| profile.native_launch.as_ref())
    else {
        return;
    };
    for event in damage.read() {
        let Ok((player, pose, vitality)) = players.get(event.impact.target) else {
            continue;
        };
        if vitality.state() != LifeState::Standing {
            continue;
        }
        let percent = event.impact.amount.saturating_mul(100) / game.player_health.maximum.max(1);
        let amount = (percent.min(127) as f32 * native.view_kick[0])
            .clamp(native.view_kick[1], native.view_kick[2])
            .to_radians();
        let direction = event
            .impact
            .point
            .or_else(|| {
                poses
                    .get(event.impact.attacker)
                    .ok()
                    .map(|pose| pose.translation)
            })
            .map(|origin| (pose.translation - origin).normalize_or_zero());
        let rotation = Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch);
        let angles = direction.map_or(Vec2::new(amount, 0.), |direction| {
            Vec2::new(
                -direction.dot(rotation * Vec3::NEG_Z) * amount,
                direction.dot(rotation * Vec3::X) * amount,
            )
        });
        commands.entity(event.impact.target).insert(WeaponViewKick {
            started: time.elapsed_secs_f64() - 0.02,
            angles,
        });
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
            fresh: true,
            step_age: 0.,
            angular_velocity: Vec3::ZERO,
            angles: Vec3::ZERO,
            wobble: 0.,
            curve: 0.,
            random: owner.to_bits(),
            contacts: 0,
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
                damage_view_kick
                    .in_set(GameStep::Damage)
                    .after(crate::damage::resolve)
                    .run_if(in_state(GameState::Playing)),
            )
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
    mut players: Query<&mut GrenadeInventory, With<Player>>,
    mut evidence: ResMut<EquipmentEvidence>,
) {
    for event in started.read() {
        if event.session != *session {
            continue;
        }
        for mut inventory in &mut players {
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
        Option<&WeaponViewKick>,
    )>,
    mut evidence: ResMut<EquipmentEvidence>,
    mut impacts: ResMut<crate::damage::DamageQueue>,
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
        view_kick,
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
                    weapons.cancel_action();
                    inventory.state = ThrowState::Priming {
                        remaining: tuning.prime_seconds,
                    };
                    evidence.primed += 1;
                }
                previous => {
                    launch = inventory
                        .advance(held, tuning, dt)
                        .map(|(fuse, cooked)| (fuse, cooked, false));
                    if launch.is_some_and(|(_, cooked, _)| cooked) {
                        evidence.cooked_off += 1;
                    }
                    if matches!(previous, ThrowState::Cooking { .. })
                        && matches!(inventory.state, ThrowState::Releasing { .. })
                    {
                        evidence.record(serde_json::json!({"event":"release_started",
                            "weapon":weapon.id.0,"release_delay":tuning.release_delay,
                            "throw_seconds":tuning.throw_seconds}));
                    }
                    if matches!(previous, ThrowState::Recovery { .. })
                        && matches!(inventory.state, ThrowState::Ready)
                        && let Some(native) = &tuning.native_launch
                    {
                        weapons.restore_offhand(native.restore_seconds + dt);
                    }
                }
            }
            if let Some((fuse, cooked, dropped)) = launch {
                inventory.count = inventory.count.saturating_sub(1);
                if cooked && tuning.native_launch.is_some() {
                    impacts.0.push(crate::damage::Impact {
                        point: Some(transform.translation),
                        attacker: owner,
                        target: owner,
                        source: crate::damage::ImpactSource::Weapon(weapon.id.clone()),
                        category: DamageCategory::Environmental,
                        region: HitRegion::None,
                        amount: 100000,
                    });
                }
                let rotation = Quat::from_rotation_y(player.yaw)
                    * Quat::from_rotation_x(player.pitch)
                    * view_kick.map_or(Quat::IDENTITY, |kick| {
                        kick.rotation(time.elapsed_secs_f64())
                    });
                let forward = rotation * Vec3::NEG_Z;
                let eye =
                    transform.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
                let position = if tuning.native_launch.is_some() {
                    if dropped || cooked {
                        transform.translation + Vec3::Y * (40. * 0.0254 - player.height * 0.5)
                    } else {
                        eye
                    }
                } else {
                    eye + forward * if cooked || dropped { 0. } else { 0.25 }
                };
                let velocity = if cooked || dropped {
                    Vec3::ZERO
                } else {
                    native_launch_velocity(tuning, rotation, player.velocity)
                };
                let seed = owner.to_bits().wrapping_add(evidence.primed as u64);
                let (angles, angular_velocity) =
                    grenade_spin(tuning, velocity, seed, !cooked && !dropped);
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
                            fresh: true,
                            step_age: 0.,
                            angular_velocity,
                            angles,
                            wobble: 0.,
                            curve: 0.,
                            random: seed,
                            contacts: 0,
                            lure_active: false,
                        },
                        SessionEntity,
                        Transform::from_translation(position)
                            .with_rotation(source_rotation(angles)),
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
                        remaining: if cooked {
                            tuning.throw_seconds
                        } else {
                            inventory.recovery
                        },
                    }
                };
                evidence.thrown += u32::from(!cooked && !dropped);
                evidence.record(serde_json::json!({"event":"launch","weapon":weapon.id.0,"cooked":cooked,"dropped":dropped,"fuse":fuse,"release_delay":tuning.release_delay,"position":position.to_array(),"velocity":velocity.to_array(),"count":inventory.count}));
            }
        }
    }
}
fn source_rotation(angles: Vec3) -> Quat {
    Quat::from_rotation_y(angles.y)
        * Quat::from_rotation_z(-angles.x)
        * Quat::from_rotation_x(angles.z)
}

fn grenade_spin(
    tuning: &ThrowableDefinition,
    velocity: Vec3,
    seed: u64,
    thrown: bool,
) -> (Vec3, Vec3) {
    let mut angles = Vec3::new(
        (-velocity.y).atan2(velocity.xz().length()),
        (-velocity.z).atan2(velocity.x),
        0.,
    );
    let Some(native) = &tuning.native_launch else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    if !native.rotating || !thrown {
        return (angles, Vec3::ZERO);
    }
    angles.x -= 120_f32.to_radians();
    let spin = if native.rotation_mode == 1 {
        Vec3::new(1500_f32.to_radians(), 0., 0.)
    } else {
        let signed = |offset: u64, low: f32, high: f32| {
            let sign = if crate::weapons::sample(seed.wrapping_add(offset)) < 0.5 {
                -1.
            } else {
                1.
            };
            ((low + (high - low) * crate::weapons::sample(seed.wrapping_add(offset + 1))) * sign)
                .trunc()
                .to_radians()
        };
        Vec3::new(signed(1, 340., 800.), 0., signed(3, 180., 540.))
    };
    (angles, spin)
}

impl Grenade {
    fn random(&mut self) -> f32 {
        self.random = self.random.wrapping_add(1);
        crate::weapons::sample(self.random)
    }
}

fn native_contact(
    grenade: &mut Grenade,
    tuning: &ThrowableDefinition,
    native: &GrenadeLaunch,
    velocity: Vec3,
    normal: Vec3,
    surface: usize,
    gravity: f32,
) -> (Vec3, bool, Vec3) {
    let impact_speed = velocity.dot(normal);
    let mut next = velocity - normal * impact_speed * if native.rolling { 1. } else { 2. };
    let mut displacement = Vec3::ZERO;
    let mut rolling = false;
    if velocity.length_squared() > 0. && impact_speed <= 0. {
        if native.rolling {
            let speed = next.length();
            let direction = next.normalize_or_zero();
            next *= 1. - native.friction[usize::from(speed > native.friction_speed)];
            if impact_speed.abs() >= 25. * 0.0254 {
                let restitution = tuning.normal_bounce.get(surface).copied().unwrap_or(0.);
                let restitution = (1. - (1. - restitution) * normal.y)
                    .min(native.restitution_max.max(restitution));
                next -= normal * (restitution * impact_speed);
            } else {
                rolling = true;
                if grenade.random() < native.bump_frequency {
                    let bump = (native.bump_magnitude * speed).min(native.bump_speed);
                    if bump > gravity * 0.05 {
                        next += normal * bump;
                    }
                }
                if direction.y < 25. && normal.y > 0.7 {
                    let previous = grenade.wobble;
                    grenade.wobble = (previous
                        + native.wobble_frequency * speed * 0.05 * std::f32::consts::TAU)
                        .rem_euclid(std::f32::consts::TAU);
                    let oscillation = grenade.wobble.sin() - previous.sin();
                    if speed > 0.01 * 0.0254 {
                        next *= (speed + native.wobble_forward * oscillation) / speed;
                    }
                    let side = direction.cross(normal);
                    let damping = 1. - native.wobble_damping * speed;
                    if damping > 0. {
                        displacement = side * native.wobble_side * oscillation * damping;
                    }
                    if grenade.curve == 0. {
                        grenade.curve = grenade.random() * 2. - 1.;
                    }
                    next += side * native.curve * grenade.curve;
                }
                grenade.angles.y = (-next.z).atan2(next.x);
                grenade.angles.z = std::f32::consts::FRAC_PI_2;
                grenade.angular_velocity = Vec3::new(next.length() / (5. * 0.0254), 0., 0.);
            }
        } else {
            let tangent = tuning.tangent_bounce.get(surface).copied().unwrap_or(0.);
            let restitution = tuning.normal_bounce.get(surface).copied().unwrap_or(0.);
            next = next * tangent + normal * next.dot(normal) * (restitution - tangent);
        }
    }
    let speed = next.length();
    let resting = (speed < 1. * 0.0254 || normal.y > 0.7) && speed < native.rest_speed;
    if !native.preserve_rotation && (resting || (!rolling && next.dot(normal) > 25. * 0.0254)) {
        if normal.y > 0.1 {
            let (sin, cos) = grenade.angles.y.sin_cos();
            let pitch = ((normal.x * cos - normal.z * sin) / normal.y).atan();
            if resting {
                grenade.angles.x = pitch
                    + if grenade.angles.x.sin().atan2(grenade.angles.x.cos()).abs()
                        > std::f32::consts::FRAC_PI_2
                    {
                        std::f32::consts::PI
                    } else {
                        0.
                    };
                if native.rotation_mode != 0 {
                    grenade.angles.z = std::f32::consts::FRAC_PI_2;
                }
            }
        }
    }
    if resting {
        grenade.angular_velocity = Vec3::ZERO;
    }
    (next, resting, displacement)
}

fn native_trace(
    query: &SpatialQuery,
    start: Vec3,
    delta: Vec3,
    filter: &SpatialQueryFilter,
) -> Option<(Entity, Vec3, Vec3, f32, Option<Vec3>)> {
    let direction = Dir3::new(delta).ok()?;
    let mut hit = query
        .cast_ray(start, direction, delta.length(), true, filter)
        .map(|hit| {
            (
                hit.entity,
                start + direction.as_vec3() * hit.distance.max(0.),
                hit.normal,
                (hit.distance / delta.length()).clamp(0., 1.),
                None,
            )
        });
    if hit.is_none_or(|(_, _, normal, _, _)| normal.y > 0.7) {
        let end = hit.map_or(start + delta, |(_, point, _, _, _)| point);
        let top = end + Vec3::Y * (0.135 * 0.0254);
        let length = (0.135 + 1.5) * 0.0254;
        if let Some(ground) = query.cast_ray(top, Dir3::NEG_Y, length, true, filter) {
            hit = Some((
                ground.entity,
                top + Vec3::Y * (1.5 * 0.0254 - ground.distance),
                ground.normal,
                (ground.distance / length).clamp(0., 1.),
                Some(top - Vec3::Y * ground.distance),
            ));
        }
    }
    hit
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
        let dt = if grenade.fresh {
            grenade.fresh = false;
            0.
        } else {
            time.delta_secs()
        };
        grenade.remaining -= dt;
        grenade.elapsed += dt;
        let motion_dt = if tuning.native_launch.is_some() {
            grenade.step_age += dt;
            if grenade.step_age + 0.000001 >= 0.05 {
                grenade.step_age = (grenade.step_age - 0.05).max(0.);
                0.05
            } else {
                0.
            }
        } else {
            dt
        };
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
            && motion_dt > 0.
        {
            let before = velocity;
            let position_before = transform.translation;
            let before_angles = grenade.angles;
            let angular_velocity = grenade.angular_velocity;
            grenade.angles += angular_velocity * motion_dt;
            if tuning.native_launch.is_some() {
                transform.rotation = source_rotation(grenade.angles);
            }
            let gravity_step = definition.tuning.gravity * motion_dt;
            let delta = (velocity - Vec3::Y * gravity_step * 0.5) * motion_dt;
            velocity.y -= gravity_step;
            if let Ok(direction) = Dir3::new(delta) {
                let shape = tuning.collision_half_extents.map_or_else(
                    || Collider::sphere(grenade.radius),
                    |half| Collider::cuboid(half[0] * 2., half[1] * 2., half[2] * 2.),
                );
                let filter = crate::environment::solids([grenade.owner]);
                let hit = if tuning.native_launch.is_some() {
                    native_trace(&query, transform.translation, delta, &filter)
                } else {
                    query
                        .cast_shape(
                            &shape,
                            transform.translation,
                            Quat::IDENTITY,
                            direction,
                            &ShapeCastConfig::from_max_distance(delta.length()),
                            &filter,
                        )
                        .map(|hit| {
                            (
                                hit.entity,
                                transform.translation + direction.as_vec3() * hit.distance.max(0.),
                                hit.normal1,
                                (hit.distance / delta.length()).clamp(0., 1.),
                                None,
                            )
                        })
                };
                if let Some((hit_entity, contact, normal, fraction, ground_point)) = hit {
                    transform.translation = contact;
                    if tuning.native_launch.is_some() {
                        velocity = before - Vec3::Y * gravity_step * fraction;
                        grenade.angles = before_angles + angular_velocity * motion_dt * fraction;
                    } else {
                        transform.translation += normal * 0.003;
                    }
                    let surface = surfaces.get(hit_entity).map_or(
                        if targets.contains(hit_entity) {
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
                    if tuning.sticky {
                        crate::audio::emit(
                            &mut commands,
                            alias,
                            crate::audio::SoundSource::World(transform.translation),
                        );
                        transform.rotation = Quat::from_rotation_arc(Vec3::Y, normal);
                        grenade.motion = if let Ok((local, global, parent)) = poses.get(hit_entity)
                        {
                            let pose = if !parent {
                                *local
                            } else {
                                global.compute_transform()
                            };
                            GrenadeMotion::Attached {
                                target: hit_entity,
                                offset: pose
                                    .compute_affine()
                                    .inverse()
                                    .transform_point3(transform.translation),
                                rotation: pose.rotation.inverse() * transform.rotation,
                            }
                        } else {
                            GrenadeMotion::Resting
                        };
                        let actor = targets.contains(hit_entity);
                        evidence.actor_attachments += u32::from(actor);
                        evidence.surface_attachments += u32::from(!actor);
                        evidence.record(serde_json::json!({"event":"attach","target":hit_entity.to_bits(),"actor":actor,"position":transform.translation.to_array()}));
                    } else {
                        let surface_index = surfaces
                            .get(hit_entity)
                            .ok()
                            .and_then(|(_, flags)| flags)
                            .map_or(if targets.contains(hit_entity) { 7 } else { 0 }, |flags| {
                                ((flags.0 >> 20) & 0x3f) as usize
                            });
                        let incident = velocity;
                        let resting = if let Some(native) = &tuning.native_launch {
                            let (next, resting, wobble) = native_contact(
                                &mut grenade,
                                tuning,
                                native,
                                velocity,
                                normal,
                                surface_index,
                                definition.tuning.gravity,
                            );
                            velocity = next;
                            if let Ok(direction) = Dir3::new(wobble) {
                                let distance = query
                                    .cast_ray(
                                        transform.translation,
                                        direction,
                                        wobble.length(),
                                        true,
                                        &filter,
                                    )
                                    .map_or(wobble.length(), |hit| hit.distance.max(0.));
                                transform.translation += direction.as_vec3() * distance;
                            }
                            if !resting {
                                transform.translation +=
                                    Vec3::new(normal.x, normal.y.min(0.), normal.z)
                                        * (0.1 * 0.0254);
                            }
                            transform.rotation = source_rotation(grenade.angles);
                            resting
                        } else {
                            let perpendicular = normal * velocity.dot(normal);
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
                            normal.y > 0.6 && velocity.length() < 0.5
                        };
                        if tuning.native_launch.is_none()
                            || velocity.distance(incident) > 100. * 0.0254
                        {
                            crate::audio::emit(
                                &mut commands,
                                alias,
                                crate::audio::SoundSource::World(transform.translation),
                            );
                        }
                        grenade.motion = if resting {
                            GrenadeMotion::Resting
                        } else {
                            GrenadeMotion::Flying(velocity)
                        };
                        grenade.contacts += 1;
                        evidence.bounces += 1;
                        let rolling = tuning
                            .native_launch
                            .as_ref()
                            .is_some_and(|native| native.rolling)
                            && incident.dot(normal) <= 0.
                            && incident.dot(normal).abs() < 25. * 0.0254;
                        if !rolling
                            || resting
                            || grenade.contacts <= 4
                            || grenade.contacts % 16 == 0
                        {
                            evidence.record(serde_json::json!({"event":"bounce","entity":entity.to_bits(),
                                "motion":if resting {"resting"} else if rolling {"rolling"} else {"bouncing"},
                                "normal":normal.to_array(),"surface":surface_index,
                                "position":transform.translation.to_array(),"position_before":position_before.to_array(),
                                "position_after":transform.translation.to_array(),"ground_point":ground_point.map(|point| point.to_array()),
                                "speed":velocity.length(),"velocity":velocity.to_array(),"resting":resting,
                                "rotation":transform.rotation.to_array()}));
                        }
                    }
                } else {
                    if cfg!(debug_assertions)
                        && options.validation_remote.is_some()
                        && physics_evidence.projectiles.len() < 512
                    {
                        physics_evidence.projectiles.push(serde_json::json!({
                            "kind":"throwable","entity":entity.to_bits(),"weapon":weapon.id.0,"dt":motion_dt,
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

#[cfg(test)]
mod native_grenade_tests {
    use super::*;

    fn profile() -> ThrowableDefinition {
        serde_json::from_value(serde_json::json!({
            "slot":"lethal","initial":2,"capacity":4,"round_award":2,
            "prime_seconds":0.32,"throw_seconds":0.5,"release_delay":0.101,
            "fuse_seconds":4.,"speed":20.32,"up_speed":2.54,
            "radius":6.5024,"inner_radius":0.,"inner_damage":300,"outer_damage":75,
            "tangent_bounce":vec![0.25;32],"normal_bounce":vec![0.5;32],
            "collision_radius":0.0381,"collision_half_extents":vec![0.0381;3],
            "native_launch":{"right_speed":0.,"horizontal_speed":0.,"rolling":true,
                "rotating":true,"rotation_mode":1,"preserve_rotation":false,
                "rest_speed":0.508,"friction":[0.01,0.4],"friction_speed":2.54,
                "bump_frequency":0.,"bump_magnitude":0.4,"bump_speed":2.54,
                "wobble_frequency":0.,"wobble_forward":0.,"wobble_side":0.,
                "wobble_damping":0.,"curve":0.,"restitution_max":0.3,
                "view_kick":[0.175,0.5,90.],"restore_seconds":0.25},
            "cook_in_hand":true,"sticky":false
        }))
        .unwrap()
    }

    fn grenade() -> Grenade {
        Grenade {
            owner: Entity::PLACEHOLDER,
            definition: WeaponId("weapon.frag_grenade_zm".into()),
            motion: GrenadeMotion::Flying(Vec3::ZERO),
            remaining: 4.,
            elapsed: 0.,
            radius: 0.0381,
            fresh: true,
            step_age: 0.,
            angular_velocity: Vec3::ZERO,
            angles: Vec3::ZERO,
            wobble: 0.,
            curve: 0.,
            random: 1,
            contacts: 0,
            lure_active: false,
        }
    }

    #[test]
    fn native_prime_and_release_keep_tick_boundaries_and_concurrent_recovery() {
        let tuning = profile();
        let mut inventory =
            GrenadeInventory::new(WeaponId("weapon.frag_grenade_zm".into()), &tuning);
        inventory.state = ThrowState::Priming {
            remaining: tuning.prime_seconds,
        };
        let dt = 1. / 60.;
        for _ in 0..19 {
            assert!(inventory.advance(false, &tuning, dt).is_none());
        }
        assert!(matches!(inventory.state, ThrowState::Priming { .. }));
        assert!(inventory.advance(false, &tuning, dt).is_none());
        assert!(matches!(inventory.state, ThrowState::Cooking { remaining } if remaining == 4.));
        assert!(inventory.advance(false, &tuning, dt).is_none());
        for _ in 0..6 {
            assert!(inventory.advance(false, &tuning, dt).is_none());
        }
        let (fuse, cooked) = inventory.advance(false, &tuning, dt).unwrap();
        assert!(!cooked);
        assert!((fuse - (4. - 8. * dt)).abs() < 0.00001);
        assert!((inventory.recovery - (0.5 - 7. * dt)).abs() < 0.00001);
        assert_eq!(inventory.count, 2);
    }

    #[test]
    fn native_cooking_wins_over_release_and_cancel_keeps_unspent_ammunition() {
        let tuning = profile();
        let mut inventory =
            GrenadeInventory::new(WeaponId("weapon.frag_grenade_zm".into()), &tuning);
        inventory.state = ThrowState::Releasing {
            remaining: 0.01,
            fuse: 0.01,
        };
        assert_eq!(
            inventory.advance(false, &tuning, 1. / 60.),
            Some((0., true))
        );
        inventory.held = true;
        inventory.cancel();
        assert_eq!(inventory.count, 2);
        assert!(!inventory.busy());
        assert!(!inventory.held);
    }

    #[test]
    fn native_contacts_apply_friction_before_restitution_and_settle_on_the_source_threshold() {
        let tuning = profile();
        let native = tuning.native_launch.as_ref().unwrap();
        let mut body = grenade();
        let (velocity, resting, _) = native_contact(
            &mut body,
            &tuning,
            native,
            Vec3::new(200., -100., 0.) * 0.0254,
            Vec3::Y,
            0,
            20.32,
        );
        assert!(velocity.abs_diff_eq(Vec3::new(120., 50., 0.) * 0.0254, 0.00001));
        assert!(!resting);
        let (velocity, resting, _) = native_contact(
            &mut body,
            &tuning,
            native,
            Vec3::new(50., -1., 0.) * 0.0254,
            Vec3::Y,
            0,
            20.32,
        );
        assert!(velocity.abs_diff_eq(Vec3::new(49.5, 0., 0.) * 0.0254, 0.00001));
        assert!(!resting);
        assert!((body.angular_velocity.x - 9.9).abs() < 0.00001);
        let (_, resting, _) = native_contact(
            &mut body,
            &tuning,
            native,
            Vec3::new(10., -1., 0.) * 0.0254,
            Vec3::Y,
            0,
            20.32,
        );
        assert!(resting);
        assert_eq!(body.angular_velocity, Vec3::ZERO);
        assert!((body.angles.z - std::f32::consts::FRAC_PI_2).abs() < 0.00001);
    }

    #[test]
    fn native_launch_snaps_then_adds_only_projected_player_motion() {
        let tuning = profile();
        let base = native_launch_velocity(&tuning, Quat::IDENTITY, Vec3::ZERO);
        let side = Vec3::X * 5.;
        assert!(native_launch_velocity(&tuning, Quat::IDENTITY, side).abs_diff_eq(base, 0.00001));
        let forward = base.normalize();
        assert!(
            native_launch_velocity(&tuning, Quat::IDENTITY, forward * 3.)
                .abs_diff_eq(base + forward * 3., 0.00001)
        );
    }

    #[test]
    fn native_weapon_view_kick_returns_to_zero_at_half_a_second() {
        let kick = WeaponViewKick {
            started: 0.,
            angles: Vec2::new(0.1, 0.2),
        };
        assert!(kick.rotation(0.1).angle_between(Quat::IDENTITY) > 0.2);
        assert!(kick.rotation(0.499).angle_between(Quat::IDENTITY) < 0.001);
        assert_eq!(kick.rotation(0.5), Quat::IDENTITY);
    }

    #[test]
    fn native_point_trace_advances_a_rolling_grenade_and_preserves_ground_clearance() {
        use avian3d::collider_tree::{ColliderTreeProxy, ColliderTrees};
        use bevy::ecs::system::SystemState;
        let mut world = World::new();
        let floor = Collider::cuboid(20., 0.2, 20.);
        let floor_position = Vec3::new(0., -0.1, 0.);
        let bounds = floor.aabb(floor_position, Quat::IDENTITY, 0.);
        let entity = world
            .spawn((Position(floor_position), Rotation::IDENTITY, floor, bounds))
            .id();
        let mut trees = ColliderTrees::default();
        trees.static_tree.add_proxy(
            bounds.into(),
            ColliderTreeProxy {
                collider: entity,
                body: None,
                layers: CollisionLayers::default(),
                flags: default(),
            },
        );
        world.insert_resource(trees);
        let mut state = SystemState::<SpatialQuery>::new(&mut world);
        let query = state.get(&world).unwrap();
        let tuning = profile();
        let native = tuning.native_launch.as_ref().unwrap();
        let mut body = grenade();
        let mut position = Vec3::Y * (1.5 * 0.0254);
        let mut velocity = Vec3::X * (50. * 0.0254);
        for _ in 0..10 {
            let delta = (velocity - Vec3::Y * (20.32 * 0.05 * 0.5)) * 0.05;
            let (hit, contact, normal, fraction, ground_point) =
                native_trace(&query, position, delta, &SpatialQueryFilter::default()).unwrap();
            assert_eq!(hit, entity);
            assert!(contact.x > position.x + 0.05);
            assert!((contact.y - 1.5 * 0.0254).abs() < 0.00001);
            assert!(ground_point.unwrap().y.abs() < 0.00001);
            let incident = velocity - Vec3::Y * (20.32 * 0.05 * fraction);
            let (next, resting, _) =
                native_contact(&mut body, &tuning, native, incident, normal, 0, 20.32);
            assert!(!resting);
            position = contact;
            velocity = next;
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
    mut ragdoll_explosions: MessageWriter<crate::enemies::RagdollExplosion>,
) {
    for blast in queue.0.drain(..) {
        if blast.session != *session {
            evidence.retired_blasts += 1;
            continue;
        }
        let center = blast.center;
        if blast.definition.0 == "weapon.frag_grenade_zm" {
            ragdoll_explosions.write(crate::enemies::RagdollExplosion {
                center,
                inner_radius: 0.,
                radius: blast.radius,
                direction: Vec3::ZERO,
                magnitude: 1.,
                inward: false,
            });
        }
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
