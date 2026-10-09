//! Retained conservative broadphase for native local-light shadow visibility.
use alloc::vec::Vec;
use bevy_ecs::{
    entity::{Entity, EntityHashMap, EntityHashSet},
    prelude::Resource,
};
use bevy_math::{IVec3, Vec3};
use bevy_platform::collections::HashMap;

const CELL_SIZE: f32 = 16.0;
const MAX_CASTER_CELLS: u64 = 64;
const MAX_LIGHT_CELLS: u64 = 4096;

#[derive(Clone, Copy)]
struct CasterCells {
    range: Option<(IVec3, IVec3)>,
    bypass: bool,
    seen: u64,
}

/// Native visibility retains this spatial membership between frames.
#[derive(Default)]
pub struct ShadowCasterSpatialIndex {
    cells: HashMap<IVec3, EntityHashSet>,
    records: EntityHashMap<CasterCells>,
    global: EntityHashSet,
    expired: Vec<Entity>,
    frame: u64,
}

/// Optional actual-game work counts; disabled in ordinary rendering.
#[derive(Resource, Default)]
pub struct ShadowVisibilityWorkCounts {
    /// Collect work counts only during explicitly requested validation.
    pub enabled: bool,
    /// Casters retained in the latest native query.
    pub tracked_casters: usize,
    /// Casters conservatively admitted to every local-light query.
    pub global_casters: usize,
    /// Nonempty spatial cells retained after the latest update.
    pub occupied_cells: usize,
    /// Cumulative additions or changes to spatial membership.
    pub membership_changes: u64,
    /// Cumulative light queries serviced by the spatial cells.
    pub candidate_queries: u64,
    /// Cumulative deduplicated casters admitted by spatial queries.
    pub candidate_casters: u64,
    /// Cumulative light queries that required exhaustive fallback.
    pub exhaustive_queries: u64,
    /// Cumulative casters admitted by exhaustive fallback.
    pub exhaustive_casters: u64,
    /// Caster visits that repeated exhaustive queries would require.
    pub possible_caster_visits: u64,
}

pub(crate) fn begin(index: &mut ShadowCasterSpatialIndex) {
    index.frame = index.frame.wrapping_add(1);
}

pub(crate) fn needs_refresh(
    index: &ShadowCasterSpatialIndex,
    entity: Entity,
    bypass: bool,
) -> bool {
    index
        .records
        .get(&entity)
        .is_none_or(|record| record.bypass != bypass)
}

pub(crate) fn retain(index: &mut ShadowCasterSpatialIndex, entity: Entity) {
    if let Some(record) = index.records.get_mut(&entity) {
        record.seen = index.frame;
    }
}

/// Invalid/large extents are global candidates, never omitted geometry.
pub(crate) fn cell_range(min: Vec3, max: Vec3, limit: u64) -> Option<(IVec3, IVec3)> {
    let min = (min / CELL_SIZE).floor();
    let max = (max / CELL_SIZE).floor();
    if !min.is_finite()
        || !max.is_finite()
        || min.abs().max_element() > 100_000_000.0
        || max.abs().max_element() > 100_000_000.0
        || min.cmpgt(max).any()
    {
        return None;
    }
    let min = min.as_ivec3();
    let max = max.as_ivec3();
    let count = (i64::from(max.x) - i64::from(min.x) + 1) as u64;
    let count = count.saturating_mul((i64::from(max.y) - i64::from(min.y) + 1) as u64);
    let count = count.saturating_mul((i64::from(max.z) - i64::from(min.z) + 1) as u64);
    (count <= limit).then_some((min, max))
}

fn visit_cells((min, max): (IVec3, IVec3), mut visit: impl FnMut(IVec3)) {
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                visit(IVec3::new(x, y, z));
            }
        }
    }
}

fn remove_membership(
    index: &mut ShadowCasterSpatialIndex,
    entity: Entity,
    range: Option<(IVec3, IVec3)>,
) {
    if let Some(range) = range {
        visit_cells(range, |cell| {
            if let bevy_platform::collections::hash_map::Entry::Occupied(mut entry) =
                index.cells.entry(cell)
            {
                entry.get_mut().remove(&entity);
                if entry.get().is_empty() {
                    entry.remove();
                }
            }
        });
    } else {
        index.global.remove(&entity);
    }
}

pub(crate) fn update(
    index: &mut ShadowCasterSpatialIndex,
    entity: Entity,
    bounds: Option<(Vec3, Vec3)>,
    bypass: bool,
    counts: &mut ShadowVisibilityWorkCounts,
) {
    let range = bounds.and_then(|(min, max)| cell_range(min, max, MAX_CASTER_CELLS));
    if let Some(previous) = index.records.get(&entity).copied() {
        if previous.range == range {
            index.records.insert(
                entity,
                CasterCells {
                    range,
                    bypass,
                    seen: index.frame,
                },
            );
            return;
        }
        remove_membership(index, entity, previous.range);
    }
    if let Some(range) = range {
        visit_cells(range, |cell| {
            index.cells.entry(cell).or_default().insert(entity);
        });
    } else {
        index.global.insert(entity);
    }
    index.records.insert(
        entity,
        CasterCells {
            range,
            bypass,
            seen: index.frame,
        },
    );
    if counts.enabled {
        counts.membership_changes += 1;
    }
}

pub(crate) fn retire(
    index: &mut ShadowCasterSpatialIndex,
    counts: &mut ShadowVisibilityWorkCounts,
) {
    index.expired.clear();
    index.expired.extend(
        index
            .records
            .iter()
            .filter_map(|(&entity, record)| (record.seen != index.frame).then_some(entity)),
    );
    // Split ownership of scratch and index so retirement retains its capacity.
    let mut expired = core::mem::take(&mut index.expired);
    for entity in expired.drain(..) {
        if let Some(record) = index.records.remove(&entity) {
            remove_membership(index, entity, record.range);
        }
    }
    index.expired = expired;
    if counts.enabled {
        counts.tracked_casters = index.records.len();
        counts.global_casters = index.global.len();
        counts.occupied_cells = index.cells.len();
    }
}

pub(crate) fn candidates(
    index: &ShadowCasterSpatialIndex,
    center: Vec3,
    radius: f32,
    output: &mut EntityHashSet,
    counts: &mut ShadowVisibilityWorkCounts,
) {
    output.clear();
    let padding = (center.abs().max_element() + radius.abs()) * 8.0 * f32::EPSILON + 0.001;
    let range = (radius.is_finite() && radius >= 0.0)
        .then(|| {
            cell_range(
                center - Vec3::splat(radius + padding),
                center + Vec3::splat(radius + padding),
                MAX_LIGHT_CELLS,
            )
        })
        .flatten();
    if let Some(range) = range {
        output.extend(index.global.iter().copied());
        visit_cells(range, |cell| {
            if let Some(entities) = index.cells.get(&cell) {
                output.extend(entities.iter().copied());
            }
        });
        if counts.enabled {
            counts.candidate_queries += 1;
            counts.candidate_casters += output.len() as u64;
        }
    } else {
        output.extend(index.records.keys().copied());
        if counts.enabled {
            counts.exhaustive_queries += 1;
            counts.exhaustive_casters += output.len() as u64;
        }
    }
    if counts.enabled {
        counts.possible_caster_visits += index.records.len() as u64;
    }
}
