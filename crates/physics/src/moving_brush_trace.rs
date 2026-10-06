//! One native-evidenced moving brush query; no world traversal or stationary path.
//! Scalar f32 evaluation order follows findings0024. Invalid arithmetic is a safe
//! project rejection, not a claim about the original's invalid-input behavior.
use crate::trace_shape::{centered_endpoints, checked};
use bevy::math::Vec3;
use bo2_assets::brush_snapshot::BrushSnapshot;
use std::fmt;

const ENTRY_ALLOWANCE: f32 = 0.125;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceError {
    NonFiniteInput,
    InvalidExtents,
    InvalidFraction,
    UnsupportedStationary,
    NonFiniteArithmetic,
}
impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "moving brush trace failed ({self:?})")
    }
}
impl std::error::Error for TraceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactPlane {
    Axial { upper: bool, axis: usize },
    Side { index: usize },
}

/// Fields that this consumer can update. The original plane-distance field is
/// intentionally absent: this consumer never writes it. Flags are not reset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceResult {
    pub normal: Vec3,
    pub fraction: f32,
    pub surface_flags: u32,
    pub contents: u32,
    pub startsolid: bool,
    pub allsolid: bool,
    pub opaque_flag_2c: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEffect {
    MaskedOut,
    Rejected,
    Entry(ContactPlane),
    Inside,
}

/// Explicit caller parameters; construction computes the observed capsule-like
/// support. There are no player defaults. Every field remains finite.
#[derive(Debug, Clone, Copy)]
pub struct MovingBrushRequest {
    start: [f32; 3],
    end: [f32; 3],
    reciprocal: [f32; 3],
    support: [f32; 3],
    radius: f32,
    half_segment: f32,
    mask: u32,
}
impl MovingBrushRequest {
    pub fn new(start: Vec3, end: Vec3, lo: Vec3, hi: Vec3, mask: u32) -> Result<Self, TraceError> {
        let centered = centered_endpoints(start, end, lo, hi)?;
        let s = centered.start;
        let e = centered.end;
        if s == e {
            return Err(TraceError::UnsupportedStationary);
        }
        let shape = centered.capsule_support()?;
        let radius = shape.radius;
        let half_segment = shape.half_segment;
        let support = shape.axial;
        let mut reciprocal = [0.0; 3];
        for axis in 0..3 {
            let delta = checked(s[axis] - e[axis])?;
            if delta != 0.0 {
                reciprocal[axis] = checked(1.0 / delta)?;
            }
        }
        Ok(Self {
            start: s,
            end: e,
            reciprocal,
            support,
            radius,
            half_segment,
            mask,
        })
    }

    pub fn centered_endpoints(&self) -> [Vec3; 2] {
        [Vec3::from_array(self.start), Vec3::from_array(self.end)]
    }

    pub(crate) fn axial_support(&self) -> [f32; 3] {
        self.support
    }

    pub(crate) fn reciprocal(&self) -> [f32; 3] {
        self.reciprocal
    }

    pub(crate) fn mask(&self) -> u32 {
        self.mask
    }

    /// Transactional: arithmetic failure and rejected/filtered brushes leave
    /// incoming fields unchanged. Callers own aggregation order and initial flags.
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
        let mut clip = Clip {
            entry: 0.0,
            exit: incoming.fraction,
            selected: None,
            end_inside: true,
        };
        for (upper, words) in [(false, brush.mins_bits()), (true, brush.maxs_bits())] {
            let sign = if upper { 1.0 } else { -1.0 };
            for (axis, word) in words.into_iter().enumerate() {
                let bound = f32::from_bits(word);
                let a = checked(
                    checked(checked(self.start[axis] - bound)? * sign)? - self.support[axis],
                )?;
                let b = checked(
                    checked(checked(self.end[axis] - bound)? * sign)? - self.support[axis],
                )?;
                let mut normal = [0.0; 3];
                normal[axis] = sign;
                let slot = usize::from(upper) * 3 + axis;
                let contact = Contact {
                    plane: ContactPlane::Axial { upper, axis },
                    normal: Vec3::from_array(normal),
                    surface_flags: brush.axial_sflags()[slot],
                };
                if !clip.axial(a, b, self.reciprocal[axis], sign, contact)? {
                    return Ok(TraceEffect::Rejected);
                }
            }
        }
        for (index, side) in brush.sides().iter().enumerate() {
            let n = side.normal_bits.map(f32::from_bits);
            let distance = checked(f32::from_bits(side.dist_bits) + self.radius)?;
            let distance = checked(checked(self.half_segment * n[2])?.abs() + distance)?;
            let a = checked(dot(n, self.start)? - distance)?;
            let b = checked(dot(n, self.end)? - distance)?;
            let contact = Contact {
                plane: ContactPlane::Side { index },
                normal: Vec3::from_array(n),
                surface_flags: side.sflags,
            };
            if !clip.side(a, b, contact)? {
                return Ok(TraceEffect::Rejected);
            }
        }
        let mut result = *incoming;
        result.contents = brush.contents();
        let effect = if let Some(contact) = clip.selected {
            result.fraction = clip.entry;
            result.normal = contact.normal;
            result.surface_flags = contact.surface_flags;
            result.opaque_flag_2c = 0;
            TraceEffect::Entry(contact.plane)
        } else {
            result.startsolid = true;
            if clip.end_inside {
                result.allsolid = true;
                result.fraction = 0.0;
            }
            result.surface_flags = brush.axial_sflags()[5];
            result.contents = brush.axial_cflags()[5];
            TraceEffect::Inside
        };
        *incoming = result;
        Ok(effect)
    }
}

#[derive(Clone, Copy)]
struct Contact {
    plane: ContactPlane,
    normal: Vec3,
    surface_flags: u32,
}
struct Clip {
    entry: f32,
    exit: f32,
    selected: Option<Contact>,
    end_inside: bool,
}
impl Clip {
    fn axial(
        &mut self,
        a: f32,
        b: f32,
        inverse: f32,
        sign: f32,
        contact: Contact,
    ) -> Result<bool, TraceError> {
        if a > 0.0 {
            if b >= ENTRY_ALLOWANCE.min(a) {
                return Ok(false);
            }
            let candidate = checked(checked(checked(a - ENTRY_ALLOWANCE)? * inverse)? * sign)?;
            if candidate >= self.exit {
                return Ok(false);
            }
            if b > 0.0 {
                self.end_inside = false;
            }
            if candidate > self.entry {
                self.entry = candidate;
                self.selected = Some(contact);
            } else if self.selected.is_none() {
                self.selected = Some(contact);
            }
        } else if b > 0.0 {
            let candidate = checked(checked(a * inverse)? * sign)?;
            if self.entry >= candidate {
                return Ok(false);
            }
            if candidate < self.exit {
                self.exit = candidate;
            }
            self.end_inside = false;
        }
        Ok(true)
    }
    fn side(&mut self, a: f32, b: f32, contact: Contact) -> Result<bool, TraceError> {
        if a > 0.0 {
            if b >= ENTRY_ALLOWANCE.min(a) {
                return Ok(false);
            }
            if b > 0.0 {
                self.end_inside = false;
            }
            let denominator = checked(a - b)?;
            let adjusted = checked(a - ENTRY_ALLOWANCE)?;
            if adjusted > checked(denominator * self.entry)? {
                self.entry = checked(adjusted / denominator)?;
                if self.entry >= self.exit {
                    return Ok(false);
                }
                self.selected = Some(contact);
            } else if self.selected.is_none() {
                self.selected = Some(contact);
            }
        } else if b > 0.0 {
            let denominator = checked(a - b)?;
            if a > checked(denominator * self.exit)? {
                self.exit = checked(a / denominator)?;
                if self.entry >= self.exit {
                    return Ok(false);
                }
            }
            self.end_inside = false;
        }
        Ok(true)
    }
}
fn dot(n: [f32; 3], x: [f32; 3]) -> Result<f32, TraceError> {
    checked(checked(checked(n[0] * x[0])? + checked(n[1] * x[1])?)? + checked(n[2] * x[2])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_radius_selection_preserves_x_on_signed_zero_equality() {
        let q = MovingBrushRequest::new(
            Vec3::ZERO,
            Vec3::X,
            Vec3::ZERO,
            Vec3::new(-0.0, 0.0, 0.0),
            1,
        )
        .unwrap();
        assert_eq!(q.radius.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(q.support[0].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(q.support[1].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(q.support[2].to_bits(), 0);
    }
}
