# Diagnostic geometry snapshot v1

Task 0012 defines a **project-owned diagnostic exchange format**, not a discovered
BO2 disk format, authenticated source, complete asset architecture or renderable
game asset. The std-only consumer lives in `bo2-assets`; `bo2-map-inspector`
depends only on that crate. No original file is read in this task. All fixtures
are constructed from arbitrary synthetic values. The reader and inspector are
reviewed and accepted against this synthetic contract.
Overall, gameplay and visual fidelity remain **UNKNOWN**; original data
understanding remains **INVESTIGATING**.

## Reason for this boundary

[Findings 0005](../reverse_engineering/0005-gfxworld-surface-boundary.md),
[0006](../reverse_engineering/0006-gfxworld-vertex-stream-boundary.md) and
[0007](../reverse_engineering/0007-gfxworld-position-prefix.md) support a bounded
candidate of flat positions and per-surface ordered triangle references. They
establish neither original draw commands nor units, axes, winding, normals, UVs,
colors, stream-1 layout, materials or appearance. The observed separate surface
bounds fit each surface's own references; embedded `tris` bounds do not.

A later external producer may assign a contiguous snapshot base to each admitted
source offset group's complete position prefix, preserving every record and its
order. Each source group-local index becomes `snapshot_group_base + local_index`
with checked arithmetic. Surfaces and their reference sequences retain source
order, including zero declared vertex counts and degenerate triangles. The producer
must record its chosen group order externally and verify complete accounting;
byte-buffer order and logical `firstVertex` order are different in the observed
sample. The snapshot does not retain pointers, names, paths or source byte offsets.
It is not a rule that source declared counts bound referenced positions.

The consumer imposes its own explicit numeric/index contract without claiming
that its representation is the original GPU layout. It retains all positions,
including unreferenced records and equal-valued records. It neither welds vertices,
filters triangles nor changes coordinates, units, axes, winding or float bits.
Category metadata is opaque. A category absent from the original sample is valid
in this generic diagnostic format and proves no original-format support.

The downloaded Bevy 0.19.1 release's `examples/asset/custom_asset.rs` demonstrates
`AssetLoader`, `LoadContext`, `AssetServer`, `Handle` and `Assets` ownership;
`examples/3d/generate_custom_mesh.rs` demonstrates `Mesh`,
`PrimitiveTopology::TriangleList`, `Mesh::ATTRIBUTE_POSITION`, `Indices::U32`
and `RenderAssetUsages`. Its manifest and `bevy_internal` reexports gate asset
and mesh facilities separately. These local release examples were inspected;
no new Bevy features or dependency are needed for a std-only reader. A later
adapter should use those engine facilities, rather than introduce another asset
registry, ECS, renderer or lifecycle. Mesh conversion and an asynchronous Bevy
loader each require their own bounded task.

## Byte contract

Every integer and IEEE-754 binary32 is **little-endian**. Sections are contiguous,
unaligned byte records, with no implicit padding. Version 1 has exactly the
following 64-byte header; offsets are relative to the first input byte.

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 8 | Magic: ASCII `BO2DGS` followed by two zero bytes |
| 8 | 4 | Version, unsigned integer `1` |
| 12 | 4 | Flags, must be zero |
| 16 | 8 | Position record count `P`, unsigned integer |
| 24 | 8 | Index element count `I`, unsigned integer |
| 32 | 8 | Surface record count `S`, unsigned integer |
| 40 | 8 | Exact total byte length `L`, unsigned integer |
| 48 | 16 | Reserved bytes, all zero |

Sections occur in this order:

| Start offset | Record bytes | Contents |
| --- | ---: | --- |
| `64` | 12 | `P` positions, three finite binary32 components in XYZ order |
| `64 + 12*P` | 4 | `I` unsigned 32-bit indices into the flat position array |
| `64 + 12*P + 4*I` | 48 | `S` surface records described below |

`L = 64 + 12*P + 4*I + 48*S` must hold with checked arithmetic and must match
the caller's observed same-handle input length exactly. There are no trailers,
strings, blobs, material IDs, hashes, normals, UVs, colors or runtime identities.
No original sample bytes or derivative geometry may enter tracked fixtures.

Each 48-byte surface record uses offsets relative to that record:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 8 | First index element, unsigned integer |
| 8 | 8 | Index element count, unsigned integer |
| 16 | 4 | Opaque source-format category, any unsigned 32-bit value |
| 20 | 4 | Opaque source declared-vertex-count, any unsigned 32-bit value |
| 24 | 12 | Finite lower endpoint XYZ components |
| 36 | 12 | Finite upper endpoint XYZ components |

Surface ranges are in **index elements**, never bytes, vertices or triangles.
`I`, every first index and every surface index count are multiples of three.
Ranges must form an ordered contiguous partition of `0..I`: each range starts
at the previous range's end, begins at zero, fits `I`, and the final end equals
`I`. Empty surfaces are permitted at the current partition cursor. Every index
must be less than `P`. Every surface endpoint component must be finite and
lower <= upper. Only positions actually referenced by that surface must lie
within its inclusive endpoints, using exact numeric comparison with no tolerance
or repair. Shared/unreferenced data need not fit unrelated surfaces' bounds.

`P = I = S = 0` is a valid 64-byte empty snapshot. Nonempty positions with no
indices are valid. Empty surfaces with finite ordered endpoints are valid, with
vacuous containment. Nonzero indices require positions and covering surfaces.
There is no relation between the opaque source declared count and index access;
zero never removes a surface. Repeated references and degenerate triangles are
valid. Signed zero and finite subnormal values are preserved through decoding and
accessors; NaN and either infinity are rejected. Rust borrowed slices expose the
validated data; an owning decomposition permits a future adapter to transfer
vectors without another full geometry copy.

## Resource and I/O policies

Fixed default limits are project policies, **not BO2 format maxima**:

| Policy | Limit |
| --- | ---: |
| Declared total file bytes | 64 MiB (67,108,864) |
| Position records | 2,000,000 |
| Index elements | 10,000,000 |
| Surface records | 100,000 |
| Total retained vector capacity bytes | 60 MiB (62,914,560) |
| Per-surface reference visits across the complete partition | 10,000,000 |
| Interrupted read retries across one invocation | 1,024 |

The current observed sample has 191,259 candidate records, 449,721 references and
6,469 surfaces, comfortably below these policies. With these record sizes a
later full-sample snapshot would be 4,404,568 bytes; no original-derived snapshot
has been produced or read by this task. Because surface ranges partition all
indices, complete own-bounds
validation visits each index exactly once. Index and reference policies therefore
share the same 10,000,000-element admission cap. The header's three counts are
converted only after checked section-size/total arithmetic, count and visit
policies, exact length and platform-size admission. Planned retained allocation
is `P * size_of<[f32; 3]>() + I * size_of<u32>() + S * size_of<SnapshotSurface>()`.
Each vector uses fallible `try_reserve_exact`; actual reported capacities are
checked against the combined retained-byte budget before reading the body.
Allocator overhead and transient allocator behavior are not bounded by that
logical capacity policy, and this is not a hard process-memory cap.

The reader starts at offset zero. Files supply metadata length from the same
opened read-only handle. A claimed length over policy rejects before reading;
an input shorter than the header rejects without body reads. The fixed header
is read before any count-sized allocation/body read. Count/length/unsupported
header failures therefore consume at most 64 header bytes and allocate no geometry.
The body uses one reusable 4,096-byte scratch buffer and decoded vectors; no
complete input byte vector is retained. Reads are capped at section-aligned
chunks and never request beyond the expected byte length. Success then requires
one **one-byte EOF sentinel request**; a byte returned there is trailing data.
Admitted byte ranges never cross `L` except this sentinel, and total consumed
bytes are at most `L + 1`. Short reads may request an unread span again, so summed
requested buffer lengths are not bounded by `L + 1`. No trailer is retained.
Cumulative Interrupted retries have their own cap. A caller-supplied blocking `Read` can
still block indefinitely; no wall-clock deadline is claimed.

A header length mismatch rejects before body reads. Premature reader EOF is
typed truncation; an impossible `Read` return count is typed redacted I/O failure.
Unsupported magic/version/flags/reserved values, arithmetic/limit/allocation
failures, nonfinite values, invalid indices/ranges/partition/bounds and own-surface
containment failure are typed errors with **no source path or raw invalid fields**.
No I/O error message or underlying source error is retained. Chunk validation
may read later records within that bounded chunk before reporting a malformed
earlier record. EOF/length checks observe the sequential read only: concurrent
same-length rewrites can produce a mixed observation, and mutations after the
EOF check are undetectable. Success is neither a stable-source snapshot guarantee
nor authentication, origin verification, renderability or fidelity evidence.

## Inspector

`cargo run -p bo2-map-inspector -- [--] <explicit-snapshot-file>` opens exactly
one regular file read-only and writes no files. `--help` / `-h` explains this
diagnostic boundary. No recursive search, local configuration loading, automatic
original input, exporter, loader or renderer is assembled. Paths remain native
`OsString`/`Path` values, including non-Unicode paths.

Success emits bounded counts and diagnostic status only. Category bins 0 through
8 plus a combined other-category bin contain surface, zero-source-count and
reference counts; bins carry no material/runtime semantics. Coordinate/index
values, endpoint values, names, paths and malformed raw fields are never emitted.
Output does not claim original-format support or source authenticity. Exit codes
are 0 for success/help, 1 for input/inspection/output failure and 2 for invalid
arguments. Input and output I/O diagnostics retain only `ErrorKind`.

See [handoff 0012](../handoffs/0012-diagnostic-geometry-snapshot-reader.md) for
actual synthetic checks, workspace gates and remaining limits. The next geometry
task is a separately supervised external producer preserving complete source
record/reference accounting; later Bevy mesh/asset integration and appearance
comparison remain unimplemented. Complete stream-1/layout investigation is still
required before complete attribute/material import.
