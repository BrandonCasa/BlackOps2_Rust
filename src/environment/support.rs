use crate::{
    GameState,
    damage::{DamageQueue, Impact, ImpactSource},
    gameplay::GameStep,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Component, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentOwner {
    Boundary,
    Movement,
    Access,
    Ambience,
    Robot,
    Enemy,
    Vehicle,
    Quest,
    Crafting,
    Native,
    Deferred,
}

#[derive(Clone, Deserialize)]
pub struct OwnershipDefinition {
    pub id: String,
    pub owner: EnvironmentOwner,
    pub shootable: bool,
}

#[derive(Component)]
pub struct ShootableEnvironment;

pub type ShootableTargets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Collider,
        &'static GlobalTransform,
        &'static EnvironmentOwner,
        Option<&'static super::triggers::TriggerState>,
        Option<&'static crate::robots::FootTarget>,
    ),
    (
        With<ShootableEnvironment>,
        Without<super::triggers::MapTrigger>,
        Without<ColliderDisabled>,
    ),
>;

pub type DamageTriggerTargets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Collider,
        &'static Transform,
        &'static super::triggers::TriggerState,
        &'static DamageTriggerState,
    ),
    (With<DamageTriggerState>, Without<ColliderDisabled>),
>;

pub fn cast(
    targets: &ShootableTargets,
    spatial: &SpatialQuery,
    shooter: Entity,
    origin: Vec3,
    direction: Dir3,
    range: f32,
) -> Option<(Entity, f32)> {
    let candidate = targets
        .iter()
        .filter_map(|(e, shape, pose, _, state, foot)| {
            if state.is_some_and(|state| !state.enabled) {
                return None;
            }
            let transform = pose.compute_transform();
            shape
                .cast_ray(
                    transform.translation,
                    transform.rotation,
                    origin,
                    direction.as_vec3(),
                    range,
                    true,
                )
                .map(|(d, _)| (e, d, foot.map(|foot| foot.0)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    // The robot's solid movement clip surrounds the animated sole. It must
    // not absorb a shot aimed at that same foot's damage target.
    let blocker = spatial.cast_ray(
        origin,
        direction,
        range,
        true,
        &super::solids(std::iter::once(shooter).chain(candidate.2)),
    );
    if blocker.is_some_and(|b| b.distance + 0.01 < candidate.1) {
        None
    } else {
        Some((candidate.0, candidate.1))
    }
}

#[derive(Message, Clone)]
pub struct EnvironmentalHit {
    pub target: Entity,
    pub attacker: Entity,
    pub weapon: crate::origins::WeaponId,
    pub category: crate::origins::DamageCategory,
    pub native_mod: Option<u8>,
    pub amount: u32,
    pub point: Vec3,
    pub direction: Vec3,
}

pub fn weapon_damage_mod(
    weapon: Option<&crate::origins::WeaponDefinition>,
    melee: bool,
) -> Option<u8> {
    if melee {
        return Some(7);
    }
    let weapon = weapon?;
    if weapon.mechanics.delivery == crate::origins::content::WeaponDelivery::Projectile {
        return Some(5);
    }
    match weapon.mechanics.weapon_class.as_deref()? {
        "pistol" => Some(1),
        "rifle" | "sniper" | "mg" | "smg" | "spread" => Some(2),
        _ => None,
    }
}

#[derive(Component, Default)]
pub struct DamageTriggerState {
    pub brush: bool,
    threshold: u32,
    accumulate: u32,
    damage: u32,
}
impl DamageTriggerState {
    pub fn from_source(source: &std::collections::BTreeMap<String, String>) -> Self {
        Self {
            brush: source
                .get("model")
                .is_some_and(|model| model.starts_with('*')),
            threshold: source
                .get("threshold")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            accumulate: source
                .get("accumulate")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            damage: 0,
        }
    }
    fn hit(&mut self, amount: u32) -> bool {
        if amount < self.threshold {
            return false;
        }
        self.damage = self.damage.saturating_add(amount);
        if self.accumulate > self.damage {
            return false;
        }
        self.damage = 0;
        true
    }
}
impl super::triggers::TriggerState {
    pub fn accepts_damage_mod(&self, native_mod: Option<u8>) -> bool {
        if !self.enabled {
            return false;
        }
        let flags = self.spawnflags;
        let Some(kind) = native_mod else {
            return flags & 0x13f == 0;
        };
        !(flags & 1 != 0 && kind == 1
            || flags & 2 != 0 && kind == 2
            || flags & 4 != 0 && (3..=6).contains(&kind)
            || flags & 8 != 0 && ((3..=6).contains(&kind) || kind == 15)
            || flags & 0x10 != 0 && matches!(kind, 4 | 6)
            || flags & 0x20 != 0 && kind == 7
            || flags & 0x100 != 0 && (kind == 0 || (10..=14).contains(&kind)))
    }
}

pub fn trace_damage(
    targets: &DamageTriggerTargets,
    spatial: &SpatialQuery,
    hits: &mut MessageWriter<EnvironmentalHit>,
    attacker: Entity,
    weapon: &crate::origins::WeaponId,
    category: crate::origins::DamageCategory,
    native_mod: Option<u8>,
    amount: u32,
    origin: Vec3,
    direction: Dir3,
    range: f32,
) {
    let range = spatial
        .cast_ray(origin, direction, range, true, &super::solids([attacker]))
        .map_or(range, |hit| hit.distance.min(range));
    for (target, shape, pose, state, _) in targets {
        if !state.enabled || state.contents & 0x400000 == 0 {
            continue;
        }
        let transform = pose;
        if let Some((distance, _)) = shape.cast_ray(
            transform.translation,
            transform.rotation,
            origin,
            *direction,
            range,
            true,
        ) {
            hits.write(EnvironmentalHit {
                target,
                attacker,
                weapon: weapon.clone(),
                category,
                native_mod,
                amount,
                point: origin + *direction * distance,
                direction: *direction,
            });
        }
    }
}

pub fn use_candidate(
    target: Entity,
    shape: &Collider,
    pose: &Transform,
    state: &super::triggers::TriggerState,
    spatial: &SpatialQuery,
    excluded: impl IntoIterator<Item = Entity>,
    client_mode: i32,
    eye: Vec3,
    forward: Vec3,
    require_look_at: bool,
    use_radius: f32,
) -> Option<f32> {
    let actor = super::triggers::TriggerActor {
        contact_mask: 0x200000,
        ..default()
    };
    if client_mode == 7 || !state.accepts_actor(&actor, Some(client_mode)) {
        return None;
    }
    let bounds = shape.aabb(pose.translation, pose.rotation, 0.);
    let point = (bounds.min + bounds.max) * 0.5;
    let delta = point - eye;
    let distance = delta.length();
    if distance > use_radius {
        return None;
    }
    let direction = Dir3::new(delta).ok();
    let alignment = direction.map_or(1., |direction| forward.dot(*direction));
    if require_look_at && alignment < 0.76 {
        return None;
    }
    if let Some(direction) = direction
        && spatial
            .cast_ray(
                eye,
                direction,
                distance,
                true,
                &super::solids(excluded.into_iter().chain([target])),
            )
            .is_some()
    {
        return None;
    }
    Some(distance / 0.0254 + (1. - (alignment + 1.) * 0.5) * 512. - 512.)
}

#[derive(Clone, Component, Debug)]
pub enum DamageEmitter {
    Robot(String),
    Vehicle(String),
}
impl DamageEmitter {
    pub fn source(&self) -> ImpactSource {
        match self {
            Self::Robot(id) => ImpactSource::Robot(id.clone()),
            Self::Vehicle(id) => ImpactSource::Vehicle(id.clone()),
        }
    }
    pub fn matches(&self, source: &ImpactSource) -> bool {
        matches!((self, source), (Self::Robot(a), ImpactSource::Robot(b)) | (Self::Vehicle(a), ImpactSource::Vehicle(b)) if a == b)
    }
}

#[derive(Component)]
pub struct DamageContact {
    pub(crate) session: crate::session::SessionId,
    pub amount: u32,
}

#[derive(Resource, Default, Serialize)]
pub struct EnvironmentEvidence {
    pub hits: Vec<serde_json::Value>,
    pub contacts: u32,
}

pub struct SupportPlugin;
impl Plugin for SupportPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnvironmentalHit>()
            .init_resource::<EnvironmentEvidence>()
            .add_systems(
                FixedUpdate,
                (
                    contacts,
                    observe_hits
                        .after(crate::gameplay::combat)
                        .after(crate::projectiles::flight)
                        .after(crate::equipment::resolve_blasts),
                )
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn contacts(
    session: Res<crate::session::SessionId>,
    spatial: SpatialQuery,
    emitters: Query<(
        Entity,
        &DamageEmitter,
        &DamageContact,
        &Collider,
        &GlobalTransform,
    )>,
    targets: Query<(), With<crate::life::Vitality>>,
    mut damage: ResMut<DamageQueue>,
    mut evidence: ResMut<EnvironmentEvidence>,
) {
    for (owner, emitter, contact, shape, pose) in &emitters {
        if contact.session != *session {
            continue;
        }
        let pose = pose.compute_transform();
        for target in spatial.shape_intersections(
            shape,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::default().with_excluded_entities([owner]),
        ) {
            if targets.contains(target) {
                damage.0.push(Impact {
                    point: None,
                    attacker: owner,
                    target,
                    source: emitter.source(),
                    category: crate::origins::DamageCategory::Crush,
                    region: crate::weapons::HitRegion::Torso,
                    amount: contact.amount,
                });
                evidence.contacts += 1;
            }
        }
    }
}

pub(super) fn observe_hits(
    mut hits: MessageReader<EnvironmentalHit>,
    owners: Query<&EnvironmentOwner>,
    mut triggers: Query<(&super::triggers::TriggerState, &mut DamageTriggerState)>,
    mut queue: ResMut<super::triggers::TriggerQueue>,
    mut evidence: ResMut<EnvironmentEvidence>,
) {
    for hit in hits.read() {
        if let Ok((trigger, mut damage)) = triggers.get_mut(hit.target)
            && trigger.accepts_damage_mod(hit.native_mod)
            && damage.hit(hit.amount)
        {
            if trigger.spawnflags & 0x200 != 0 {
                queue.push_once(hit.target, hit.attacker);
            } else {
                queue.push(hit.target, hit.attacker);
            }
        }
        if evidence.hits.len() >= 128 {
            evidence.hits.remove(0);
        }
        evidence.hits.push(serde_json::json!({"target":hit.target.to_bits(),"attacker":hit.attacker.to_bits(),"weapon":hit.weapon.0,"amount":hit.amount,"point":hit.point.to_array(),"owner":owners.get(hit.target).ok()}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    fn trigger(class: &str, flags: u32) -> super::super::triggers::TriggerState {
        super::super::triggers::TriggerState::from_definition(
            &serde_json::from_value(serde_json::json!({
                "id":"trigger.test", "position":[0.,0.,0.],
                "source":{"classname":class,"spawnflags":flags.to_string()},
                "shape":{"kind":"cylinder","radius":1.,"height":2.}
            }))
            .unwrap(),
        )
    }

    #[test]
    fn native_damage_exclusion_flags_preserve_other_means_of_death() {
        for (flag, rejected) in [
            (1, vec![1]),
            (2, vec![2]),
            (4, vec![3, 4, 5, 6]),
            (8, vec![3, 4, 5, 6, 15]),
            (0x10, vec![4, 6]),
            (0x20, vec![7]),
            (0x100, vec![0, 10, 11, 12, 13, 14]),
        ] {
            let state = trigger("trigger_damage", flag);
            for kind in 0..=21 {
                assert_eq!(
                    state.accepts_damage_mod(Some(kind)),
                    !rejected.contains(&kind),
                    "flag={flag:#x}, means={kind}"
                );
            }
            assert!(!state.accepts_damage_mod(None));
        }
        let mut state = trigger("trigger_damage", 0);
        assert!(state.accepts_damage_mod(None));
        state.enabled = false;
        assert!(!state.accepts_damage_mod(Some(2)));
    }

    #[test]
    fn native_damage_minimum_precedes_accumulation_and_resets_on_activation() {
        let mut damage = DamageTriggerState::from_source(
            &[
                ("threshold".into(), "10".into()),
                ("accumulate".into(), "25".into()),
            ]
            .into(),
        );
        assert!(!damage.hit(9));
        assert!(!damage.hit(10));
        assert!(!damage.hit(10));
        assert!(!damage.hit(9));
        assert!(damage.hit(10));
        assert!(!damage.hit(24));
        assert!(damage.hit(10));
        let mut no_accumulation =
            DamageTriggerState::from_source(&[("threshold".into(), "10".into())].into());
        assert!(!no_accumulation.hit(9));
        assert!(no_accumulation.hit(10));
        assert!(no_accumulation.hit(10));
    }

    #[test]
    fn robot_sole_shots_ignore_their_own_movement_clip_but_not_walls() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            PhysicsPlugins::new(Update),
        ))
        .init_asset::<Mesh>();
        let foot = app
            .world_mut()
            .spawn((
                Collider::cuboid(4., 4., 4.),
                Transform::from_xyz(0., 0., -5.),
            ))
            .id();
        let sole = app
            .world_mut()
            .spawn((
                Collider::cuboid(2., 2., 0.1),
                Transform::from_xyz(0., 0., -5.),
                Sensor,
                ShootableEnvironment,
                EnvironmentOwner::Robot,
                crate::robots::FootTarget(foot),
                CollisionLayers::new(2u32, LayerMask::NONE),
            ))
            .id();
        app.finish();
        app.cleanup();
        app.update();
        let shoot = |targets: ShootableTargets, spatial: SpatialQuery| {
            cast(
                &targets,
                &spatial,
                Entity::PLACEHOLDER,
                Vec3::ZERO,
                Dir3::NEG_Z,
                10.,
            )
        };
        assert_eq!(
            app.world_mut()
                .run_system_once(shoot)
                .unwrap()
                .map(|hit| hit.0),
            Some(sole)
        );
        app.world_mut().spawn((
            Collider::cuboid(2., 2., 0.1),
            Transform::from_xyz(0., 0., -1.),
        ));
        app.update();
        assert!(app.world_mut().run_system_once(shoot).unwrap().is_none());
    }

    #[test]
    fn source_damage_sensors_do_not_absorb_the_actor_shot_query() {
        let mut world = World::new();
        let sensor = world
            .spawn((
                super::super::triggers::MapTrigger("trigger.test".into()),
                trigger("trigger_damage", 0),
                DamageTriggerState::default(),
                Collider::cuboid(1., 1., 1.),
                GlobalTransform::IDENTITY,
                Transform::IDENTITY,
                EnvironmentOwner::Quest,
                ShootableEnvironment,
            ))
            .id();
        let mut query = SystemState::<(ShootableTargets, DamageTriggerTargets)>::new(&mut world);
        let (blocking, damage) = query.get(&world).unwrap();
        assert!(!blocking.contains(sensor));
        assert!(damage.contains(sensor));
    }

    #[test]
    fn native_use_selector_ignores_own_door_but_still_rejects_walls() {
        use avian3d::collider_tree::{ColliderTreeProxy, ColliderTrees};
        let mut world = World::new();
        let player = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let mut trees = ColliderTrees::default();
        let mut add_solid = |position: Vec3| {
            let shape = Collider::cuboid(0.2, 3., 3.);
            let bounds = shape.aabb(position, Quat::IDENTITY, 0.);
            let entity = world
                .spawn((Position(position), Rotation::IDENTITY, shape, bounds))
                .id();
            trees.static_tree.add_proxy(
                bounds.into(),
                ColliderTreeProxy {
                    collider: entity,
                    body: None,
                    layers: CollisionLayers::default(),
                    flags: default(),
                },
            );
            entity
        };
        let door = add_solid(Vec3::X);
        let wall = add_solid(Vec3::X * 0.5);
        world.insert_resource(trees);
        let mut query = SystemState::<SpatialQuery>::new(&mut world);
        let spatial = query.get(&world).unwrap();
        let shape = Collider::cuboid(0.5, 1., 1.);
        let pose = Transform::from_translation(Vec3::X);
        let state = trigger("trigger_use", 0);
        let candidate = |excluded: Vec<Entity>| {
            use_candidate(
                target,
                &shape,
                &pose,
                &state,
                &spatial,
                excluded,
                0,
                Vec3::ZERO,
                Vec3::X,
                false,
                72. * 0.0254,
            )
        };
        assert!(
            candidate(vec![player, wall]).is_none(),
            "closed door blocks its own trigger"
        );
        assert!(
            candidate(vec![player, wall, door]).is_some(),
            "own blocker may be excluded"
        );
        assert!(
            candidate(vec![player, door]).is_none(),
            "unrelated wall must still block use"
        );
    }

    #[test]
    fn native_use_selector_checks_radius_look_requirement_and_client_eligibility() {
        let mut world = World::new();
        world.init_resource::<avian3d::collider_tree::ColliderTrees>();
        let player = world.spawn_empty().id();
        let target = world.spawn_empty().id();
        let mut query = SystemState::<SpatialQuery>::new(&mut world);
        let spatial = query.get(&world).unwrap();
        let shape = Collider::cuboid(1., 1., 1.);
        let mut state = trigger("trigger_use", 0);
        let pose = Transform::from_translation(Vec3::X * (72. * 0.0254));
        assert!(
            use_candidate(
                target,
                &shape,
                &pose,
                &state,
                &spatial,
                [player],
                0,
                Vec3::ZERO,
                Vec3::X,
                false,
                72. * 0.0254
            )
            .is_some()
        );
        let outside = Transform::from_translation(Vec3::X * (72. * 0.0254 + 0.001));
        assert!(
            use_candidate(
                target,
                &shape,
                &outside,
                &state,
                &spatial,
                [player],
                0,
                Vec3::ZERO,
                Vec3::X,
                false,
                72. * 0.0254
            )
            .is_none()
        );
        assert!(
            use_candidate(
                target,
                &shape,
                &pose,
                &state,
                &spatial,
                [player],
                0,
                Vec3::ZERO,
                Vec3::NEG_X,
                false,
                72. * 0.0254
            )
            .is_some()
        );
        assert!(
            use_candidate(
                target,
                &shape,
                &pose,
                &state,
                &spatial,
                [player],
                0,
                Vec3::ZERO,
                Vec3::NEG_X,
                true,
                72. * 0.0254
            )
            .is_none()
        );
        for mode in [2, 7] {
            assert!(
                use_candidate(
                    target,
                    &shape,
                    &pose,
                    &state,
                    &spatial,
                    [player],
                    mode,
                    Vec3::ZERO,
                    Vec3::X,
                    false,
                    72. * 0.0254
                )
                .is_none()
            );
        }
        state.excluded_clients.insert(0);
        assert!(
            use_candidate(
                target,
                &shape,
                &pose,
                &state,
                &spatial,
                [player],
                0,
                Vec3::ZERO,
                Vec3::X,
                false,
                72. * 0.0254
            )
            .is_none()
        );
    }
}
