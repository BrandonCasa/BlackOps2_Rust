# Origins diagnostic geometry snapshot

Recorded: 2026-10-05. Task 0013 connects the accepted candidate position/index
evidence to the accepted safe Rust diagnostic consumer through one external v1
producer. The complete observed world is preserved. Original data understanding
remains **INVESTIGATING**; overall, gameplay and visual fidelity remain **UNKNOWN**.
The native producer is external inspection tooling. Rust with Bevy remains the
tracked engine/game implementation and engine/asset/rendering ownership boundary.

## Source and contract

The only original input is unchanged `zone/all/zm_tomb.ff`, Steam Zombies build
`24784239`, English, 78,753,344 bytes, SHA-256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
The selected world is `maps/mp/zm_tomb.d3dbsp`. Access uses external OAT
`v0.33.0`, pinned commit `7d027e8f89118196713e955b0e11f8404149c54d`,
normal single-zone signed LIST loading and unchanged signature rejection.
There are no other original inputs, dependencies/preloads, raw-container probes,
source writes/copies, game launch, original executable or shader/script inspection.
Identity hashing separately reads the whole source read-only. Normal OAT signature
acceptance is not independent authentication.

[Findings 0005](0005-gfxworld-surface-boundary.md),
[0006](0006-gfxworld-vertex-stream-boundary.md) and
[0007](0007-gfxworld-position-prefix.md) provide the accepted range, complete-group
and finite own-surface bounds evidence. The pinned T6 declarations/generated
loader and public C2M commit `9a3accfb75c9a2f9780446c2b7bef040570357cf`
were inspected in the preceding tasks and their source identities preserved.
C2M supplies an addressing candidate, not original-engine truth. Its scaling,
normal/UV transformations and other attribute assumptions are not adopted.

The producer uses the unchanged project-owned
[diagnostic geometry snapshot v1 contract](../architecture/diagnostic-geometry-snapshot.md):
64-byte `BO2DGS` header, explicitly little-endian integers/binary32, 12-byte XYZ
records, four-byte u32 index elements and 48-byte ordered surface records. This
is our diagnostic exchange format, not a BO2 disk format. No pointers, native
structure dumps, offsets, names or runtime identities enter the snapshot.

## Complete preservation procedure

Surface-array admission covers each complete 80-byte native record. Same-pool,
non-reference material and technique-set joins precede format-scalar access.
Signed offsets/first/base fields, count/null/capacity/policy checks and checked
arithmetic precede relevant index and byte reads. Allocation extents remain trusted
to the pinned decoder; the adapter does not independently reconstruct them.

For each surface, derive its extent as one plus the maximum referenced local index.
Group by equal stream-0 byte offset, retain the maximum member extent, and require
one consistent logical first position and format per group. Admit every complete
36-byte group-record span against its next distinct offset, or stream capacity for
the final group. Its end must round to that boundary at 16 bytes. Require zero
leading gap. Group logical intervals partition the declared logical capacity.
Separately sort admitted nonempty source index intervals and require complete
coverage with zero overlap/gaps; matching count sums alone are insufficient.
Sorting serves admission and never changes source surface/reference order.

All groups, capacities and total output length are admitted before the first stream
body read. Scan every candidate XYZ prefix for finite values, then compare every
surface's own referenced positions to that surface's separate `bounds[0]`/`bounds[1]`
endpoints with exact inclusive comparisons. Embedded `tris` endpoints are not read
or substituted. IEEE-754 little-endian binary32 representation is selfchecked;
unaligned sources use `memcpy`, not unaligned casts. Invalid/nonfinite/inconsistent
evidence rejects without numeric repair or truncation.

Assign snapshot bases in **ascending stream-0 offset order**, appending every
complete group's record in local-index order. Each source reference becomes the
checked u32 `snapshot_group_base + local_index`. Preserve source surface order and
every reference sequence; the snapshot's contiguous surface ranges partition all
output indices. Preserve raw XYZ and separate endpoint bits, opaque format and
declared count metadata. Zero counts, duplicate/equal/unreferenced records and
degenerate triangles never cause filtering, welding or removal. No scaling,
axis/winding change or tolerance is applied. Logical `firstVertex` order is a
different source order and is used only for admission.

The producer derives exact output length before exclusive creation of the fixed
`diagnostic-v1.INVALID` file. Explicit field serialization checks every output
range. Short/write/flush/close failures propagate and remain labelled invalid.
After complete write, `FlushFileBuffers` and checked close, an independent
sequential readback checks header fields, section ends, total length/EOF and every
record against the admitted source: direct raw 12-byte prefix comparison, inverse
group remapping against original local indices, and exact metadata/endpoint bits.
Native separate endpoint bits are reread and compared as well. This verifier does
not reuse serialized record buffers or serializer section-offset helpers.
Checked readback close precedes exclusive no-replace publication as
`diagnostic-v1.bo2dgs`. Existing invalid/final targets reject; no overwrite occurs.
An incomplete or merely prefix-valid file cannot become an accepted observation.

## Predeclared limits

These are diagnostic policies, not BO2 format maxima. External
`predeclared-policy.md` and synthetic proof precede original access.

| Policy | Bound |
| --- | ---: |
| Snapshot file / consumer retained logical capacities | 64 MiB / 60 MiB |
| Complete position records | 2,000,000 |
| References in each of four source-index scans | 10,000,000 |
| Surface / group / pool entries | 100,000 each |
| Material / technique-set pool entries | 20,000 each |
| Source stream capacity | 512 MiB |
| Additional retained vector capacities combined | 32 MiB |
| One-byte unique-address markers | 2,000,000 bytes |
| Index visits across extent, containment, emission and readback | 40,000,000 |
| Prefix visits across finite scan, containment, emission and readback | 16,000,000 |
| Position component visits across those passes | 48,000,000 |
| Separate bounds components across copy and native recheck | 1,200,000 |
| Combined fixed writer/readback buffers | 8,192 bytes |
| Native invocation / combined stdout+stderr / aggregate record | 120 seconds / 256 KiB / 8,192 bytes |

The 32-MiB capacity accounting covers the actual reported capacities of surface
and group vectors, one reusable interval vector and markers. Reservation arithmetic
is admitted before allocation; reported capacities are checked before body reads.
No full raw geometry, position cache or reference vector is retained. Writer and
readback each retain a 4,096-byte buffer while both objects remain in scope, plus
small fixed header/record scratch; individual I/O blocks are at most 4,096 bytes.
Pool node/allocator overhead, transient allocation behavior and the decoder are
outside this logical container policy. The decoder reads/allocates the **whole
zone**; this is neither a bounded-prefix decoder nor a hard process-memory cap.

The accepted Rust reader retains its default policies, including at most 1,024
cumulative Interrupted retries and ten million own-surface reference visits.
Each Rust invocation has its own supervisor deadline/capture limits. Consumer
success alone proves neither source bit preservation nor original support;
paired native source readback and independent file accounting supply separate
checks.

## Complete observations

The authoritative `observations-v2` pair agrees exactly. Each native run exits 0,
reports zero warnings/errors and has empty stderr. Each isolated directory contains
only the one completed snapshot and supervisor stdout/stderr logs. No policy,
deadline or capture/artifact cap is exhausted.

| Accounting | Result |
| --- | ---: |
| Source surfaces / preserved surfaces / zero-count surfaces | 6,469 / 6,469 / 6,236 |
| Complete offset groups | 320 |
| Source candidate / preserved / unique addressed records | 191,259 / 191,259 / 191,259 |
| Unreferenced records in this original sample | 0 |
| Preserved ordered references / triangles | 449,721 / 149,907 |
| Source record bytes / alignment padding / stream-0 bytes | 6,885,324 / 1,492 / 6,886,816 |
| Output header / position / index / surface section bytes | 64 / 2,295,108 / 1,798,884 / 310,512 |
| Exact complete output bytes | 4,404,568 |
| Source index visits across four scans | 1,798,884 |
| Source prefix / position component visits across all passes | 1,023,498 / 3,070,494 |
| Separate bounds components read / preserved | 77,628 / 38,814 |
| Peak additional logical vector capacity / consumer logical capacity bytes | 1,278,051 / 4,404,504 |

Every admitted group record is finite; every own-surface exact bounds comparison
succeeds. Every emitted prefix/reference/metadata/bounds record agrees bit for bit
with its admitted source counterpart under the documented group remapping. No
source surface or reference is omitted, including all zero-count cases.

| Opaque format | Surfaces | Zero-count | Groups | Records | References |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | 4,154 | 4,105 | 144 | 132,709 | 281,460 |
| 1 | 313 | 297 | 27 | 3,618 | 6,972 |
| 2 | 1,679 | 1,518 | 94 | 49,724 | 147,819 |
| 3 | 8 | 8 | 4 | 54 | 102 |
| 4 | 105 | 105 | 17 | 1,343 | 3,186 |
| 5 | 209 | 202 | 33 | 3,806 | 10,173 |
| 7 | 1 | 1 | 1 | 5 | 9 |
| Total | 6,469 | 6,236 | 320 | 191,259 | 449,721 |

Formats 6 and 8 are absent; no original-format support claim is made for them.
The diagnostic consumer's generic opaque category support remains unchanged.

Both snapshots have SHA-256
`ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`.
Independent Python streaming inspection verifies explicit header/section sizes,
finite values, exact reference partitions/own-bounds, categories and section hashes.
The accepted Rust `bo2-map-inspector` validates **each** full file, reproducing
all source/producer counts, triangles, zero counts, category bins and byte length.
Neither validator emits coordinates, individual indices, endpoints, paths or names.
Same-handle snapshot hashes, lengths, write times and file IDs are unchanged after
these read-only checks. Whole-snapshot and section hashes repeat exactly.

Outer independent reproduction in external `parent-review-065505` passes all
seven supervisor selfchecks and a fresh pair: native 0.782/0.750 seconds,
2,436 captured bytes each; Rust 0.078/0.078 seconds, 1,138 bytes each. All native
and Rust exits are 0 with empty stderr. Independent Python section/numeric/
reference accounting and source/tool preservation hold. All four authoritative
snapshots, section hashes and aggregates agree exactly. Outer review accepts this
bounded diagnostic bridge; no fidelity status is promoted.

| Authoritative observation | Run 1 | Run 2 |
| --- | ---: | ---: |
| Native elapsed seconds | 0.750 | 0.750 |
| Native combined capture bytes | 2,436 | 2,436 |
| Native observed peak working set bytes | 392,138,752 | 391,995,392 |
| Rust consumer elapsed seconds | 0.062 | 0.078 |
| Rust consumer capture bytes | 1,138 | 1,138 |

These are local inspection timings, not gameplay or optimization measurements.
The original source's SHA-256/length/write-time/same-handle file ID match before
and after the pair. Official, accepted task9 backup, separate task10/task11, new
task13 and Rust consumer binary identities likewise match.

## Synthetic checks, build and restoration

Before original access, the producer passes **129 native synthetic checks**, plus
the accepted 46 position, 22 vertex and 26 surface checks, with `/W4 /WX`.
Cases cover explicit LE header/sections and deterministic complete accounting;
multigroup remapping/source order; zero/positive declared counts and opaque
categories; duplicate/unreferenced records/degenerate triangles; signed zero,
subnormal bits and exact own bounds; empty/zero/null/negative/exact-end/truncated/
overflow/policy/reference cases; overlap plus a compensating source-index gap
despite equal count sums; all-group admission before prefix reads; unsupported
representation; nonfinite/reversed/outside bounds; write/short-write/flush/close/
partial/existing-target and final-rename failures; independent readback corruptions,
truncation and trailing bytes. Allocation failures propagate; forced allocator
exhaustion is not tested.

An entirely synthetic 244-byte C++ output passes the accepted Rust reader and
independent Python checker before the first proprietary output. Its signed-zero,
subnormal, opaque metadata, duplicate/unreferenced and degenerate cases are
preserved. No original-derived tracked fixture exists. The outer agent separately
reports passing all 129+46+22+26 native checks with binary identity equality;
its external `parent-native-064929` retains that independent result.

The same Hostx64 process-local Visual Studio environment builds the actual
`OpenAssetTools.slnx`, `Tools\UnlinkerCli`, Release/Win32 target. The production
translation unit recompiles, preventing stale output. Build exits 0 without
compiler warnings/errors in 12.171 seconds; the two standalone compilation passes
take 5.594/5.375 seconds. All compilation stays within the initial cumulative
900-second allowance. Schema 13 / `diagnostic_producer_revision=1` is required;
the snapshot itself remains v1. No global environment/runtime/toolchain changes.

Supervisor synthetic capture/deadline, invalid/final artifact-size cap and
unexpected-output children pass before originals. The first `observations-v1`
attempt exposed a legitimate invalid-to-final rename between enumeration and
`stat`. Its supervisor aborted before capturing the native result. No child
remained; the complete-sized file is explicitly `SUPERVISOR-INVALID`, excluded
from acceptance, with failed driver/logs preserved. The repaired supervisor
tolerates only disappearance of an admitted filename and rescans, performs stable
final checks, and guarantees child kill/reap/capture-thread cleanup on exceptions.
Deterministic rename-race and injected-exception cleanup checks pass before the
fresh authoritative pair. Three worker original loads include that excluded attempt,
plus two parent loads, for five total. No uncertain observation is silently
relabelled successful.

All task13 code/helpers/patches/binaries/fixtures/logs/geometry remain external
under `0013-origins-diagnostic-geometry-snapshot/`, resolved through ignored local
configuration. `reproduction.md`, `predeclared-policy.md`, `tooling-identities.json`,
`pristine-source-identities.json`, `observations-v2/observation.json` and
`task9-restoration.json` preserve exact procedures, identities and results.
Run external `observe.py` with the absolute configuration argument and a fresh
alphanumeric tag to reproduce; actual roots never enter tracked documentation.

| Final external producer artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Unlinker.exe` | 4,805,632 | `c7e22dcfee4f3d76cefd988e5aa3480fe647e476b701b9d893a67a82f545051e` |
| `ContentPrinter.cpp` | 2,161 | `6eb5e7dc6c9ea09776755082ec980455eb23a50b0d17d2e7bd499ee1f00dc3b5` |
| `DiagnosticSnapshot.h` | 15,848 | `e222ef7d20ca7f84fd96710b4e6ba03949c58da2eb4e3eb47fa4b7966ced568c` |
| `DiagnosticSnapshotFile.h` | 3,276 | `64d6c04210e6946a51e552d49a27dc4f8cf2d506358725cc2d123e921e90462e` |
| `DiagnosticSnapshotNative.h` | 4,161 | `75e7ccc18d865b58fbd1d1fb331de22e3386222f6aa4732d398eab88810a7fe9` |
| `observe.py` | 13,103 | `e4995ad19768521967cce81408747315955df92a71c2ea54b09271a7dcb457f2` |
| `independent_snapshot.py` | 2,971 | `3ceba812c47d5a7ad032b46b17eb5eac236694020d0496ad7f866285d24cc72a` |

Before staging, accepted task9 source/header/executable/exact Git patch were backed
up and their identities checked. Final task13 instrumentation was preserved before
restoring all four byte for byte and removing only its five added headers from
the reused checkout. Thirteen pristine decoder/loader/crypto/signature/dumper/
generated/public source identities and nine clean pinned submodules hold. Official
and separate task10/task11 artifacts remain unchanged. No proprietary values,
buffers, material/name lists, pointers, keys, binaries or local paths are tracked.

Parent preservation review independently confirms all thirteen pristine working
identities, eleven upstream Git blobs (line-ending normalization only for blob
comparison), 28 prior task10/task11 artifact hashes, pinned HEAD, nine individually
clean submodules, exact task9 cpp/header/executable/patch and two-item checkout
scope. All thirty frozen task13 artifact identities also pass independent review.
Owned documentation checks pass for five candidates, 59 relative links, ten handoff
headings, configured-root/secret exclusions and all-file whitespace;
`git diff --check` passes. The parent independently confirms those content/link/
heading/identity checks and accepts the five-file documentation integration.

## Limits and next work

This completes only the external diagnostic bridge and complete candidate
position/index preservation. It supplies no original draw call, GPU layout,
units/axes/winding, source authenticity or appearance comparison. No renderer,
Bevy Mesh/asset loader, other attributes, stream-1 body, materials/textures,
lights/models/collision or gameplay is implemented. No `MATCHED`/`VERIFIED` or
overall-goal-complete status is claimed. Complete attribute/material support
still needs separate stream-1/layout investigation.

The next bounded task should integrate this accepted consumer with **Rust Bevy
Mesh** facilities, keeping engine asset/mesh/lifecycle ownership in Bevy and
preserving raw coordinates/reference order. Appearance, material decisions and
original-game comparison remain subsequent bounded tasks. The tracked Rust code,
manifests/dependencies/unsafe policy and prior 61-test/Cargo/Bevy smoke gates are
unchanged; those gates are intentionally not rerun for this documentation-only
task. See [handoff 0013](../handoffs/0013-origins-diagnostic-geometry-snapshot.md)
for scope, validation and unresolved work. This bounded task is reviewed and accepted.
