//! Source-qualified 1d1230 moving point/triangle subset, not terrain aggregation.
use crate::{
    moving_brush_trace::{TraceError, TraceResult},
    trace_shape::checked,
};
use bevy::math::Vec3;
use bo2_assets::terrain_snapshot::TerrainSnapshot;
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPointError {
    Trace(TraceError),
    InvalidTriangle,
    InvalidVertex,
}
impl From<TraceError> for TerrainPointError {
    fn from(error: TraceError) -> Self {
        Self::Trace(error)
    }
}
impl fmt::Display for TerrainPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "terrain point triangle failed ({self:?})")
    }
}
impl std::error::Error for TerrainPointError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPointEffect {
    NoHit,
    Hit { triangle_ordinal: u32 },
}
#[derive(Debug, Clone, Copy)]
pub struct TerrainPointRequest {
    start: [f32; 3],
    displacement: [f32; 3],
}
impl TerrainPointRequest {
    /// Explicit centered start and native stored displacement; second vector is
    /// NOT an endpoint. No hull, mask, timing or source-builder defaults supplied.
    pub fn new(start: Vec3, displacement: Vec3) -> Result<Self, TerrainPointError> {
        if !start.is_finite() || !displacement.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        Ok(Self {
            start: start.to_array(),
            displacement: displacement.to_array(),
        })
    }
    /// Transactional scalar-f32 point consumer. Material, partition, stamp and
    /// eligibility writes belong to separately qualified assembly wrappers.
    pub fn trace(
        &self,
        snapshot: &TerrainSnapshot,
        triangle_ordinal: u32,
        incoming: &mut TraceResult,
    ) -> Result<TerrainPointEffect, TerrainPointError> {
        if !incoming.normal.is_finite() || !incoming.fraction.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        if !(0.0..=1.0).contains(&incoming.fraction) {
            return Err(TraceError::InvalidFraction.into());
        }
        let indices = snapshot
            .triangles()
            .get(triangle_ordinal as usize)
            .ok_or(TerrainPointError::InvalidTriangle)?;
        let mut vertices = [[0.; 3]; 3];
        for (out, &index) in vertices.iter_mut().zip(indices) {
            *out = snapshot
                .vertices_bits()
                .get(usize::from(index))
                .ok_or(TerrainPointError::InvalidVertex)?
                .map(f32::from_bits);
        }
        let [a, b, c] = vertices;
        let e1 = sub(a, b)?;
        let e2 = sub(a, c)?;
        let normal = cross(e2, e1)?;
        let d = dot_xyz(self.displacement, normal)?;
        if d >= 0. {
            return Ok(TerrainPointEffect::NoHit);
        }
        let offset = sub(a, self.start)?;
        let plane = dot_yxz(offset, normal)?;
        if plane > 0. || checked(incoming.fraction * d)? >= plane {
            return Ok(TerrainPointEffect::NoHit);
        }
        let side = cross(self.displacement, offset)?;
        let u = dot_yxz(side, e1)?;
        if u > 0. || d > u {
            return Ok(TerrainPointEffect::NoHit);
        }
        let v = dot_yxz(side, e2)?;
        if 0. > v || d > checked(u - v)? {
            return Ok(TerrainPointEffect::NoHit);
        }
        // 07f8d0: separately rounded XYZ squares/sum/sqrt, reciprocal guard1
        // for zero length. Returned x87 length is discarded by this caller.
        let length = checked(dot_xyz(normal, normal)?)?.sqrt();
        let denominator = if -length < 0. { length } else { 1. };
        let reciprocal = checked(1. / denominator)?;
        let normalized = [
            checked(normal[0] * reciprocal)?,
            checked(normal[1] * reciprocal)?,
            checked(normal[2] * reciprocal)?,
        ];
        let fraction = checked(plane / d)?;
        let mut result = *incoming;
        result.normal = Vec3::from_array(normalized);
        result.fraction = fraction;
        result.opaque_flag_2c = 0;
        *incoming = result;
        Ok(TerrainPointEffect::Hit { triangle_ordinal })
    }
}
fn sub(a: [f32; 3], b: [f32; 3]) -> Result<[f32; 3], TraceError> {
    Ok([
        checked(a[0] - b[0])?,
        checked(a[1] - b[1])?,
        checked(a[2] - b[2])?,
    ])
}
fn cross(a: [f32; 3], b: [f32; 3]) -> Result<[f32; 3], TraceError> {
    Ok([
        checked(checked(a[1] * b[2])? - checked(a[2] * b[1])?)?,
        checked(checked(a[2] * b[0])? - checked(a[0] * b[2])?)?,
        checked(checked(a[0] * b[1])? - checked(a[1] * b[0])?)?,
    ])
}
fn dot_xyz(a: [f32; 3], b: [f32; 3]) -> Result<f32, TraceError> {
    checked(checked(checked(a[0] * b[0])? + checked(a[1] * b[1])?)? + checked(a[2] * b[2])?)
}
fn dot_yxz(a: [f32; 3], b: [f32; 3]) -> Result<f32, TraceError> {
    checked(checked(checked(a[1] * b[1])? + checked(a[0] * b[0])?)? + checked(a[2] * b[2])?)
}
