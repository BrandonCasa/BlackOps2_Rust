//! Scalar source-qualified stationary sphere/one-triangle subset (482e40).
use crate::{
    moving_brush_trace::{TraceError, TraceResult},
    trace_shape::checked,
};
use bevy::math::Vec3;
use bo2_assets::terrain_snapshot::TerrainSnapshot;
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainStationarySphereError {
    Trace(TraceError),
    InvalidTriangle,
    InvalidVertex,
    InvalidRadius,
}
impl From<TraceError> for TerrainStationarySphereError {
    fn from(e: TraceError) -> Self {
        Self::Trace(e)
    }
}
impl fmt::Display for TerrainStationarySphereError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stationary terrain sphere failed ({self:?})")
    }
}
impl std::error::Error for TerrainStationarySphereError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainStationarySphereEffect {
    NoHit,
    Overlap,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainStationarySphereStep {
    pub effect: TerrainStationarySphereEffect,
    pub triangle_ordinal: u32,
    /// Native 4821a0 output, not a generic geometric closest-distance replacement.
    pub distance_squared: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct TerrainStationarySphereRequest {
    center: [f32; 3],
    radius: f32,
}
impl TerrainStationarySphereRequest {
    /// Explicit sphere only. No mask, hull, vertical segment or builder defaults.
    pub fn new(center: Vec3, radius: f32) -> Result<Self, TerrainStationarySphereError> {
        if !center.is_finite() || !radius.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        if radius < 0. {
            return Err(TerrainStationarySphereError::InvalidRadius);
        }
        Ok(Self {
            center: center.to_array(),
            radius,
        })
    }
    /// Raw triangle kernel. All failures preserve every authoritative result field.
    pub fn trace(
        &self,
        snapshot: &TerrainSnapshot,
        triangle_ordinal: u32,
        incoming: &mut TraceResult,
    ) -> Result<TerrainStationarySphereStep, TerrainStationarySphereError> {
        if !incoming.normal.is_finite() || !incoming.fraction.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        if !(0.0..=1.0).contains(&incoming.fraction) {
            return Err(TraceError::InvalidFraction.into());
        }
        let indices = snapshot
            .triangles()
            .get(triangle_ordinal as usize)
            .ok_or(TerrainStationarySphereError::InvalidTriangle)?;
        let mut vertices = [[0.; 3]; 3];
        for (out, &index) in vertices.iter_mut().zip(indices) {
            *out = snapshot
                .vertices_bits()
                .get(usize::from(index))
                .ok_or(TerrainStationarySphereError::InvalidVertex)?
                .map(f32::from_bits);
        }
        let distance_squared = distance(vertices, self.center)?;
        let overlap = checked(self.radius * self.radius)? > distance_squared;
        if overlap {
            let mut result = *incoming;
            result.startsolid = true;
            result.allsolid = true;
            result.fraction = 0.;
            *incoming = result;
        }
        Ok(TerrainStationarySphereStep {
            effect: if overlap {
                TerrainStationarySphereEffect::Overlap
            } else {
                TerrainStationarySphereEffect::NoHit
            },
            triangle_ordinal,
            distance_squared,
        })
    }
}
fn sub(a: [f32; 3], b: [f32; 3]) -> Result<[f32; 3], TraceError> {
    Ok([
        checked(a[0] - b[0])?,
        checked(a[1] - b[1])?,
        checked(a[2] - b[2])?,
    ])
}
fn yxz(a: [f32; 3], b: [f32; 3]) -> Result<f32, TraceError> {
    checked(checked(checked(a[1] * b[1])? + checked(a[0] * b[0])?)? + checked(a[2] * b[2])?)
}
fn xyz(a: [f32; 3], b: [f32; 3]) -> Result<f32, TraceError> {
    checked(checked(checked(a[0] * b[0])? + checked(a[1] * b[1])?)? + checked(a[2] * b[2])?)
}
fn project_b(u: f32, q: f32) -> Result<(f32, f32), TraceError> {
    Ok(if u >= 0. {
        (0., 0.)
    } else if -u >= q {
        (1., 0.)
    } else {
        (-checked(u / q)?, 0.)
    })
}
fn project_c(v: f32, s: f32) -> Result<(f32, f32), TraceError> {
    Ok(if v >= 0. {
        (0., 0.)
    } else if -v >= s {
        (0., 1.)
    } else {
        (0., -checked(v / s)?)
    })
}
fn opposite(q: f32, r: f32, s: f32, u: f32, v: f32) -> Result<(f32, f32), TraceError> {
    let numerator = checked(checked(checked(s - r)? + v)? - u)?;
    if numerator <= 0. {
        return Ok((0., 1.));
    }
    let denominator = checked(checked(q - checked(r * 2.)?)? + s)?;
    if numerator >= denominator {
        return Ok((1., 0.));
    }
    let b = checked(numerator / denominator)?;
    Ok((b, checked(1. - b)?))
}
fn distance([a, b, c]: [[f32; 3]; 3], center: [f32; 3]) -> Result<f32, TraceError> {
    let e = sub(b, a)?;
    let g = sub(c, a)?;
    let offset = sub(center, a)?;
    let q = yxz(e, e)?;
    let r = yxz(g, e)?;
    let s = yxz(g, g)?;
    let u = yxz(e, offset)?;
    let v = xyz(g, offset)?;
    let determinant = checked(checked(q * s)? - checked(r * r)?)?;
    let x = checked(checked(v * r)? - checked(u * s)?)?;
    let y = checked(checked(u * r)? - checked(v * q)?)?;
    // Regions preserve 481f10's center-minus-A signs and equality branches.
    let weights = if determinant >= checked(y + x)? {
        if x < 0. {
            if y < 0. {
                if u < 0. {
                    project_b(u, q)?
                } else if v >= 0. {
                    (0., 0.)
                } else {
                    project_c(v, s)?
                }
            } else {
                project_c(v, s)?
            }
        } else if y < 0. {
            project_b(u, q)?
        } else {
            let reciprocal = checked(1. / determinant)?;
            (checked(reciprocal * x)?, checked(reciprocal * y)?)
        }
    } else if x < 0. {
        let h = checked(v + s)?;
        if h > 0. {
            project_c(v, s)?
        } else if h > checked(u + r)? {
            opposite(q, r, s, u, v)?
        } else {
            (0., 1.)
        }
    } else if y < 0. {
        let h = checked(u + q)?;
        if h > 0. {
            project_b(u, q)?
        } else if h <= checked(v + r)? {
            (1., 0.)
        } else {
            opposite(q, r, s, u, v)?
        }
    } else {
        opposite(q, r, s, u, v)?
    };
    let mut point = [0.; 3];
    for i in 0..3 {
        point[i] =
            checked(checked(checked(e[i] * weights.0)? + a[i])? + checked(g[i] * weights.1)?)?;
    }
    let delta = sub(point, center)?;
    yxz(delta, delta)
}
