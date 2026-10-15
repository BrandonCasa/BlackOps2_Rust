use super::VolumeDefinition;
use crate::{assets::GameDefinition, player::Player};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const TRIGGER_LAYER: u32 = 8;

#[derive(Clone, Deserialize)]
pub struct TriggerDefinition {
    pub id: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub target_position: Option<[f32; 3]>,
    pub source: BTreeMap<String, String>,
    pub shape: TriggerShape,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerShape {
    Convex {
        volumes: Vec<VolumeDefinition>,
    },
    Cylinder {
        radius: f32,
        height: f32,
    },
    Brush {
        rotation: [f32; 4],
        volumes: Vec<VolumeDefinition>,
        triangle_shapes: Vec<super::brushes::TriangleShape>,
    },
}

impl TriggerDefinition {
    pub fn removed_at_spawn(&self) -> bool {
        self.source
            .get("classname")
            .is_some_and(|class| matches!(class.as_str(), "trigger_multiple" | "trigger_once"))
            && self
                .source
                .get("spawnflags")
                .and_then(|flags| flags.parse::<u32>().ok())
                .is_some_and(|flags| flags & 0x80 != 0)
            && !self
                .source
                .get("targetname")
                .is_some_and(|name| matches!(name.as_str(), "sndBG" | "sndCaptureZone"))
    }
    pub fn collider(&self) -> Option<Collider> {
        match &self.shape {
            TriggerShape::Brush {
                volumes,
                triangle_shapes,
                ..
            } => super::brushes::compound(volumes, triangle_shapes),
            TriggerShape::Convex { volumes } => {
                let pieces: Option<Vec<_>> = volumes
                    .iter()
                    .map(|v| {
                        Collider::convex_hull(
                            &v.vertices
                                .iter()
                                .copied()
                                .map(Vec3::from_array)
                                .collect::<Vec<_>>(),
                        )
                        .map(|shape| (Vec3::from_array(v.position), Quat::IDENTITY, shape))
                    })
                    .collect();
                pieces.filter(|p| !p.is_empty()).map(Collider::compound)
            }
            TriggerShape::Cylinder { radius, height } => Some(Collider::compound(vec![(
                Vec3::Y * height * 0.5,
                Quat::IDENTITY,
                Collider::cylinder(*radius, *height),
            )])),
        }
    }
    pub fn transform(&self) -> Transform {
        let rotation = match &self.shape {
            TriggerShape::Brush { rotation, .. } => Quat::from_array(*rotation),
            _ => Quat::IDENTITY,
        };
        Transform::from_translation(Vec3::from_array(self.position)).with_rotation(rotation)
    }
    pub fn interior_points(&self) -> Vec<Vec3> {
        let origin = Vec3::from_array(self.position);
        match &self.shape {
            TriggerShape::Brush {
                volumes,
                triangle_shapes,
                ..
            } => {
                let pose = self.transform();
                volumes
                    .iter()
                    .map(|volume| pose.transform_point(volume_center(volume)))
                    .chain(triangle_shapes.iter().flat_map(|shape| {
                        shape.indices.iter().map(move |indices| {
                            pose.transform_point(
                                (Vec3::from_array(shape.vertices[indices[0] as usize])
                                    + Vec3::from_array(shape.vertices[indices[1] as usize])
                                    + Vec3::from_array(shape.vertices[indices[2] as usize]))
                                    / 3.,
                            )
                        })
                    }))
                    .collect()
            }
            TriggerShape::Convex { volumes } => {
                volumes.iter().map(|v| origin + volume_center(v)).collect()
            }
            TriggerShape::Cylinder { height, .. } => vec![origin + Vec3::Y * height * 0.5],
        }
    }
    pub fn outside_point(&self) -> Vec3 {
        let max = match &self.shape {
            TriggerShape::Brush {
                volumes,
                triangle_shapes,
                ..
            } => {
                let rotation = self.transform().rotation;
                volumes
                    .iter()
                    .flat_map(|volume| {
                        volume.vertices.iter().map(move |vertex| {
                            rotation
                                * (Vec3::from_array(volume.position) + Vec3::from_array(*vertex))
                        })
                    })
                    .chain(triangle_shapes.iter().flat_map(|shape| {
                        shape
                            .vertices
                            .iter()
                            .map(|vertex| rotation * Vec3::from_array(*vertex))
                    }))
                    .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max)
            }
            TriggerShape::Convex { volumes } => volumes
                .iter()
                .flat_map(|v| {
                    v.vertices
                        .iter()
                        .map(|p| Vec3::from_array(v.position) + Vec3::from_array(*p))
                })
                .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
            TriggerShape::Cylinder { radius, height } => Vec3::new(*radius, *height, *radius),
        };
        Vec3::from_array(self.position) + max + Vec3::splat(0.1)
    }
}

fn volume_center(volume: &VolumeDefinition) -> Vec3 {
    Vec3::from_array(volume.position)
        + volume
            .vertices
            .iter()
            .copied()
            .map(Vec3::from_array)
            .sum::<Vec3>()
            / volume.vertices.len() as f32
}

fn stable_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() < 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}



#[derive(Component)]
pub struct MapTrigger(pub String);

pub(super) fn spawn(
    mut commands: Commands,
    game: Res<GameDefinition>,
    existing: Query<(), With<MapTrigger>>,
) {
    if !existing.is_empty() {
        return;
    }
    for trigger in &game.environment.triggers {
        if game
            .environment
            .crafting_benches
            .iter()
            .any(|bench| bench.source_trigger == trigger.id && bench.delete_source)
        {
            continue;
        }
        if trigger.removed_at_spawn() {
            continue;
        }
        if let Some(shape) = trigger.collider() {
            let mut entity = commands.spawn((
                MapTrigger(trigger.id.clone()),
                crate::gameplay::SessionEntity,
                shape,
                Sensor,
                CollisionLayers::new(TRIGGER_LAYER, LayerMask::NONE),
                trigger.transform(),
                TriggerState::from_definition(trigger),
                Name::new(trigger.id.clone()),
            ));
            if let Some(owner) = game
                .environment
                .ownership
                .iter()
                .find(|o| o.id == trigger.id)
            {
                entity.insert(owner.owner);
                if owner.shootable {
                    entity.insert(super::support::ShootableEnvironment);
                }
            }
            if trigger
                .source
                .get("classname")
                .is_some_and(|class| class == "trigger_damage")
            {
                entity.insert(super::support::DamageTriggerState::from_source(
                    &trigger.source,
                ));
            }
            if let Some(boundary) = super::hazards::boundary(trigger) {
                entity.insert(boundary);
            }
        }
    }
}

pub type TriggerSites<'w, 's> =
    Query<'w, 's, (Entity, &'static MapTrigger, &'static Transform), Without<Player>>;

#[derive(Component, Clone, Debug)]
pub struct TriggerActor {
    pub contact_mask: u32,
    pub client_index: Option<u32>,
    pub team: Option<u8>,
    pub touch_callback: bool,
}
impl Default for TriggerActor {
    fn default() -> Self {
        Self {
            contact_mask: 0x40000000,
            client_index: Some(0),
            team: None,
            touch_callback: false,
        }
    }
}
impl TriggerActor {
    pub fn sentient() -> Self {
        Self {
            contact_mask: 0x001c0000,
            client_index: None,
            team: None,
            touch_callback: true,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct TriggerState {
    pub enabled: bool,
    pub contents: u32,
    pub spawnflags: u32,
    pub excluded_clients: BTreeSet<u32>,
    pub team: Option<u8>,
    pub team_inverted: bool,
    pub automatic: bool,
    pub touch_callback: bool,
    pub look_at: bool,
    pub delete_on_activation: bool,
}
impl TriggerState {
    pub fn from_definition(definition: &TriggerDefinition) -> Self {
        let class = definition
            .source
            .get("classname")
            .map(String::as_str)
            .unwrap_or("");
        let mut spawnflags = definition
            .source
            .get("spawnflags")
            .and_then(|flags| flags.parse::<u32>().ok())
            .unwrap_or(0);
        let client_sensor = spawnflags & 0x80 != 0
            && definition
                .source
                .get("targetname")
                .is_some_and(|name| matches!(name.as_str(), "sndBG" | "sndCaptureZone"));
        let automatic = !client_sensor
            && matches!(
                class,
                "trigger_multiple" | "trigger_once" | "trigger_box" | "trigger_radius"
            );
        if automatic
            && (class == "trigger_once"
                || definition
                    .source
                    .get("wait")
                    .and_then(|wait| wait.parse::<f32>().ok())
                    .is_some_and(|wait| wait <= 0.))
        {
            spawnflags |= 0x40;
        }
        if class == "trigger_damage"
            && definition
                .source
                .get("wait")
                .and_then(|wait| wait.parse::<f32>().ok())
                .is_some_and(|wait| wait <= 0.)
        {
            spawnflags |= 0x200;
        }
        let contents = if automatic {
            contact_contents(spawnflags)
                | if matches!(class, "trigger_box" | "trigger_radius") {
                    0x200000
                } else {
                    0
                }
        } else if class == "trigger_box_use" {
            contact_contents(spawnflags) | 0x200000
        } else if matches!(
            class,
            "trigger_use" | "trigger_use_touch" | "trigger_radius_use"
        ) {
            0x200000
        } else if class == "trigger_lookat" {
            0x20000000
        } else if class == "trigger_damage" {
            0x405c0008
        } else {
            0
        };
        Self {
            enabled: !definition.removed_at_spawn(),
            contents,
            spawnflags,
            excluded_clients: BTreeSet::new(),
            team: definition
                .source
                .get("teamnum")
                .and_then(|team| team.parse::<u8>().ok()),
            team_inverted: false,
            automatic,
            touch_callback: automatic,
            look_at: class == "trigger_lookat",
            delete_on_activation: automatic && spawnflags & 0x40 != 0,
        }
    }
    pub fn accepts_player(&self, mode: i32) -> bool {
        self.accepts_actor(&TriggerActor::default(), Some(mode))
    }
    pub fn accepts_contact(&self, actor: &TriggerActor, mode: Option<i32>) -> bool {
        (self.touch_callback || actor.touch_callback) && self.accepts_actor(actor, mode)
    }
    pub fn accepts_actor(&self, actor: &TriggerActor, mode: Option<i32>) -> bool {
        self.enabled
            && self.contents & actor.contact_mask != 0
            && mode.is_none_or(|mode| mode < 2 || matches!(mode, 7..=9))
            && actor
                .client_index
                .is_none_or(|client| !self.excluded_clients.contains(&client))
            && actor
                .team
                .zip(self.team)
                .is_none_or(|(actor, trigger)| (actor == trigger) != self.team_inverted)
    }
}

pub fn contact_contents(flags: u32) -> u32 {
    (if flags & 8 == 0 { 0x40000000 } else { 0 })
        | (if flags & 1 != 0 { 0x40000 } else { 0 })
        | (if flags & 2 != 0 { 0x80000 } else { 0 })
        | (if flags & 4 != 0 { 0x100000 } else { 0 })
        | (if flags & 16 != 0 { 8 } else { 0 })
}

#[derive(Component)]
#[relationship(relationship_target = OwnedTriggers)]
pub struct OwnedTrigger(pub Entity);

#[derive(Component, Default)]
#[relationship_target(relationship = OwnedTrigger, linked_spawn)]
pub struct OwnedTriggers(Vec<Entity>);

pub fn spawn_owned(
    commands: &mut Commands,
    definition: &TriggerDefinition,
    owner: Entity,
    category: super::support::EnvironmentOwner,
) -> Option<Entity> {
    if definition.removed_at_spawn() {
        return None;
    }
    let mut entity = commands.spawn((
        MapTrigger(definition.id.clone()),
        OwnedTrigger(owner),
        crate::gameplay::SessionEntity,
        definition.collider()?,
        Sensor,
        CollisionLayers::new(TRIGGER_LAYER, LayerMask::NONE),
        definition.transform(),
        TriggerState::from_definition(definition),
        category,
    ));
    if definition
        .source
        .get("classname")
        .is_some_and(|class| class == "trigger_damage")
    {
        entity.insert(super::support::DamageTriggerState::from_source(
            &definition.source,
        ));
    }
    Some(entity.id())
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct TriggerActivated {
    #[event_target]
    pub trigger: Entity,
    pub activator: Entity,
}
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct TriggerTouched {
    #[event_target]
    pub trigger: Entity,
    pub activator: Entity,
}
#[derive(Clone, Copy)]
struct PendingTrigger {
    trigger: Entity,
    activator: Entity,
    delete: bool,
}
#[derive(Resource, Default)]
pub struct TriggerQueue {
    pending: Vec<PendingTrigger>,
    immediate: Vec<PendingTrigger>,
    deletions: BTreeSet<Entity>,
}
impl TriggerQueue {
    pub fn push(&mut self, trigger: Entity, activator: Entity) {
        self.enqueue(trigger, activator, false, false);
    }
    pub fn push_once(&mut self, trigger: Entity, activator: Entity) {
        self.enqueue(trigger, activator, true, false);
    }
    pub fn push_and_delete(&mut self, trigger: Entity, activator: Entity) {
        self.enqueue(trigger, activator, false, true);
    }
    fn enqueue(&mut self, trigger: Entity, activator: Entity, wait_delete: bool, delete: bool) {
        let event = PendingTrigger {
            trigger,
            activator,
            delete,
        };
        if wait_delete {
            self.deletions.insert(trigger);
        }
        if self.pending.len() == 256 {
            self.immediate.push(event);
        } else {
            self.pending.push(event);
        }
    }
}

#[derive(Clone, Copy)]
struct CollectedContact {
    pending: PendingTrigger,
    automatic: bool,
    delete_at_resume: bool,
}

fn overlapping_contacts(
    spatial: SpatialQuery,
    actors: Query<
        (
            Entity,
            &Transform,
            &Collider,
            &TriggerActor,
            Option<&crate::access::TriggerClientMode>,
        ),
        Without<ColliderDisabled>,
    >,
    triggers: Query<&TriggerState, Without<ColliderDisabled>>,
) -> Vec<CollectedContact> {
    let mut contacts = Vec::new();
    for (activator, pose, shape, actor, mode) in &actors {
        for trigger in spatial.shape_intersections(
            shape,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::from_mask(TRIGGER_LAYER).with_excluded_entities([activator]),
        ) {
            let Ok(state) = triggers.get(trigger) else {
                continue;
            };
            if state.accepts_contact(actor, mode.map(|mode| mode.0)) {
                contacts.push(CollectedContact {
                    pending: PendingTrigger {
                        trigger,
                        activator,
                        delete: false,
                    },
                    automatic: state.automatic,
                    delete_at_resume: state.delete_on_activation,
                });
            }
        }
    }
    contacts
}

fn process_contact(world: &mut World, contact: CollectedContact) {
    let pending = contact.pending;
    if world.get_entity(pending.trigger).is_err() || world.get_entity(pending.activator).is_err() {
        return;
    }
    world.trigger(TriggerTouched {
        trigger: pending.trigger,
        activator: pending.activator,
    });
    world.trigger(TriggerTouched {
        trigger: pending.activator,
        activator: pending.trigger,
    });
    if !contact.automatic {
        return;
    }
    if world.resource::<TriggerQueue>().pending.len() == 256 {
        deliver(world, pending);
        world.flush();
    } else {
        world.resource_mut::<TriggerQueue>().pending.push(pending);
    }
    if contact.delete_at_resume {
        world
            .resource_mut::<TriggerQueue>()
            .deletions
            .insert(pending.trigger);
    }
}

pub(crate) fn contacts(world: &mut World) {
    let contacts = world
        .run_system_cached(overlapping_contacts)
        .expect("Trigger overlap system failed");
    for contact in contacts {
        process_contact(world, contact);
    }
}

const LOOK_MASK: u32 = 0x2280e803;
const LOOK_RANGE: f32 = 15000. * 0.0254;

fn first_look_target<'a>(
    origin: Vec3,
    direction: Dir3,
    blockers: Option<f32>,
    candidates: impl Iterator<Item = (Entity, &'a Collider, Transform, &'a TriggerState)>,
) -> Option<Entity> {
    let (entity, distance, look_at) = candidates
        .filter_map(|(entity, shape, pose, state)| {
            if !state.enabled || state.contents & LOOK_MASK == 0 {
                return None;
            }
            shape
                .cast_ray(
                    pose.translation,
                    pose.rotation,
                    origin,
                    *direction,
                    LOOK_RANGE,
                    true,
                )
                .map(|(distance, _)| (entity, distance, state.look_at))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    (look_at && blockers.is_none_or(|blocker| distance <= blocker)).then_some(entity)
}

fn look(
    spatial: SpatialQuery,
    players: Query<(Entity, &Transform, &Player)>,
    triggers: Query<(Entity, &Collider, &Transform, &TriggerState), Without<ColliderDisabled>>,
    mut queue: ResMut<TriggerQueue>,
) {
    for (activator, pose, player) in &players {
        let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
        let direction =
            Dir3::new(Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z)
                .expect("Player view direction is invalid");
        let blocker = spatial.cast_ray(
            eye,
            direction,
            LOOK_RANGE,
            true,
            &SpatialQueryFilter::from_mask(1u32 | super::brushes::BRUSH_LAYER)
                .with_excluded_entities([activator]),
        );
        if let Some(trigger) = first_look_target(
            eye,
            direction,
            blocker.map(|blocker| blocker.distance),
            triggers
                .iter()
                .map(|(entity, shape, pose, state)| (entity, shape, *pose, state)),
        ) {
            queue.push(trigger, activator);
        }
    }
}

fn deliver(world: &mut World, pending: PendingTrigger) {
    if world.get_entity(pending.trigger).is_err() || world.get_entity(pending.activator).is_err() {
        return;
    }
    world.trigger(TriggerActivated {
        trigger: pending.trigger,
        activator: pending.activator,
    });
    if pending.delete {
        world.commands().entity(pending.trigger).try_despawn();
    }
}

pub(super) fn dispatch(world: &mut World) {
    let mut queue = std::mem::take(&mut *world.resource_mut::<TriggerQueue>());
    for pending in queue.immediate.drain(..) {
        deliver(world, pending);
        world.flush();
    }
    loop {
        let mut delivered = BTreeSet::new();
        let mut index = 0;
        while index < queue.pending.len() {
            let pending = queue.pending[index];
            if world.get_entity(pending.trigger).is_err()
                || world.get_entity(pending.activator).is_err()
            {
                queue.pending.swap_remove(index);
            } else if delivered.insert(pending.trigger) {
                queue.pending.swap_remove(index);
                deliver(world, pending);
            } else {
                index += 1;
            }
        }
        world.flush();
        queue.deletions.extend(std::mem::take(
            &mut world.resource_mut::<TriggerQueue>().deletions,
        ));
        for entity in std::mem::take(&mut queue.deletions) {
            if world.get_entity(entity).is_ok() {
                world.despawn(entity);
            }
        }
        world.flush();
        if queue.pending.is_empty() {
            break;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<TriggerQueue>()
        .register_required_components::<Player, TriggerActor>()
        .register_required_components_with::<crate::enemies::Zombie, TriggerActor>(
            TriggerActor::sentient,
        )
        .add_systems(
            FixedUpdate,
            (contacts, look, dispatch)
                .chain()
                .in_set(crate::gameplay::GameStep::Movement)
                .after(crate::player::move_player)
                .before(crate::access::apply_flags)
                .run_if(in_state(crate::GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            dispatch
                .in_set(crate::gameplay::GameStep::Combat)
                .after(super::support::observe_hits)
                .run_if(in_state(crate::GameState::Playing)),
        );
}

pub(crate) fn reset(world: &mut World) {
    world.insert_resource(TriggerQueue::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::collision::collider::contact_query::intersection_test;

    fn definition(class: &str, flags: u32) -> TriggerDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"trigger.test", "position":[0.,0.,0.],
            "source":{"classname":class,"spawnflags":flags.to_string()},
            "shape":{"kind":"cylinder","radius":1.,"height":2.}
        }))
        .unwrap()
    }

    #[test]
    fn native_contact_modes_categories_and_client_filters() {
        let mut state = TriggerState::from_definition(&definition("trigger_box", 1024));
        for mode in [-2, -1, 0, 1, 7, 8, 9] {
            assert!(state.accepts_player(mode));
        }
        for mode in [2, 3, 4, 5, 6, 10, 11] {
            assert!(!state.accepts_player(mode));
        }
        for mask in [8, 0x400000, 0x1c0000] {
            assert!(!state.accepts_actor(
                &TriggerActor {
                    contact_mask: mask,
                    client_index: None,
                    team: None,
                    touch_callback: false,
                },
                None
            ));
        }
        assert_eq!(state.contents, 0x40200000);
        state.excluded_clients.insert(0);
        assert!(!state.accepts_player(0));
        state.excluded_clients.clear();
        state.team = Some(2);
        let actor = TriggerActor {
            contact_mask: 0x40000000,
            client_index: Some(0),
            team: Some(2),
            touch_callback: false,
        };
        assert!(state.accepts_actor(&actor, Some(0)));
        state.team_inverted = true;
        assert!(!state.accepts_actor(&actor, Some(0)));
        state.enabled = false;
        assert!(!state.accepts_player(0));
        assert_eq!(contact_contents(8 | 1 | 2 | 4 | 16), 0x001c0008);
        assert_eq!(
            TriggerState::from_definition(&definition("trigger_multiple", 1024)).contents,
            0x40000000
        );
    }

    #[test]
    fn touch_notifications_require_either_native_callback() {
        let damage = TriggerState::from_definition(&definition("trigger_damage", 0));
        let player = TriggerActor::default();
        assert!(damage.accepts_actor(&player, Some(0)));
        assert!(!damage.accepts_contact(&player, Some(0)));
        assert!(damage.accepts_contact(&TriggerActor::sentient(), None));
        let box_trigger = TriggerState::from_definition(&definition("trigger_box", 0));
        assert!(box_trigger.accepts_contact(&player, Some(0)));
        let use_trigger = TriggerState::from_definition(&definition("trigger_use", 0));
        let use_actor = TriggerActor {
            contact_mask: 0x200000,
            ..default()
        };
        assert!(use_trigger.accepts_actor(&use_actor, Some(0)));
        assert!(!use_trigger.accepts_contact(&use_actor, Some(0)));
    }

    #[test]
    fn source_unitrigger_box_uses_centered_geometry_without_automatic_touch_activation() {
        let source: TriggerDefinition = serde_json::from_value(serde_json::json!({
            "id":"trigger.generator_start_bunker.use", "position":[0.,0.,0.],
            "source":{"classname":"trigger_box_use"},
            "shape":{"kind":"convex","volumes":[{
                "id":"trigger.generator_start_bunker.use.volume", "position":[0.,0.,0.],
                "vertices":(0..8).map(|corner| [
                    if corner & 1 == 0 {-1.} else {1.},
                    if corner & 2 == 0 {-1.} else {1.},
                    if corner & 4 == 0 {-1.} else {1.}
                ]).collect::<Vec<_>>()
            }]}
        }))
        .unwrap();
        let bounds = source
            .collider()
            .unwrap()
            .aabb(Vec3::ZERO, Quat::IDENTITY, 0.);
        assert_eq!(bounds.min, Vec3::splat(-1.));
        assert_eq!(bounds.max, Vec3::ONE);
        let state = TriggerState::from_definition(&source);
        assert_eq!(state.contents, 0x40200000);
        let actor = TriggerActor {
            contact_mask: 0x200000,
            ..default()
        };
        assert!(state.accepts_actor(&actor, Some(0)));
        assert!(!state.accepts_contact(&TriggerActor::default(), Some(0)));
        assert!(!state.automatic);
    }

    #[test]
    fn wait_only_marks_native_single_use_without_positive_delay() {
        let mut source = definition("trigger_box", 0);
        source.source.insert("wait".into(), "2.5".into());
        assert!(!TriggerState::from_definition(&source).delete_on_activation);
        source.source.insert("wait".into(), "0".into());
        assert!(TriggerState::from_definition(&source).delete_on_activation);
        source
            .source
            .insert("classname".into(), "trigger_damage".into());
        let damage = TriggerState::from_definition(&source);
        assert_eq!(damage.spawnflags & 0x200, 0x200);
        assert!(!damage.automatic);
        assert_eq!(damage.contents, 0x405c0008);
    }

    #[test]
    fn rotated_compound_overlap_uses_geometry_and_distinct_pieces() {
        let vertices: Vec<_> = [-0.1, 0.1]
            .into_iter()
            .flat_map(|x| {
                [-0.5, 0.5]
                    .into_iter()
                    .flat_map(move |y| [-2., 2.].into_iter().map(move |z| [x, y, z]))
            })
            .collect();
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let source = TriggerDefinition {
            id: "trigger.compound".into(),
            position: [4., 0., 3.],
            target_position: None,
            source: BTreeMap::new(),
            shape: TriggerShape::Brush {
                rotation: rotation.to_array(),
                volumes: vec![
                    VolumeDefinition {
                        id: "a".into(),
                        position: [0., 0., 0.],
                        vertices: vertices.clone(),
                        contents: 0,
                        surface_flags: vec![],
                    },
                    VolumeDefinition {
                        id: "b".into(),
                        position: [4., 0., 0.],
                        vertices,
                        contents: 0,
                        surface_flags: vec![],
                    },
                ],
                triangle_shapes: vec![],
            },
        };
        let shape = source.collider().unwrap();
        let pose = source.transform();
        let probe = Collider::sphere(0.01);
        for point in source.interior_points() {
            assert!(
                intersection_test(
                    &shape,
                    pose.translation,
                    pose.rotation,
                    &probe,
                    point,
                    Quat::IDENTITY
                )
                .unwrap()
            );
        }
        let gap = pose.transform_point(Vec3::new(2., 0., 0.));
        assert!(
            !intersection_test(
                &shape,
                pose.translation,
                pose.rotation,
                &probe,
                gap,
                Quat::IDENTITY
            )
            .unwrap()
        );
        let aabb_corner = pose.transform_point(Vec3::new(1., 0., 1.));
        assert!(
            !intersection_test(
                &shape,
                pose.translation,
                pose.rotation,
                &probe,
                aabb_corner,
                Quat::IDENTITY
            )
            .unwrap()
        );
    }

    #[test]
    fn look_ray_selects_actual_first_native_hit_and_rejects_occlusion() {
        let mut world = World::new();
        let far = world.spawn_empty().id();
        let near = world.spawn_empty().id();
        let look = TriggerState::from_definition(&definition("trigger_lookat", 0));
        let mut damage = TriggerState::from_definition(&definition("trigger_damage", 0));
        assert_eq!(damage.contents & LOOK_MASK, 0);
        let shape = Collider::cuboid(2., 2., 2.);
        let direction = Dir3::NEG_Z;
        let candidates = || {
            [
                (far, &shape, Transform::from_xyz(0., 0., -20.), &look),
                (near, &shape, Transform::from_xyz(0., 0., -10.), &look),
            ]
            .into_iter()
        };
        assert_eq!(
            first_look_target(Vec3::ZERO, direction, None, candidates()),
            Some(near)
        );
        assert_eq!(
            first_look_target(Vec3::ZERO, direction, Some(5.), candidates()),
            None
        );
        assert_eq!(
            first_look_target(Vec3::ZERO, Dir3::X, None, candidates()),
            None
        );
        assert_eq!(
            first_look_target(
                Vec3::ZERO,
                direction,
                None,
                [
                    (far, &shape, Transform::from_xyz(0., 0., -20.), &look),
                    (near, &shape, Transform::from_xyz(0., 0., -10.), &damage)
                ]
                .into_iter()
            ),
            Some(far)
        );
        damage.contents |= 0x20000000;
        assert_eq!(
            first_look_target(
                Vec3::ZERO,
                direction,
                None,
                [
                    (far, &shape, Transform::from_xyz(0., 0., -20.), &look),
                    (near, &shape, Transform::from_xyz(0., 0., -10.), &damage),
                ]
                .into_iter()
            ),
            None
        );
        assert_eq!(
            first_look_target(
                Vec3::ZERO,
                direction,
                None,
                [(
                    far,
                    &shape,
                    Transform::from_xyz(0., 0., -LOOK_RANGE - 2.),
                    &look
                )]
                .into_iter()
            ),
            None
        );
    }

    #[derive(Resource, Default)]
    struct Delivered(Vec<TriggerActivated>);
    fn observe(event: On<TriggerActivated>, mut delivered: ResMut<Delivered>) {
        delivered.0.push(*event);
    }
    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<TriggerQueue>();
        world.init_resource::<Delivered>();
        world.add_observer(observe);
        world.flush();
        world
    }

    #[test]
    fn repeated_contacts_keep_recipient_and_activator() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        for _ in 0..3 {
            world
                .resource_mut::<TriggerQueue>()
                .push(trigger, activator);
        }
        dispatch(&mut world);
        assert_eq!(world.resource::<Delivered>().0.len(), 3);
        world
            .resource_mut::<TriggerQueue>()
            .push(trigger, activator);
        dispatch(&mut world);
        let delivered = &world.resource::<Delivered>().0;
        assert_eq!(delivered.len(), 4);
        assert!(
            delivered
                .iter()
                .all(|event| event.trigger == trigger && event.activator == activator)
        );
    }

    #[test]
    fn deferred_consumer_deletion_invalidates_following_passes() {
        let mut world = world();
        world.add_observer(|event: On<TriggerActivated>, mut commands: Commands| {
            commands.entity(event.trigger).despawn();
        });
        world.flush();
        let trigger = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        for _ in 0..3 {
            world
                .resource_mut::<TriggerQueue>()
                .push(trigger, activator);
        }
        dispatch(&mut world);
        assert_eq!(world.resource::<Delivered>().0.len(), 1);
        assert!(world.get_entity(trigger).is_err());
    }

    #[test]
    fn replacement_activator_deletion_and_reset_discard_pending() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        world
            .resource_mut::<TriggerQueue>()
            .push_once(trigger, activator);
        world.despawn(trigger);
        let replacement = world.spawn_empty().id();
        assert_ne!(trigger, replacement);
        dispatch(&mut world);
        assert!(world.resource::<Delivered>().0.is_empty());
        assert!(world.get_entity(replacement).is_ok());
        world
            .resource_mut::<TriggerQueue>()
            .push(replacement, activator);
        world.despawn(activator);
        dispatch(&mut world);
        assert!(world.resource::<Delivered>().0.is_empty());
        let activator = world.spawn_empty().id();
        world
            .resource_mut::<TriggerQueue>()
            .push_once(replacement, activator);
        reset(&mut world);
        dispatch(&mut world);
        assert!(world.resource::<Delivered>().0.is_empty());
        assert!(world.get_entity(replacement).is_ok());
    }

    #[test]
    fn owner_cleanup_and_single_use_invalidate_queued_contacts() {
        let mut world = world();
        let owner = world.spawn_empty().id();
        let trigger = world.spawn(OwnedTrigger(owner)).id();
        let activator = world.spawn_empty().id();
        world
            .resource_mut::<TriggerQueue>()
            .push(trigger, activator);
        world.despawn(owner);
        assert!(world.get_entity(trigger).is_err());
        dispatch(&mut world);
        assert!(world.resource::<Delivered>().0.is_empty());
        let trigger = world.spawn_empty().id();
        for _ in 0..3 {
            world
                .resource_mut::<TriggerQueue>()
                .push_once(trigger, activator);
        }
        dispatch(&mut world);
        assert_eq!(world.resource::<Delivered>().0.len(), 1);
        assert!(world.get_entity(trigger).is_err());
    }

    #[test]
    fn explicit_script_delete_happens_after_immediate_notification_without_wait() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let overflow = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        for _ in 0..256 {
            world
                .resource_mut::<TriggerQueue>()
                .push(trigger, activator);
        }
        world
            .resource_mut::<TriggerQueue>()
            .push_and_delete(overflow, activator);
        let event = world
            .resource_mut::<TriggerQueue>()
            .immediate
            .pop()
            .unwrap();
        deliver(&mut world, event);
        world.flush();
        assert_eq!(world.resource::<Delivered>().0.len(), 1);
        assert!(world.get_entity(overflow).is_err());
    }

    #[test]
    fn native_wait_zero_deletes_even_when_the_queued_activator_is_gone() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        world
            .resource_mut::<TriggerQueue>()
            .push_once(trigger, activator);
        world.despawn(activator);
        assert!(world.get_entity(trigger).is_ok());
        dispatch(&mut world);
        assert!(world.resource::<Delivered>().0.is_empty());
        assert!(world.get_entity(trigger).is_err());
    }

    #[test]
    fn native_wait_zero_overflow_yields_until_the_dispatch_resume_point() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let overflow = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        for _ in 0..256 {
            world
                .resource_mut::<TriggerQueue>()
                .push(trigger, activator);
        }
        process_contact(
            &mut world,
            CollectedContact {
                pending: PendingTrigger {
                    trigger: overflow,
                    activator,
                    delete: false,
                },
                automatic: true,
                delete_at_resume: true,
            },
        );
        assert_eq!(world.resource::<Delivered>().0.len(), 1);
        assert!(world.get_entity(overflow).is_ok());
        dispatch(&mut world);
        assert!(world.get_entity(overflow).is_err());
    }

    #[test]
    fn immediate_overflow_invalidates_later_contacts_during_collection() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let overflow = world.spawn_empty().id();
        let later = world.spawn_empty().id();
        let activator = world.spawn_empty().id();
        world.add_observer(move |event: On<TriggerActivated>, mut commands: Commands| {
            if event.trigger == overflow {
                commands.entity(later).despawn();
            }
        });
        world.flush();
        for _ in 0..256 {
            world
                .resource_mut::<TriggerQueue>()
                .push(trigger, activator);
        }
        process_contact(
            &mut world,
            CollectedContact {
                pending: PendingTrigger {
                    trigger: overflow,
                    activator,
                    delete: false,
                },
                automatic: true,
                delete_at_resume: false,
            },
        );
        assert!(world.get_entity(later).is_err());
        process_contact(
            &mut world,
            CollectedContact {
                pending: PendingTrigger {
                    trigger: later,
                    activator,
                    delete: false,
                },
                automatic: true,
                delete_at_resume: false,
            },
        );
        dispatch(&mut world);
        let delivered = &world.resource::<Delivered>().0;
        assert_eq!(delivered.len(), 257);
        assert_eq!(delivered[0].trigger, overflow);
        assert!(delivered.iter().all(|event| event.trigger != later));
    }

    #[test]
    fn overflow_delivers_before_deferred_contacts_and_swap_removal() {
        let mut world = world();
        let trigger = world.spawn_empty().id();
        let activators: Vec<_> = (0..257).map(|_| world.spawn_empty().id()).collect();
        for &activator in &activators {
            world
                .resource_mut::<TriggerQueue>()
                .push(trigger, activator);
        }
        dispatch(&mut world);
        let delivered = &world.resource::<Delivered>().0;
        assert_eq!(delivered.len(), 257);
        assert_eq!(delivered[0].activator, activators[256]);
        assert_eq!(delivered[1].activator, activators[0]);
        assert_eq!(delivered[2].activator, activators[255]);
    }
}
