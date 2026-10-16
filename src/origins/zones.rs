//! Source-owned Origins zone graph and ordinary externally exported trigger hulls.
use super::{GameDefinition, ZoneConnection, ZoneDefinition, ZoneId};
use crate::{
    access::{TriggerActivationDefinition, ZoneFlagLinkDefinition, ZoneVolumeDefinition},
    environment::{
        VolumeDefinition,
        support::{EnvironmentOwner, OwnershipDefinition},
        triggers::{TriggerDefinition, TriggerShape, TriggerState},
    },
};
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct AuthoredZones {
    zones: Vec<ZoneDefinition>,
    source_zones: BTreeMap<String, ZoneId>,
    zone_connections: Vec<ZoneConnection>,
    flag_links: Vec<ZoneFlagLinkDefinition>,
    trigger_activation_sources: Vec<ActivationSource>,
}
#[derive(Deserialize)]
struct ActivationSource {
    source_targetname: String,
    source_entity_id: u32,
    flags: Vec<String>,
}
#[derive(Deserialize)]
struct Metadata {
    entities: Vec<EntityRecord>,
}
#[derive(Deserialize)]
struct EntityRecord {
    id: u32,
    #[serde(flatten)]
    fields: BTreeMap<String, serde_json::Value>,
}
impl EntityRecord {
    fn text(&self, key: &str) -> Option<&str> {
        self.fields.get(key)?.as_str()
    }
    fn source(&self) -> BTreeMap<String, String> {
        self.fields
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_owned())))
            .collect()
    }
    fn vector(&self, key: &str) -> Result<Vec3, String> {
        let text = self
            .text(key)
            .ok_or_else(|| format!("Source entity {} lacks {key}", self.id))?;
        let values = text
            .split_whitespace()
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Source entity {} invalid {key}: {error}", self.id))?;
        let [x, y, z]: [f32; 3] = values
            .try_into()
            .map_err(|_| format!("Source entity {} {key} is not a vector", self.id))?;
        let value = Vec3::new(x, y, z);
        if !value.is_finite() {
            return Err(format!("Source entity {} {key} is nonfinite", self.id));
        }
        Ok(value)
    }
    fn pose(&self) -> Result<Transform, String> {
        let source = self.vector("origin")?;
        let translation = Vec3::new(source.x, source.z, -source.y) / 39.37;
        let angles = if self.text("angles").is_some() {
            self.vector("angles")?
        } else {
            Vec3::ZERO
        };
        let frame = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let source_rotation = Quat::from_euler(
            EulerRot::ZYX,
            angles.y.to_radians(),
            angles.x.to_radians(),
            angles.z.to_radians(),
        );
        Ok(Transform::from_translation(translation)
            .with_rotation(frame * source_rotation * frame.inverse()))
    }
}
#[derive(Deserialize)]
struct ClipMap {
    cmodels: Vec<BrushModel>,
}
#[derive(Deserialize)]
struct BrushModel {
    brushes: Vec<usize>,
    partitions: Vec<usize>,
}
#[derive(Deserialize)]
struct Hull {
    contents: u32,
    vertices: Vec<[f32; 3]>,
}

pub(crate) fn bind(
    game: &mut GameDefinition,
    metadata: &[u8],
    clip: &[u8],
    hulls: &[u8],
) -> Result<(), String> {
    let authored: AuthoredZones =
        serde_json::from_str(include_str!("../../gameplay/origins/zones.json"))
            .map_err(|error| format!("Invalid authored Origins zones: {error}"))?;
    let metadata: Metadata = serde_json::from_slice(metadata)
        .map_err(|e| format!("Invalid external entity metadata: {e}"))?;
    let spawn_entities: BTreeMap<_, _> = metadata
        .entities
        .iter()
        .filter(|entity| {
            matches!(
                entity.text("script_noteworthy"),
                Some("spawn_location" | "riser_location")
            )
        })
        .filter_map(|entity| {
            Some((
                format!("spawn.{}", entity.text("guid")?.to_ascii_lowercase()),
                entity,
            ))
        })
        .collect();
    for spawn in &mut game.access.spawns {
        let entity = spawn_entities
            .get(&spawn.id)
            .ok_or_else(|| format!("Spawn {} has no source entity", spawn.id))?;
        spawn.find_flesh = entity.text("script_string") == Some("find_flesh");
        spawn.yaw = if entity.text("angles").is_some() {
            entity.vector("angles")?.y.to_radians()
        } else {
            0.
        };
    }
    let clip: ClipMap =
        serde_json::from_slice(clip).map_err(|e| format!("Invalid external clipmap: {e}"))?;
    let hulls: BTreeMap<String, Hull> =
        serde_json::from_slice(hulls).map_err(|e| format!("Invalid external brush hulls: {e}"))?;
    let mut triggers = Vec::new();
    let mut ownership = Vec::new();
    let mut volumes = Vec::new();
    let mut activations = Vec::new();
    for entity in &metadata.entities {
        let zone = entity
            .text("targetname")
            .and_then(|name| authored.source_zones.get(name))
            .and_then(|id| authored.zones.iter().find(|zone| zone.id == *id));
        let zone = zone.filter(|_| {
            entity.text("classname") == Some("info_volume")
                && entity.text("script_noteworthy") == Some("player_volume")
        });
        let activation = authored
            .trigger_activation_sources
            .iter()
            .find(|source| source.source_entity_id == entity.id);
        if zone.is_none() && activation.is_none() {
            continue;
        }
        if let Some(source) = activation {
            if entity.text("targetname") != Some(source.source_targetname.as_str()) {
                return Err(format!(
                    "Tank trigger source {} changed identity",
                    entity.id
                ));
            }
        }
        let id = format!("trigger.native.{}", entity.id);
        let pose = entity.pose()?;
        let mut pieces = Vec::new();
        if let Some(model) = entity.text("model") {
            let index = model
                .strip_prefix('*')
                .ok_or_else(|| format!("Trigger {id} has invalid brush model {model}"))?
                .parse::<usize>()
                .map_err(|_| format!("Trigger {id} has invalid brush model {model}"))?;
            let model = clip
                .cmodels
                .get(index)
                .ok_or_else(|| format!("Trigger {id} brush model {index} is missing"))?;
            if !model.partitions.is_empty() {
                return Err(format!(
                    "Trigger {id} terrain partitions require external triangle export"
                ));
            }
            for index in &model.brushes {
                let hull = hulls.get(&index.to_string()).ok_or_else(|| {
                    format!("Trigger {id} external brush hull {index} is missing")
                })?;
                let mut vertices = Vec::new();
                for vertex in &hull.vertices {
                    if !vertices.contains(vertex) {
                        vertices.push(*vertex);
                    }
                }
                if !(4..=128).contains(&vertices.len())
                    || vertices.iter().flatten().any(|x| !x.is_finite())
                {
                    return Err(format!(
                        "Trigger {id} external hull {index} has invalid vertices"
                    ));
                }
                pieces.push(VolumeDefinition {
                    id: format!("{id}.brush.{index}"),
                    position: [0.; 3],
                    vertices,
                    contents: hull.contents,
                    surface_flags: Vec::new(),
                });
            }
        } else if entity.text("classname") == Some("trigger_box") {
            let scalar = |key: &str| -> Result<f32, String> {
                let value = entity
                    .text(key)
                    .ok_or_else(|| format!("Trigger {id} lacks {key}"))?
                    .parse::<f32>()
                    .map_err(|_| format!("Trigger {id} invalid {key}"))?;
                if !value.is_finite() || value <= 0. {
                    return Err(format!("Trigger {id} invalid {key}"));
                }
                Ok(value / 39.37)
            };
            let half = Vec3::new(scalar("width")?, scalar("height")?, scalar("length")?) * 0.5;
            let vertices = (0..8)
                .map(|corner| {
                    [
                        if corner & 1 == 0 { -half.x } else { half.x },
                        if corner & 2 == 0 { -half.y } else { half.y },
                        if corner & 4 == 0 { -half.z } else { half.z },
                    ]
                })
                .collect();
            pieces.push(VolumeDefinition {
                id: format!("{id}.box"),
                position: [0.; 3],
                vertices,
                contents: 0,
                surface_flags: Vec::new(),
            });
        } else {
            return Err(format!("Trigger {id} has no exported volume"));
        }
        if pieces.is_empty() {
            return Err(format!("Trigger {id} has no convex source brushes"));
        }
        let trigger = TriggerDefinition {
            id: id.clone(),
            position: pose.translation.to_array(),
            target_position: None,
            source: entity.source(),
            shape: TriggerShape::Brush {
                rotation: pose.rotation.to_array(),
                volumes: pieces.clone(),
                triangle_shapes: Vec::new(),
            },
        };
        if trigger.collider().is_none() {
            return Err(format!("Trigger {id} source hull cannot form a collider"));
        }
        if let Some(zone) = zone {
            volumes.extend(pieces.into_iter().map(|mut volume| {
                volume.vertices = volume
                    .vertices
                    .into_iter()
                    .map(|vertex| (pose.rotation * Vec3::from_array(vertex)).to_array())
                    .collect();
                volume.position = pose.translation.to_array();
                ZoneVolumeDefinition {
                    zone: zone.id.clone(),
                    volume,
                }
            }));
        }
        // Player volumes belong to access::ZoneSensor; retain map triggers only
        // for the separate native activation callbacks.
        if activation.is_none() {
            continue;
        }
        if let Some(source) = activation {
            activations.push(TriggerActivationDefinition {
                trigger: id.clone(),
                flags: source.flags.clone(),
                contents: TriggerState::from_definition(&trigger).contents,
            });
        }
        ownership.push(OwnershipDefinition {
            id,
            owner: EnvironmentOwner::Access,
            shootable: false,
        });
        triggers.push(trigger);
    }
    for zone in &authored.zones {
        if !volumes.iter().any(|volume| volume.zone == zone.id) {
            return Err(format!(
                "Origins gameplay zone {} has no external player volume",
                zone.id.0
            ));
        }
    }
    for source in &authored.trigger_activation_sources {
        if !activations.iter().any(|activation| {
            activation.trigger == format!("trigger.native.{}", source.source_entity_id)
        }) {
            return Err(format!(
                "Origins tank source trigger {} is missing",
                source.source_entity_id
            ));
        }
    }
    for trigger in &triggers {
        if game
            .environment
            .triggers
            .iter()
            .any(|existing| existing.id == trigger.id)
        {
            return Err(format!("Duplicate source zone trigger {}", trigger.id));
        }
    }
    // Preserve the recorded non-spatial owner for native nodes outside player volumes.
    let unassigned = game
        .content
        .zones
        .iter()
        .find(|zone| zone.id.0 == "zone.unassigned")
        .cloned();
    game.content.zones = authored.zones;
    if let Some(zone) = unassigned {
        game.content.zones.push(zone);
    }
    game.content.zone_connections = authored.zone_connections;
    game.access.flag_links = authored.flag_links;
    game.environment.triggers.extend(triggers);
    game.environment.ownership.extend(ownership);
    game.access.volumes.extend(volumes);
    game.access.trigger_activations.extend(activations);
    #[derive(Deserialize)]
    struct SourceSpawns {
        spawners: Vec<crate::access::ActorSpawnerDefinition>,
        entrances: Vec<crate::access::SpawnEntranceDefinition>,
    }
    let source: SourceSpawns =
        serde_json::from_str(include_str!("../../gameplay/origins/access_spawns.json"))
            .map_err(|error| format!("Invalid source-authored spawn bindings: {error}"))?;
    game.access.spawners = source.spawners;
    game.access.entrances = source.entrances;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_tunnel_spawns_pursue_players_and_keep_source_facing() {
        let game = GameDefinition::load_from_root(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets"),
        );
        let tunnel = game
            .access
            .spawns
            .iter()
            .find(|spawn| spawn.id == "spawn.c0340470")
            .unwrap();
        assert!(tunnel.find_flesh);
        assert!(!tunnel.rising);
        assert!((tunnel.yaw - 300_f32.to_radians()).abs() < 0.0001);
        assert_eq!(
            game.access
                .spawns
                .iter()
                .filter(|spawn| spawn.find_flesh)
                .count(),
            305
        );
        assert_eq!(game.access.spawns.len(), 338);
    }

    #[test]
    fn source_zone_names_bind_to_existing_game_ids_without_losing_graph() {
        let data: AuthoredZones =
            serde_json::from_str(include_str!("../../gameplay/origins/zones.json")).unwrap();
        assert_eq!(data.zones.len(), 98);
        assert_eq!(data.zone_connections.len(), 160);
        assert_eq!(data.source_zones["zone_start"].0, "zone.start");
        assert_eq!(data.source_zones["zone_bunker_4b"].0, "zone.bunker_4b");
        assert_eq!(data.source_zones["zone_robot_head"].0, "zone.robot_head");
        assert_eq!(data.source_zones["ug_bottom_zone"].0, "zone.ug_bottom_zone");
        for (source, id) in &data.source_zones {
            assert_eq!(
                id.0,
                format!("zone.{}", source.strip_prefix("zone_").unwrap_or(source))
            );
            assert!(data.zones.iter().any(|zone| zone.id == *id));
        }
        for connection in &data.zone_connections {
            assert!(data.zones.iter().any(|zone| zone.id == connection.from));
            assert!(data.zones.iter().any(|zone| zone.id == connection.to));
        }
        assert_eq!(
            data.zones
                .iter()
                .filter(|zone| zone.initially_open)
                .map(|zone| zone.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["zone.start"]
        );
    }
}
