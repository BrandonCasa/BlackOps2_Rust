use crate::{
    assets::GameDefinition,
    environment::brushes::{BRUSH_LAYER, MapBrush},
    player::Player,
    staff_puzzles::{IceTile, StaffPuzzles},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Sites<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MapBrush,
        &'static GlobalTransform,
        Option<&'static Collider>,
        Has<ColliderDisabled>,
        &'static Visibility,
        Option<&'static IceTile>,
    ),
    (Without<Player>, Without<crate::hud::PlayerOverlay>),
>;

pub(super) fn observe(
    game: &GameDefinition,
    sites: &Sites,
    query: &SpatialQuery,
    puzzles: &StaffPuzzles,
) -> serde_json::Value {
    let mut ids = BTreeSet::new();
    let mut errors = Vec::new();
    let mut classes = BTreeMap::<String, usize>::new();
    let (mut interiors, mut outside, mut visuals, mut collider_roots) = (0, 0, 0, 0);
    let filter = SpatialQueryFilter::from_mask(BRUSH_LAYER);
    let probe = Collider::sphere(0.005);
    let ice_slots: Vec<_> = game
        .environment
        .brush_entities
        .iter()
        .filter(|b| {
            b.source
                .get("targetname")
                .is_some_and(|name| name == "ice_tile_original")
        })
        .map(|b| Vec3::from_array(b.position))
        .collect();
    let mut occupied_slots = BTreeSet::new();
    let mut active_tiles = BTreeSet::new();
    let mut disabled_tiles = 0;
    for (entity, root, global, collider, disabled, visibility, ice_tile) in sites {
        if !ids.insert(&root.id) {
            errors.push(format!("Duplicate brush root {}", root.id));
        }
        let Some(source) = game
            .environment
            .brush_entities
            .iter()
            .find(|b| b.id == root.id)
        else {
            errors.push(format!("Unknown brush root {}", root.id));
            continue;
        };
        *classes
            .entry(source.source["classname"].clone())
            .or_default() += 1;
        let mut pose = source.transform();
        let actual = global.compute_transform();
        let is_ice_tile = source
            .source
            .get("script_noteworthy")
            .is_some_and(|name| name == "ice_ceiling_tile");
        if is_ice_tile {
            let value = source
                .source
                .get("script_string")
                .and_then(|s| s.parse::<u8>().ok());
            let selected = value.is_some_and(|value| puzzles.ice_tiles.contains(&value));
            if value.is_none()
                || ice_tile.map(|tile| tile.0) != value.filter(|_| selected)
                || disabled == selected
                || *visibility
                    != if selected {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    }
            {
                errors.push(format!("Wrong Ice Staff tile activation {}", root.id));
            }
            if selected {
                if let Some(slot) = ice_slots
                    .iter()
                    .position(|position| actual.translation.distance_squared(*position) <= 0.000001)
                {
                    pose.translation = ice_slots[slot];
                    if !occupied_slots.insert(slot) {
                        errors.push(format!("Duplicate Ice Staff tile slot {}", root.id));
                    }
                } else {
                    errors.push(format!("Ice Staff tile outside authored slots {}", root.id));
                }
                if let Some(value) = value {
                    if !active_tiles.insert(value) {
                        errors.push(format!("Duplicate Ice Staff tile value {}", root.id));
                    }
                }
            } else {
                disabled_tiles += 1;
            }
        } else if ice_tile.is_some() {
            errors.push(format!(
                "Ice Staff tile bound to ordinary brush {}",
                root.id
            ));
        }
        if actual.translation.distance_squared(pose.translation) > 0.000001
            || actual.rotation.angle_between(pose.rotation) > 0.0001
            || actual.scale != Vec3::ONE
        {
            errors.push(format!("Wrong authored brush transform {}", root.id));
        }
        visuals += root.visual_meshes;
        if root.visual_meshes != source.visual_meshes
            || collider.is_some()
                != (!source.volumes.is_empty() || !source.triangle_shapes.is_empty())
        {
            errors.push(format!("Wrong brush mesh/hull closure {}", root.id));
        }
        if collider.is_some() {
            collider_roots += 1;
        }
        for volume in &source.volumes {
            interiors += 1;
            let center = volume
                .vertices
                .iter()
                .copied()
                .map(Vec3::from_array)
                .sum::<Vec3>()
                / volume.vertices.len() as f32;
            let point = pose.transform_point(Vec3::from_array(volume.position) + center);
            let touching = query
                .shape_intersections(&probe, point, Quat::IDENTITY, &filter)
                .contains(&entity);
            if is_ice_tile && disabled {
                if touching {
                    errors.push(format!(
                        "Disabled Ice Staff tile still collides {}",
                        volume.id
                    ));
                }
            } else if !touching {
                errors.push(format!("Missing brush hull {}", volume.id));
            }
        }
        for shape in &source.triangle_shapes {
            for indices in &shape.indices {
                interiors += 1;
                let center = indices
                    .iter()
                    .map(|&index| Vec3::from_array(shape.vertices[index as usize]))
                    .sum::<Vec3>()
                    / 3.;
                let touching = query
                    .shape_intersections(
                        &probe,
                        pose.transform_point(center),
                        Quat::IDENTITY,
                        &filter,
                    )
                    .contains(&entity);
                if touching == (is_ice_tile && disabled) {
                    errors.push(format!("Wrong brush triangle touching {}", shape.id));
                }
            }
        }
        if collider.is_some() {
            outside += 1;
            let max = source
                .volumes
                .iter()
                .flat_map(|v| {
                    v.vertices
                        .iter()
                        .map(|p| Vec3::from_array(v.position) + Vec3::from_array(*p))
                })
                .chain(
                    source
                        .triangle_shapes
                        .iter()
                        .flat_map(|shape| shape.vertices.iter().copied().map(Vec3::from_array)),
                )
                .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
            if query
                .shape_intersections(
                    &probe,
                    pose.transform_point(max + Vec3::splat(0.1)),
                    Quat::IDENTITY,
                    &filter,
                )
                .contains(&entity)
            {
                errors.push(format!("Brush admits outside point {}", root.id));
            }
        }
    }
    if ids.len() != game.environment.brush_entities.len() {
        errors.push("Incomplete brush scene closure".into());
    }
    if active_tiles != puzzles.ice_tiles || occupied_slots.len() != ice_slots.len() {
        errors.push("Incomplete Ice Staff tile selection or authored slot coverage".into());
    }
    serde_json::json!({"passed":errors.is_empty(),"authored_entities":game.environment.brush_entities.len(),"runtime_entities":ids.len(),
        "classes":classes,"visual_meshes":visuals,"collider_roots":collider_roots,"interior_probes":interiors,"outside_probes":outside,
        "ice_tiles":{"selected":active_tiles,"occupied_slots":occupied_slots,"disabled":disabled_tiles},"errors":errors})
}
