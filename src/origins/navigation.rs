//! Bind externally exported native path records to the existing game navigation domain.
use super::{
    ContentId, GameDefinition, NativePathNode, NavigationLink, NavigationNode, Traversal, ZoneId,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct PathData {
    nodes: Vec<PathNode>,
}
#[derive(Deserialize)]
struct PathNode {
    index: u16,
    r#type: u8,
    origin: [f32; 3],
    spawnflags: i32,
    animscript: String,
    links: Vec<PathLink>,
}
#[derive(Deserialize)]
struct PathLink {
    target: u16,
    #[serde(rename = "fDist")]
    distance: f32,
    flags: u8,
    #[serde(rename = "negotiationLink")]
    negotiation: u8,
    #[serde(rename = "disconnectCount")]
    disconnect_count: u8,
}

/// Authored game-zone ownership is separate from native streaming zones.
/// No nodes or directed links are filtered by area.
pub(crate) fn bind(
    game: &mut GameDefinition,
    bytes: &[u8],
    zones: &BTreeMap<u16, ZoneId>,
) -> Result<(), String> {
    let data: PathData = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid external pathdata: {error}"))?;
    if data.nodes.is_empty() {
        return Err("External pathdata has no nodes".into());
    }
    let ids: BTreeSet<_> = data.nodes.iter().map(|node| node.index).collect();
    if ids.len() != data.nodes.len() {
        return Err("External pathdata has duplicate native node IDs".into());
    }
    let authored: BTreeMap<_, _> = game
        .content
        .navigation_links
        .iter()
        .map(|link| (link.id.0.clone(), link))
        .collect();
    let mut nodes = Vec::with_capacity(data.nodes.len());
    let mut links = Vec::new();
    for node in &data.nodes {
        let zone = zones.get(&node.index).ok_or_else(|| {
            format!(
                "Native node {} has no authored gameplay-zone owner",
                node.index
            )
        })?;
        if !game.content.zones.iter().any(|defined| defined.id == *zone) {
            return Err(format!(
                "Native node {} names unknown gameplay zone {}",
                node.index, zone.0
            ));
        }
        let [x, y, z] = node.origin;
        let position = [x * 0.0254, z * 0.0254, -y * 0.0254];
        if position.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "Native node {} has a nonfinite position",
                node.index
            ));
        }
        nodes.push(NavigationNode {
            id: ContentId(format!("node.native.{}", node.index)),
            position,
            zone: zone.clone(),
            native: Some(NativePathNode {
                r#type: node.r#type,
                spawn_flags: node.spawnflags,
            }),
        });
        for (ordinal, link) in node.links.iter().enumerate() {
            if !ids.contains(&link.target) || !link.distance.is_finite() || link.distance < 0. {
                return Err(format!(
                    "Native node {} link {ordinal} has an invalid target or cost",
                    node.index
                ));
            }
            let id = format!("link.native.{}.{ordinal}", node.index);
            let previous = authored.get(&id);
            let mut row = NavigationLink {
                id: ContentId(id.clone()),
                from: ContentId(format!("node.native.{}", node.index)),
                to: ContentId(format!("node.native.{}", link.target)),
                traversal: Traversal::Walk,
                cost: Some(link.distance * 0.0254),
                connection: previous.and_then(|value| value.connection.clone()),
                blockers: previous.map_or_else(Vec::new, |value| value.blockers.clone()),
                source_animation: None,
                negotiation: None,
            };
            if link.negotiation != 0 {
                let source = previous.ok_or_else(|| format!(
                    "Native negotiation {id} ({}) requires authored source traversal and endpoint metadata; external pathdata omits radius/minimum-use-distance/overlap", node.animscript))?;
                let native = source.negotiation.as_ref().ok_or_else(|| {
                    format!("Native negotiation {id} has no source endpoint metadata")
                })?;
                if native.start.index != node.index
                    || native.end.index != link.target
                    || native.flags != link.flags
                    || native.disconnect_count != link.disconnect_count
                    || (native.cost - link.distance * 0.0254).abs() > 0.0001
                {
                    return Err(format!(
                        "Authored negotiation {id} disagrees with external native link"
                    ));
                }
                row.cost = Some(native.cost);
                row.traversal = source.traversal;
                row.source_animation = source.source_animation.clone();
                row.negotiation = Some(native.clone());
                if row.source_animation.is_none() {
                    return Err(format!(
                        "Native negotiation {id} lacks source animation binding"
                    ));
                }
            }
            links.push(row);
        }
    }
    // Do not erase separately authored portal/rise connections.
    links.extend(
        game.content
            .navigation_links
            .iter()
            .filter(|link| !link.id.0.starts_with("link.native."))
            .cloned(),
    );
    nodes.extend(
        game.content
            .navigation_nodes
            .iter()
            .filter(|node| !node.id.0.starts_with("node.native."))
            .cloned(),
    );
    game.content.navigation_nodes = nodes;
    game.content.navigation_links = links;
    Ok(())
}

/// Resolve source node ownership from exact authored source volumes with Avian.
/// Explicit game-owned overrides resolve overlapping volumes and exterior nodes.
pub(crate) fn derive_zones(
    game: &GameDefinition,
    bytes: &[u8],
    overrides: &BTreeMap<u16, ZoneId>,
) -> Result<BTreeMap<u16, ZoneId>, String> {
    use avian3d::prelude::Collider;
    use bevy::prelude::{Quat, Vec3};
    let data: PathData = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid external pathdata: {error}"))?;
    let shapes = game
        .access
        .volumes
        .iter()
        .map(|volume| {
            let vertices = volume
                .volume
                .vertices
                .iter()
                .copied()
                .map(Vec3::from_array)
                .collect::<Vec<_>>();
            let collider = Collider::convex_hull(&vertices).ok_or_else(|| {
                format!(
                    "Gameplay zone {} has an invalid source volume {}",
                    volume.zone.0, volume.volume.id
                )
            })?;
            Ok((
                volume.zone.clone(),
                Vec3::from_array(volume.volume.position),
                collider,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut zones = overrides.clone();
    let mut unresolved = Vec::new();
    for node in data.nodes {
        if zones.contains_key(&node.index) {
            continue;
        }
        let [x, y, z] = node.origin;
        let position = Vec3::new(x, z, -y) * 0.0254;
        if !position.is_finite() {
            return Err(format!(
                "Native node {} has nonfinite source coordinates",
                node.index
            ));
        }
        let owners: BTreeSet<_> = shapes
            .iter()
            .filter_map(|(zone, origin, shape)| {
                shape
                    .contains_point(*origin, Quat::IDENTITY, position)
                    .then_some(zone.clone())
            })
            .collect();
        if owners.len() == 1 {
            zones.insert(node.index, owners.into_iter().next().unwrap());
        } else {
            unresolved.push(format!(
                "{}:{}",
                node.index,
                if owners.is_empty() {
                    "outside source volumes".into()
                } else {
                    owners
                        .iter()
                        .map(|zone| zone.0.as_str())
                        .collect::<Vec<_>>()
                        .join("+")
                }
            ));
        }
    }
    if !unresolved.is_empty() {
        return Err(format!(
            "{} native navigation nodes require explicit authored zone ownership: {}",
            unresolved.len(),
            unresolved
                .iter()
                .take(24)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(zones)
}
