//! Qualified stationary leaf/global-node brush subset, not a complete world query.
use crate::{
    moving_brush_trace::{TraceEffect, TraceError, TraceResult},
    stationary_brush_trace::StationaryBrushRequest,
};
use bo2_assets::{
    brush_collection::{BrushCollectionSnapshot, SourceKey},
    collision_hierarchy::{CollisionHierarchy, HierarchyError},
    terrain_snapshot::TerrainLeaf,
};
use std::fmt;
pub const MAX_PENDING_NODES: usize = 1_000_000;
pub const MAX_NODE_VISITS: usize = 4_000_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationaryTraversalLimits {
    pub max_node_visits: usize,
    pub max_pending_nodes: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationaryLeafError {
    Hierarchy(HierarchyError),
    Trace(TraceError),
    InvalidLimits,
    AllocationFailed,
    NonFiniteLeaf,
    InvalidLeafBounds,
    InvalidReference,
    MissingProvider,
    NodeVisitLimit,
    PendingCapacity,
    ArithmeticOverflow,
}
impl fmt::Display for StationaryLeafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stationary brush-only leaf query failed ({self:?})")
    }
}
impl std::error::Error for StationaryLeafError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationaryLeafOutcome {
    MaskedOut,
    BoundsRejected,
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationaryLeafStep {
    pub outcome: StationaryLeafOutcome,
    pub first_overlap: Option<SourceKey>,
    /// Adapter diagnostics: entered nodes and actually invoked brush consumers.
    pub visited_nodes: usize,
    pub tested_brushes: usize,
}
#[derive(Clone, Copy)]
enum Frame {
    Enter(usize),
    AfterAdjacent(usize),
    AfterChild0(usize),
}
pub struct StationaryBrushOnly<'a> {
    hierarchy: &'a CollisionHierarchy,
    brushes: &'a BrushCollectionSnapshot,
    lookup: Vec<Option<usize>>,
    pending: Vec<Frame>,
    limits: StationaryTraversalLimits,
}
impl fmt::Debug for StationaryBrushOnly<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StationaryBrushOnly")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
impl<'a> StationaryBrushOnly<'a> {
    /// Identity declarations and plane/provider references are checked. Actual payload
    /// authentication and leaf/scene ownership remain the assembling caller's task.
    pub fn new(
        hierarchy: &'a CollisionHierarchy,
        brushes: &'a BrushCollectionSnapshot,
        limits: StationaryTraversalLimits,
    ) -> Result<Self, StationaryLeafError> {
        if limits.max_pending_nodes > MAX_PENDING_NODES || limits.max_node_visits > MAX_NODE_VISITS
        {
            return Err(StationaryLeafError::InvalidLimits);
        }
        hierarchy
            .validate_brush_collection(brushes)
            .map_err(StationaryLeafError::Hierarchy)?;
        let mut lookup = Vec::new();
        lookup
            .try_reserve_exact(hierarchy.declared_brush_count())
            .map_err(|_| StationaryLeafError::AllocationFailed)?;
        lookup.resize(hierarchy.declared_brush_count(), None);
        for (index, record) in brushes.records().iter().enumerate() {
            let key = record.key();
            if key.provider_id == 0
                && let Some(slot) = lookup.get_mut(key.brush_id as usize)
            {
                *slot = Some(index);
            }
        }
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(limits.max_pending_nodes)
            .map_err(|_| StationaryLeafError::AllocationFailed)?;
        Ok(Self {
            hierarchy,
            brushes,
            lookup,
            pending,
            limits,
        })
    }
    /// All errors leave every incoming authoritative field unchanged. Scratch is reset
    /// after success/error; queries allocate no workspace. Incoming allsolid does not
    /// cause an entry early-out absent from the admitted direct leaf consumer.
    pub fn trace_leaf(
        &mut self,
        leaf: &TerrainLeaf,
        request: &StationaryBrushRequest,
        incoming: &mut TraceResult,
    ) -> Result<StationaryLeafStep, StationaryLeafError> {
        self.pending.clear();
        let mut result = *incoming;
        let answer = self.trace_inner(leaf, request, &mut result);
        self.pending.clear();
        if answer.is_ok() {
            *incoming = result;
        }
        answer
    }
    fn push(&mut self, frame: Frame) -> Result<(), StationaryLeafError> {
        if self.pending.len() >= self.limits.max_pending_nodes {
            return Err(StationaryLeafError::PendingCapacity);
        }
        self.pending.push(frame);
        Ok(())
    }
    fn split(
        &mut self,
        index: usize,
        bounds: [bevy::math::Vec3; 2],
    ) -> Result<(), StationaryLeafError> {
        let node = self
            .hierarchy
            .brush_nodes()
            .get(index)
            .ok_or(StationaryLeafError::InvalidReference)?;
        let axis = usize::from(node.axis);
        if axis > 2 {
            return Err(StationaryLeafError::InvalidReference);
        }
        let distance = f32::from_bits(node.distance_bits);
        let c0 = index + usize::from(node.child_offsets[0]);
        let c1 = index + usize::from(node.child_offsets[1]);
        if bounds[0][axis] > distance {
            self.push(Frame::Enter(c0))
        } else if bounds[1][axis] >= distance {
            self.push(Frame::AfterChild0(c1))?;
            self.push(Frame::Enter(c0))
        } else {
            self.push(Frame::Enter(c1))
        }
    }
    fn trace_inner(
        &mut self,
        leaf: &TerrainLeaf,
        request: &StationaryBrushRequest,
        result: &mut TraceResult,
    ) -> Result<StationaryLeafStep, StationaryLeafError> {
        if !result.normal.is_finite() || !result.fraction.is_finite() {
            return Err(StationaryLeafError::Trace(TraceError::NonFiniteInput));
        }
        if !(0.0..=1.0).contains(&result.fraction) {
            return Err(StationaryLeafError::Trace(TraceError::InvalidFraction));
        }
        let mut step = StationaryLeafStep {
            outcome: StationaryLeafOutcome::Completed,
            first_overlap: None,
            visited_nodes: 0,
            tested_brushes: 0,
        };
        let mask = request.mask();
        // Native outer leaf caller gates brushContents before invoking483980.
        if leaf.brush_contents & mask == 0 {
            step.outcome = StationaryLeafOutcome::MaskedOut;
            return Ok(step);
        }
        let mins = leaf.mins_bits.map(f32::from_bits);
        let maxs = leaf.maxs_bits.map(f32::from_bits);
        if mins.into_iter().chain(maxs).any(|f| !f.is_finite()) {
            return Err(StationaryLeafError::NonFiniteLeaf);
        }
        if (0..3).any(|i| mins[i] > maxs[i]) {
            return Err(StationaryLeafError::InvalidLeafBounds);
        }
        let bounds = request.bounds();
        if (0..3).any(|i| mins[i] >= bounds[1][i] || bounds[0][i] >= maxs[i]) {
            step.outcome = StationaryLeafOutcome::BoundsRejected;
            return Ok(step);
        }
        let root =
            usize::try_from(leaf.brush_root).map_err(|_| StationaryLeafError::InvalidReference)?;
        if root >= self.hierarchy.brush_nodes().len() {
            return Err(StationaryLeafError::InvalidReference);
        }
        self.push(Frame::Enter(root))?;
        while let Some(frame) = self.pending.pop() {
            match frame {
                Frame::AfterAdjacent(index) => {
                    if result.allsolid {
                        break;
                    }
                    self.split(index, bounds)?;
                }
                Frame::AfterChild0(index) => {
                    if result.allsolid {
                        break;
                    }
                    self.push(Frame::Enter(index))?;
                }
                Frame::Enter(index) => {
                    if step.visited_nodes >= self.limits.max_node_visits {
                        return Err(StationaryLeafError::NodeVisitLimit);
                    }
                    step.visited_nodes += 1;
                    let node = self
                        .hierarchy
                        .brush_nodes()
                        .get(index)
                        .ok_or(StationaryLeafError::InvalidReference)?;
                    if node.contents & mask == 0 {
                        continue;
                    }
                    if node.count > 0 {
                        let first = node.terminal_first as usize;
                        let end = first
                            .checked_add(node.count as usize)
                            .ok_or(StationaryLeafError::InvalidReference)?;
                        let indices = self
                            .hierarchy
                            .brush_indices()
                            .get(first..end)
                            .ok_or(StationaryLeafError::InvalidReference)?;
                        for &id in indices {
                            let record_index = self
                                .lookup
                                .get(usize::from(id))
                                .and_then(|v| *v)
                                .ok_or(StationaryLeafError::MissingProvider)?;
                            let record = &self.brushes.records()[record_index];
                            let brush = record.snapshot();
                            if brush.contents() & mask == 0 {
                                continue;
                            }
                            step.tested_brushes = step
                                .tested_brushes
                                .checked_add(1)
                                .ok_or(StationaryLeafError::ArithmeticOverflow)?;
                            let effect = request
                                .trace(brush, result)
                                .map_err(StationaryLeafError::Trace)?;
                            if effect == TraceEffect::Inside && step.first_overlap.is_none() {
                                step.first_overlap = Some(record.key());
                            }
                            // Native checks allsolid after actual brush calls, including
                            // a rejected call when allsolid was already seeded.
                            if result.allsolid {
                                return Ok(step);
                            }
                        }
                    } else if node.count < 0 {
                        self.push(Frame::AfterAdjacent(index))?;
                        self.push(Frame::Enter(index + 1))?;
                    } else {
                        self.split(index, bounds)?;
                    }
                }
            }
        }
        Ok(step)
    }
}
