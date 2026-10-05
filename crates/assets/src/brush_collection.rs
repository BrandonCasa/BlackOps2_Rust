//! Project-owned schema34 exchange. Hashes are declarations until the caller
//! compares them with independently obtained identities. No original decoder.
use crate::brush_snapshot::{BrushError, BrushSnapshot, RawBrushSnapshot, read_bounded};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    io::Read,
};

pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_RECORDS: usize = 65_535;
pub const MAX_AGGREGATE: usize = 1_000_000;
pub const INTERPRETATION: &str = "diagnostic_relative_negative_adjacent_zero_empty";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceKey {
    pub provider_id: u32,
    pub brush_id: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub bytes: u64,
    pub sha256: [u8; 32],
}
impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Identity { .. }")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionIdentity {
    pub source: Identity,
    pub membership: Identity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionError {
    Brush(BrushError),
    UnsupportedSchema,
    LimitExceeded,
    InvalidIdentity,
    IdentityMismatch,
    UnsupportedInterpretation,
    InvalidCoverage,
    DuplicateKey,
    SelectionMismatch,
    InvalidReference,
    ConflictingReference,
    AllocationFailed,
}
impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "brush collection rejected ({self:?})")
    }
}
impl std::error::Error for CollectionError {}
impl From<BrushError> for CollectionError {
    fn from(e: BrushError) -> Self {
        Self::Brush(e)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub id: u32,
    pub planes: u32,
    pub sides: u32,
    pub vertices: u32,
    pub brushes: u32,
    pub optional_bounds_present: bool,
    pub optional_contents_present: bool,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub models: u32,
    pub absent_model_providers: u32,
    pub box_excluded: bool,
    pub triangles: u32,
    pub partitions: u32,
    pub aabb_trees: u32,
    pub static_models: u32,
    pub dynents: [u32; 4],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIdentity {
    bytes: u64,
    sha256: String,
    membership_bytes: u64,
    membership_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    status: String,
    reached_nodes: u32,
    keys: Vec<SourceKey>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRecord {
    provider_id: u32,
    brush: RawBrushSnapshot,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCollection {
    schema: u32,
    interpretation: String,
    source: RawIdentity,
    selection: Selection,
    coverage: Coverage,
    providers: Vec<Provider>,
    record_count: usize,
    records: Vec<RawRecord>,
}

#[derive(Debug)]
pub struct BrushRecord {
    key: SourceKey,
    snapshot: BrushSnapshot,
}
impl BrushRecord {
    pub fn key(&self) -> SourceKey {
        self.key
    }
    pub fn snapshot(&self) -> &BrushSnapshot {
        &self.snapshot
    }
}

#[derive(Debug)]
pub struct BrushCollectionSnapshot {
    identity: CollectionIdentity,
    selection: Vec<SourceKey>,
    reached_nodes: u32,
    coverage: Coverage,
    providers: Vec<Provider>,
    records: Vec<BrushRecord>,
}
impl BrushCollectionSnapshot {
    pub fn declared_identity(&self) -> CollectionIdentity {
        self.identity
    }
    /// Explicit binding to caller identities; it does not read/authenticate originals.
    pub fn check_identity(&self, expected: CollectionIdentity) -> Result<(), CollectionError> {
        if self.identity == expected {
            Ok(())
        } else {
            Err(CollectionError::IdentityMismatch)
        }
    }
    pub fn selected_keys(&self) -> &[SourceKey] {
        &self.selection
    }
    pub fn reached_nodes(&self) -> u32 {
        self.reached_nodes
    }
    pub fn coverage(&self) -> &Coverage {
        &self.coverage
    }
    pub fn providers(&self) -> &[Provider] {
        &self.providers
    }
    pub fn records(&self) -> &[BrushRecord] {
        &self.records
    }
}

fn identity(bytes: u64, text: &str) -> Result<Identity, CollectionError> {
    if bytes == 0 || text.len() != 64 {
        return Err(CollectionError::InvalidIdentity);
    }
    let mut sha256 = [0; 32];
    for (out, pair) in sha256.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let digit = |c| match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(CollectionError::InvalidIdentity),
        };
        *out = digit(pair[0])? * 16 + digit(pair[1])?;
    }
    Ok(Identity { bytes, sha256 })
}

/// Byte framing/retry limits are shared with schema28. JSON recursion remains
/// bounded. A 32MiB byte cap bounds temporary allocations before count admission;
/// aggregate limits bound admitted owned geometry. Every record must be selected.
pub fn read_brush_collection(
    reader: &mut impl Read,
    expected_bytes: u64,
) -> Result<BrushCollectionSnapshot, CollectionError> {
    let bytes = read_bounded(reader, expected_bytes, MAX_BYTES)?;
    let raw: RawCollection = serde_json::from_slice(&bytes)
        .map_err(|_| CollectionError::Brush(BrushError::InvalidJson))?;
    if raw.schema != 34 {
        return Err(CollectionError::UnsupportedSchema);
    }
    if raw.interpretation != INTERPRETATION || raw.selection.status != "admitted" {
        return Err(CollectionError::UnsupportedInterpretation);
    }
    if raw.record_count > MAX_RECORDS
        || raw.providers.len() > 16
        || raw.providers.is_empty()
        || raw.selection.reached_nodes > 100_000
        || raw.coverage.models > 4096
    {
        return Err(CollectionError::LimitExceeded);
    }
    if raw.record_count != raw.records.len() || raw.record_count != raw.selection.keys.len() {
        return Err(CollectionError::SelectionMismatch);
    }
    if !raw.coverage.box_excluded || raw.coverage.absent_model_providers > raw.coverage.models {
        return Err(CollectionError::InvalidCoverage);
    }
    let declared = CollectionIdentity {
        source: identity(raw.source.bytes, &raw.source.sha256)?,
        membership: identity(raw.source.membership_bytes, &raw.source.membership_sha256)?,
    };
    let mut provider_ids = BTreeSet::new();
    for p in &raw.providers {
        if !provider_ids.insert(p.id) {
            return Err(CollectionError::DuplicateKey);
        }
        if p.brushes as usize > MAX_RECORDS
            || [p.planes, p.sides, p.vertices]
                .into_iter()
                .any(|n| n as usize > MAX_AGGREGATE)
        {
            return Err(CollectionError::LimitExceeded);
        }
    }
    let mut selected = BTreeSet::new();
    for &key in &raw.selection.keys {
        if !selected.insert(key) {
            return Err(CollectionError::DuplicateKey);
        }
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(raw.record_count)
        .map_err(|_| CollectionError::AllocationFailed)?;
    let (mut sides, mut vertices) = (0usize, 0usize);
    let mut source_sides = BTreeMap::new();
    let mut source_planes = BTreeMap::new();
    let mut source_vertices = BTreeMap::new();
    for entry in raw.records {
        // Schema34 explicitly permits zero sides; schema28 still requires one.
        let snapshot = entry.brush.admit(34, 0)?;
        let key = SourceKey {
            provider_id: entry.provider_id,
            brush_id: snapshot.brush_index(),
        };
        if !selected.remove(&key) {
            return Err(CollectionError::SelectionMismatch);
        }
        let p = raw
            .providers
            .iter()
            .find(|p| p.id == key.provider_id)
            .ok_or(CollectionError::InvalidReference)?;
        if key.brush_id >= p.brushes
            || snapshot
                .side_first()
                .checked_add(snapshot.sides().len() as u32)
                .is_none_or(|n| n > p.sides)
            || snapshot
                .vertex_first()
                .checked_add(snapshot.vertices_bits().len() as u32)
                .is_none_or(|n| n > p.vertices)
            || snapshot.sides().iter().any(|s| s.plane_index >= p.planes)
        {
            return Err(CollectionError::InvalidReference);
        }
        if snapshot.midpoint_bits().is_some() != p.optional_bounds_present
            || snapshot.parallel_contents().is_some() != p.optional_contents_present
        {
            return Err(CollectionError::InvalidCoverage);
        }
        sides = sides
            .checked_add(snapshot.sides().len())
            .ok_or(CollectionError::LimitExceeded)?;
        vertices = vertices
            .checked_add(snapshot.vertices_bits().len())
            .ok_or(CollectionError::LimitExceeded)?;
        if sides > MAX_AGGREGATE || vertices > MAX_AGGREGATE {
            return Err(CollectionError::LimitExceeded);
        }
        // A shared source-pool reference denotes one exact ordered raw record.
        // These maps retain at most the charged aggregate, never query-time work.
        for (offset, side) in snapshot.sides().iter().enumerate() {
            let index = (key.provider_id, snapshot.side_first() + offset as u32);
            if source_sides
                .insert(index, side.clone())
                .is_some_and(|old| old != *side)
            {
                return Err(CollectionError::ConflictingReference);
            }
            let plane = (
                side.normal_bits,
                side.dist_bits,
                side.plane_type,
                side.signbits,
            );
            if source_planes
                .insert((key.provider_id, side.plane_index), plane)
                .is_some_and(|old| old != plane)
            {
                return Err(CollectionError::ConflictingReference);
            }
        }
        for (offset, &vertex) in snapshot.vertices_bits().iter().enumerate() {
            let index = (key.provider_id, snapshot.vertex_first() + offset as u32);
            if source_vertices
                .insert(index, vertex)
                .is_some_and(|old| old != vertex)
            {
                return Err(CollectionError::ConflictingReference);
            }
        }
        records.push(BrushRecord { key, snapshot });
    }
    if !selected.is_empty() {
        return Err(CollectionError::SelectionMismatch);
    }
    Ok(BrushCollectionSnapshot {
        identity: declared,
        selection: raw.selection.keys,
        reached_nodes: raw.selection.reached_nodes,
        coverage: raw.coverage,
        providers: raw.providers,
        records,
    })
}
