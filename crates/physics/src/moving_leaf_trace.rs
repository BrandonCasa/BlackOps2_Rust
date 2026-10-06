//! Qualified moving leaf/global-node brush subset, not a complete world query.
use crate::{
    moving_brush_trace::MovingBrushRequest,
    moving_brush_trace::{TraceEffect, TraceError, TraceResult},
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
pub struct MovingTraversalLimits {
    pub max_node_visits: usize,
    pub max_pending_nodes: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovingLeafError {
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
impl fmt::Display for MovingLeafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "moving brush-only leaf query failed ({self:?})")
    }
}
impl std::error::Error for MovingLeafError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovingLeafOutcome {
    FractionExhausted,
    MaskedOut,
    BoundsRejected,
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovingLeafStep {
    pub outcome: MovingLeafOutcome,
    pub first_overlap: Option<SourceKey>,
    /// Adapter diagnostics: entered nodes and actually invoked brush consumers.
    pub visited_nodes: usize,
    pub tested_brushes: usize,
}
#[derive(Clone, Copy)]
enum Frame {
    Enter(usize, [f32; 4], [f32; 4]),
    AfterAdjacent(usize, [f32; 4], [f32; 4]),
}
pub struct MovingBrushOnly<'a> {
    hierarchy: &'a CollisionHierarchy,
    brushes: &'a BrushCollectionSnapshot,
    lookup: Vec<Option<usize>>,
    pending: Vec<Frame>,
    limits: MovingTraversalLimits,
}
impl fmt::Debug for MovingBrushOnly<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MovingBrushOnly")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
impl<'a> MovingBrushOnly<'a> {
    /// Identity declarations and plane/provider references are checked. Actual payload
    /// authentication and leaf/scene ownership remain the assembling caller's task.
    pub fn new(
        hierarchy: &'a CollisionHierarchy,
        brushes: &'a BrushCollectionSnapshot,
        limits: MovingTraversalLimits,
    ) -> Result<Self, MovingLeafError> {
        if limits.max_pending_nodes > MAX_PENDING_NODES || limits.max_node_visits > MAX_NODE_VISITS
        {
            return Err(MovingLeafError::InvalidLimits);
        }
        hierarchy
            .validate_brush_collection(brushes)
            .map_err(MovingLeafError::Hierarchy)?;
        let mut lookup = Vec::new();
        lookup
            .try_reserve_exact(hierarchy.declared_brush_count())
            .map_err(|_| MovingLeafError::AllocationFailed)?;
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
            .map_err(|_| MovingLeafError::AllocationFailed)?;
        Ok(Self {
            hierarchy,
            brushes,
            lookup,
            pending,
            limits,
        })
    }
    /// All errors leave every incoming authoritative field unchanged. Scratch is reset
    /// after success/error; queries allocate no workspace. Zero incoming fraction
    /// exits before leaf admission, matching the qualified outer caller.
    pub fn trace_leaf(
        &mut self,
        leaf: &TerrainLeaf,
        request: &MovingBrushRequest,
        incoming: &mut TraceResult,
    ) -> Result<MovingLeafStep, MovingLeafError> {
        self.pending.clear();
        let mut result = *incoming;
        let answer = self.trace_inner(leaf, request, &mut result);
        self.pending.clear();
        if answer.is_ok() {
            *incoming = result;
        }
        answer
    }
    fn push(&mut self, frame: Frame) -> Result<(), MovingLeafError> {
        if self.pending.len() >= self.limits.max_pending_nodes {
            return Err(MovingLeafError::PendingCapacity);
        }
        self.pending.push(frame);
        Ok(())
    }
    fn split(
        &mut self,
        index: usize,
        start: [f32; 4],
        end: [f32; 4],
        request: &MovingBrushRequest,
        fraction: f32,
    ) -> Result<(), MovingLeafError> {
        let n = self
            .hierarchy
            .brush_nodes()
            .get(index)
            .ok_or(MovingLeafError::InvalidReference)?;
        let axis = usize::from(n.axis);
        if axis > 2 {
            return Err(MovingLeafError::InvalidReference);
        }
        let a = finite(start[axis] - f32::from_bits(n.distance_bits))?;
        let b = finite(end[axis] - f32::from_bits(n.distance_bits))?;
        let padding =
            finite(finite(request.axial_support()[axis] + 0.125)? - f32::from_bits(n.range_bits))?;
        let maximum = if finite(a - b)? >= 0. { a } else { b };
        let delta = finite(b - a)?;
        let minimum = if delta >= 0. { a } else { b };
        let children = [
            index + usize::from(n.child_offsets[0]),
            index + usize::from(n.child_offsets[1]),
        ];
        if minimum >= padding {
            if -padding >= maximum {
                return Ok(());
            }
            return self.push(Frame::Enter(children[0], start, end));
        }
        if -padding >= maximum {
            return self.push(Frame::Enter(children[1], start, end));
        }
        if start[3] >= fraction {
            return Ok(());
        }
        let (first, mut near, mut far) = if delta.abs() > f32::from_bits(0x35000000) {
            let oriented = if delta >= 0. { -a } else { a };
            let reciprocal = finite(1. / delta.abs())?;
            (
                usize::from(delta >= 0.),
                finite(finite(oriented + padding)? * reciprocal)?,
                finite(finite(oriented - padding)? * reciprocal)?,
            )
        } else {
            (0, 1., 0.)
        };
        if finite(1. - near)? < 0. {
            near = 1.;
        }
        if far < 0. {
            far = 0.;
        }
        let first_end = interpolate(start, end, near)?;
        let second_start = interpolate(start, end, far)?;
        self.push(Frame::Enter(children[1 - first], second_start, end))?;
        self.push(Frame::Enter(children[first], start, first_end))
    }
    fn trace_inner(
        &mut self,
        leaf: &TerrainLeaf,
        request: &MovingBrushRequest,
        result: &mut TraceResult,
    ) -> Result<MovingLeafStep, MovingLeafError> {
        if !result.normal.is_finite() || !result.fraction.is_finite() {
            return Err(MovingLeafError::Trace(TraceError::NonFiniteInput));
        }
        if !(0.0..=1.0).contains(&result.fraction) {
            return Err(MovingLeafError::Trace(TraceError::InvalidFraction));
        }
        let mut step = MovingLeafStep {
            outcome: MovingLeafOutcome::Completed,
            first_overlap: None,
            visited_nodes: 0,
            tested_brushes: 0,
        };
        let mask = request.mask();
        if result.fraction == 0. {
            step.outcome = MovingLeafOutcome::FractionExhausted;
            return Ok(step);
        }
        if leaf.brush_contents & mask == 0 {
            step.outcome = MovingLeafOutcome::MaskedOut;
            return Ok(step);
        }
        let mins = leaf.mins_bits.map(f32::from_bits);
        let maxs = leaf.maxs_bits.map(f32::from_bits);
        if mins.into_iter().chain(maxs).any(|f| !f.is_finite()) {
            return Err(MovingLeafError::NonFiniteLeaf);
        }
        if (0..3).any(|i| mins[i] > maxs[i]) {
            return Err(MovingLeafError::InvalidLeafBounds);
        }
        let support = request.axial_support();
        let mut lower = [0.; 3];
        let mut upper = [0.; 3];
        for i in 0..3 {
            lower[i] = finite(mins[i] - support[i])?;
            upper[i] = finite(maxs[i] + support[i])?;
        }
        let endpoints = request.centered_endpoints();
        let start = endpoints[0].to_array();
        let end = endpoints[1].to_array();
        if !leaf_clip(
            start,
            end,
            request.reciprocal(),
            lower,
            upper,
            result.fraction,
        )? {
            step.outcome = MovingLeafOutcome::BoundsRejected;
            return Ok(step);
        }
        let start = [start[0], start[1], start[2], 0.];
        let end = [end[0], end[1], end[2], result.fraction];
        let root =
            usize::try_from(leaf.brush_root).map_err(|_| MovingLeafError::InvalidReference)?;
        if root >= self.hierarchy.brush_nodes().len() {
            return Err(MovingLeafError::InvalidReference);
        }
        self.push(Frame::Enter(root, start, end))?;
        while let Some(frame) = self.pending.pop() {
            match frame {
                Frame::AfterAdjacent(index, start, end) => {
                    self.split(index, start, end, request, result.fraction)?;
                }
                Frame::Enter(index, start, end) => {
                    if step.visited_nodes >= self.limits.max_node_visits {
                        return Err(MovingLeafError::NodeVisitLimit);
                    }
                    step.visited_nodes += 1;
                    let node = self
                        .hierarchy
                        .brush_nodes()
                        .get(index)
                        .ok_or(MovingLeafError::InvalidReference)?;
                    if node.contents & mask == 0 {
                        continue;
                    }
                    if node.count > 0 {
                        let first = node.terminal_first as usize;
                        let end = first
                            .checked_add(node.count as usize)
                            .ok_or(MovingLeafError::InvalidReference)?;
                        let indices = self
                            .hierarchy
                            .brush_indices()
                            .get(first..end)
                            .ok_or(MovingLeafError::InvalidReference)?;
                        for &id in indices {
                            let record_index = self
                                .lookup
                                .get(usize::from(id))
                                .and_then(|v| *v)
                                .ok_or(MovingLeafError::MissingProvider)?;
                            let record = &self.brushes.records()[record_index];
                            let brush = record.snapshot();
                            if brush.contents() & mask == 0 {
                                continue;
                            }
                            step.tested_brushes = step
                                .tested_brushes
                                .checked_add(1)
                                .ok_or(MovingLeafError::ArithmeticOverflow)?;
                            let effect = request
                                .trace(brush, result)
                                .map_err(MovingLeafError::Trace)?;
                            if effect == TraceEffect::Inside && step.first_overlap.is_none() {
                                step.first_overlap = Some(record.key());
                            }
                        }
                    } else if node.count < 0 {
                        self.push(Frame::AfterAdjacent(index, start, end))?;
                        self.push(Frame::Enter(index + 1, start, end))?;
                    } else {
                        self.split(index, start, end, request, result.fraction)?;
                    }
                }
            }
        }
        Ok(step)
    }
}

fn finite(value: f32) -> Result<f32, MovingLeafError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MovingLeafError::ArithmeticOverflow)
    }
}
fn interpolate(start: [f32; 4], end: [f32; 4], fraction: f32) -> Result<[f32; 4], MovingLeafError> {
    let mut out = [0.; 4];
    for i in 0..4 {
        out[i] = finite(finite(finite(end[i] - start[i])? * fraction)? + start[i])?;
    }
    Ok(out)
}
/// Source 1831e0: two ordered halfspace passes, strict entry/exit limits.
fn leaf_clip(
    start: [f32; 3],
    end: [f32; 3],
    reciprocal: [f32; 3],
    lower: [f32; 3],
    upper: [f32; 3],
    mut exit: f32,
) -> Result<bool, MovingLeafError> {
    let mut entry = 0.;
    for (sign, bounds) in [(-1., lower), (1., upper)] {
        for i in 0..3 {
            let a = finite(finite(start[i] - bounds[i])? * sign)?;
            let b = finite(finite(end[i] - bounds[i])? * sign)?;
            if a > 0. {
                if b > 0. {
                    return Ok(false);
                }
                let t = finite(finite(a * reciprocal[i])? * sign)?;
                if t >= exit {
                    return Ok(false);
                }
                if finite(entry - t)? < 0. {
                    entry = t;
                }
            } else if b > 0. {
                let t = finite(finite(a * reciprocal[i])? * sign)?;
                if entry >= t {
                    return Ok(false);
                }
                if finite(t - exit)? < 0. {
                    exit = t;
                }
            }
        }
    }
    Ok(true)
}
