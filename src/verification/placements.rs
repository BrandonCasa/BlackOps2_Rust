use crate::{
    assets::GameDefinition,
    interactions::Usable,
    origins::{InteractionId, PlacementId},
};
use bevy::prelude::*;
use std::collections::BTreeMap;

pub(super) struct Binding {
    placement: PlacementId,
    entity: Entity,
}
pub(super) type Snapshot = BTreeMap<InteractionId, Binding>;
pub(super) type Sites<'w, 's> =
    Query<'w, 's, (Entity, &'static Usable, Option<&'static PlacementId>)>;

pub(super) fn snapshot(game: &GameDefinition, sites: &Sites) -> Result<Snapshot, String> {
    let mut snapshot = Snapshot::new();
    for (entity, usable, placement) in sites {
        let Some(placement) = placement else {
            return Err(format!(
                "Missing runtime placement ID for {}",
                usable.source.id.0
            ));
        };
        if *placement != usable.source.placement
            || !game.content.placements.iter().any(|p| p.id == *placement)
            || !game
                .content
                .interactions
                .iter()
                .any(|i| i.id == usable.source.id && i.placement == *placement)
        {
            return Err(format!(
                "Wrong runtime placement binding for {}",
                usable.source.id.0
            ));
        }
        if snapshot
            .insert(
                usable.source.id.clone(),
                Binding {
                    placement: placement.clone(),
                    entity,
                },
            )
            .is_some()
        {
            return Err(format!(
                "Duplicate runtime interaction {}",
                usable.source.id.0
            ));
        }
    }
    if snapshot.len() != game.content.interactions.len() {
        return Err("Incomplete runtime placement bindings".into());
    }
    Ok(snapshot)
}

pub(super) fn restart_matches(before: &Snapshot, after: &Snapshot) -> bool {
    !before.is_empty()
        && before.len() == after.len()
        && before.iter().all(|(id, old)| {
            after
                .get(id)
                .is_some_and(|new| old.placement == new.placement && old.entity != new.entity)
        })
}

pub(super) fn receipt(snapshot: &Snapshot) -> serde_json::Value {
    serde_json::json!(
        snapshot
            .iter()
            .map(|(interaction, binding)| {
                serde_json::json!({"interaction": interaction.0, "placement": binding.placement.0,
            "entity": binding.entity.to_bits()})
            })
            .collect::<Vec<_>>()
    )
}
