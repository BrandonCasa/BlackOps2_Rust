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
use std::collections::BTreeMap;
#[derive(Clone, serde::Deserialize)]
pub struct LureDefinition {
    pub range: f32,
    pub capacity: u32,
    pub spacing: f32,
    pub animation: ContentId,
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
#[require(crate::animation::SceneClips)]
pub struct LureRig {
    pub owner: Entity,
    pub clip: Handle<AnimationClip>,
    ready: bool,
}
impl LureRig {
    pub fn new(owner: Entity, clip: Handle<AnimationClip>) -> Self {
        Self {
            owner,
            clip,
            ready: false,
        }
    }
}
pub fn install(app: &mut App) {
    app.init_resource::<Attractions>()
        .init_resource::<LureEvidence>()
        .add_observer(ready)
        .add_systems(
            FixedUpdate,
            assign
                .before(crate::enemies::lifecycle)
                .in_set(GameStep::EnemyIntent)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, animate);
}
fn assign(
    definition: Res<GameDefinition>,
    bombs: Query<(
        Entity,
        &Grenade,
        &Transform,
        Has<crate::tablets::BeaconAccepted>,
    )>,
    enemies: Query<(Entity, &Transform, &Vitality, &PopulationKind), With<crate::enemies::Zombie>>,
    query: SpatialQuery,
    players: Query<(), With<crate::player::Player>>,
    mut attractions: ResMut<Attractions>,
    mut evidence: ResMut<LureEvidence>,
) {
    let beacon_profile = definition
        .content
        .weapons
        .iter()
        .find_map(|w| w.lure.clone())
        .map(|mut profile| {
            profile.range = 39.0144;
            profile.capacity = 96;
            profile.spacing = 1.143;
            profile
        });
    let mut active = Vec::new();
    for (entity, grenade, pose, beacon_accepted) in &bombs {
        if grenade.definition.0 == "weapon.beacon_zm" && !beacon_accepted {
            continue;
        }
        if !grenade.lure_active {
            continue;
        }
        let Some(profile) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == grenade.definition)
            .and_then(|w| w.lure.as_ref())
            .or_else(|| {
                (grenade.definition.0 == "weapon.beacon_zm")
                    .then_some(beacon_profile.as_ref())
                    .flatten()
            })
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
        .filter(|(_, _, v, k)| v.state() == LifeState::ActorAlive && **k != PopulationKind::Special)
        .collect();
    candidates.sort_by_key(|(entity, ..)| entity.to_bits());
    for (entity, pose, _, _) in candidates {
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
fn ready(event: On<WorldInstanceReady>, mut rigs: Query<&mut LureRig>) {
    let Ok(mut rig) = rigs.get_mut(event.entity) else {
        return;
    };
    rig.ready = true;
}
fn animate(
    state: Res<State<GameState>>,
    mut rigs: Query<(
        &LureRig,
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
    grenades: Query<&Grenade>,
    mut assets: crate::animation::ClipAssets,
) {
    for (rig, mut clips, mut player, mut graph) in &mut rigs {
        if !rig.ready || !clips.ready {
            continue;
        }
        let Ok(grenade) = grenades.get(rig.owner) else {
            continue;
        };
        if !grenade.lure_active {
            continue;
        }
        let path = rig.clip.path().expect("Source lure clip path").to_string();
        let Some((node, _)) = clips.node(&path, &mut graph, &mut assets) else {
            continue;
        };
        let active = player.play(node);
        active.repeat();
        if *state == GameState::Playing {
            active.resume();
        } else {
            active.pause();
        }
    }
}
