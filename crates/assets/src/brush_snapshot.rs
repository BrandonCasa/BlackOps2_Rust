//! Bounded reader for the external, project-owned one-brush schema28 exchange.
//! This is not an original disk decoder. All words and opaque metadata are retained.
use serde::{Deserialize, Deserializer};
use std::{
    fmt,
    io::{self, Read},
};

pub const MAX_BYTES: usize = 64 * 1024;
pub const MAX_SIDES: usize = 64;
pub const MAX_VERTICES: usize = 256;
const MAX_INTERRUPTED_READS: usize = 1024;

/// No error retains source paths, raw values or parser/I/O messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushError {
    Io(io::ErrorKind),
    LimitExceeded,
    InterruptedReadLimit,
    AllocationFailed,
    LengthMismatch,
    InvalidJson,
    UnsupportedSchema,
    CountMismatch,
    InvalidReadCount,
    PresenceMismatch,
    NonFinite,
    InvalidBounds,
    ZeroNormal,
    ReferenceOverflow,
}
impl fmt::Display for BrushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "brush record rejected ({self:?})")
    }
}
impl std::error::Error for BrushError {}

/// Side order, float words and uninterpreted source tags survive admission.
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrushSide {
    pub plane_index: u32,
    pub cflags: u32,
    pub sflags: u32,
    pub normal_bits: [u32; 3],
    pub dist_bits: u32,
    #[serde(rename = "type")]
    pub plane_type: u8,
    pub signbits: u8,
}

// deserialize_with makes every nullable key REQUIRED: absent differs from null.
pub(crate) fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawBrushSnapshot {
    schema: u32,
    brush_index: u32,
    side_first: u32,
    vertex_first: u32,
    side_count: usize,
    vertex_count: usize,
    contents: u32,
    parallel_bounds_present: bool,
    parallel_contents_present: bool,
    #[serde(deserialize_with = "nullable")]
    parallel_contents: Option<u32>,
    mins_bits: [u32; 3],
    maxs_bits: [u32; 3],
    #[serde(deserialize_with = "nullable")]
    midpoint_bits: Option<[u32; 3]>,
    #[serde(deserialize_with = "nullable")]
    halfsize_bits: Option<[u32; 3]>,
    axial_cflags: [u32; 6],
    axial_sflags: [u32; 6],
    sides: Vec<BrushSide>,
    vertices_bits: Vec<[u32; 3]>,
}

/// Admitted record; construction is available only through the bounded reader.
pub struct BrushSnapshot(RawBrushSnapshot);

impl fmt::Debug for BrushSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrushSnapshot")
            .field("sides", &self.0.sides.len())
            .field("vertices", &self.0.vertices_bits.len())
            .finish_non_exhaustive()
    }
}
impl BrushSnapshot {
    pub fn schema(&self) -> u32 {
        self.0.schema
    }
    pub fn brush_index(&self) -> u32 {
        self.0.brush_index
    }
    pub fn side_first(&self) -> u32 {
        self.0.side_first
    }
    pub fn vertex_first(&self) -> u32 {
        self.0.vertex_first
    }
    pub fn contents(&self) -> u32 {
        self.0.contents
    }
    pub fn parallel_contents(&self) -> Option<u32> {
        self.0.parallel_contents
    }
    pub fn mins_bits(&self) -> [u32; 3] {
        self.0.mins_bits
    }
    pub fn maxs_bits(&self) -> [u32; 3] {
        self.0.maxs_bits
    }
    pub fn midpoint_bits(&self) -> Option<[u32; 3]> {
        self.0.midpoint_bits
    }
    pub fn halfsize_bits(&self) -> Option<[u32; 3]> {
        self.0.halfsize_bits
    }
    pub fn axial_cflags(&self) -> &[u32; 6] {
        &self.0.axial_cflags
    }
    pub fn axial_sflags(&self) -> &[u32; 6] {
        &self.0.axial_sflags
    }
    pub fn sides(&self) -> &[BrushSide] {
        &self.0.sides
    }
    pub fn vertices_bits(&self) -> &[[u32; 3]] {
        &self.0.vertices_bits
    }
}
impl RawBrushSnapshot {
    pub(crate) fn admit(
        self,
        schema: u32,
        minimum_sides: usize,
    ) -> Result<BrushSnapshot, BrushError> {
        self.validate(schema, minimum_sides)?;
        Ok(BrushSnapshot(self))
    }
    fn validate(&self, schema: u32, minimum_sides: usize) -> Result<(), BrushError> {
        if self.schema != schema {
            return Err(BrushError::UnsupportedSchema);
        }
        if !(minimum_sides..=MAX_SIDES).contains(&self.side_count)
            || self.vertex_count > MAX_VERTICES
        {
            return Err(BrushError::LimitExceeded);
        }
        if self.sides.len() != self.side_count || self.vertices_bits.len() != self.vertex_count {
            return Err(BrushError::CountMismatch);
        }
        self.side_first
            .checked_add(self.side_count as u32)
            .ok_or(BrushError::ReferenceOverflow)?;
        self.vertex_first
            .checked_add(self.vertex_count as u32)
            .ok_or(BrushError::ReferenceOverflow)?;
        if self.parallel_bounds_present != self.midpoint_bits.is_some()
            || self.parallel_bounds_present != self.halfsize_bits.is_some()
            || self.parallel_contents_present != self.parallel_contents.is_some()
        {
            return Err(BrushError::PresenceMismatch);
        }
        let lo = finite(self.mins_bits)?;
        let hi = finite(self.maxs_bits)?;
        if lo.into_iter().zip(hi).any(|(a, b)| a >= b) {
            return Err(BrushError::InvalidBounds);
        }
        if let Some(mid) = self.midpoint_bits {
            finite(mid)?;
        }
        if let Some(half) = self.halfsize_bits
            && finite(half)?.into_iter().any(|x| x < 0.0)
        {
            return Err(BrushError::InvalidBounds);
        }
        for side in &self.sides {
            if finite(side.normal_bits)?.into_iter().all(|x| x == 0.0) {
                return Err(BrushError::ZeroNormal);
            }
            finite([side.dist_bits; 3])?;
        }
        for &vertex in &self.vertices_bits {
            finite(vertex)?;
        }
        Ok(())
    }
}
fn finite(bits: [u32; 3]) -> Result<[f32; 3], BrushError> {
    let result = bits.map(f32::from_bits);
    if result.iter().all(|x| x.is_finite()) {
        Ok(result)
    } else {
        Err(BrushError::NonFinite)
    }
}

/// Reads at most 64KiB plus one length probe from the same handle, with bounded
/// interrupted-read retries. serde's recursion limit remains enabled. The byte
/// cap also bounds temporary JSON allocations; array counts are checked on admission.
pub fn read_brush_snapshot(
    reader: &mut impl Read,
    expected_bytes: u64,
) -> Result<BrushSnapshot, BrushError> {
    let bytes = read_bounded(reader, expected_bytes, MAX_BYTES)?;
    let snapshot: RawBrushSnapshot =
        serde_json::from_slice(&bytes).map_err(|_| BrushError::InvalidJson)?;
    snapshot.admit(28, 1)
}

pub(crate) fn read_bounded(
    reader: &mut impl Read,
    expected_bytes: u64,
    maximum: usize,
) -> Result<Vec<u8>, BrushError> {
    if expected_bytes > maximum as u64 {
        return Err(BrushError::LimitExceeded);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected_bytes as usize)
        .map_err(|_| BrushError::AllocationFailed)?;
    let mut scratch = [0u8; 4096];
    let mut interruptions = 0;
    loop {
        let limit = scratch.len().min(maximum + 1 - bytes.len());
        let n = match reader.read(&mut scratch[..limit]) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                interruptions += 1;
                if interruptions > MAX_INTERRUPTED_READS {
                    return Err(BrushError::InterruptedReadLimit);
                }
                continue;
            }
            Err(e) => return Err(BrushError::Io(e.kind())),
        };
        if n == 0 {
            break;
        }
        if n > limit {
            return Err(BrushError::InvalidReadCount);
        }
        if bytes.len() + n > maximum {
            return Err(BrushError::LimitExceeded);
        }
        bytes
            .try_reserve(n)
            .map_err(|_| BrushError::AllocationFailed)?;
        bytes.extend_from_slice(&scratch[..n]);
        if bytes.len() as u64 > expected_bytes {
            return Err(BrushError::LengthMismatch);
        }
    }
    if bytes.len() as u64 != expected_bytes {
        return Err(BrushError::LengthMismatch);
    }
    Ok(bytes)
}
