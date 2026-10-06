//! One native-evidenced stationary brush overlap, distinct from moving clipping.
//! Source: docs/reverse_engineering/stationary-world-contract.md. No world
//! traversal, player defaults or other collision families are supplied here.
use crate::{
    moving_brush_trace::{TraceEffect, TraceError, TraceResult},
    trace_shape::{centered_endpoints, checked},
};
use bevy::math::Vec3;
use bo2_assets::brush_snapshot::BrushSnapshot;

#[derive(Debug, Clone, Copy)]
pub struct StationaryBrushRequest {
    center: [f32; 3],
    mins: [f32; 3],
    maxs: [f32; 3],
    radius: f32,
    half_segment: f32,
    mask: u32,
}

impl StationaryBrushRequest {
    /// Explicit source-space origin and lower/upper extent endpoints. The native
    /// builder derives capsule-like support; unequal Y extent is not box support.
    pub fn new(origin: Vec3, lo: Vec3, hi: Vec3, mask: u32) -> Result<Self, TraceError> {
        let centered = centered_endpoints(origin, origin, lo, hi)?;
        let shape = centered.capsule_support()?;
        let center = centered.start;
        let mins = [
            checked(center[0] - shape.radius)?,
            checked(center[1] - shape.radius)?,
            checked(checked(center[2] - shape.half_segment)? - shape.radius)?,
        ];
        let maxs = [
            checked(center[0] + shape.radius)?,
            checked(center[1] + shape.radius)?,
            checked(checked(center[2] + shape.half_segment)? + shape.radius)?,
        ];
        Ok(Self {
            center,
            mins,
            maxs,
            radius: shape.radius,
            half_segment: shape.half_segment,
            mask,
        })
    }

    pub fn centered_origin(&self) -> Vec3 {
        Vec3::from_array(self.center)
    }

    /// Transactional: invalid input/arithmetic, masks and geometric rejection
    /// leave every incoming field unchanged. Callers own first-overlap order.
    pub fn trace(
        &self,
        brush: &BrushSnapshot,
        incoming: &mut TraceResult,
    ) -> Result<TraceEffect, TraceError> {
        if !incoming.normal.is_finite() || !incoming.fraction.is_finite() {
            return Err(TraceError::NonFiniteInput);
        }
        if !(0.0..=1.0).contains(&incoming.fraction) {
            return Err(TraceError::InvalidFraction);
        }
        if brush.contents() & self.mask == 0 {
            return Ok(TraceEffect::MaskedOut);
        }
        for (axis, max) in brush.maxs_bits().into_iter().enumerate() {
            if self.mins[axis] >= f32::from_bits(max) {
                return Ok(TraceEffect::Rejected);
            }
        }
        for (axis, min) in brush.mins_bits().into_iter().enumerate() {
            if f32::from_bits(min) >= self.maxs[axis] {
                return Ok(TraceEffect::Rejected);
            }
        }
        for side in brush.sides() {
            let n = side.normal_bits.map(f32::from_bits);
            // Stationary native order begins Y then X, followed by Z. Keep each
            // scalar operation separate; do not substitute a vector dot/FMA.
            let dot = checked(
                checked(checked(n[1] * self.center[1])? + checked(n[0] * self.center[0])?)?
                    + checked(n[2] * self.center[2])?,
            )?;
            let distance = checked(f32::from_bits(side.dist_bits) + self.radius)?;
            let support = checked(checked(self.half_segment * n[2])?.abs() + distance)?;
            if checked(dot - support)? > 0.0 {
                return Ok(TraceEffect::Rejected);
            }
        }
        let mut result = *incoming;
        result.startsolid = true;
        result.allsolid = true;
        result.fraction = 0.0;
        result.contents = brush.contents();
        result.surface_flags = 0;
        *incoming = result;
        Ok(TraceEffect::Inside)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_radius_keeps_signed_zero() {
        let request =
            StationaryBrushRequest::new(Vec3::ZERO, Vec3::ZERO, Vec3::new(-0.0, 0.0, 0.0), 1)
                .unwrap();
        assert_eq!(request.radius.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(request.half_segment.to_bits(), 0);
    }
}
