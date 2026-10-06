//! Source-qualified classic per-instance point segments, not a player hull/world.
//! Native sector/instance gates and collmap geometry are intentionally separate.
use crate::{
    moving_brush_trace::{TraceError, TraceResult},
    trace_shape::checked,
};
use bevy::math::Vec3;
use bo2_assets::static_model_snapshot::{StaticModelSnapshot, StaticModelTriangle};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticPointError {
    Trace(TraceError),
    InvalidReference,
    /// Native skips world-normal/owner promotion after a negative bone return,
    /// despite local writes. This safe adapter instead rejects atomically.
    NegativeBoneContact,
}
impl From<TraceError> for StaticPointError {
    fn from(error: TraceError) -> Self {
        Self::Trace(error)
    }
}
impl fmt::Display for StaticPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "static point trace failed ({self:?})")
    }
}
impl std::error::Error for StaticPointError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticPointContact {
    pub instance_ordinal: u32,
    pub provider_ordinal: u32,
    pub surface_ordinal: u32,
    /// Original ordinal within the selected surface.
    pub triangle_ordinal: u32,
    pub bone_index: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticPointEffect {
    NoHit,
    Hit(StaticPointContact),
}

#[derive(Debug, Clone, Copy)]
pub struct StaticPointRequest {
    start: [f32; 3],
    end: [f32; 3],
    mask: u32,
}
impl StaticPointRequest {
    pub fn new(start: Vec3, end: Vec3, mask: u32) -> Result<Self, StaticPointError> {
        if !start.is_finite() || !end.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        Ok(Self {
            start: start.to_array(),
            end: end.to_array(),
            mask,
        })
    }
    /// Transactional. Raw 29acc0/00dfd0 behavior: source endpoints are transformed
    /// before provider LOD/surface gates. No instance contents or absolute bounds
    /// gate is inferred from the separate sector constructor/traversal.
    pub fn trace(
        &self,
        snapshot: &StaticModelSnapshot,
        instance_ordinal: u32,
        incoming: &mut TraceResult,
    ) -> Result<StaticPointEffect, StaticPointError> {
        if !incoming.normal.is_finite() || !incoming.fraction.is_finite() {
            return Err(TraceError::NonFiniteInput.into());
        }
        if !(0.0..=1.0).contains(&incoming.fraction) {
            return Err(TraceError::InvalidFraction.into());
        }
        let instance = snapshot
            .instances()
            .get(instance_ordinal as usize)
            .ok_or(StaticPointError::InvalidReference)?;
        let provider = snapshot
            .providers()
            .get(instance.provider as usize)
            .ok_or(StaticPointError::InvalidReference)?;
        let origin = instance.origin_bits.map(f32::from_bits);
        let axes = instance
            .inverse_axis_bits
            .map(|row| row.map(f32::from_bits));
        let start = transform(self.start, origin, axes)?;
        let end = transform(self.end, origin, axes)?;
        if provider.collision_lod < 0 {
            return Ok(StaticPointEffect::NoHit);
        }
        let segment = Segment::new(start, end)?;
        let mut result = *incoming;
        let mut selected = None;
        for surface in range(
            snapshot.surfaces(),
            provider.first_surface,
            provider.surface_count,
        )? {
            if surface.contents & self.mask == 0 {
                continue;
            }
            if !segment.bounds(
                surface.mins_bits.map(f32::from_bits),
                surface.maxs_bits.map(f32::from_bits),
                result.fraction,
            )? {
                continue;
            }
            for (index, triangle) in range(
                snapshot.triangles(),
                surface.first_triangle,
                surface.triangle_count,
            )?
            .iter()
            .enumerate()
            {
                if let Some((fraction, normal)) = segment.triangle(triangle, result.fraction)? {
                    result.fraction = fraction;
                    result.normal = Vec3::from_array(normal);
                    result.surface_flags = surface.surface_flags;
                    result.contents = surface.contents;
                    result.allsolid = false;
                    result.startsolid = false;
                    selected = Some(StaticPointContact {
                        instance_ordinal: instance.ordinal,
                        provider_ordinal: provider.ordinal,
                        surface_ordinal: surface.ordinal,
                        triangle_ordinal: index as u32,
                        bone_index: surface.bone_index,
                    });
                }
            }
        }
        let Some(contact) = selected else {
            return Ok(StaticPointEffect::NoHit);
        };
        if contact.bone_index < 0 {
            return Err(StaticPointError::NegativeBoneContact);
        }
        result.normal = Vec3::from_array(promote_normal(result.normal.to_array(), axes)?);
        *incoming = result;
        Ok(StaticPointEffect::Hit(contact))
    }
}

fn range<T>(values: &[T], first: u32, count: u32) -> Result<&[T], StaticPointError> {
    let first = first as usize;
    let end = first
        .checked_add(count as usize)
        .ok_or(StaticPointError::InvalidReference)?;
    values
        .get(first..end)
        .ok_or(StaticPointError::InvalidReference)
}
fn transform(
    point: [f32; 3],
    origin: [f32; 3],
    axes: [[f32; 3]; 3],
) -> Result<[f32; 3], TraceError> {
    let d = [
        checked(point[0] - origin[0])?,
        checked(point[1] - origin[1])?,
        checked(point[2] - origin[2])?,
    ];
    let mut local = [0.; 3];
    for j in 0..3 {
        // Source column transform adds the Y product to X before adding Z.
        local[j] = checked(
            checked(checked(d[1] * axes[1][j])? + checked(d[0] * axes[0][j])?)?
                + checked(d[2] * axes[2][j])?,
        )?;
    }
    Ok(local)
}
fn promote_normal(n: [f32; 3], axes: [[f32; 3]; 3]) -> Result<[f32; 3], TraceError> {
    let mut normal = [0.; 3];
    for i in 0..3 {
        normal[i] = dot(n, axes[i])?;
    }
    // The native squared-length accumulation begins Y, then X, then Z.
    let squared = checked(
        checked(checked(normal[1] * normal[1])? + checked(normal[0] * normal[0])?)?
            + checked(normal[2] * normal[2])?,
    )?;
    let length = checked(squared.sqrt())?;
    let denominator = if -length < 0. { length } else { 1. };
    let scale = checked(1. / denominator)?;
    Ok([
        checked(normal[0] * scale)?,
        checked(normal[1] * scale)?,
        checked(normal[2] * scale)?,
    ])
}
fn dot(a: [f32; 3], b: [f32; 3]) -> Result<f32, TraceError> {
    checked(checked(checked(a[0] * b[0])? + checked(a[1] * b[1])?)? + checked(a[2] * b[2])?)
}
fn start_dot(normal: [f32; 3], start: [f32; 3]) -> Result<f32, TraceError> {
    // 00dfd0 starts with plane.Y * start.Y, then adds plane.X * start.X.
    checked(
        checked(checked(normal[1] * start[1])? + checked(normal[0] * start[0])?)?
            + checked(normal[2] * start[2])?,
    )
}
struct Segment {
    start: [f32; 3],
    end: [f32; 3],
    reciprocal: [f32; 3],
    delta: [f32; 3],
}
impl Segment {
    fn new(start: [f32; 3], end: [f32; 3]) -> Result<Self, TraceError> {
        let mut reciprocal = [0.; 3];
        for i in 0..3 {
            let difference = checked(start[i] - end[i])?;
            if difference != 0. {
                reciprocal[i] = checked(1. / difference)?;
            }
        }
        let delta = [
            checked(end[0] - start[0])?,
            checked(end[1] - start[1])?,
            checked(end[2] - start[2])?,
        ];
        Ok(Self {
            start,
            end,
            reciprocal,
            delta,
        })
    }
    fn bounds(&self, mins: [f32; 3], maxs: [f32; 3], limit: f32) -> Result<bool, TraceError> {
        let (mut entry, mut exit) = (0., limit);
        for (bound, sign) in [(mins, -1.), (maxs, 1.)] {
            for (i, bound) in bound.into_iter().enumerate() {
                let a = checked(checked(self.start[i] - bound)? * sign)?;
                let b = checked(checked(self.end[i] - bound)? * sign)?;
                if a > 0. {
                    if b > 0. {
                        return Ok(false);
                    }
                    let time = checked(checked(a * self.reciprocal[i])? * sign)?;
                    if time >= exit {
                        return Ok(false);
                    }
                    if checked(entry - time)? < 0. {
                        entry = time;
                    }
                } else if b > 0. {
                    let time = checked(checked(a * self.reciprocal[i])? * sign)?;
                    if entry >= time {
                        return Ok(false);
                    }
                    if checked(time - exit)? < 0. {
                        exit = time;
                    }
                }
            }
        }
        Ok(true)
    }
    fn triangle(
        &self,
        triangle: &StaticModelTriangle,
        limit: f32,
    ) -> Result<Option<(f32, [f32; 3])>, TraceError> {
        let plane = triangle.plane_bits.map(f32::from_bits);
        let normal = [plane[0], plane[1], plane[2]];
        let b = checked(dot(normal, self.end)? - plane[3])?;
        if b >= 0. {
            return Ok(None);
        }
        let a = checked(start_dot(normal, self.start)? - plane[3])?;
        if a <= 0. {
            return Ok(None);
        }
        let denominator = checked(a - b)?;
        let fraction = checked(checked(a - 0.125)? / denominator)?;
        let fraction = if fraction < 0. { 0. } else { fraction };
        if fraction >= limit {
            return Ok(None);
        }
        let exact = checked(a / denominator)?;
        let point = [
            checked(checked(self.delta[0] * exact)? + self.start[0])?,
            checked(checked(self.delta[1] * exact)? + self.start[1])?,
            checked(checked(self.delta[2] * exact)? + self.start[2])?,
        ];
        let s = triangle.s_bits.map(f32::from_bits);
        let t = triangle.t_bits.map(f32::from_bits);
        let u = checked(dot([s[0], s[1], s[2]], point)? - s[3])?;
        const LOWER: f32 = f32::from_bits(0xba83_126f);
        const UPPER: f32 = f32::from_bits(0x3f80_20c5);
        if u < LOWER || u > UPPER {
            return Ok(None);
        }
        let v = checked(dot([t[0], t[1], t[2]], point)? - t[3])?;
        if v < LOWER || checked(v + u)? > UPPER {
            return Ok(None);
        }
        Ok(Some((fraction, normal)))
    }
}
