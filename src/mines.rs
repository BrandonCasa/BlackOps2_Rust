use crate::{
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    life::{LifeState, Vitality},
    origins::content::{ContentId, WeaponId},
    player::Player,
};
use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldAssetRoot};

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MineDefinition {
    pub stock: u32,
    pub limit: u32,
    pub place_seconds: f32,
    pub recovery_seconds: f32,
    pub trigger_radius: f32,
    pub trigger_cos: f32,
    pub minimum_forward: f32,
    pub alert_seconds: f32,
    pub unsafe_seconds: f32,
    pub radius: f32,
    pub inner_damage: u32,
    pub outer_damage: u32,
    pub blast_cos: f32,
    pub presentation: ContentId,
}
impl MineDefinition {
    pub fn valid(&self) -> bool {
        self.stock > 0
            && self.limit > 0
            && self.inner_damage >= self.outer_damage
            && self.outer_damage > 0
            && [
                self.place_seconds,
                self.recovery_seconds,
                self.trigger_radius,
                self.minimum_forward,
                self.alert_seconds,
                self.unsafe_seconds,
                self.radius,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            && [self.trigger_cos, self.blast_cos]
                .iter()
                .all(|v| v.is_finite() && (-1. ..=1.).contains(v))
    }
}
#[derive(Component, Default)]
pub struct MineInventory {
    pub definition: Option<WeaponId>,
    pub count: u32,
    pub remaining: f32,
    held: bool,
    placing: bool,
}
impl MineInventory {
    pub(crate) fn presentation_phase(&self) -> Option<(bool, f32)> {
        self.busy().then_some((self.placing, self.remaining))
    }
    pub fn busy(&self) -> bool {
        self.remaining > 0.
    }
    pub fn receive(
        &mut self,
        weapon: &crate::origins::content::WeaponDefinition,
    ) -> Result<(), crate::interactions::InteractionRejection> {
        if self.busy() {
            return Err(crate::interactions::InteractionRejection::Busy);
        }
        if self.definition.is_some() {
            return Err(crate::interactions::InteractionRejection::Duplicate);
        }
        let Some(profile) = &weapon.mine else {
            return Err(crate::interactions::InteractionRejection::Unavailable);
        };
        self.definition = Some(weapon.id.clone());
        self.count = profile.stock;
        Ok(())
    }
}
#[derive(Component)]
pub struct PlantedMine {
    pub owner: Entity,
    pub definition: WeaponId,
    pub countdown: Option<f32>,
    pub unsafe_plant: bool,
}
#[derive(Resource, Default)]
pub struct MineEvidence {
    pub placed: u32,
    pub unsafe_plants: u32,
    pub triggered: u32,
    pub detonated: u32,
    pub blocked: u32,
    pub replenished: u32,
    pub peak_owned: usize,
    pub events: Vec<serde_json::Value>,
}
impl MineEvidence {
    fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event)
        }
    }
}
pub fn install(app: &mut App) {
    app.init_resource::<MineEvidence>()
        .add_systems(
            FixedUpdate,
            (place, detect)
                .chain()
                .in_set(GameStep::Combat)
                .after(crate::equipment::throw_input)
                .before(crate::equipment::resolve_blasts)
                .run_if(in_state(crate::GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            replenish
                .in_set(GameStep::Rounds)
                .after(crate::rounds::advance)
                .run_if(in_state(crate::GameState::Playing)),
        );
}
fn replenish(
    mut started: MessageReader<crate::rounds::RoundStarted>,
    definition: Res<GameDefinition>,
    mut players: Query<&mut MineInventory>,
    mut evidence: ResMut<MineEvidence>,
) {
    for event in started.read() {
        for mut inventory in &mut players {
            if let Some(profile) = inventory
                .definition
                .as_ref()
                .and_then(|id| definition.content.weapons.iter().find(|w| w.id == *id))
                .and_then(|w| w.mine.as_ref())
            {
                inventory.count = profile.stock;
                evidence.replenished += 1;
                evidence.record(serde_json::json!({"event":"replenish","round":event.round,"stock":inventory.count}));
            }
        }
    }
}
fn place(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    input: Res<GameInput>,
    definition: Res<GameDefinition>,
    query: SpatialQuery,
    server: Res<AssetServer>,
    mines: Query<&PlantedMine>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut MineInventory,
        &mut crate::weapons::WeaponInventory,
        &crate::equipment::GrenadeInventory,
        &crate::equipment::TacticalInventory,
        Has<crate::perks::PerkDrinking>,
    )>,
    mut evidence: ResMut<MineEvidence>,
) {
    for (owner, player, pose, vitality, mut inventory, mut weapon, grenades, tactical, drinking) in
        &mut players
    {
        let pressed = input.mine_held && !inventory.held;
        inventory.held = input.mine_held;
        let Some(id) = inventory.definition.clone() else {
            continue;
        };
        let Some(profile) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == id)
            .and_then(|w| w.mine.as_ref())
        else {
            continue;
        };
        if vitality.state() != LifeState::Standing || drinking {
            inventory.placing = false;
            inventory.remaining = 0.;
            continue;
        }
        if inventory.remaining > 0. {
            inventory.remaining = (inventory.remaining - time.delta_secs()).max(0.);
            if inventory.remaining > 0. || !inventory.placing {
                continue;
            }
            inventory.placing = false;
            let forward = Quat::from_rotation_y(player.yaw) * Vec3::NEG_Z;
            let origin = pose.translation + forward * 0.7;
            let filter = crate::environment::solids([owner]);
            let Some(hit) = query.cast_ray(origin, Dir3::NEG_Y, player.height + 1., true, &filter)
            else {
                evidence.blocked += 1;
                continue;
            };
            if hit.normal.y < 0.7 {
                evidence.blocked += 1;
                continue;
            }
            let position = origin - Vec3::Y * hit.distance + Vec3::Y * 0.08;
            if !query
                .shape_intersections(&Collider::sphere(0.06), position, Quat::IDENTITY, &filter)
                .is_empty()
            {
                evidence.blocked += 1;
                continue;
            }
            let owned = mines
                .iter()
                .filter(|m| m.owner == owner && !m.unsafe_plant)
                .count();
            let unsafe_plant = owned >= profile.limit as usize;
            let Some(scene) = definition
                .content
                .presentations
                .iter()
                .find(|p| p.id == profile.presentation)
            else {
                continue;
            };
            let entity = commands
                .spawn((
                    SessionEntity,
                    PlantedMine {
                        owner,
                        definition: id.clone(),
                        unsafe_plant,
                        countdown: unsafe_plant.then_some(profile.unsafe_seconds),
                    },
                    Transform::from_translation(position)
                        .with_rotation(Quat::from_rotation_y(player.yaw)),
                    Visibility::default(),
                    Name::new("Claymore"),
                ))
                .id();
            commands.entity(entity).with_children(|children| {
                children.spawn((
                    WorldAssetRoot(server.load(scene.scene.clone().expect("validated mine scene"))),
                    Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                    Visibility::default(),
                ));
            });
            inventory.count -= 1;
            inventory.remaining = profile.recovery_seconds;
            evidence.placed += 1;
            evidence.unsafe_plants += u32::from(unsafe_plant);
            evidence.peak_owned = evidence.peak_owned.max(owned + usize::from(!unsafe_plant));
            evidence.record(serde_json::json!({"event":"place","position":position.to_array(),"owned":owned,"unsafe":unsafe_plant,"stock":inventory.count}));
        } else if pressed
            && inventory.count > 0
            && !grenades.busy()
            && !tactical.busy()
            && weapon.action == crate::weapons::WeaponAction::Ready
        {
            inventory.placing = true;
            inventory.remaining = profile.place_seconds;
            weapon.cancel_action();
        }
    }
}
fn detect(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    query: SpatialQuery,
    mut mines: Query<(Entity, &mut PlantedMine, &Transform)>,
    targets: Query<(Entity, &Vitality, &Transform, &Collider), Without<PlantedMine>>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    mut evidence: ResMut<MineEvidence>,
) {
    for (entity, mut mine, pose) in &mut mines {
        let Some(profile) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == mine.definition)
            .and_then(|w| w.mine.as_ref())
        else {
            continue;
        };
        let forward = pose.rotation * Vec3::NEG_Z;
        if let Some(remaining) = &mut mine.countdown {
            *remaining -= time.delta_secs();
            if *remaining <= 0. {
                blasts.0.push(crate::equipment::BlastRequest {
                    owner: mine.owner,
                    definition: mine.definition.clone(),
                    center: pose.translation,
                    radius: profile.radius,
                    inner_radius: 0.,
                    inner_damage: profile.inner_damage,
                    outer_damage: profile.outer_damage,
                    direction: Some((forward, profile.blast_cos)),
                });
                evidence.detonated += 1;
                evidence.record(serde_json::json!({"event":"detonate","unsafe":mine.unsafe_plant}));
                commands.entity(entity).despawn();
            }
            continue;
        }
        let trigger = Collider::cylinder(profile.trigger_radius, 4.8768);
        for (target, life, target_pose, collider) in &targets {
            if life.state() != LifeState::ActorAlive {
                continue;
            }
            if target == mine.owner
                || !avian3d::collision::collider::contact_query::intersection_test(
                    &trigger,
                    pose.translation,
                    Quat::IDENTITY,
                    collider,
                    target_pose.translation,
                    target_pose.rotation,
                )
                .unwrap_or(false)
            {
                continue;
            }
            let delta = target_pose.translation - pose.translation;
            let Ok(direction) = Dir3::new(delta) else {
                continue;
            };
            if forward.dot(delta) < profile.minimum_forward
                || forward.dot(direction.as_vec3()) <= profile.trigger_cos
            {
                continue;
            }
            if query
                .cast_ray_predicate(
                    pose.translation,
                    direction,
                    delta.length(),
                    true,
                    &crate::environment::solids([target, mine.owner]),
                    &|e| !targets.contains(e),
                )
                .is_some()
            {
                continue;
            }
            mine.countdown = Some(profile.alert_seconds);
            evidence.triggered += 1;
            evidence.record(serde_json::json!({"event":"trigger","target":target.to_bits(),"delay":profile.alert_seconds,"distance":delta.length()}));
            break;
        }
    }
}
