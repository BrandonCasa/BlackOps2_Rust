//! The explicitly selected diagnostic convention is n·x - d <= 0. Original
//! coefficients are never normalized or adjusted for queries. Precision analysis
//! is separate, and does not select a contact epsilon or gameplay convention.
use bevy::math::DVec3;
use bo2_assets::brush_snapshot::{BrushSnapshot, MAX_SIDES};
use std::fmt;

const MAX_PLANES: usize = MAX_SIDES + 6;
const MAX_RECONSTRUCTED: usize = 1000;
const OLD_RESIDUAL: f64 = 1e-8;
const OLD_DEDUP: f64 = 1e-7;
const OLD_DETERMINANT: f64 = 1e-10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Convention {
    DiagnosticMinusDistanceAtMostZero,
}

#[derive(Clone, Copy)]
pub struct Plane {
    pub normal: DVec3,
    pub distance: f64,
}
impl Plane {
    const ZERO: Self = Self {
        normal: DVec3::ZERO,
        distance: 0.0,
    };
    fn residual(self, point: DVec3) -> Result<f64, QueryError> {
        checked(self.normal.dot(point) - self.distance)
    }
    fn old_tolerance(self, point: DVec3) -> f64 {
        OLD_RESIDUAL
            * 1.0_f64
                .max(self.distance.abs())
                .max((self.normal * point).abs().element_sum())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    NonFiniteInput,
    NonFiniteArithmetic,
    DiagnosticCapacity,
    InvalidSnapshot,
    SnapshotMismatch,
}
impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "brush query failed ({self:?})")
    }
}
impl std::error::Error for QueryError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentHit {
    /// Closed [0,1] parameter interval, including a zero-width tangent.
    pub entering: f64,
    pub exiting: f64,
    /// Boundary counts as inside; no epsilon is applied.
    pub start_inside: bool,
}

pub struct ConvexBrush {
    planes: [Plane; MAX_PLANES],
    plane_count: usize,
    mins: DVec3,
    maxs: DVec3,
}
impl fmt::Debug for ConvexBrush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConvexBrush")
            .field("planes", &self.plane_count)
            .finish_non_exhaustive()
    }
}
impl ConvexBrush {
    pub fn from_snapshot(snapshot: &BrushSnapshot, _: Convention) -> Result<Self, QueryError> {
        let side_count = snapshot.sides().len();
        if side_count == 0 || side_count > MAX_SIDES {
            return Err(QueryError::InvalidSnapshot);
        }
        let mins = decode(snapshot.mins_bits());
        let maxs = decode(snapshot.maxs_bits());
        if !mins.is_finite() || !maxs.is_finite() || !mins.cmplt(maxs).all() {
            return Err(QueryError::InvalidSnapshot);
        }
        let mut result = Self {
            planes: [Plane::ZERO; MAX_PLANES],
            plane_count: side_count + 6,
            mins,
            maxs,
        };
        for (plane, side) in result.planes.iter_mut().zip(snapshot.sides()) {
            *plane = Plane {
                normal: decode(side.normal_bits),
                distance: f64::from(f32::from_bits(side.dist_bits)),
            };
            if !plane.normal.is_finite()
                || plane.normal == DVec3::ZERO
                || !plane.distance.is_finite()
            {
                return Err(QueryError::InvalidSnapshot);
            }
        }
        for axis in 0..3 {
            let mut n = DVec3::ZERO;
            n[axis] = 1.0;
            result.planes[side_count + 2 * axis] = Plane {
                normal: n,
                distance: maxs[axis],
            };
            result.planes[side_count + 2 * axis + 1] = Plane {
                normal: -n,
                distance: -mins[axis],
            };
        }
        Ok(result)
    }
    pub fn planes(&self) -> &[Plane] {
        &self.planes[..self.plane_count]
    }
    pub fn endpoints(&self) -> [DVec3; 2] {
        [self.mins, self.maxs]
    }
    pub fn contains(&self, point: DVec3) -> Result<bool, QueryError> {
        if !point.is_finite() {
            return Err(QueryError::NonFiniteInput);
        }
        for &plane in self.planes() {
            if plane.residual(point)? > 0.0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Allocation-free halfspace clipping in preserved source order followed by
    /// six axial planes. Pure deterministic arithmetic for a fixed platform/input.
    /// Zero-length segments, parallel planes and boundary tangencies are included.
    pub fn clip_segment(&self, start: DVec3, end: DVec3) -> Result<Option<SegmentHit>, QueryError> {
        if !start.is_finite() || !end.is_finite() {
            return Err(QueryError::NonFiniteInput);
        }
        let (mut entering, mut exiting, mut start_inside) = (0.0_f64, 1.0_f64, true);
        for &plane in self.planes() {
            let a = plane.residual(start)?;
            let b = plane.residual(end)?;
            start_inside &= a <= 0.0;
            if a > 0.0 && b > 0.0 {
                return Ok(None);
            }
            if a <= 0.0 && b <= 0.0 {
                continue;
            }
            let denominator = checked(a - b)?;
            let crossing = checked(a / denominator)?;
            if a > 0.0 {
                entering = entering.max(crossing);
            } else {
                exiting = exiting.min(crossing);
            }
            if entering > exiting {
                return Ok(None);
            }
        }
        Ok(Some(SegmentHit {
            entering,
            exiting,
            start_inside,
        }))
    }

    /// Reproduces task0028's declared diagnostic solver thresholds, then tests a
    /// distinct storage-rounding hypothesis. These thresholds never enter queries.
    pub fn precision_diagnostics(
        &self,
        snapshot: &BrushSnapshot,
    ) -> Result<PrecisionDiagnostics, QueryError> {
        let check = Self::from_snapshot(snapshot, Convention::DiagnosticMinusDistanceAtMostZero)?;
        if self.plane_count != check.plane_count
            || self.planes().iter().zip(check.planes()).any(|(a, b)| {
                a.normal.to_array().map(f64::to_bits) != b.normal.to_array().map(f64::to_bits)
                    || a.distance.to_bits() != b.distance.to_bits()
            })
        {
            return Err(QueryError::SnapshotMismatch);
        }
        let vertices: Vec<_> = snapshot
            .vertices_bits()
            .iter()
            .copied()
            .map(decode)
            .collect();
        let mut report = PrecisionDiagnostics::default();
        for &v in &vertices {
            for &plane in self.planes() {
                let r = plane.residual(v)?;
                report.max_positive_residual = report.max_positive_residual.max(r);
                report.strict_violations += usize::from(r > 0.0);
                report.old_tolerant_violations += usize::from(r > plane.old_tolerance(v));
            }
        }
        let reconstructed = reconstruct(self.planes(), self.mins, self.maxs)?;
        report.reconstructed_vertices = reconstructed.len();
        report.vertex_set_max_error = set_error(&vertices, &reconstructed);
        if vertices.is_empty() {
            return Ok(report);
        }
        // Construct ONE joint coefficient witness, not independent per-residual
        // tolerance allowances: keep stored vertex/endpoints at their exact centers,
        // scale each side's normal uniformly and move d onto its support plane.
        // All altered coefficients must round back to their original words.
        let mut witness = self.planes;
        for (i, side) in snapshot.sides().iter().enumerate() {
            let plane = self.planes[i];
            let support = vertices
                .iter()
                .map(|v| plane.normal.dot(*v))
                .fold(f64::NEG_INFINITY, f64::max);
            let mut scale = [0.0_f64, f64::INFINITY];
            for axis in 0..3 {
                let value = plane.normal[axis];
                if value != 0.0 {
                    intersect_ratio(&mut scale, rounding_cell(side.normal_bits[axis])?, value);
                }
            }
            if support == 0.0 {
                let cell = rounding_cell(side.dist_bits)?;
                if !(cell[0] <= 0.0 && 0.0 <= cell[1]) {
                    report.rounding_witness = RoundingWitness::NotFound;
                    return Ok(report);
                }
            } else {
                intersect_ratio(&mut scale, rounding_cell(side.dist_bits)?, support);
            }
            if scale[0] >= scale[1] || !scale[0].is_finite() || !scale[1].is_finite() {
                report.rounding_witness = RoundingWitness::NotFound;
                return Ok(report);
            }
            let factor = scale[0] + (scale[1] - scale[0]) / 2.0;
            let adjusted = Plane {
                normal: plane.normal * factor,
                distance: support * factor,
            };
            let roundtrip = adjusted.normal.to_array().map(|x| (x as f32).to_bits());
            if roundtrip != side.normal_bits
                || (adjusted.distance as f32).to_bits() != side.dist_bits
            {
                report.rounding_witness = RoundingWitness::NotFound;
                return Ok(report);
            }
            for axis in 0..3 {
                report.max_coefficient_cell_fraction = report.max_coefficient_cell_fraction.max(
                    cell_fraction(adjusted.normal[axis], side.normal_bits[axis])?,
                );
            }
            report.max_coefficient_cell_fraction = report
                .max_coefficient_cell_fraction
                .max(cell_fraction(adjusted.distance, side.dist_bits)?);
            witness[i] = adjusted;
        }
        let witness_vertices = reconstruct(&witness[..self.plane_count], self.mins, self.maxs)?;
        report.witness_vertex_set_max_error = set_error(&vertices, &witness_vertices);
        // The solver has numerical error. Require a bijective proximity matching
        // within *the actual f32 cells*, plus equal counts, not an arbitrary epsilon.
        // Each solved point rounds to the corresponding source vertex words.
        // Real zero has no sign: when a solved coordinate is exactly zero,
        // choose the source's signed-zero representative (no numerical snapping).
        let mut seen = [false; bo2_assets::brush_snapshot::MAX_VERTICES];
        let mut matching = witness_vertices.len() == vertices.len();
        for point in &witness_vertices {
            if let Some(index) = snapshot
                .vertices_bits()
                .iter()
                .enumerate()
                .position(|(i, v)| !seen[i] && rounds_to(*point, *v))
            {
                seen[index] = true;
                for axis in 0..3 {
                    report.max_vertex_cell_fraction = report.max_vertex_cell_fraction.max(
                        cell_fraction(point[axis], snapshot.vertices_bits()[index][axis])?,
                    );
                }
            } else {
                matching = false;
                break;
            }
        }
        report.rounding_witness = if matching {
            RoundingWitness::NumericallyConstructed
        } else {
            RoundingWitness::NotFound
        };
        Ok(report)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RoundingWitness {
    #[default]
    Unavailable,
    /// One joint set of f64 coefficients and numerically reconstructed vertices
    /// rounds back to all stored words. Reconstruction uses the declared old
    /// diagnostic solver thresholds. Not an exact feasibility proof or evidence
    /// of original producer/runtime rounding.
    NumericallyConstructed,
    /// This restricted uniform-scale witness failed. Not a disproof of all possible
    /// independently rounded coefficient/vertex cells.
    NotFound,
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PrecisionDiagnostics {
    pub strict_violations: usize,
    pub old_tolerant_violations: usize,
    pub max_positive_residual: f64,
    pub reconstructed_vertices: usize,
    /// Bidirectional L-infinity set distance, as in the old analyzer.
    pub vertex_set_max_error: Option<f64>,
    pub rounding_witness: RoundingWitness,
    pub witness_vertex_set_max_error: Option<f64>,
    /// Fraction of the available directional half-ULP cell width consumed by
    /// the numerical witness. <=1 stays in the original storage cell.
    pub max_coefficient_cell_fraction: f64,
    pub max_vertex_cell_fraction: f64,
}

/// Round-to-nearest f32 storage cell in f64, including asymmetric adjacent ULPs.
/// End ties are candidates only; actual bit roundtrips qualify the witness.
pub fn rounding_cell(bits: u32) -> Result<[f64; 2], QueryError> {
    let value = f32::from_bits(bits);
    if !value.is_finite() {
        return Err(QueryError::NonFiniteInput);
    }
    let center = f64::from(value);
    let mut down = f64::from(value.next_down());
    let mut up = f64::from(value.next_up());
    if !down.is_finite() {
        down = center - (up - center);
    }
    if !up.is_finite() {
        up = center + (center - down);
    }
    Ok([down + (center - down) / 2.0, center + (up - center) / 2.0])
}
fn intersect_ratio(scale: &mut [f64; 2], cell: [f64; 2], value: f64) {
    let a = cell[0] / value;
    let b = cell[1] / value;
    scale[0] = scale[0].max(a.min(b));
    scale[1] = scale[1].min(a.max(b));
}
fn cell_fraction(value: f64, bits: u32) -> Result<f64, QueryError> {
    let center = f64::from(f32::from_bits(bits));
    if value == center {
        return Ok(0.0);
    }
    let cell = rounding_cell(bits)?;
    let radius = if value < center {
        center - cell[0]
    } else {
        cell[1] - center
    };
    checked((value - center).abs() / radius)
}
fn checked(x: f64) -> Result<f64, QueryError> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(QueryError::NonFiniteArithmetic)
    }
}
fn decode(bits: [u32; 3]) -> DVec3 {
    DVec3::from_array(bits.map(|x| f64::from(f32::from_bits(x))))
}
fn rounds_to(point: DVec3, source: [u32; 3]) -> bool {
    point.to_array().into_iter().zip(source).all(|(x, bits)| {
        let representative = if x == 0.0 && f32::from_bits(bits) == 0.0 {
            f64::from(f32::from_bits(bits))
        } else {
            x
        };
        (representative as f32).to_bits() == bits
    })
}
fn set_error(a: &[DVec3], b: &[DVec3]) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let one_way = |left: &[DVec3], right: &[DVec3]| {
        left.iter()
            .map(|x| {
                right
                    .iter()
                    .map(|y| (*x - *y).abs().max_element())
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0_f64, f64::max)
    };
    Some(one_way(a, b).max(one_way(b, a)))
}
fn reconstruct(planes: &[Plane], mins: DVec3, maxs: DVec3) -> Result<Vec<DVec3>, QueryError> {
    let mut result = Vec::new();
    let step = OLD_DEDUP * (maxs - mins).max_element().max(1.0);
    for i in 0..planes.len() {
        for j in i + 1..planes.len() {
            for k in j + 1..planes.len() {
                let [a, b, c] = [planes[i], planes[j], planes[k]];
                let det = a.normal.dot(b.normal.cross(c.normal));
                let scale = a.normal.length() * b.normal.length() * c.normal.length();
                if det.abs() / scale <= OLD_DETERMINANT {
                    continue;
                }
                let point = (a.distance * b.normal.cross(c.normal)
                    + b.distance * c.normal.cross(a.normal)
                    + c.distance * a.normal.cross(b.normal))
                    / det;
                if !point.is_finite() {
                    return Err(QueryError::NonFiniteArithmetic);
                }
                if planes
                    .iter()
                    .any(|p| p.normal.dot(point) - p.distance > p.old_tolerance(point))
                {
                    continue;
                }
                if result
                    .iter()
                    .any(|p: &DVec3| (*p - point).abs().max_element() <= step)
                {
                    continue;
                }
                if result.len() == MAX_RECONSTRUCTED {
                    return Err(QueryError::DiagnosticCapacity);
                }
                result.push(point);
            }
        }
    }
    Ok(result)
}
