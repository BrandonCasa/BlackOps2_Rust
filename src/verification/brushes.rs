//! Observe converted brush scene roots and their Avian geometry in the actual application.
use crate::{
    assets::GameDefinition,
    environment::brushes::{BRUSH_LAYER, BrushSites},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn observe(
    game: &GameDefinition,
    sites: &BrushSites,
    query: &SpatialQuery,
) -> serde_json::Value {
    let mut ids = BTreeSet::new();
    let mut errors = Vec::new();
    let mut classes = BTreeMap::<String, usize>::new();
    let (mut interiors, mut outside, mut visuals, mut collider_roots) = (0, 0, 0, 0);
    let filter = SpatialQueryFilter::from_mask(BRUSH_LAYER);
    let probe = Collider::sphere(0.005);
    for (entity, root, global, collider) in sites {
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
        let pose = source.transform();
        let actual = global.compute_transform();
        if actual.translation.distance_squared(pose.translation) > 0.000001
            || actual.rotation.angle_between(pose.rotation) > 0.0001
            || actual.scale != Vec3::ONE
        {
            errors.push(format!("Wrong authored brush transform {}", root.id));
        }
        visuals += root.visual_meshes;
        if root.visual_meshes != source.visual_meshes
            || collider.is_some() != !source.volumes.is_empty()
        {
            errors.push(format!("Wrong brush mesh/hull closure {}", root.id));
        }
        if collider.is_some() {
            collider_roots += 1;
        }
        for volume in &source.volumes {
            interiors += 1;
            let point = pose.transform_point(Vec3::from_array(volume.position));
            if !query
                .shape_intersections(&probe, point, Quat::IDENTITY, &filter)
                .contains(&entity)
            {
                errors.push(format!("Missing brush hull {}", volume.id));
            }
        }
        if !source.volumes.is_empty() {
            outside += 1;
            let max = source
                .volumes
                .iter()
                .flat_map(|v| {
                    v.vertices
                        .iter()
                        .map(|p| Vec3::from_array(v.position) + Vec3::from_array(*p))
                })
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
    serde_json::json!({"passed":errors.is_empty(),"authored_entities":game.environment.brush_entities.len(),"runtime_entities":ids.len(),
        "classes":classes,"visual_meshes":visuals,"collider_roots":collider_roots,"interior_probes":interiors,"outside_probes":outside,"errors":errors})
}
