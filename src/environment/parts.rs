use crate::origins::{ContentId};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap};

#[derive(Clone, Deserialize)]
pub struct PartDefinition {
    pub id: ContentId,
    pub family: String,
    pub presentation: ContentId,
    pub dig_zone: Option<String>,
    pub initial_misses: u32,
}



#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PartState {
    Consumed,
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
