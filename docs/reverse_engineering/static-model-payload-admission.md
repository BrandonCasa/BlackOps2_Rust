# Classic static-model payload admission

Status: INVESTIGATING. This is a complete declared classic point-collision
exchange, not complete motor provider coverage. It implements the bounded data
boundary from [static01](static-xmodel-collision-contract.md). Static sector
construction, point-query numerics, stationary/swept player hulls, and the motor
backend remain separate work.

## Safe Rust boundary

`bo2_assets::static_model_snapshot::read_static_model_snapshot(reader, length)`
returns an immutable `StaticModelSnapshot` or typed `StaticModelError`.
Private construction admits all declared array references, ordering and scalar
fields before publishing the snapshot. Records have read-only slice access;
debug formatting reports counts, not payloads. Reading never changes an existing
snapshot or authoritative gameplay state.

The little-endian project-owned exchange is `BO2STAT\0`, version 1. Header is
144 bytes: magic 8, version 4, reserved 4, total length 8; source identity
8-byte length plus 32-byte SHA256; association-metadata identity of the same
shape; five u32 counts (instances/providers/surfaces/triangles/collmaps); 20
reserved zero bytes. Identities are declarations. `check_identity` compares
caller-supplied expected declarations; callers must authenticate actual inputs
independently. No signature or input authentication is inferred from a header.

| Array | Exchange bytes / record | Preserved fields |
| --- | ---: | --- |
| Instance | 88 | Original ordinal, first-use provider ID, serialized next u16, zero padding u16, contents, exact origin/3x3 inverse-axis/min/max words |
| Provider | 56 | First-use ordinal, contents, signed collLod i16, zero padding u16, first/count surface and collmap spans, source min/max/radius words |
| Surface | 48 | Original ordinal within provider, first/count triangles, source min/max words, signed bone index, contents, surface flags |
| Triangle | 48 | All four plane, s-vector and t-vector words |
| Collmap metadata | 16 | Ordinal within provider, geomList presence, unresolved geomList count, geomList contents |

Source float words and record order are retained exactly, including negative
zero. Negative collLod is valid metadata; the already qualified point consumer
rejects it. Zero contents and degenerate ordered bounds remain valid data and
are not silently omitted. Axis invertibility/orthogonality, triangle area and
plane normalization are not invented admission predicates.

Instance IDs must equal array ordinals; referenced provider IDs must exist and
appear in first-use order without gaps. Every provider participates in at least
one declared instance. Serialized next is checked as a nullable one-based
instance reference and retained only as metadata; it is not traversed or used
as a ready native sector list. Provider surface/collmap spans partition their
flat exchange arrays exactly in provider order. Surface triangle spans partition
the triangle array exactly. Local surface/collmap ordinals must match their
owner-relative ordering. This representation intentionally permits copying
immutable geometry across distinct providers rather than misinterpreting
native pointer aliases as ownership errors.

Collmap geometry is expressly unresolved. Presence/count/contents are scalar
fields from Collmap's referenced PhysGeomList. No PhysGeomInfo orientation,
offset, half-length, type, BrushWrapper, or other per-geom fields are admitted.
An absent list has schema-owned zero count/contents; presence metadata is not a
query provider. Missing participating geometry must still produce an explicit
error when a scene/backend consumer is assembled.

## Bounded reads and allocations

The reader rejects supplied lengths above 64 MiB or below the fixed header
before reading. Count caps: instances/providers 65,535 each, surfaces 65,536,
triangles/collmaps 1,000,000 each. Per-provider collmaps cannot exceed the native
u8 count domain. Cumulative referenced unresolved geometry count is capped at
1,000,000 as metadata, with no associated geometry allocation or traversal.
It checks exact header/computed/supplied length and all arithmetic before
allocation. Requested retained element storage is capped at 96 MiB; allocator
metadata/capacity rounding and caller Read implementation are outside this cap.
Reservations use fallible `try_reserve_exact`. Reads consume only the declared
exchange, reject impossible returned read counts, and permit at most 1,024
Interrupted errors across the entire request. Truncation and I/O failures return
typed errors without publication. All source vectors/triangle words must be
finite; bounds must be ordered and radius finite/nonnegative. Reserved schema
bytes and noncanonical presence values are rejected.

## Complete retained source recovery

The owned external `movement-static-model-02` producer is a copied OAT adapter,
using the same retained decoder revision `7d027e8f89118196713e955b0e11f8404149c54d`.
Only already decoded static instances, their 420 referenced XModels' collision
surfaces/triangles and collmap scalar metadata are emitted. It executes no
original code and accesses no game process. All copyrighted payloads, producer
sources, build products and inspection receipts remain external; no asset names
or host pointers appear in the exchange. The accepted schemas 33/34, terrain
exchange, old decoder and static01 metadata/producer remain unchanged.

Source identity: 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
Static01 association-metadata identity: 3,209,708 bytes, SHA256
`d322619cd3b97284006f0673e1b3fc817cfaf6a41528b34a1e5752213ee8271b`.
Recovery checks both identities and old producer/helper hashes before/after;
the independent Python admission compares every instance/provider association,
pose/bounds word and ordered surface/collmap scalar against static01 metadata.

The producer checks cumulative limits of 65,536 surfaces and 1,000,000 triangles/
unresolved geoms, array counts/alignment/arithmetic, finite copied words,
ordered bounds, reference-only status and expected source counts. It reserves
only computed output within the 64 MiB cap, writes an INVALID staging file,
flushes/closes it and atomically renames on complete publication. The copied
supervisor caps diagnostic streams at 2 MiB, the single payload at 64 MiB and
execution at 120 seconds. Unexpected output or failed predicates reject the run.

Successful payload: **4,760,040 bytes**, SHA256
`a4f4024660d26cf564012c629099d64941b5b47ce77701aa89da62fe50b8fce0`.

- 10,837 ordered instances sharing 420 decoded, nonreference providers.
- 510 ordered surfaces and all 78,296 plane/s/t triangle records.
- Two collmap metadata records, each with one unresolved geom; contents
  536,871,936 and 1,024, respectively.
- All serialized next links zero; all source/provider associations retained.
- 210 zero-content instances retained. Constructor exclusion remains scoped to
  the classic source path and is not a full-map motor exclusion.

Native storage observation: 420 distinct `(surface-array pointer,count)` pairs
and 510 distinct `(triangle-array pointer,count)` pairs, with zero overlapping
nonempty storage intervals in either family. No cross-provider exact/partial
array alias was observed in this retained set. The project exchange would copy
such immutable arrays intentionally if present; its contiguous spans describe
exchange ownership rather than a claim that native allocations never share.

External reproducibility entries: `ContentPrinter.cpp`, `build.py`,
`supervisor_v4.py`, `recover.py` and `recovery-receipt.json` under the configured
inspection root's `movement-static-model-02`. Supply the produced binary through
`BO2_STATIC_MODEL_SNAPSHOT` to opt into the ignored Rust retained-admission test.
Local paths are deliberately absent from tracked documentation.

## Validation and remaining limits

Six synthetic tests passed: sharing/order/negative LOD/word preservation;
malformed references/ordinals/spans/padding/presence; finite/bounds/radius/atomic
failure; every truncated prefix/exact lengths/trailing-byte preservation;
partial/invalid/error/Interrupted Read implementations; all family count caps
and empty exchange admission. Synthetic duplicated triangle records demonstrate
that immutable geometry duplication does not erase instance provider sharing.

The separate opt-in test passed against the complete external payload, checking
the exact whole-file SHA256, identities, every admitted reference, counts,
contents/LOD distributions, metadata and all source float words through the
reader. Owned Rust files pass formatting; focused warning-denying validation
is recorded in the handoff. Original comparison fidelity remains INVESTIGATING.
No Rust numerical collision query, runtime sector constructor, active scene,
motor geometry binding, collmap geometry, camera or controller was implemented.
