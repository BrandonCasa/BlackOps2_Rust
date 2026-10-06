//! Bounded project-owned terrain exchange v1, not a BO2 decoder or scene manifest.
//! Exact source words/order are retained; identities require caller verification.
use crate::brush_collection::Identity;
use std::{
    fmt,
    io::{self, Read},
    mem::size_of,
};

pub const MAGIC: [u8; 8] = *b"BO2TERR\0";
pub const VERSION: u32 = 1;
pub const HEADER_BYTES: usize = 144;
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_ALLOCATION_BYTES: usize = 96 * 1024 * 1024;
pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_CHILD_REFERENCES: usize = 4_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainError {
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
    InvalidReference,
    InvalidPresence,
    InvalidModelOrdinal,
    PackedEdgeLength,
    Cycle,
    IdentityMismatch,
}
impl fmt::Display for TerrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "terrain snapshot rejected ({self:?})")
    }
}
impl std::error::Error for TerrainError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainIdentity {
    pub source: Identity,
    pub membership: Identity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainMaterial {
    pub surface_flags: u32,
    pub contents: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainAabb {
    pub origin_bits: [u32; 3],
    pub material_index: u16,
    pub child_count: u16,
    pub halfsize_bits: [u32; 3],
    pub reference: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainPartition {
    pub triangle_count: u8,
    pub first_triangle: i32,
    pub nuinds: i32,
    pub fuind: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainLeaf {
    pub first_aabb: u16,
    pub aabb_count: u16,
    pub brush_contents: u32,
    pub terrain_contents: u32,
    pub mins_bits: [u32; 3],
    pub maxs_bits: [u32; 3],
    pub brush_root: i32,
    pub cluster: i16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainModel {
    pub ordinal: u32,
    pub serialized_info_present: bool,
    pub mins_bits: [u32; 3],
    pub maxs_bits: [u32; 3],
    pub radius_bits: u32,
    pub leaf: TerrainLeaf,
}

/// Private construction guarantees terrain array/reference admission. Brush-root
/// references and runtime scene ownership require separate scene admission. This does
/// not establish runtime global binding, active poses, collision numerics or fidelity.
pub struct TerrainSnapshot {
    identity: TerrainIdentity,
    materials: Vec<TerrainMaterial>,
    aabbs: Vec<TerrainAabb>,
    partitions: Vec<TerrainPartition>,
    triangles: Vec<[u16; 3]>,
    vertices: Vec<[u32; 3]>,
    packed_edges: Vec<u8>,
    map_leaves: Vec<TerrainLeaf>,
    models: Vec<TerrainModel>,
}
impl fmt::Debug for TerrainSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerrainSnapshot")
            .field("materials", &self.materials.len())
            .field("aabbs", &self.aabbs.len())
            .field("partitions", &self.partitions.len())
            .field("triangles", &self.triangles.len())
            .field("vertices", &self.vertices.len())
            .field("map_leaves", &self.map_leaves.len())
            .field("models", &self.models.len())
            .finish_non_exhaustive()
    }
}
impl TerrainSnapshot {
    pub fn identity(&self) -> TerrainIdentity {
        self.identity
    }
    pub fn check_identity(&self, expected: TerrainIdentity) -> Result<(), TerrainError> {
        if self.identity == expected {
            Ok(())
        } else {
            Err(TerrainError::IdentityMismatch)
        }
    }
    pub fn materials(&self) -> &[TerrainMaterial] {
        &self.materials
    }
    pub fn aabbs(&self) -> &[TerrainAabb] {
        &self.aabbs
    }
    pub fn partitions(&self) -> &[TerrainPartition] {
        &self.partitions
    }
    pub fn triangles(&self) -> &[[u16; 3]] {
        &self.triangles
    }
    pub fn vertices_bits(&self) -> &[[u32; 3]] {
        &self.vertices
    }
    pub fn packed_edges(&self) -> &[u8] {
        &self.packed_edges
    }
    pub fn map_leaves(&self) -> &[TerrainLeaf] {
        &self.map_leaves
    }
    pub fn models(&self) -> &[TerrainModel] {
        &self.models
    }
}

struct Input<'a, R> {
    reader: &'a mut R,
    interrupted: usize,
}
impl<R: Read> Input<'_, R> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], TerrainError> {
        let mut result = [0; N];
        let mut offset = 0;
        while offset < N {
            match self.reader.read(&mut result[offset..]) {
                Ok(0) => return Err(TerrainError::Truncated),
                Ok(n) if n <= N - offset => offset += n,
                Ok(_) => return Err(TerrainError::InvalidReadCount),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    self.interrupted += 1;
                    if self.interrupted > 1024 {
                        return Err(TerrainError::InterruptedReadLimit);
                    }
                }
                Err(e) => return Err(TerrainError::Io(e.kind())),
            }
        }
        Ok(result)
    }
    fn u32(&mut self) -> Result<u32, TerrainError> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn i32(&mut self) -> Result<i32, TerrainError> {
        Ok(i32::from_le_bytes(self.bytes()?))
    }
    fn u16(&mut self) -> Result<u16, TerrainError> {
        Ok(u16::from_le_bytes(self.bytes()?))
    }
    fn triple(&mut self) -> Result<[u32; 3], TerrainError> {
        Ok([self.u32()?, self.u32()?, self.u32()?])
    }
    fn leaf(&mut self) -> Result<TerrainLeaf, TerrainError> {
        let leaf = TerrainLeaf {
            first_aabb: self.u16()?,
            aabb_count: self.u16()?,
            brush_contents: self.u32()?,
            terrain_contents: self.u32()?,
            mins_bits: self.triple()?,
            maxs_bits: self.triple()?,
            brush_root: self.i32()?,
            cluster: i16::from_le_bytes(self.bytes()?),
        };
        if self.u16()? != 0 {
            return Err(TerrainError::NonzeroReserved);
        }
        Ok(leaf)
    }
}
fn vector<T>(count: usize) -> Result<Vec<T>, TerrainError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| TerrainError::AllocationFailed)?;
    Ok(values)
}
fn finite(bits: [u32; 3]) -> Result<(), TerrainError> {
    if bits.into_iter().all(|b| f32::from_bits(b).is_finite()) {
        Ok(())
    } else {
        Err(TerrainError::NonFinite)
    }
}
fn bounds(mins: [u32; 3], maxs: [u32; 3]) -> Result<(), TerrainError> {
    finite(mins)?;
    finite(maxs)?;
    if (0..3).all(|i| f32::from_bits(mins[i]) <= f32::from_bits(maxs[i])) {
        Ok(())
    } else {
        Err(TerrainError::InvalidBounds)
    }
}
fn span(first: i32, count: usize, length: usize) -> Result<usize, TerrainError> {
    let first = usize::try_from(first).map_err(|_| TerrainError::InvalidReference)?;
    if first.checked_add(count).is_some_and(|end| end <= length) {
        Ok(first)
    } else {
        Err(TerrainError::InvalidReference)
    }
}

/// Reads only the declared exchange, with checked fixed/header and graph limits.
/// Requested retained and validation-vector capacities are bounded; allocator
/// metadata/capacity rounding and host implementation of Read are outside this cap.
pub fn read_terrain_snapshot<R: Read>(
    reader: &mut R,
    length: u64,
) -> Result<TerrainSnapshot, TerrainError> {
    if length > MAX_BYTES {
        return Err(TerrainError::LimitExceeded);
    }
    let mut input = Input {
        reader,
        interrupted: 0,
    };
    let header = input.bytes::<HEADER_BYTES>()?;
    if header[..8] != MAGIC {
        return Err(TerrainError::UnsupportedMagic);
    }
    let word = |start| {
        u32::from_le_bytes(
            header[start..start + 4]
                .try_into()
                .expect("fixed header slice"),
        )
    };
    let wide = |start| {
        u64::from_le_bytes(
            header[start..start + 8]
                .try_into()
                .expect("fixed header slice"),
        )
    };
    if word(8) != VERSION {
        return Err(TerrainError::UnsupportedVersion);
    }
    if word(12) != 0 || header[136..].iter().any(|b| *b != 0) {
        return Err(TerrainError::NonzeroReserved);
    }
    let identity = TerrainIdentity {
        source: Identity {
            bytes: wide(24),
            sha256: header[32..64].try_into().expect("fixed header hash"),
        },
        membership: Identity {
            bytes: wide(64),
            sha256: header[72..104].try_into().expect("fixed header hash"),
        },
    };
    let mut counts = [0_usize; 8];
    for (i, count) in counts.iter_mut().enumerate() {
        *count = word(104 + i * 4) as usize;
    }
    let [
        materials_count,
        aabbs_count,
        partitions_count,
        triangles_count,
        vertices_count,
        edges_count,
        leaves_count,
        models_count,
    ] = counts;
    if counts
        .iter()
        .enumerate()
        .any(|(i, c)| i != 5 && *c > MAX_RECORDS)
        || materials_count > 65_536
        || vertices_count > 65_536
        || models_count > 4096
        || leaves_count > 65_535
    {
        return Err(TerrainError::LimitExceeded);
    }
    let packed_size = triangles_count
        .checked_mul(3)
        .and_then(|n| n.checked_add(31))
        .map(|n| n / 32 * 4)
        .ok_or(TerrainError::ArithmeticOverflow)?;
    if edges_count != packed_size {
        return Err(TerrainError::PackedEdgeLength);
    }
    let mut expected = HEADER_BYTES;
    for (count, size) in counts.into_iter().zip([8_usize, 32, 16, 6, 12, 1, 44, 80]) {
        expected = expected
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or(TerrainError::ArithmeticOverflow)?,
            )
            .ok_or(TerrainError::ArithmeticOverflow)?;
    }
    if wide(16) != length || expected as u64 != length {
        return Err(TerrainError::LengthMismatch);
    }
    let allocation_sizes = [
        size_of::<TerrainMaterial>(),
        size_of::<TerrainAabb>(),
        size_of::<TerrainPartition>(),
        size_of::<[u16; 3]>(),
        size_of::<[u32; 3]>(),
        1,
        size_of::<TerrainLeaf>(),
        size_of::<TerrainModel>(),
    ];
    let mut allocation = aabbs_count
        .checked_mul(size_of::<u8>() + size_of::<(usize, usize)>())
        .ok_or(TerrainError::ArithmeticOverflow)?;
    for (count, size) in counts.into_iter().zip(allocation_sizes) {
        allocation = allocation
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or(TerrainError::ArithmeticOverflow)?,
            )
            .ok_or(TerrainError::ArithmeticOverflow)?;
    }
    if allocation > MAX_ALLOCATION_BYTES {
        return Err(TerrainError::LimitExceeded);
    }
    let mut result = TerrainSnapshot {
        identity,
        materials: vector(materials_count)?,
        aabbs: vector(aabbs_count)?,
        partitions: vector(partitions_count)?,
        triangles: vector(triangles_count)?,
        vertices: vector(vertices_count)?,
        packed_edges: vector(edges_count)?,
        map_leaves: vector(leaves_count)?,
        models: vector(models_count)?,
    };
    for _ in 0..materials_count {
        result.materials.push(TerrainMaterial {
            surface_flags: input.u32()?,
            contents: input.u32()?,
        });
    }
    let mut references = 0_usize;
    for _ in 0..aabbs_count {
        let aabb = TerrainAabb {
            origin_bits: input.triple()?,
            material_index: input.u16()?,
            child_count: input.u16()?,
            halfsize_bits: input.triple()?,
            reference: input.i32()?,
        };
        finite(aabb.origin_bits)?;
        finite(aabb.halfsize_bits)?;
        if aabb
            .halfsize_bits
            .into_iter()
            .any(|v| f32::from_bits(v) < 0.0)
        {
            return Err(TerrainError::InvalidBounds);
        }
        if usize::from(aabb.material_index) >= materials_count {
            return Err(TerrainError::InvalidReference);
        }
        if aabb.child_count == 0 {
            span(aabb.reference, 1, partitions_count)?;
        } else {
            span(aabb.reference, usize::from(aabb.child_count), aabbs_count)?;
        }
        references = references
            .checked_add(usize::from(aabb.child_count))
            .ok_or(TerrainError::ArithmeticOverflow)?;
        if references > MAX_CHILD_REFERENCES {
            return Err(TerrainError::LimitExceeded);
        }
        result.aabbs.push(aabb);
    }
    for _ in 0..partitions_count {
        let triangle_count = input.bytes::<1>()?[0];
        if input.bytes::<3>()? != [0; 3] {
            return Err(TerrainError::NonzeroReserved);
        }
        let partition = TerrainPartition {
            triangle_count,
            first_triangle: input.i32()?,
            nuinds: input.i32()?,
            fuind: input.i32()?,
        };
        span(
            partition.first_triangle,
            usize::from(partition.triangle_count),
            triangles_count,
        )?;
        result.partitions.push(partition);
    }
    for _ in 0..triangles_count {
        let triangle = [input.u16()?, input.u16()?, input.u16()?];
        if triangle
            .into_iter()
            .any(|v| usize::from(v) >= vertices_count)
        {
            return Err(TerrainError::InvalidReference);
        }
        result.triangles.push(triangle);
    }
    for _ in 0..vertices_count {
        let vertex = input.triple()?;
        finite(vertex)?;
        result.vertices.push(vertex);
    }
    for _ in 0..edges_count {
        result.packed_edges.push(input.bytes::<1>()?[0]);
    }
    for _ in 0..leaves_count {
        let leaf = input.leaf()?;
        admit_leaf(&leaf, aabbs_count)?;
        result.map_leaves.push(leaf);
    }
    for ordinal in 0..models_count {
        let encoded_ordinal = input.u32()?;
        if encoded_ordinal as usize != ordinal {
            return Err(TerrainError::InvalidModelOrdinal);
        }
        let present = input.u32()?;
        if present > 1 {
            return Err(TerrainError::InvalidPresence);
        }
        let model = TerrainModel {
            ordinal: encoded_ordinal,
            serialized_info_present: present == 1,
            mins_bits: input.triple()?,
            maxs_bits: input.triple()?,
            radius_bits: input.u32()?,
            leaf: input.leaf()?,
        };
        bounds(model.mins_bits, model.maxs_bits)?;
        let radius = f32::from_bits(model.radius_bits);
        if !radius.is_finite() {
            return Err(TerrainError::NonFinite);
        }
        if radius < 0.0 {
            return Err(TerrainError::InvalidBounds);
        }
        admit_leaf(&model.leaf, aabbs_count)?;
        result.models.push(model);
    }
    admit_graph(&result.aabbs)?;
    Ok(result)
}
fn admit_leaf(leaf: &TerrainLeaf, aabbs: usize) -> Result<(), TerrainError> {
    bounds(leaf.mins_bits, leaf.maxs_bits)?;
    span(
        i32::from(leaf.first_aabb),
        usize::from(leaf.aabb_count),
        aabbs,
    )?;
    Ok(())
}
fn admit_graph(aabbs: &[TerrainAabb]) -> Result<(), TerrainError> {
    let mut colors = vector::<u8>(aabbs.len())?;
    colors.resize(aabbs.len(), 0);
    let mut stack = vector::<(usize, usize)>(aabbs.len())?;
    for root in 0..aabbs.len() {
        if colors[root] != 0 {
            continue;
        }
        colors[root] = 1;
        stack.push((root, 0));
        while let Some((node, next)) = stack.last_mut() {
            let aabb = &aabbs[*node];
            if *next == usize::from(aabb.child_count) {
                colors[*node] = 2;
                stack.pop();
                continue;
            }
            let child = aabb.reference as usize + *next;
            *next += 1;
            match colors[child] {
                1 => return Err(TerrainError::Cycle),
                0 => {
                    colors[child] = 1;
                    stack.push((child, 0));
                }
                _ => {}
            }
        }
    }
    Ok(())
}
