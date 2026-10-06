//! Bounded hierarchy exchange. Source identities are declarations, not authentication.
//! Terrain owns leaf/model records; this module preserves only ordered global hierarchy.
use crate::{
    brush_collection::{BrushCollectionSnapshot, Identity},
    terrain_snapshot::TerrainSnapshot,
};
use std::{
    fmt,
    io::{self, Read},
    mem::size_of,
};

pub const MAGIC: [u8; 8] = *b"BO2HIER\0";
pub const VERSION: u32 = 1;
pub const HEADER_BYTES: usize = 216;
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_INDICES: usize = 4_000_000;
pub const MAX_ALLOCATION_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyError {
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
    InvalidReference,
    InvalidAxis,
    InvalidPlaneMetadata,
    InvalidFlags,
    InvalidBoxMetadata,
    Cycle,
    IdentityMismatch,
    MissingBrush,
    ConflictingPlane,
}
impl fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "collision hierarchy rejected ({self:?})")
    }
}
impl std::error::Error for HierarchyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HierarchyIdentity {
    pub source: Identity,
    pub membership: Identity,
    pub brush_payload: Identity,
    pub terrain_payload: Identity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionPlane {
    pub normal_bits: [u32; 3],
    pub distance_bits: u32,
    pub kind: u8,
    pub sign_bits: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrushNode {
    pub axis: u8,
    pub external_terminal: bool,
    pub inert_box_terminal: bool,
    pub count: i16,
    pub contents: u32,
    pub distance_bits: u32,
    pub range_bits: u32,
    pub child_offsets: [u16; 2],
    pub terminal_first: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BspNode {
    pub plane: u32,
    /// Nonnegative node index; negative encodes leaf index as -1-child.
    pub children: [i16; 2],
}
pub struct CollisionHierarchy {
    identity: HierarchyIdentity,
    planes: Vec<CollisionPlane>,
    brush_nodes: Vec<BrushNode>,
    brush_indices: Vec<u16>,
    bsp_nodes: Vec<BspNode>,
    original_pool_count: usize,
    brush_count: usize,
    leaf_count: usize,
}
impl fmt::Debug for CollisionHierarchy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CollisionHierarchy")
            .field("planes", &self.planes.len())
            .field("brush_nodes", &self.brush_nodes.len())
            .field("brush_indices", &self.brush_indices.len())
            .field("bsp_nodes", &self.bsp_nodes.len())
            .finish_non_exhaustive()
    }
}
impl CollisionHierarchy {
    pub fn identity(&self) -> HierarchyIdentity {
        self.identity
    }
    pub fn check_identity(&self, expected: HierarchyIdentity) -> Result<(), HierarchyError> {
        if self.identity == expected {
            Ok(())
        } else {
            Err(HierarchyError::IdentityMismatch)
        }
    }
    pub fn planes(&self) -> &[CollisionPlane] {
        &self.planes
    }
    pub fn brush_nodes(&self) -> &[BrushNode] {
        &self.brush_nodes
    }
    pub fn brush_indices(&self) -> &[u16] {
        &self.brush_indices
    }
    pub fn bsp_nodes(&self) -> &[BspNode] {
        &self.bsp_nodes
    }
    pub fn original_pool_count(&self) -> usize {
        self.original_pool_count
    }
    pub fn declared_brush_count(&self) -> usize {
        self.brush_count
    }
    pub fn declared_leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Checks shared source declarations and roots, not the actual terrain file hash.
    /// The scene owner must independently bind the actual payload to terrain_payload.
    pub fn validate_terrain(&self, terrain: &TerrainSnapshot) -> Result<(), HierarchyError> {
        let id = terrain.identity();
        if id.source != self.identity.source || id.membership != self.identity.membership {
            return Err(HierarchyError::IdentityMismatch);
        }
        if terrain.map_leaves().len() != self.leaf_count {
            return Err(HierarchyError::InvalidReference);
        }
        for leaf in terrain
            .map_leaves()
            .iter()
            .chain(terrain.models().iter().map(|m| &m.leaf))
        {
            let root =
                usize::try_from(leaf.brush_root).map_err(|_| HierarchyError::InvalidReference)?;
            if root >= self.brush_nodes.len() {
                return Err(HierarchyError::InvalidReference);
            }
        }
        Ok(())
    }
    /// Checks every participating global terminal against existing provider0 geometry.
    /// Payload-file hashes remain the scene owner's independently bound responsibility.
    pub fn validate_brush_collection(
        &self,
        brushes: &BrushCollectionSnapshot,
    ) -> Result<(), HierarchyError> {
        let id = brushes.declared_identity();
        if id.source != self.identity.source || id.membership != self.identity.membership {
            return Err(HierarchyError::IdentityMismatch);
        }
        let mut available = vector(self.brush_count)?;
        available.resize(self.brush_count, false);
        for record in brushes.records() {
            let key = record.key();
            if key.provider_id == 0
                && let Some(slot) = available.get_mut(key.brush_id as usize)
            {
                *slot = true;
            }
            if key.provider_id == 0 {
                for side in record.snapshot().sides() {
                    let plane = self
                        .planes
                        .get(side.plane_index as usize)
                        .ok_or(HierarchyError::InvalidReference)?;
                    if plane.normal_bits != side.normal_bits
                        || plane.distance_bits != side.dist_bits
                        || plane.kind != side.plane_type
                        || plane.sign_bits != side.signbits
                    {
                        return Err(HierarchyError::ConflictingPlane);
                    }
                }
            }
        }
        for node in &self.brush_nodes {
            if node.count <= 0 || node.contents == 0 {
                continue;
            }
            let first = node.terminal_first as usize;
            for &brush_id in &self.brush_indices[first..first + node.count as usize] {
                if !available[usize::from(brush_id)] {
                    return Err(HierarchyError::MissingBrush);
                }
            }
        }
        Ok(())
    }
}
struct Input<'a, R> {
    reader: &'a mut R,
    interrupted: usize,
}
impl<R: Read> Input<'_, R> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], HierarchyError> {
        let mut bytes = [0; N];
        let mut at = 0;
        while at < N {
            match self.reader.read(&mut bytes[at..]) {
                Ok(0) => return Err(HierarchyError::Truncated),
                Ok(n) if n <= N - at => at += n,
                Ok(_) => return Err(HierarchyError::InvalidReadCount),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    self.interrupted += 1;
                    if self.interrupted > 1024 {
                        return Err(HierarchyError::InterruptedReadLimit);
                    }
                }
                Err(e) => return Err(HierarchyError::Io(e.kind())),
            }
        }
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32, HierarchyError> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn u16(&mut self) -> Result<u16, HierarchyError> {
        Ok(u16::from_le_bytes(self.bytes()?))
    }
    fn triple(&mut self) -> Result<[u32; 3], HierarchyError> {
        Ok([self.u32()?, self.u32()?, self.u32()?])
    }
}
fn vector<T>(count: usize) -> Result<Vec<T>, HierarchyError> {
    let mut v = Vec::new();
    v.try_reserve_exact(count)
        .map_err(|_| HierarchyError::AllocationFailed)?;
    Ok(v)
}
fn graph(
    count: usize,
    mut children: impl FnMut(usize) -> ([usize; 3], usize),
) -> Result<(), HierarchyError> {
    let mut color = vector(count)?;
    color.resize(count, 0_u8);
    let mut stack = vector::<(usize, usize)>(count)?;
    for root in 0..count {
        if color[root] != 0 {
            continue;
        }
        color[root] = 1;
        stack.push((root, 0));
        while let Some(&(node, next)) = stack.last() {
            let (edges, n) = children(node);
            if next == n {
                color[node] = 2;
                stack.pop();
                continue;
            }
            let child = edges[next];
            stack.last_mut().expect("nonempty stack").1 += 1;
            if child >= count {
                return Err(HierarchyError::InvalidReference);
            }
            if color[child] == 1 {
                return Err(HierarchyError::Cycle);
            }
            if color[child] == 0 {
                color[child] = 1;
                stack.push((child, 0));
            }
        }
    }
    Ok(())
}
pub fn read_collision_hierarchy<R: Read>(
    reader: &mut R,
    length: u64,
) -> Result<CollisionHierarchy, HierarchyError> {
    if length > MAX_BYTES {
        return Err(HierarchyError::LimitExceeded);
    }
    let mut input = Input {
        reader,
        interrupted: 0,
    };
    let h = input.bytes::<HEADER_BYTES>()?;
    if h[..8] != MAGIC {
        return Err(HierarchyError::UnsupportedMagic);
    }
    let word = |p| u32::from_le_bytes(h[p..p + 4].try_into().expect("fixed header word"));
    let wide = |p| u64::from_le_bytes(h[p..p + 8].try_into().expect("fixed header wide"));
    if word(8) != VERSION {
        return Err(HierarchyError::UnsupportedVersion);
    }
    if word(12) != 0 || word(212) != 0 {
        return Err(HierarchyError::NonzeroReserved);
    }
    let id = |p| Identity {
        bytes: wide(p),
        sha256: h[p + 8..p + 40].try_into().expect("fixed hash"),
    };
    let identity = HierarchyIdentity {
        source: id(24),
        membership: id(64),
        brush_payload: id(104),
        terrain_payload: id(144),
    };
    let counts = [
        word(184) as usize,
        word(188) as usize,
        word(192) as usize,
        word(196) as usize,
    ];
    let [pn, nn, ni, bn] = counts;
    let brush_count = word(200) as usize;
    let leaf_count = word(204) as usize;
    let original_pool_count = word(208) as usize;
    if pn > MAX_RECORDS
        || nn > MAX_RECORDS
        || ni > MAX_INDICES
        || bn > 32768
        || brush_count > 65535
        || leaf_count > 32768
    {
        return Err(HierarchyError::LimitExceeded);
    }
    if original_pool_count > ni {
        return Err(HierarchyError::InvalidReference);
    }
    let mut expected = HEADER_BYTES;
    let mut allocation = 0_usize;
    for ((count, size), alloc) in counts.into_iter().zip([20, 24, 2, 8]).zip([
        size_of::<CollisionPlane>(),
        size_of::<BrushNode>() + size_of::<u8>() + size_of::<(usize, usize)>(),
        2,
        size_of::<BspNode>() + size_of::<u8>() + size_of::<(usize, usize)>(),
    ]) {
        expected = expected
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or(HierarchyError::ArithmeticOverflow)?,
            )
            .ok_or(HierarchyError::ArithmeticOverflow)?;
        allocation = allocation
            .checked_add(
                count
                    .checked_mul(alloc)
                    .ok_or(HierarchyError::ArithmeticOverflow)?,
            )
            .ok_or(HierarchyError::ArithmeticOverflow)?;
    }
    if expected as u64 != length || wide(16) != length {
        return Err(HierarchyError::LengthMismatch);
    }
    if allocation > MAX_ALLOCATION_BYTES {
        return Err(HierarchyError::LimitExceeded);
    }
    let mut result = CollisionHierarchy {
        identity,
        planes: vector(pn)?,
        brush_nodes: vector(nn)?,
        brush_indices: vector(ni)?,
        bsp_nodes: vector(bn)?,
        original_pool_count,
        brush_count,
        leaf_count,
    };
    for _ in 0..pn {
        let normal_bits = input.triple()?;
        let distance_bits = input.u32()?;
        let [kind, sign_bits] = input.bytes::<2>()?;
        if input.u16()? != 0 {
            return Err(HierarchyError::NonzeroReserved);
        }
        if sign_bits > 7 {
            return Err(HierarchyError::InvalidPlaneMetadata);
        }
        if normal_bits
            .into_iter()
            .chain([distance_bits])
            .any(|b| !f32::from_bits(b).is_finite())
        {
            return Err(HierarchyError::NonFinite);
        }
        result.planes.push(CollisionPlane {
            normal_bits,
            distance_bits,
            kind,
            sign_bits,
        });
    }
    for i in 0..nn {
        let [axis, flags] = input.bytes::<2>()?;
        let count = i16::from_le_bytes(input.bytes()?);
        let contents = input.u32()?;
        let distance_bits = input.u32()?;
        let range_bits = input.u32()?;
        let child_offsets = [input.u16()?, input.u16()?];
        let terminal_first = input.u32()?;
        if flags & !3 != 0 || flags == 2 {
            return Err(HierarchyError::InvalidFlags);
        }
        if count <= 0 && axis > 2 {
            return Err(HierarchyError::InvalidAxis);
        }
        if count > 0 {
            let first = terminal_first as usize;
            let end = first
                .checked_add(count as usize)
                .ok_or(HierarchyError::ArithmeticOverflow)?;
            if end > ni
                || (flags & 1 == 0 && end > original_pool_count)
                || (flags & 1 != 0 && first < original_pool_count)
            {
                return Err(HierarchyError::InvalidReference);
            }
            if distance_bits != 0 || range_bits != 0 || child_offsets != [0, 0] {
                return Err(HierarchyError::NonzeroReserved);
            }
        } else {
            if flags != 0 || terminal_first != 0 {
                return Err(HierarchyError::InvalidFlags);
            }
            if !f32::from_bits(distance_bits).is_finite() || !f32::from_bits(range_bits).is_finite()
            {
                return Err(HierarchyError::NonFinite);
            }
            if child_offsets.into_iter().any(|c| i + usize::from(c) >= nn)
                || (count < 0 && i + 1 >= nn)
            {
                return Err(HierarchyError::InvalidReference);
            }
        }
        if flags & 2 != 0 && (i + 1 != nn || count != 1 || contents != 0) {
            return Err(HierarchyError::InvalidBoxMetadata);
        }
        result.brush_nodes.push(BrushNode {
            axis,
            external_terminal: flags & 1 != 0,
            inert_box_terminal: flags & 2 != 0,
            count,
            contents,
            distance_bits,
            range_bits,
            child_offsets,
            terminal_first,
        });
    }
    for i in 0..ni {
        let index = input.u16()?;
        let inert_box_index = result
            .brush_nodes
            .last()
            .filter(|n| n.inert_box_terminal)
            .map(|n| n.terminal_first as usize);
        if usize::from(index) >= brush_count
            && !(usize::from(index) == brush_count && inert_box_index == Some(i))
        {
            return Err(HierarchyError::InvalidReference);
        }
        result.brush_indices.push(index);
    }
    for node in &result.brush_nodes {
        if node.count <= 0 {
            continue;
        }
        let first = node.terminal_first as usize;
        if node.inert_box_terminal {
            if usize::from(result.brush_indices[first]) != brush_count {
                return Err(HierarchyError::InvalidBoxMetadata);
            }
        } else if result.brush_indices[first..first + node.count as usize]
            .iter()
            .any(|b| usize::from(*b) >= brush_count)
        {
            return Err(HierarchyError::InvalidReference);
        }
    }
    for _ in 0..bn {
        let plane = input.u32()?;
        let children = [
            i16::from_le_bytes(input.bytes()?),
            i16::from_le_bytes(input.bytes()?),
        ];
        if plane as usize >= pn
            || children.into_iter().any(|c| {
                if c >= 0 {
                    c as usize >= bn
                } else {
                    (-1 - i32::from(c)) as usize >= leaf_count
                }
            })
        {
            return Err(HierarchyError::InvalidReference);
        }
        result.bsp_nodes.push(BspNode { plane, children });
    }
    graph(nn, |i| {
        let n = &result.brush_nodes[i];
        if n.contents == 0 || n.count > 0 {
            ([0; 3], 0)
        } else {
            (
                [
                    i + usize::from(n.child_offsets[0]),
                    i + usize::from(n.child_offsets[1]),
                    i + 1,
                ],
                if n.count < 0 { 3 } else { 2 },
            )
        }
    })?;
    graph(bn, |i| {
        let mut edges = [0; 3];
        let mut n = 0;
        for c in result.bsp_nodes[i].children {
            if c >= 0 {
                edges[n] = c as usize;
                n += 1;
            }
        }
        (edges, n)
    })?;
    Ok(result)
}
