//! A pure owned sequence of source-keyed diagnostic convex hulls. Engine/ECS,
//! collision masks, player hulls and original static ownership are not supplied.
use crate::{
    DVec3,
    brush::{Convention, ConvexBrush, QueryError, SegmentHit},
};
use bo2_assets::brush_collection::{BrushCollectionSnapshot, SourceKey};

#[derive(Debug)]
pub struct CollectionBrush {
    key: SourceKey,
    hull: ConvexBrush,
}
impl CollectionBrush {
    pub fn key(&self) -> SourceKey {
        self.key
    }
    pub fn hull(&self) -> &ConvexBrush {
        &self.hull
    }
}
#[derive(Debug)]
pub struct BrushCollection {
    brushes: Vec<CollectionBrush>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollectionHit {
    pub key: SourceKey,
    pub interval: SegmentHit,
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ResidualSummary {
    pub brushes: usize,
    pub source_vertices: usize,
    pub strict_violations: usize,
    pub old_tolerant_violations: usize,
    pub strict_affected_brushes: usize,
    pub old_tolerant_affected_brushes: usize,
    pub max_positive_residual: f64,
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CollectionSourceDiagnostics {
    pub all: ResidualSummary,
    pub zero_side: ResidualSummary,
}
impl BrushCollection {
    /// Admission is all-or-error: no failed hull may be omitted silently.
    pub fn from_snapshot(
        snapshot: &BrushCollectionSnapshot,
        convention: Convention,
    ) -> Result<Self, QueryError> {
        let mut brushes = Vec::new();
        brushes
            .try_reserve_exact(snapshot.records().len())
            .map_err(|_| QueryError::DiagnosticCapacity)?;
        for record in snapshot.records() {
            brushes.push(CollectionBrush {
                key: record.key(),
                hull: ConvexBrush::from_snapshot(record.snapshot(), convention)?,
            });
        }
        Ok(Self { brushes })
    }
    pub fn brushes(&self) -> &[CollectionBrush] {
        &self.brushes
    }
    /// Inexpensive data/convention disagreement scan over all retained vertices.
    /// No exhaustive corner reconstruction or storage witness is implied.
    pub fn source_vertex_diagnostics(
        &self,
        snapshot: &BrushCollectionSnapshot,
    ) -> Result<CollectionSourceDiagnostics, QueryError> {
        if self.brushes.len() != snapshot.records().len() {
            return Err(QueryError::SnapshotMismatch);
        }
        let mut summary = CollectionSourceDiagnostics::default();
        for (owned, record) in self.brushes.iter().zip(snapshot.records()) {
            if owned.key != record.key() {
                return Err(QueryError::SnapshotMismatch);
            }
            let brush = record.snapshot();
            let d = owned.hull.source_vertex_diagnostics(brush)?;
            for totals in [&mut summary.all, &mut summary.zero_side]
                .into_iter()
                .take(if brush.sides().is_empty() { 2 } else { 1 })
            {
                totals.brushes += 1;
                totals.source_vertices += d.vertices;
                totals.strict_violations += d.strict_violations;
                totals.old_tolerant_violations += d.old_tolerant_violations;
                totals.strict_affected_brushes += usize::from(d.strict_violations > 0);
                totals.old_tolerant_affected_brushes += usize::from(d.old_tolerant_violations > 0);
                totals.max_positive_residual =
                    totals.max_positive_residual.max(d.max_positive_residual);
            }
        }
        Ok(summary)
    }
    /// Visits every admitted hull, preserving each coefficient's source order.
    /// Minimum entering parameter wins; EXACT equality ties choose the lowest
    /// (provider_id, brush_id), independent of collection order. Start-inside and
    /// stationary hits therefore use the same rule. No epsilon/allocations.
    pub fn clip_segment(
        &self,
        start: DVec3,
        end: DVec3,
    ) -> Result<Option<CollectionHit>, QueryError> {
        if !start.is_finite() || !end.is_finite() {
            return Err(QueryError::NonFiniteInput);
        }
        let mut nearest: Option<CollectionHit> = None;
        for brush in &self.brushes {
            if let Some(interval) = brush.hull.clip_segment(start, end)? {
                let candidate = CollectionHit {
                    key: brush.key,
                    interval,
                };
                if nearest.is_none_or(|old| {
                    interval.entering < old.interval.entering
                        || (interval.entering == old.interval.entering && candidate.key < old.key)
                }) {
                    nearest = Some(candidate);
                }
            }
        }
        Ok(nearest)
    }
}
