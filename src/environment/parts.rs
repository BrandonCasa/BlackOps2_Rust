use crate::origins::{ContentCatalog, ContentId};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartDefinition {
    pub id: ContentId,
    pub family: String,
    pub presentation: ContentId,
    pub dig_zone: Option<String>,
    pub initial_misses: u32,
}

pub(super) fn valid(parts: &[PartDefinition], content: &ContentCatalog) -> bool {
    let mut ids = BTreeSet::new();
    parts.len() <= 128
        && parts.iter().all(|p| {
            ids.insert(&p.id)
                && content.definitions.contains(&p.id)
                && !p.family.is_empty()
                && p.family.len() < 64
                && p.initial_misses <= 4
                && p.dig_zone
                    .as_ref()
                    .is_none_or(|z| matches!(z.as_str(), "bunker" | "nml" | "village"))
                && content
                    .presentations
                    .iter()
                    .any(|s| s.id == p.presentation && s.scene.is_some())
        })
        && parts
            .iter()
            .filter_map(|p| p.dig_zone.as_ref())
            .collect::<BTreeSet<_>>()
            .len()
            == parts.iter().filter(|p| p.dig_zone.is_some()).count()
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PartState {
    Available { entity: u64 },
    Owned { player: u64 },
}

#[derive(Resource, Default, Serialize)]
pub struct Parts {
    pub pieces: BTreeMap<String, PartState>,
}
impl Parts {
    pub fn reveal(&mut self, id: &str, entity: Entity) -> bool {
        if self.pieces.contains_key(id) {
            return false;
        }
        self.pieces.insert(
            id.into(),
            PartState::Available {
                entity: entity.to_bits(),
            },
        );
        true
    }
    pub fn take(&mut self, id: &str, entity: Entity, owner: Entity) -> bool {
        if !matches!(self.pieces.get(id), Some(PartState::Available { entity: e }) if *e == entity.to_bits())
        {
            return false;
        }
        self.pieces.insert(
            id.into(),
            PartState::Owned {
                player: owner.to_bits(),
            },
        );
        true
    }
}
