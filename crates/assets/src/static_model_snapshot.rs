//! Bounded project-owned classic static point-collision exchange.
//! This does not admit collmap geometry, runtime sectors, or the player motor backend.
use crate::brush_collection::Identity;
use std::{
    fmt,
    io::{self, Read},
    mem::size_of,
};

pub const MAGIC: [u8; 8] = *b"BO2STAT\0";
pub const VERSION: u32 = 1;
pub const HEADER_BYTES: usize = 144;
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_ALLOCATION_BYTES: usize = 96 * 1024 * 1024;
pub const MAX_INSTANCES: usize = 65_535;
pub const MAX_PROVIDERS: usize = 65_535;
pub const MAX_SURFACES: usize = 65_536;
pub const MAX_TRIANGLES: usize = 1_000_000;
pub const MAX_COLLMAPS: usize = 1_000_000;
pub const MAX_UNRESOLVED_GEOMETRIES: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticModelError {
    Io(io::ErrorKind),
    Truncated,
    InvalidReadCount,
    InterruptedReadLimit,
    UnsupportedMagic,
    UnsupportedVersion,
    NonzeroReserved,
    LengthMismatch,
    LimitExceeded,
    ArithmeticOverflow,
    AllocationFailed,
    NonFinite,
    InvalidBounds,
    InvalidRadius,
    InvalidReference,
    InvalidOrdinal,
    InvalidOrder,
    InvalidPresence,
    IdentityMismatch,
}
impl fmt::Display for StaticModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "static model snapshot rejected ({self:?})")
    }
}
impl std::error::Error for StaticModelError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelIdentity {
    pub source: Identity,
    pub associations: Identity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelInstance {
    pub ordinal: u32,
    pub provider: u32,
    /// Serialized source metadata, not an admitted runtime sector list.
    pub serialized_next: u16,
    pub contents: u32,
    pub origin_bits: [u32; 3],
    pub inverse_axis_bits: [[u32; 3]; 3],
    pub mins_bits: [u32; 3],
    pub maxs_bits: [u32; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelProvider {
    /// First-use ordinal across ordered source instances.
    pub ordinal: u32,
    pub contents: u32,
    /// Negative values are retained; the qualified point consumer rejects them.
    pub collision_lod: i16,
    pub first_surface: u32,
    pub surface_count: u32,
    pub first_collmap: u32,
    pub collmap_count: u32,
    pub mins_bits: [u32; 3],
    pub maxs_bits: [u32; 3],
    pub radius_bits: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelSurface {
    /// Ordinal within the provider, not the flat exchange array index.
    pub ordinal: u32,
    pub first_triangle: u32,
    pub triangle_count: u32,
    pub mins_bits: [u32; 3],
    pub maxs_bits: [u32; 3],
    pub bone_index: i32,
    pub contents: u32,
    pub surface_flags: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelTriangle {
    pub plane_bits: [u32; 4],
    pub s_bits: [u32; 4],
    pub t_bits: [u32; 4],
}
/// Collmap / PhysGeomList scalar metadata only. Per-geom fields are not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticModelCollmap {
    pub ordinal: u32,
    pub geometry_list_present: bool,
    pub unresolved_geometry_count: u32,
    pub geometry_list_contents: u32,
}

/// Private construction guarantees immutable array/reference admission for this
/// exchange. Native cross-provider array aliases may be duplicated by the producer;
/// instance-to-provider sharing and source record order remain explicit.
pub struct StaticModelSnapshot {
    identity: StaticModelIdentity,
    instances: Vec<StaticModelInstance>,
    providers: Vec<StaticModelProvider>,
    surfaces: Vec<StaticModelSurface>,
    triangles: Vec<StaticModelTriangle>,
    collmaps: Vec<StaticModelCollmap>,
}
impl fmt::Debug for StaticModelSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticModelSnapshot")
            .field("instances", &self.instances.len())
            .field("providers", &self.providers.len())
            .field("surfaces", &self.surfaces.len())
            .field("triangles", &self.triangles.len())
            .field("collmaps", &self.collmaps.len())
            .finish_non_exhaustive()
    }
}
impl StaticModelSnapshot {
    pub fn identity(&self) -> StaticModelIdentity {
        self.identity
    }
    /// Compares declarations. The caller must independently authenticate inputs.
    pub fn check_identity(&self, expected: StaticModelIdentity) -> Result<(), StaticModelError> {
        if self.identity == expected {
            Ok(())
        } else {
            Err(StaticModelError::IdentityMismatch)
        }
    }
    pub fn instances(&self) -> &[StaticModelInstance] {
        &self.instances
    }
    pub fn providers(&self) -> &[StaticModelProvider] {
        &self.providers
    }
    pub fn surfaces(&self) -> &[StaticModelSurface] {
        &self.surfaces
    }
    pub fn triangles(&self) -> &[StaticModelTriangle] {
        &self.triangles
    }
    pub fn collmaps(&self) -> &[StaticModelCollmap] {
        &self.collmaps
    }
}

struct Input<'a, R> {
    reader: &'a mut R,
    interrupted: usize,
}
impl<R: Read> Input<'_, R> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], StaticModelError> {
        let mut result = [0; N];
        let mut offset = 0;
        while offset < N {
            match self.reader.read(&mut result[offset..]) {
                Ok(0) => return Err(StaticModelError::Truncated),
                Ok(n) if n <= N - offset => offset += n,
                Ok(_) => return Err(StaticModelError::InvalidReadCount),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    self.interrupted += 1;
                    if self.interrupted > 1024 {
                        return Err(StaticModelError::InterruptedReadLimit);
                    }
                }
                Err(e) => return Err(StaticModelError::Io(e.kind())),
            }
        }
        Ok(result)
    }
    fn u32(&mut self) -> Result<u32, StaticModelError> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn u16(&mut self) -> Result<u16, StaticModelError> {
        Ok(u16::from_le_bytes(self.bytes()?))
    }
    fn words<const N: usize>(&mut self) -> Result<[u32; N], StaticModelError> {
        let mut words = [0; N];
        for word in &mut words {
            *word = self.u32()?;
        }
        Ok(words)
    }
    fn zero16(&mut self) -> Result<(), StaticModelError> {
        if self.u16()? == 0 {
            Ok(())
        } else {
            Err(StaticModelError::NonzeroReserved)
        }
    }
}
fn vector<T>(count: usize) -> Result<Vec<T>, StaticModelError> {
    let mut vector = Vec::new();
    vector
        .try_reserve_exact(count)
        .map_err(|_| StaticModelError::AllocationFailed)?;
    Ok(vector)
}
fn finite<const N: usize>(bits: [u32; N]) -> Result<(), StaticModelError> {
    if bits.into_iter().all(|v| f32::from_bits(v).is_finite()) {
        Ok(())
    } else {
        Err(StaticModelError::NonFinite)
    }
}
fn bounds(mins: [u32; 3], maxs: [u32; 3]) -> Result<(), StaticModelError> {
    finite(mins)?;
    finite(maxs)?;
    if (0..3).all(|i| f32::from_bits(mins[i]) <= f32::from_bits(maxs[i])) {
        Ok(())
    } else {
        Err(StaticModelError::InvalidBounds)
    }
}
fn owned_span(
    first: u32,
    count: u32,
    cursor: &mut usize,
    length: usize,
) -> Result<(), StaticModelError> {
    if first as usize != *cursor {
        return Err(StaticModelError::InvalidOrder);
    }
    let end = cursor
        .checked_add(count as usize)
        .ok_or(StaticModelError::ArithmeticOverflow)?;
    if end > length {
        return Err(StaticModelError::InvalidReference);
    }
    *cursor = end;
    Ok(())
}

/// Reads exactly the declared exchange, without consuming trailing host bytes.
/// Requested retained element storage is capped; allocator metadata/rounding and
/// the caller's Read implementation are outside this cap. No state is published
/// before complete admission. Identities are declarations, not authentication.
pub fn read_static_model_snapshot<R: Read>(
    reader: &mut R,
    length: u64,
) -> Result<StaticModelSnapshot, StaticModelError> {
    if length > MAX_BYTES {
        return Err(StaticModelError::LimitExceeded);
    }
    if length < HEADER_BYTES as u64 {
        return Err(StaticModelError::Truncated);
    }
    let mut input = Input {
        reader,
        interrupted: 0,
    };
    let header = input.bytes::<HEADER_BYTES>()?;
    if header[..8] != MAGIC {
        return Err(StaticModelError::UnsupportedMagic);
    }
    let word = |i| u32::from_le_bytes(header[i..i + 4].try_into().expect("fixed header slice"));
    let wide = |i| u64::from_le_bytes(header[i..i + 8].try_into().expect("fixed header slice"));
    if word(8) != VERSION {
        return Err(StaticModelError::UnsupportedVersion);
    }
    if word(12) != 0 || header[124..].iter().any(|v| *v != 0) {
        return Err(StaticModelError::NonzeroReserved);
    }
    let identity = StaticModelIdentity {
        source: Identity {
            bytes: wide(24),
            sha256: header[32..64].try_into().expect("fixed header hash"),
        },
        associations: Identity {
            bytes: wide(64),
            sha256: header[72..104].try_into().expect("fixed header hash"),
        },
    };
    let counts: [usize; 5] = std::array::from_fn(|i| word(104 + i * 4) as usize);
    let limits = [
        MAX_INSTANCES,
        MAX_PROVIDERS,
        MAX_SURFACES,
        MAX_TRIANGLES,
        MAX_COLLMAPS,
    ];
    if counts.iter().zip(limits).any(|(n, cap)| *n > cap) {
        return Err(StaticModelError::LimitExceeded);
    }
    let sizes = [88, 56, 48, 48, 16];
    let mut expected = HEADER_BYTES;
    for (count, size) in counts.into_iter().zip(sizes) {
        expected = expected
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or(StaticModelError::ArithmeticOverflow)?,
            )
            .ok_or(StaticModelError::ArithmeticOverflow)?;
    }
    if wide(16) != length || expected as u64 != length {
        return Err(StaticModelError::LengthMismatch);
    }
    let mut allocation = 0_usize;
    for (count, size) in counts.into_iter().zip([
        size_of::<StaticModelInstance>(),
        size_of::<StaticModelProvider>(),
        size_of::<StaticModelSurface>(),
        size_of::<StaticModelTriangle>(),
        size_of::<StaticModelCollmap>(),
    ]) {
        allocation = allocation
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or(StaticModelError::ArithmeticOverflow)?,
            )
            .ok_or(StaticModelError::ArithmeticOverflow)?;
    }
    if allocation > MAX_ALLOCATION_BYTES {
        return Err(StaticModelError::LimitExceeded);
    }
    let [
        instance_count,
        provider_count,
        surface_count,
        triangle_count,
        collmap_count,
    ] = counts;
    let mut instances = vector(instance_count)?;
    let mut next_provider = 0;
    for ordinal in 0..instance_count {
        let source_ordinal = input.u32()?;
        let provider = input.u32()?;
        let serialized_next = input.u16()?;
        input.zero16()?;
        let contents = input.u32()?;
        let origin_bits = input.words()?;
        let inverse_axis_bits = [input.words()?, input.words()?, input.words()?];
        let mins_bits = input.words()?;
        let maxs_bits = input.words()?;
        if source_ordinal as usize != ordinal {
            return Err(StaticModelError::InvalidOrdinal);
        }
        if provider as usize >= provider_count || serialized_next as usize > instance_count {
            return Err(StaticModelError::InvalidReference);
        }
        if provider as usize > next_provider {
            return Err(StaticModelError::InvalidOrder);
        }
        if provider as usize == next_provider {
            next_provider += 1;
        }
        finite(origin_bits)?;
        for axis in inverse_axis_bits {
            finite(axis)?;
        }
        bounds(mins_bits, maxs_bits)?;
        instances.push(StaticModelInstance {
            ordinal: source_ordinal,
            provider,
            serialized_next,
            contents,
            origin_bits,
            inverse_axis_bits,
            mins_bits,
            maxs_bits,
        });
    }
    if next_provider != provider_count {
        return Err(StaticModelError::InvalidOrder);
    }
    let mut providers = vector(provider_count)?;
    let (mut surface_cursor, mut collmap_cursor) = (0, 0);
    for ordinal in 0..provider_count {
        let source_ordinal = input.u32()?;
        let contents = input.u32()?;
        let collision_lod = i16::from_le_bytes(input.bytes()?);
        input.zero16()?;
        let first_surface = input.u32()?;
        let owned_surfaces = input.u32()?;
        let first_collmap = input.u32()?;
        let owned_collmaps = input.u32()?;
        let mins_bits = input.words()?;
        let maxs_bits = input.words()?;
        let radius_bits = input.u32()?;
        if source_ordinal as usize != ordinal {
            return Err(StaticModelError::InvalidOrdinal);
        }
        owned_span(
            first_surface,
            owned_surfaces,
            &mut surface_cursor,
            surface_count,
        )?;
        owned_span(
            first_collmap,
            owned_collmaps,
            &mut collmap_cursor,
            collmap_count,
        )?;
        if owned_collmaps > u8::MAX.into() {
            return Err(StaticModelError::LimitExceeded);
        }
        bounds(mins_bits, maxs_bits)?;
        finite([radius_bits])?;
        if f32::from_bits(radius_bits) < 0. {
            return Err(StaticModelError::InvalidRadius);
        }
        providers.push(StaticModelProvider {
            ordinal: source_ordinal,
            contents,
            collision_lod,
            first_surface,
            surface_count: owned_surfaces,
            first_collmap,
            collmap_count: owned_collmaps,
            mins_bits,
            maxs_bits,
            radius_bits,
        });
    }
    if surface_cursor != surface_count || collmap_cursor != collmap_count {
        return Err(StaticModelError::InvalidOrder);
    }
    let mut surfaces = vector(surface_count)?;
    let mut triangle_cursor = 0;
    for provider in &providers {
        for ordinal in 0..provider.surface_count {
            let source_ordinal = input.u32()?;
            let first_triangle = input.u32()?;
            let owned_triangles = input.u32()?;
            let mins_bits = input.words()?;
            let maxs_bits = input.words()?;
            let bone_index = i32::from_le_bytes(input.bytes()?);
            let contents = input.u32()?;
            let surface_flags = input.u32()?;
            if source_ordinal != ordinal {
                return Err(StaticModelError::InvalidOrdinal);
            }
            owned_span(
                first_triangle,
                owned_triangles,
                &mut triangle_cursor,
                triangle_count,
            )?;
            bounds(mins_bits, maxs_bits)?;
            surfaces.push(StaticModelSurface {
                ordinal: source_ordinal,
                first_triangle,
                triangle_count: owned_triangles,
                mins_bits,
                maxs_bits,
                bone_index,
                contents,
                surface_flags,
            });
        }
    }
    if triangle_cursor != triangle_count {
        return Err(StaticModelError::InvalidOrder);
    }
    let mut triangles = vector(triangle_count)?;
    for _ in 0..triangle_count {
        let plane_bits = input.words()?;
        let s_bits = input.words()?;
        let t_bits = input.words()?;
        finite(plane_bits)?;
        finite(s_bits)?;
        finite(t_bits)?;
        triangles.push(StaticModelTriangle {
            plane_bits,
            s_bits,
            t_bits,
        });
    }
    let mut collmaps = vector(collmap_count)?;
    let mut unresolved_geometry_total = 0_usize;
    for provider in &providers {
        for ordinal in 0..provider.collmap_count {
            let source_ordinal = input.u32()?;
            let presence = input.u32()?;
            let unresolved_geometry_count = input.u32()?;
            let geometry_list_contents = input.u32()?;
            if source_ordinal != ordinal {
                return Err(StaticModelError::InvalidOrdinal);
            }
            if presence > 1 {
                return Err(StaticModelError::InvalidPresence);
            }
            unresolved_geometry_total = unresolved_geometry_total
                .checked_add(unresolved_geometry_count as usize)
                .ok_or(StaticModelError::ArithmeticOverflow)?;
            if unresolved_geometry_total > MAX_UNRESOLVED_GEOMETRIES {
                return Err(StaticModelError::LimitExceeded);
            }
            if presence == 0 && (unresolved_geometry_count != 0 || geometry_list_contents != 0) {
                return Err(StaticModelError::InvalidPresence);
            }
            collmaps.push(StaticModelCollmap {
                ordinal: source_ordinal,
                geometry_list_present: presence != 0,
                unresolved_geometry_count,
                geometry_list_contents,
            });
        }
    }
    Ok(StaticModelSnapshot {
        identity,
        instances,
        providers,
        surfaces,
        triangles,
        collmaps,
    })
}
