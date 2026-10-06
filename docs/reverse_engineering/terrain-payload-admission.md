# Terrain exchange v1: complete payload admission

Movement terrain task 02, 2026-10-05. This implements bounded external payload
access following the [qualified family](terrain-trace-contract.md). It establishes
complete declared array/reference admission, not runtime ownership, active poses,
native numerical queries or player/full-map fidelity. Source/data access remains
**INVESTIGATING**; gameplay remains **UNKNOWN**.

## Recovery and identities

An owned external copy of the retained task0034 OAT ContentPrinter/build/supervisor
accesses one decoded T6 Origins clipmap. It emits only ordered metadata and exact
collision words; material names, original scripts, host pointers and other assets
are not emitted. The original decoder/helpers and schemas33/34 were not modified.
No original executable or game/process was opened. Only the owned OAT decoder
executable processed one licensed fastfile read-only.

Input fastfile: 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
Its size/hash and same-handle identity were checked before/after decoding and agree.
Retained membership: 2,634,532 bytes, SHA256
`6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3`.
All 4,791 map leaf and 265 model leaf scalar fields agree with that membership.

Recovered terrain exchange: **3,084,512 bytes**, SHA256
`e5ecb51a076898aeae66cfb6fd38ef79218768c29d3089279303e39801315a67`.
The binary and source/build/recovery receipts remain outside the repository under
the configured inspection root's `movement-terrain-02` task directory. Reproduction
uses its preserved private producer, `build.py`, `recover.py` and copied bounded
supervisor; output uses exclusive creation in a fresh owned run directory.
The old producer/decoder/helpers were rehashed after recovery and remain unchanged.

| Ordered array | Complete count |
| --- | ---: |
| Collision materials | 509 |
| Collision AABBs | 51,129 |
| Collision partitions | 18,968 |
| Triangle triplets | 67,196 |
| Collision vertices | 40,025 |
| Packed edge bytes | 25,200 |
| Map leaves | 4,791 |
| Models with embedded leaves | 265 |

Every AABB material index resolves; no descendant sentinel relaxation was needed.
Every child/partition/triangle/vertex/leaf span resolves and the complete AABB graph
is acyclic, including otherwise unreachable records. The 214 root-zero map terrain
leaves retain 253 AABB references; 27 root-zero model terrain leaves retain 28.
All model serialized-info pointers are absent, retained as presence metadata only;
this does not remove their geometry or assign runtime providers.

## Project-owned little-endian binary format

All counts precede body allocation. No native pointer/padding is serialized.

| Header offset | Bytes | Meaning |
| --- | ---: | --- |
| 0 | 8 | Magic `BO2TERR` followed by zero |
| 8 | 4 | Version 1 |
| 12 | 4 | Schema-owned reserved zero |
| 16 | 8 | Exact total exchange length |
| 24 | 8 | Source bytes |
| 32 | 32 | Declared source SHA256 bytes |
| 64 | 8 | Membership bytes |
| 72 | 32 | Declared membership SHA256 bytes |
| 104 | 32 | Eight u32 counts in the table's order |
| 136 | 8 | Schema-owned reserved zero |

Body follows at byte144 in that same array order:

- Material8: surface-flags u32, contents u32; opaque words unchanged.
- AABB32: origin words3, material u16, child-count u16, halfsize words3,
  signed reference i32. Nonzero children select a contiguous AABB span; zero
  children select one partition. Source order and sharing are retained.
- Partition16: triangle-count u8, schema-owned reserved zeros3, first-triangle i32,
  original `nuinds` i32 and `fuind` i32. The latter two are preserved metadata;
  their semantics/references are not supplied by the qualified consumer.
- Triangle6: three ordered u16 vertex ordinals.
- Vertex12: three exact finite f32 words, source basis.
- Packed edges: exact declared bytes, including padding bits. No interpretation or
  repair of the qualified consumer's partition-local addressing is made.
- Leaf44: first-AABB/count u16, brush/terrain contents u32, minima words3,
  maxima words3, brush-root i32, cluster i16, schema-owned reserved zero u16.
- Model80: ordinal u32, serialized-info presence u32 (0 or1), minima/maxima words3,
  radius word, embedded Leaf44. No host pointer or inferred provider is stored.

## Reader boundaries and rejection

`bo2_assets::terrain_snapshot::read_terrain_snapshot(reader,length)` constructs an
immutable `TerrainSnapshot` only after complete admission. Its slices retain the
source order and exact words; `check_identity(TerrainIdentity)` compares source
and membership declarations against the caller's independently obtained values.
Declarations are not authentication. A scene owner should additionally bind the
actual payload identity, as the retained opt-in test does.

The input cap is64MiB. Requested retained and graph-validation element capacities
are capped at96MiB; allocator metadata/rounding and the caller's Read implementation
are outside that bound. No full input copy or JSON allocations are required.
Each array is capped at1,000,000 records, additionally limited to65,536 materials
and vertices,65,535 map leaves and4,096 models. Aggregate child references are
capped at4,000,000. Graph admission uses an iterative bounded depth-first stack,
so source depth cannot overflow the host call stack. Only1,024 interrupted reads
are tolerated, and impossible read counts reject.

Before vector reservation the reader checks version/magic/reserved fields, all
count/size arithmetic, exact supplied/header/computed length and packed edge size
`4*ceil(3*triangleCount/32)`. During reading it rejects nonfinite floats, negative
halfsize/radius, reversed bounds, invalid spans/material/triangle/vertex references,
model ordinal disorder and invalid presence values. It then rejects cycles across
all AABBs. Shared acyclic nodes and owners are accepted. All errors are typed and
retain no paths, raw invalid values or underlying I/O messages. Normal Debug output
contains counts rather than complete payload arrays. A buffered Read is appropriate
for files because the typed decoder requests small fields.

Brush roots, clusters and unused partition metadata are retained but require
separate scene/global-provider qualification. An empty payload is structurally
admitted and never establishes a complete Origins scene. Degenerate triangles
are not silently removed or reinterpreted; numerical admission belongs to the
future qualified query adapter.

The producer validates the same array graph/count/bounds/reference constraints
before publishing any binary. Its output is capped at64MiB, captured stdout/stderr
at2MiB and execution at120seconds by the copied owned-process supervisor. Input
identity reads are capped at128MiB. These are producer/output limits, not a total
memory cap for the external OAT decoder itself; its peak is recorded externally.

## Actual validation and remaining limits

External producer compilation/linking passes warning-denying C++ flags; its first
actual decode passes all predicates. An initial compile used the wrong declaration
member name and was corrected to `contentFlags` before any decode. Independent
Python admission rechecks the binary layout, all references/cycles, finite/bounds,
all map/model membership fields and complete counts. Source and old-decoder
preservation checks pass.

Six synthetic Rust tests pass: exact order/bits/metadata/identity, every truncated
prefix, malformed format/reference/nonfinite/bounds/presence, referenced and
unreferenced cycles, shared acyclic owners, bounded partial/interrupted/invalid
reads, oversize rejection and structurally empty payloads. The explicit external
opt-in separately admits the full payload, checks all counts/root-zero groups and
exact source/membership declarations and payload SHA256. Strict focused all-target
Clippy and owned-source formatting pass. Final workspace gates belong to primary
integration. No native numerical queries, original comparisons, current transformed
poses, activation or complete scene acceptance is claimed.
