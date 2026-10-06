//! Shared scalar-f32 request construction; no player defaults or world ownership.
use crate::moving_brush_trace::TraceError;
use bevy::math::Vec3;

pub(crate) struct CenteredBounds {
    pub start: [f32; 3],
    pub end: [f32; 3],
    extent: [f32; 3],
}

pub(crate) struct CapsuleSupport {
    pub radius: f32,
    pub half_segment: f32,
    pub axial: [f32; 3],
}

pub(crate) fn centered_endpoints(
    start: Vec3,
    end: Vec3,
    lo: Vec3,
    hi: Vec3,
) -> Result<CenteredBounds, TraceError> {
    if ![start, end, lo, hi].into_iter().all(Vec3::is_finite) {
        return Err(TraceError::NonFiniteInput);
    }
    let mut start = start.to_array();
    let mut end = end.to_array();
    let mut extent = [0.0; 3];
    for axis in 0..3 {
        if lo[axis] > hi[axis] {
            return Err(TraceError::InvalidExtents);
        }
        let center = checked(checked(hi[axis] + lo[axis])? * 0.5)?;
        extent[axis] = checked(hi[axis] - center)?;
        if extent[axis] < 0.0 {
            return Err(TraceError::InvalidExtents);
        }
        start[axis] = checked(start[axis] + center)?;
        end[axis] = checked(end[axis] + center)?;
    }
    Ok(CenteredBounds { start, end, extent })
}

impl CenteredBounds {
    pub fn capsule_support(&self) -> Result<CapsuleSupport, TraceError> {
        // Ordered comparison retains X on equality, including signed zero.
        let radius = if self.extent[0] > self.extent[2] {
            self.extent[2]
        } else {
            self.extent[0]
        };
        let half_segment = checked(self.extent[2] - radius)?;
        Ok(CapsuleSupport {
            radius,
            half_segment,
            axial: [radius, radius, checked(radius + half_segment)?],
        })
    }
}

pub(crate) fn checked(value: f32) -> Result<f32, TraceError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(TraceError::NonFiniteArithmetic)
    }
}
