//! Tactical points of interest and original Bevy-owned grounded equipment animation.
use crate::{
    GameState,
    assets::GameDefinition,
    equipment::Grenade,
    gameplay::GameStep,
    life::{LifeState, Vitality},
    origins::content::ContentId,
    rounds::PopulationKind,
};
use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldInstanceReady};
use std::collections::{BTreeMap, HashMap};
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LureDefinition {
    pub range: f32,
    pub capacity: u32,
    pub spacing: f32,
    pub animation: ContentId,
}
impl LureDefinition {
    pub fn valid(&self) -> bool {
        self.range.is_finite()
            && self.range > 0.
            && self.spacing.is_finite()
            && self.spacing > 0.
            && self.spacing < self.range
            && self.capacity > 0
    }
}
#[derive(Clone, Copy)]
pub struct Attraction {
    pub bomb: Entity,
    pub position: Vec3,
}
#[derive(Resource, Default)]
pub struct Attractions(pub BTreeMap<Entity, Attraction>);
#[derive(Resource, Default)]
pub struct LureEvidence {
    pub activated: u32,
    pub acquired: u32,
    pub released: u32,
    pub detonation: u32,
    pub frames: u32,
    pub maximum_attached: usize,
    pub events: Vec<serde_json::Value>,
}
impl LureEvidence {
    pub fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event)
        }
    }
}
#[derive(Component)]
pub struct LureRig {
    pub owner: Entity,
    pub clip: Handle<AnimationClip>,
    ready: bool,
    players: Vec<Entity>,
    started: bool,
}
impl LureRig {
    pub fn new(owner: Entity, clip: Handle<AnimationClip>) -> Self {
        Self {
            owner,
            clip,
            ready: false,
            players: Vec::new(),
            started: false,
        }
    }
}
#[derive(Resource, Default)]
struct EquipmentMaterials(HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>);
#[derive(Resource, Default)]
struct Graphs(HashMap<AssetId<AnimationClip>, (Handle<AnimationGraph>, AnimationNodeIndex)>);
pub fn install(app: &mut App) {
    app.init_resource::<Attractions>()
        .init_resource::<LureEvidence>()
        .init_resource::<Graphs>()
        .init_resource::<EquipmentMaterials>()
        .add_observer(ready)
        .add_systems(
            FixedUpdate,
            assign
                .before(crate::enemies::lifecycle)
                .in_set(GameStep::EnemyIntent)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, animate.before(bevy::app::AnimationSystems));
}
fn assign(
    definition: Res<GameDefinition>,
    bombs: Query<(Entity, &Grenade, &Transform)>,
    enemies: Query<(
        Entity,
        &crate::enemies::Zombie,
        &Transform,
        &Vitality,
        &PopulationKind,
    )>,
    query: SpatialQuery,
    players: Query<(), With<crate::player::Player>>,
    mut attractions: ResMut<Attractions>,
    mut evidence: ResMut<LureEvidence>,
) {
    let mut active = Vec::new();
    for (entity, grenade, pose) in &bombs {
        if !grenade.lure_active {
            continue;
        }
        let Some(profile) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == grenade.definition)
            .and_then(|w| w.lure.as_ref())
        else {
            continue;
        };
        let mut slots = Vec::new();
        for dir in [Vec3::X, Vec3::Z, Vec3::NEG_X, Vec3::NEG_Z] {
            let probe = pose.translation + dir * profile.spacing;
            let hit = query.cast_ray_predicate(
                probe + Vec3::Y,
                Dir3::NEG_Y,
                2.,
                false,
                &crate::environment::solids([]),
                &|e| enemies.get(e).is_err() && !players.contains(e),
            );
            if let Some(hit) = hit
                && hit.normal.y > 0.65
            {
                slots.push(probe + Vec3::Y * (1. - hit.distance + 0.85));
            }
        }
        if slots.is_empty() {
            slots.push(pose.translation + Vec3::Y * 0.8);
        }
        active.push((entity, pose.translation, profile, slots, 0_u32));
    }
    active.sort_by_key(|b| b.0.to_bits());
    let mut next = BTreeMap::new();
    let mut candidates: Vec<_> = enemies
        .iter()
        .filter(|(_, _, _, v, k)| {
            v.state() == LifeState::ActorAlive && **k != PopulationKind::Special
        })
        .collect();
    candidates.sort_by_key(|(entity, ..)| entity.to_bits());
    for (entity, _, pose, _, _) in candidates {
        let selected = active
            .iter()
            .enumerate()
            .filter(|(_, b)| b.4 < b.2.capacity && pose.translation.distance(b.1) <= b.2.range)
            .min_by(|(_, a), (_, b)| {
                pose.translation
                    .distance_squared(a.1)
                    .total_cmp(&pose.translation.distance_squared(b.1))
            })
            .map(|(index, _)| index);
        let Some(index) = selected else { continue };
        let bomb = &mut active[index];
        let position = bomb.3[bomb.4 as usize % bomb.3.len()];
        bomb.4 += 1;
        next.insert(
            entity,
            Attraction {
                bomb: bomb.0,
                position,
            },
        );
        if attractions
            .0
            .get(&entity)
            .is_none_or(|old| old.bomb != bomb.0)
        {
            evidence.acquired += 1;
            evidence.record(serde_json::json!({"event":"acquire","actor":entity.to_bits(),"bomb":bomb.0.to_bits(),"position":position.to_array(),"distance":pose.translation.distance(bomb.1)}));
        }
    }
    for (entity, old) in &attractions.0 {
        if !next.contains_key(entity) {
            evidence.released += 1;
            evidence.record(serde_json::json!({"event":"release","actor":entity.to_bits(),"bomb":old.bomb.to_bits()}));
        }
    }
    evidence.maximum_attached = evidence.maximum_attached.max(next.len());
    attractions.0 = next;
}
fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut cache: ResMut<EquipmentMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
    mut rigs: Query<&mut LureRig>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
) {
    let Ok(mut rig) = rigs.get_mut(event.entity) else {
        return;
    };
    rig.ready = true;
    for entity in children.iter_descendants(event.entity) {
        let Ok(material) = meshes.get(entity) else {
            continue;
        };
        let handle = if let Some(handle) = cache.0.get(&material.0.id()) {
            handle.clone()
        } else {
            let Some(source) = materials.get(&material.0) else {
                continue;
            };
            let source = crate::rendering::with_actor_fill(source);
            let handle = materials.add(source);
            cache.0.insert(material.0.id(), handle.clone());
            handle
        };
        commands.entity(entity).insert(MeshMaterial3d(handle));
    }
    rig.players = children
        .iter_descendants(event.entity)
        .filter(|e| players.contains(*e))
        .collect();
}
fn animate(
    mut commands: Commands,
    state: Res<State<GameState>>,
    mut rigs: Query<&mut LureRig>,
    grenades: Query<&Grenade>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<Graphs>,
    mut players: Query<&mut AnimationPlayer>,
    mut evidence: ResMut<LureEvidence>,
) {
    for mut rig in &mut rigs {
        if !rig.ready {
            continue;
        }
        let Ok(grenade) = grenades.get(rig.owner) else {
            continue;
        };
        if !grenade.lure_active {
            continue;
        }
        if !cache.0.contains_key(&rig.clip.id()) {
            if !clips.contains(rig.clip.id()) {
                continue;
            }
            let (graph, node) = AnimationGraph::from_clip(rig.clip.clone());
            cache.0.insert(rig.clip.id(), (graphs.add(graph), node));
        }
        let (graph, node) = &cache.0[&rig.clip.id()];
        for entity in &rig.players {
            let Ok(mut player) = players.get_mut(*entity) else {
                continue;
            };
            if !rig.started {
                commands
                    .entity(*entity)
                    .insert(AnimationGraphHandle(graph.clone()));
                player.play(*node).repeat();
            }
            if *state == GameState::Playing {
                player.resume_all();
                evidence.frames += 1;
            } else {
                player.pause_all();
            }
        }
        rig.started = true;
    }
}
