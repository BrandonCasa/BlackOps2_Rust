use crate::{
    assets::GameDefinition,
    environment::triggers::{TRIGGER_LAYER, TriggerSites},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn observe(
    game: &GameDefinition,
    sites: &TriggerSites,
    query: &SpatialQuery,
) -> serde_json::Value {
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    let mut classes = BTreeMap::<String, usize>::new();
    let mut interior_probes = 0;
    let mut outside_probes = 0;
    let filter = SpatialQueryFilter::from_mask(TRIGGER_LAYER);
    let probe = Collider::sphere(0.005);
    for (entity, site, pose) in sites {
        if !ids.insert(&site.0) {
            errors.push(format!("Duplicate runtime trigger {}", site.0));
        }
        let Some(source) = game.environment.triggers.iter().find(|t| t.id == site.0) else {
            errors.push(format!("Unknown runtime trigger {}", site.0));
            continue;
        };
        *classes
            .entry(source.source["classname"].clone())
            .or_default() += 1;
        if pose
            .translation
            .distance_squared(Vec3::from_array(source.position))
            > 0.000001
        {
            errors.push(format!("Wrong trigger origin {}", site.0));
        }
        for point in source.interior_points() {
            interior_probes += 1;
            if !query
                .shape_intersections(&probe, point, Quat::IDENTITY, &filter)
                .contains(&entity)
            {
                errors.push(format!("Missing trigger brush touching {}", site.0));
            }
        }
        outside_probes += 1;
        if query
            .shape_intersections(&probe, source.outside_point(), Quat::IDENTITY, &filter)
            .contains(&entity)
        {
            errors.push(format!("Trigger admits outside point {}", site.0));
        }
    }
    if ids.len() != game.environment.triggers.len() {
        errors.push("Incomplete runtime trigger closure".into());
    }
    serde_json::json!({"passed":errors.is_empty(),"authored_triggers":game.environment.triggers.len(),
        "runtime_triggers":ids.len(),"classes":classes,"interior_probes":interior_probes,
        "outside_probes":outside_probes,"errors":errors})
}
