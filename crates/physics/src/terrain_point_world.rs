//! Ordered source-qualified moving point terrain aggregation, not a player world.
use crate::{
    moving_brush_trace::{TraceError, TraceResult},
    terrain_point_trace::{TerrainPointEffect, TerrainPointError, TerrainPointRequest},
    trace_shape::checked,
};
use bevy::math::Vec3;
use bo2_assets::terrain_snapshot::{TerrainAabb, TerrainLeaf, TerrainSnapshot};
use std::fmt;
const MAX_TESTS: usize = 4_000_000;
const MAX_PENDING: usize = 1_000_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainPointLimits {
    pub max_leaves: usize,
    pub max_aabb_tests: usize,
    pub max_pending_nodes: usize,
    pub max_triangles: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainLeafSource {
    Map(u32),
    Model(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainPointContact {
    pub material_ordinal: u32,
    pub partition_ordinal: u32,
    /// Diagnostic triangle of the last strict fraction reduction in this partition.
    pub triangle_ordinal: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainPointWorldStep {
    pub generation: u16,
    pub contact: Option<TerrainPointContact>,
    pub tested_aabbs: usize,
    pub tested_triangles: usize,
    pub admitted_partitions: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPointWorldError {
    Trace(TraceError),
    Point(TerrainPointError),
    InvalidLimits,
    AllocationFailed,
    WorkspaceMismatch,
    UnresolvedGenerationWrap,
    InvalidReference,
    LeafLimit,
    AabbTestLimit,
    PendingCapacity,
    TriangleLimit,
}
impl From<TraceError> for TerrainPointWorldError {
    fn from(e: TraceError) -> Self {
        Self::Trace(e)
    }
}
impl fmt::Display for TerrainPointWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "point terrain aggregation failed ({self:?})")
    }
}
impl std::error::Error for TerrainPointWorldError {}
#[derive(Debug, Clone, Copy)]
pub struct TerrainPointWorldRequest {
    point: TerrainPointRequest,
    midpoint: [f32; 3],
    half: [f32; 3],
    absolute_half: [f32; 3],
    mask: u32,
}
impl TerrainPointWorldRequest {
    /// Endpoints are already source-builder centered coordinates. Second vector is
    /// an endpoint here; the contained per-triangle request receives displacement.
    pub fn new(
        centered_start: Vec3,
        centered_end: Vec3,
        mask: u32,
    ) -> Result<Self, TerrainPointWorldError> {
        if !centered_start.is_finite() || !centered_end.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        if centered_start == centered_end {
            return Err(TraceError::UnsupportedStationary.into());
        }
        let s = centered_start.to_array();
        let e = centered_end.to_array();
        let mut d = [0.; 3];
        let mut midpoint = [0.; 3];
        let mut half = [0.; 3];
        for i in 0..3 {
            d[i] = checked(e[i] - s[i])?;
            midpoint[i] = checked(checked(e[i] + s[i])? * 0.5)?;
            half[i] = checked(d[i] * 0.5)?;
        }
        // Preserve the admitted finite builder domain, including its travel
        // length and volume heuristic even though point extents derive bypass0.
        let squared = checked(
            checked(checked(d[1] * d[1])? + checked(d[0] * d[0])?)? + checked(d[2] * d[2])?,
        )?;
        let length = squared.sqrt();
        let mut widths = [0.; 3];
        for i in 0..3 {
            let (low, high) = if e[i] > s[i] {
                (checked(s[i] - 0.)?, checked(e[i] + 0.)?)
            } else {
                (checked(e[i] - 0.)?, checked(s[i] + 0.)?)
            };
            widths[i] = checked(high - low)?;
        }
        let point_work = checked(0. * length)?;
        let swept_volume = checked(checked(widths[1] * widths[0])? * widths[2])?;
        debug_assert!(point_work <= swept_volume);
        let point = TerrainPointRequest::new(centered_start, Vec3::from_array(d))
            .map_err(TerrainPointWorldError::Point)?;
        Ok(Self {
            point,
            midpoint,
            half,
            absolute_half: half.map(f32::abs),
            mask,
        })
    }
}
pub struct TerrainPointWorld<'a> {
    snapshot: &'a TerrainSnapshot,
    limits: TerrainPointLimits,
}
pub struct TerrainPointWorkspace<'a> {
    snapshot: &'a TerrainSnapshot,
    limits: TerrainPointLimits,
    generation: u16,
    stamps: Vec<u16>,
    pending: Vec<usize>,
}
impl fmt::Debug for TerrainPointWorkspace<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerrainPointWorkspace")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}
impl TerrainPointWorkspace<'_> {
    pub fn generation(&self) -> u16 {
        self.generation
    }
}
impl<'a> TerrainPointWorld<'a> {
    pub fn new(
        snapshot: &'a TerrainSnapshot,
        limits: TerrainPointLimits,
    ) -> Result<Self, TerrainPointWorldError> {
        if limits.max_leaves > MAX_TESTS
            || limits.max_aabb_tests > MAX_TESTS
            || limits.max_pending_nodes > MAX_PENDING
            || limits.max_triangles > MAX_TESTS
        {
            return Err(TerrainPointWorldError::InvalidLimits);
        }
        Ok(Self { snapshot, limits })
    }
    pub fn new_workspace(&self) -> Result<TerrainPointWorkspace<'a>, TerrainPointWorldError> {
        let mut stamps = Vec::new();
        stamps
            .try_reserve_exact(self.snapshot.partitions().len())
            .map_err(|_| TerrainPointWorldError::AllocationFailed)?;
        stamps.resize(self.snapshot.partitions().len(), 0);
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(self.limits.max_pending_nodes)
            .map_err(|_| TerrainPointWorldError::AllocationFailed)?;
        Ok(TerrainPointWorkspace {
            snapshot: self.snapshot,
            limits: self.limits,
            generation: 0,
            stamps,
            pending,
        })
    }
    /// One generation covers the entire ordered selector list. No BSP enumeration.
    /// Prevalidation errors reserve none. Once reserved, even failed queries burn
    /// the generation/stamp writes; scratch clears, authoritative result is atomic,
    /// and retry must use the next distinct generation. No rollover policy chosen.
    pub fn trace_leaves(
        &self,
        leaves: &[TerrainLeafSource],
        request: &TerrainPointWorldRequest,
        workspace: &mut TerrainPointWorkspace<'a>,
        incoming: &mut TraceResult,
    ) -> Result<TerrainPointWorldStep, TerrainPointWorldError> {
        if !std::ptr::eq(self.snapshot, workspace.snapshot) || self.limits != workspace.limits {
            return Err(TerrainPointWorldError::WorkspaceMismatch);
        }
        if !incoming.normal.is_finite() || !incoming.fraction.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        if !(0.0..=1.0).contains(&incoming.fraction) {
            return Err(TraceError::InvalidFraction.into());
        }
        if workspace.generation == u16::MAX {
            return Err(TerrainPointWorldError::UnresolvedGenerationWrap);
        }
        workspace.generation += 1;
        workspace.pending.clear();
        let mut result = *incoming;
        let outcome = self.trace_inner(leaves, request, workspace, &mut result);
        workspace.pending.clear();
        if outcome.is_ok() {
            *incoming = result;
        }
        outcome
    }
    fn leaf(&self, source: TerrainLeafSource) -> Result<&TerrainLeaf, TerrainPointWorldError> {
        match source {
            TerrainLeafSource::Map(i) => self.snapshot.map_leaves().get(i as usize),
            TerrainLeafSource::Model(i) => self.snapshot.models().get(i as usize).map(|m| &m.leaf),
        }
        .ok_or(TerrainPointWorldError::InvalidReference)
    }
    fn push(
        &self,
        workspace: &mut TerrainPointWorkspace<'a>,
        index: usize,
    ) -> Result<(), TerrainPointWorldError> {
        if workspace.pending.len() >= self.limits.max_pending_nodes {
            return Err(TerrainPointWorldError::PendingCapacity);
        }
        workspace.pending.push(index);
        Ok(())
    }
    fn bounds(
        &self,
        index: usize,
        request: &TerrainPointWorldRequest,
        step: &mut TerrainPointWorldStep,
    ) -> Result<bool, TerrainPointWorldError> {
        if step.tested_aabbs >= self.limits.max_aabb_tests {
            return Err(TerrainPointWorldError::AabbTestLimit);
        }
        step.tested_aabbs += 1;
        let aabb = self
            .snapshot
            .aabbs()
            .get(index)
            .ok_or(TerrainPointWorldError::InvalidReference)?;
        sat(request, aabb).map_err(Into::into)
    }
    fn trace_inner(
        &self,
        leaves: &[TerrainLeafSource],
        request: &TerrainPointWorldRequest,
        workspace: &mut TerrainPointWorkspace<'a>,
        result: &mut TraceResult,
    ) -> Result<TerrainPointWorldStep, TerrainPointWorldError> {
        let mut step = TerrainPointWorldStep {
            generation: workspace.generation,
            contact: None,
            tested_aabbs: 0,
            tested_triangles: 0,
            admitted_partitions: 0,
        };
        for (ordinal, &source) in leaves.iter().enumerate() {
            if ordinal >= self.limits.max_leaves {
                return Err(TerrainPointWorldError::LeafLimit);
            }
            if result.fraction == 0. {
                continue;
            }
            let leaf = self.leaf(source)?;
            if leaf.terrain_contents & request.mask == 0 {
                continue;
            }
            let first = usize::from(leaf.first_aabb);
            let end = first
                .checked_add(usize::from(leaf.aabb_count))
                .ok_or(TerrainPointWorldError::InvalidReference)?;
            if end > self.snapshot.aabbs().len() {
                return Err(TerrainPointWorldError::InvalidReference);
            }
            for root in first..end {
                if result.fraction == 0. {
                    break;
                }
                let material_id = self.snapshot.aabbs()[root].material_index;
                let material = self
                    .snapshot
                    .materials()
                    .get(usize::from(material_id))
                    .ok_or(TerrainPointWorldError::InvalidReference)?;
                if material.contents & request.mask == 0 {
                    continue;
                }
                let before = result.fraction;
                let mut promoted = None;
                if !self.bounds(root, request, &mut step)? {
                    continue;
                }
                self.push(workspace, root)?;
                while let Some(index) = workspace.pending.pop() {
                    let aabb = &self.snapshot.aabbs()[index];
                    if aabb.child_count != 0 {
                        let first = usize::try_from(aabb.reference)
                            .map_err(|_| TerrainPointWorldError::InvalidReference)?;
                        let end = first
                            .checked_add(usize::from(aabb.child_count))
                            .ok_or(TerrainPointWorldError::InvalidReference)?;
                        if end > self.snapshot.aabbs().len() {
                            return Err(TerrainPointWorldError::InvalidReference);
                        }
                        for child in (first..end).rev() {
                            if self.bounds(child, request, &mut step)? {
                                self.push(workspace, child)?;
                            }
                        }
                    } else {
                        let partition_id = usize::try_from(aabb.reference)
                            .map_err(|_| TerrainPointWorldError::InvalidReference)?;
                        let stamp = workspace
                            .stamps
                            .get_mut(partition_id)
                            .ok_or(TerrainPointWorldError::InvalidReference)?;
                        if *stamp == workspace.generation {
                            continue;
                        }
                        *stamp = workspace.generation;
                        step.admitted_partitions += 1;
                        let partition = self
                            .snapshot
                            .partitions()
                            .get(partition_id)
                            .ok_or(TerrainPointWorldError::InvalidReference)?;
                        let first = usize::try_from(partition.first_triangle)
                            .map_err(|_| TerrainPointWorldError::InvalidReference)?;
                        let before_partition = result.fraction;
                        let mut triangle_contact = None;
                        for triangle in first..first + usize::from(partition.triangle_count) {
                            if step.tested_triangles >= self.limits.max_triangles {
                                return Err(TerrainPointWorldError::TriangleLimit);
                            }
                            step.tested_triangles += 1;
                            let before_triangle = result.fraction;
                            let effect = request
                                .point
                                .trace(self.snapshot, triangle as u32, result)
                                .map_err(TerrainPointWorldError::Point)?;
                            if matches!(effect, TerrainPointEffect::Hit { .. })
                                && before_triangle > result.fraction
                            {
                                triangle_contact = Some(triangle as u32);
                            }
                        }
                        if before_partition > result.fraction {
                            promoted = Some(TerrainPointContact {
                                material_ordinal: u32::from(material_id),
                                partition_ordinal: partition_id as u32,
                                triangle_ordinal: triangle_contact
                                    .ok_or(TerrainPointWorldError::InvalidReference)?,
                            });
                        }
                    }
                }
                if before > result.fraction {
                    result.surface_flags = material.surface_flags;
                    result.contents = material.contents;
                    step.contact = promoted;
                }
            }
        }
        Ok(step)
    }
}
/// 482ff0 initial and child SAT: point support zero; bypass derives false.
/// Separate operations/order are retained; no generic segment slab replacement.
fn sat(r: &TerrainPointWorldRequest, a: &TerrainAabb) -> Result<bool, TraceError> {
    let origin = a.origin_bits.map(f32::from_bits);
    let half = a.halfsize_bits.map(f32::from_bits);
    let mut d = [0.; 3];
    let mut h = [0.; 3];
    for i in 0..3 {
        d[i] = checked(r.midpoint[i] - origin[i])?;
        h[i] = checked(half[i] + 0.)?;
    }
    for i in 0..3 {
        if d[i].abs() > checked(r.absolute_half[i] + h[i])? {
            return Ok(false);
        }
    }
    let left = checked(checked(r.half[1] * d[2])? - checked(r.half[2] * d[1])?)?.abs();
    let right = checked(checked(r.absolute_half[2] * h[1])? + checked(r.absolute_half[1] * h[2])?)?;
    if left > right {
        return Ok(false);
    }
    let left = checked(checked(r.half[2] * d[0])? - checked(r.half[0] * d[2])?)?.abs();
    let right = checked(checked(r.absolute_half[2] * h[0])? + checked(r.absolute_half[0] * h[2])?)?;
    if left > right {
        return Ok(false);
    }
    let left = checked(checked(r.half[0] * d[1])? - checked(r.half[1] * d[0])?)?.abs();
    let right = checked(checked(r.absolute_half[1] * h[0])? + checked(r.absolute_half[0] * h[1])?)?;
    Ok(left <= right)
}
