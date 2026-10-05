//! Safe reader for the project-owned diagnostic geometry snapshot v1.
//!
//! This exchange format is not a BO2 disk format or an engine asset. It preserves
//! finite float bits and ordered references; success proves no authenticity,
//! rendering or original-game fidelity. See the architecture contract document.

use std::fmt;
use std::io::{self, Read};
use std::mem::size_of;
use std::ops::Range;

pub const HEADER_BYTES: usize = 64;
pub const POSITION_BYTES: usize = 12;
pub const INDEX_BYTES: usize = 4;
pub const SURFACE_BYTES: usize = 48;
pub const VERSION: u32 = 1;
pub const MAGIC: [u8; 8] = *b"BO2DGS\0\0";

pub const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_POSITIONS: u64 = 2_000_000;
pub const MAX_INDICES: u64 = 10_000_000;
pub const MAX_SURFACES: u64 = 100_000;
pub const MAX_RETAINED_BYTES: u64 = 60 * 1024 * 1024;
// The ordered complete partition visits each index exactly once.
pub const MAX_REFERENCE_VISITS: u64 = MAX_INDICES;
pub const MAX_INTERRUPTED_READS: u32 = 1_024;

const SCRATCH_BYTES: usize = 4_096;

/// Fixed inspection policies, not source-format maxima.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotLimit {
    TotalBytes,
    Positions,
    Indices,
    Surfaces,
    RetainedBytes,
    InterruptedReads,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotSection {
    Header,
    Positions,
    Indices,
    Surfaces,
}

/// Errors retain neither paths, malformed field values nor underlying messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    Io { kind: io::ErrorKind },
    Truncated(SnapshotSection),
    UnsupportedMagic,
    UnsupportedVersion,
    UnsupportedFlags,
    UnsupportedReserved,
    ArithmeticOverflow,
    LimitExceeded(SnapshotLimit),
    LengthMismatch,
    TrailingData,
    AllocationFailed,
    NonFinitePosition,
    InvalidIndex,
    NonTriangularIndices,
    InvalidSurfaceRange,
    NonPartitioningSurfaces,
    InvalidSurfaceBounds,
    PositionOutsideSurfaceBounds,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { kind } => write!(output, "snapshot read failed ({kind:?})"),
            Self::Truncated(section) => write!(output, "truncated snapshot ({section:?})"),
            Self::UnsupportedMagic => output.write_str("unsupported diagnostic snapshot magic"),
            Self::UnsupportedVersion => output.write_str("unsupported diagnostic snapshot version"),
            Self::UnsupportedFlags => output.write_str("unsupported diagnostic snapshot flags"),
            Self::UnsupportedReserved => {
                output.write_str("unsupported diagnostic snapshot reserved bytes")
            }
            Self::ArithmeticOverflow => output.write_str("snapshot size arithmetic overflow"),
            Self::LimitExceeded(limit) => write!(output, "snapshot policy exceeded ({limit:?})"),
            Self::LengthMismatch => output.write_str("snapshot length mismatch"),
            Self::TrailingData => output.write_str("snapshot has trailing data"),
            Self::AllocationFailed => output.write_str("snapshot allocation failed"),
            Self::NonFinitePosition => output.write_str("snapshot contains a nonfinite position"),
            Self::InvalidIndex => output.write_str("snapshot index is out of range"),
            Self::NonTriangularIndices => {
                output.write_str("snapshot index count is not triangular")
            }
            Self::InvalidSurfaceRange => output.write_str("snapshot surface range is invalid"),
            Self::NonPartitioningSurfaces => {
                output.write_str("snapshot surfaces do not partition indices")
            }
            Self::InvalidSurfaceBounds => output.write_str("snapshot surface bounds are invalid"),
            Self::PositionOutsideSurfaceBounds => {
                output.write_str("snapshot position is outside its surface bounds")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

/// An ordered triangle range with opaque source metadata and validated endpoints.
#[derive(Debug, PartialEq)]
pub struct SnapshotSurface {
    first_index: usize,
    index_count: usize,
    source_format_category: u32,
    source_declared_vertex_count: u32,
    bounds: [[f32; 3]; 2],
}

impl SnapshotSurface {
    pub fn index_range(&self) -> Range<usize> {
        // Both fields and their sum were checked against the retained index array.
        self.first_index..self.first_index + self.index_count
    }

    pub fn triangle_count(&self) -> usize {
        self.index_count / 3
    }

    pub fn source_format_category(&self) -> u32 {
        self.source_format_category
    }

    pub fn source_declared_vertex_count(&self) -> u32 {
        self.source_declared_vertex_count
    }

    /// Inclusive lower and upper XYZ endpoints, retained without conversion.
    pub fn bounds(&self) -> &[[f32; 3]; 2] {
        &self.bounds
    }
}

/// Validated diagnostic arrays, without engine or source-asset identity.
pub struct GeometrySnapshot {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    surfaces: Vec<SnapshotSurface>,
    total_bytes: u64,
    retained_capacity_bytes: u64,
}

impl fmt::Debug for GeometrySnapshot {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output
            .debug_struct("GeometrySnapshot")
            .field("positions", &self.positions.len())
            .field("indices", &self.indices.len())
            .field("surfaces", &self.surfaces.len())
            .finish_non_exhaustive()
    }
}

impl GeometrySnapshot {
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn surfaces(&self) -> &[SnapshotSurface] {
        &self.surfaces
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Reported Vec capacities times element sizes, excluding allocator overhead.
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.retained_capacity_bytes
    }

    pub fn into_parts(self) -> (Vec<[f32; 3]>, Vec<u32>, Vec<SnapshotSurface>) {
        (self.positions, self.indices, self.surfaces)
    }
}

/// Read from offset zero with a length from the same opened file handle.
///
/// All size/count/length policies are checked before body reads or count-sized
/// allocations. Reads use fixed buffers plus fallibly reserved decoded vectors.
/// Success consumes `total_length` bytes and performs a one-byte EOF probe.
/// Admitted ranges never cross that length except the probe; short reads may
/// request an unread span again. At most 1024 Interrupted results are handled.
/// A custom blocking reader has no time bound. Same-length concurrent rewrites and
/// changes after the EOF probe cannot be detected; authenticity is not checked.
pub fn read_geometry_snapshot<R: Read + ?Sized>(
    reader: &mut R,
    total_length: u64,
) -> Result<GeometrySnapshot, SnapshotError> {
    admit(total_length, MAX_TOTAL_BYTES, SnapshotLimit::TotalBytes)?;
    if total_length < HEADER_BYTES as u64 {
        return Err(SnapshotError::Truncated(SnapshotSection::Header));
    }
    let mut input = SnapshotInput {
        reader,
        interrupted: 0,
    };
    let mut header = [0; HEADER_BYTES];
    input.exact(&mut header, SnapshotSection::Header)?;
    let layout = Layout::read(&header, total_length)?;

    let mut retained_capacity_bytes = 0;
    let mut positions = reserve(layout.positions, &mut retained_capacity_bytes)?;
    let mut indices = reserve(layout.indices, &mut retained_capacity_bytes)?;
    let mut surfaces = reserve(layout.surfaces, &mut retained_capacity_bytes)?;
    let mut scratch = [0; SCRATCH_BYTES];

    while positions.len() < layout.positions {
        let count = (layout.positions - positions.len()).min(SCRATCH_BYTES / POSITION_BYTES);
        let bytes = &mut scratch[..count * POSITION_BYTES];
        input.exact(bytes, SnapshotSection::Positions)?;
        for record in bytes.as_chunks::<POSITION_BYTES>().0 {
            let position = triple(record);
            if position.iter().any(|component| !component.is_finite()) {
                return Err(SnapshotError::NonFinitePosition);
            }
            positions.push(position);
        }
    }
    while indices.len() < layout.indices {
        let count = (layout.indices - indices.len()).min(SCRATCH_BYTES / INDEX_BYTES);
        let bytes = &mut scratch[..count * INDEX_BYTES];
        input.exact(bytes, SnapshotSection::Indices)?;
        for record in bytes.as_chunks::<INDEX_BYTES>().0 {
            let index = word(record);
            if u64::from(index) >= layout.positions as u64 {
                return Err(SnapshotError::InvalidIndex);
            }
            indices.push(index);
        }
    }
    let mut partition_end = 0;
    while surfaces.len() < layout.surfaces {
        let count = (layout.surfaces - surfaces.len()).min(SCRATCH_BYTES / SURFACE_BYTES);
        let bytes = &mut scratch[..count * SURFACE_BYTES];
        input.exact(bytes, SnapshotSection::Surfaces)?;
        for record in bytes.as_chunks::<SURFACE_BYTES>().0 {
            let surface = parse_surface(record, layout.indices, partition_end)?;
            let range = surface.index_range();
            for &index in &indices[range.clone()] {
                let position = &positions[index as usize];
                for (axis, &component) in position.iter().enumerate() {
                    if component < surface.bounds[0][axis] || component > surface.bounds[1][axis] {
                        return Err(SnapshotError::PositionOutsideSurfaceBounds);
                    }
                }
            }
            partition_end = range.end;
            surfaces.push(surface);
        }
    }
    if partition_end != layout.indices {
        return Err(SnapshotError::NonPartitioningSurfaces);
    }
    input.eof()?;
    Ok(GeometrySnapshot {
        positions,
        indices,
        surfaces,
        total_bytes: total_length,
        retained_capacity_bytes,
    })
}

struct Layout {
    positions: usize,
    indices: usize,
    surfaces: usize,
}

impl Layout {
    fn read(header: &[u8; HEADER_BYTES], total_length: u64) -> Result<Self, SnapshotError> {
        if header[..8] != MAGIC {
            return Err(SnapshotError::UnsupportedMagic);
        }
        if word(&header[8..12]) != VERSION {
            return Err(SnapshotError::UnsupportedVersion);
        }
        if word(&header[12..16]) != 0 {
            return Err(SnapshotError::UnsupportedFlags);
        }
        if header[48..].iter().any(|&byte| byte != 0) {
            return Err(SnapshotError::UnsupportedReserved);
        }
        let positions = wide(&header[16..24]);
        let indices = wide(&header[24..32]);
        let surfaces = wide(&header[32..40]);
        let surface_bytes = checked_bytes(surfaces, SURFACE_BYTES)?;
        let expected = checked_bytes(positions, POSITION_BYTES)?
            .checked_add(checked_bytes(indices, INDEX_BYTES)?)
            .and_then(|size| size.checked_add(surface_bytes))
            .and_then(|size| size.checked_add(HEADER_BYTES as u64))
            .ok_or(SnapshotError::ArithmeticOverflow)?;
        admit(positions, MAX_POSITIONS, SnapshotLimit::Positions)?;
        admit(indices, MAX_REFERENCE_VISITS, SnapshotLimit::Indices)?;
        admit(surfaces, MAX_SURFACES, SnapshotLimit::Surfaces)?;
        admit(expected, MAX_TOTAL_BYTES, SnapshotLimit::TotalBytes)?;
        let retained_surface_bytes = checked_bytes(surfaces, size_of::<SnapshotSurface>())?;
        let retained = checked_bytes(positions, size_of::<[f32; 3]>())?
            .checked_add(checked_bytes(indices, size_of::<u32>())?)
            .and_then(|size| size.checked_add(retained_surface_bytes))
            .ok_or(SnapshotError::ArithmeticOverflow)?;
        admit(retained, MAX_RETAINED_BYTES, SnapshotLimit::RetainedBytes)?;
        if expected != total_length || wide(&header[40..48]) != expected {
            return Err(SnapshotError::LengthMismatch);
        }
        if !indices.is_multiple_of(3) {
            return Err(SnapshotError::NonTriangularIndices);
        }
        Ok(Self {
            positions: platform_count(positions)?,
            indices: platform_count(indices)?,
            surfaces: platform_count(surfaces)?,
        })
    }
}

fn parse_surface(
    record: &[u8],
    index_count: usize,
    previous_end: usize,
) -> Result<SnapshotSurface, SnapshotError> {
    let first = wide(&record[..8]);
    let count = wide(&record[8..16]);
    let end = first
        .checked_add(count)
        .ok_or(SnapshotError::InvalidSurfaceRange)?;
    if end > index_count as u64 || !first.is_multiple_of(3) || !count.is_multiple_of(3) {
        return Err(SnapshotError::InvalidSurfaceRange);
    }
    if first != previous_end as u64 {
        return Err(SnapshotError::NonPartitioningSurfaces);
    }
    let bounds = [triple(&record[24..36]), triple(&record[36..48])];
    if (0..3).any(|axis| {
        !bounds[0][axis].is_finite()
            || !bounds[1][axis].is_finite()
            || bounds[0][axis] > bounds[1][axis]
    }) {
        return Err(SnapshotError::InvalidSurfaceBounds);
    }
    Ok(SnapshotSurface {
        first_index: platform_count(first)?,
        index_count: platform_count(count)?,
        source_format_category: word(&record[16..20]),
        source_declared_vertex_count: word(&record[20..24]),
        bounds,
    })
}

fn reserve<T>(count: usize, retained: &mut u64) -> Result<Vec<T>, SnapshotError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| SnapshotError::AllocationFailed)?;
    let capacity_bytes = checked_bytes(values.capacity() as u64, size_of::<T>())?;
    *retained = retained
        .checked_add(capacity_bytes)
        .ok_or(SnapshotError::ArithmeticOverflow)?;
    admit(*retained, MAX_RETAINED_BYTES, SnapshotLimit::RetainedBytes)?;
    Ok(values)
}

fn checked_bytes(count: u64, stride: usize) -> Result<u64, SnapshotError> {
    count
        .checked_mul(stride as u64)
        .ok_or(SnapshotError::ArithmeticOverflow)
}

fn platform_count(count: u64) -> Result<usize, SnapshotError> {
    usize::try_from(count).map_err(|_| SnapshotError::ArithmeticOverflow)
}

fn admit(value: u64, maximum: u64, limit: SnapshotLimit) -> Result<(), SnapshotError> {
    if value > maximum {
        Err(SnapshotError::LimitExceeded(limit))
    } else {
        Ok(())
    }
}

// Only fixed-size, internally admitted record slices reach these functions.
fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
fn wide(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
fn triple(bytes: &[u8]) -> [f32; 3] {
    [
        f32::from_bits(word(&bytes[..4])),
        f32::from_bits(word(&bytes[4..8])),
        f32::from_bits(word(&bytes[8..12])),
    ]
}

struct SnapshotInput<'a, R: ?Sized> {
    reader: &'a mut R,
    interrupted: u32,
}

impl<R: Read + ?Sized> SnapshotInput<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, SnapshotError> {
        loop {
            match self.reader.read(bytes) {
                Ok(count) if count <= bytes.len() => return Ok(count),
                Ok(_) => {
                    return Err(SnapshotError::Io {
                        kind: io::ErrorKind::InvalidData,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                    self.interrupted += 1;
                    if self.interrupted > MAX_INTERRUPTED_READS {
                        return Err(SnapshotError::LimitExceeded(
                            SnapshotLimit::InterruptedReads,
                        ));
                    }
                }
                Err(error) => return Err(SnapshotError::Io { kind: error.kind() }),
            }
        }
    }

    fn exact(&mut self, bytes: &mut [u8], section: SnapshotSection) -> Result<(), SnapshotError> {
        let mut filled = 0;
        while filled < bytes.len() {
            let count = self.read(&mut bytes[filled..])?;
            if count == 0 {
                return Err(SnapshotError::Truncated(section));
            }
            filled += count; // count <= remaining is checked by read().
        }
        Ok(())
    }

    fn eof(&mut self) -> Result<(), SnapshotError> {
        if self.read(&mut [0])? == 0 {
            Ok(())
        } else {
            Err(SnapshotError::TrailingData)
        }
    }
}
