# GfxWorld static surface boundary

Recorded: 2026-10-05. Question: do the single decoded `zm_tomb` world's
declared surface/index/vertex ranges and material pointers form a coherent
boundary for later geometry investigation? Structure availability is
**INVESTIGATING**. Overall, gameplay and visual fidelity remain **UNKNOWN**.
This task adds external observation tooling and documentation, not a Rust
decoder, geometry adapter or rendering implementation.

## Source and access identity

The sole original input is the same unchanged source as
[directory findings 0004](0004-origins-asset-directory.md): relative
`zone/all/zm_tomb.ff`, Steam Zombies manifest build `24784239`, English,
78,753,344 bytes, SHA-256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
The sole selected world identifier is `maps/mp/zm_tomb.d3dbsp`.

Public OAT source is pinned to release `v0.33.0`, commit
`7d027e8f89118196713e955b0e11f8404149c54d`. The official verified release
package remains unchanged. A separate external checkout changes only its
`ContentPrinter.cpp` LIST output and adds a local metadata-check header.
No loading, cryptography, signature, container or dumping code is changed.
GPL source, dependencies, generated source/build files, patches, binaries,
raw logs and helper records remain outside this repository.

The build follows the pinned [Windows requirements](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/README.md#L83-L94)
and [release procedure](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/.github/workflows/release.yaml#L56-L75):
Visual Studio 2026, Premake `5.0.0-beta8`, version `v0.33.0`, Release/Win32.
Premake's executable digest matches the pinned
[download/hash declaration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/generate.bat#L3-L4).
Every submodule is checked out at its parent-recorded gitlink and verified
clean. MSBuild is limited to the `Tools\UnlinkerCli` solution target and its
required dependencies; no other final tools or test targets are requested.
The generated solution is `.slnx`; use that actual output rather than the
README's older `.sln` spelling. A pinned JSON dependency checkout required
process-local Git long-path support. The adapter's first compilation required
adding the public T6 enum header; the correction changes no decoding semantics.
No global environment, PATH, runtime or installation changes are made.

The corrected target exited 0 and produced the observation binary within the
authorized 40-minute total setup/build bound. The initial 15-minute bound was
extended to 30, then by at most ten further minutes while the same dependency
build demonstrably progressed. The initial build preserved useful prerequisites
but exited 1 on the corrected enum-header issue; the same target then completed
incrementally. Both build logs and adjustment/progress records remain external.
MSBuild file version was `18.10.1.42706`; the selected x86 MSVC compiler file
version was `19.51.36260.0`. This is external tooling provisioning, not a Rust
toolchain or architecture change.

| Observation artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Instrumented `Unlinker.exe` | 4,776,960 | `6ae454d3961d197067a116ebf23a5206c8532bea9fab16944f59ab0f131105f4` |
| Instrumented `ContentPrinter.cpp` | 10,316 | `48abba68e3dd6c99f80399bae64a76ab97c5b453fcf1bacd12287e386d78f502` |
| `SurfaceObservationChecks.h` | 2,162 | `03827d179ed215c9d332f7c9ba9ad83062d2e31248fa17371d92f44f453fac3f` |
| External `observe.py` supervisor | 8,565 | `fe8a6f305e8e2e6676d49135f7e562aa159a3e8e4876f3072eff25f6623ff019` |

The helper/native-test identities, exact submodule commits, untouched loader/source
hashes and binary identities are retained in external `tooling-identities.json`
and `source-identities.json`. The official release binary remains the
4,756,480-byte, `9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63`
artifact accepted in findings 0004. Instrumentation is local evidence tooling;
its differing binary digest is expected and does not identify an official release.

## Source corroboration and tested candidates

All source links target the exact commit and use raw-file line numbers.

| Source-supported declaration/loading rule | Boundary and limit |
| --- | --- |
| [World surface count](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1344-L1350) and [surface loader count](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/GfxWorld.txt#L161) | Signed world `surfaceCount` counts `dpvs.surfaces`; it does not mean static models or `dpvs.staticSurfaceCount`. |
| [World draw fields](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1231-L1247) and [index loader count](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/GfxWorld.txt#L83) | Unsigned logical `vertexCount`; signed `indexCount` counts 16-bit unsigned index elements. |
| [Vertex stream declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1219-L1229) and [stream loading](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/GfxWorld.txt#L89-L98) | Two separately counted byte streams. The aligned-char alias named `byte128` is not evidence of a 128-byte vertex stride. Encoding and complete per-surface byte spans remain unestablished. |
| [Surface triangle descriptor](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3901-L3912) | Signed `baseIndex`, `firstVertex` and two vertex-data offsets; unsigned 16-bit `triCount` and `vertexCount`. Names alone do not establish the original draw command. |
| [Surface material pointer](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3914-L3923) and [pool reference predicate](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCommon/Pool/XAssetInfo.cpp#L96-L103) | Join surface `Material*` to loaded-zone material entry pointers; leading-comma names denote reference-marked entries under OAT's convention. No names or addresses need emitting. |

The tested index-range candidate is
`[baseIndex, baseIndex + 3 * triCount)` in index elements. Three-element
triangle storage elsewhere in the same T6 source
([XSurface](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1079))
corroborates the triangle convention, but does not independently prove this
world's draw operation. The tested logical vertex candidate is
`[firstVertex, firstVertex + surface.vertexCount)` in logical vertices.
Checked arithmetic rejects signed negatives, overflow and out-of-capacity
endpoints before any index-element read. A nonzero counted array requires a
nonnull pointer; zero counts permit null pointers. Policy exhaustion is distinct
from invalid candidate arithmetic. Native allocation extents are entrusted to
the pinned decoder, not independently rederived by this metadata adapter.

For admitted ranges, aggregate union coverage measures covered, repeated and gap
elements. Individual intervals are never emitted. Index values are read only
inside admitted index ranges to test two competing interpretations:

- Surface-local candidate: every stored value is below the surface vertex count,
  and checked `firstVertex + value` is below the world's logical vertex count.
- Already-global candidate: every stored value lies within that surface's
  logical vertex interval.

Agreement distinguishes numeric consistency; it is not an original rendering
call trace. Byte-stream checks inspect only each declared starting offset against
the corresponding counted byte capacity and pointer presence. They establish
neither vertex stride nor complete byte-span validity, and do not establish that
both streams are required for every material/layout.

## Exact read/emit and resource contract

1. Resolve installation/output roots from ignored `config/local_assets.toml` and
   require both outside the repository. Select only the stated relative fastfile;
   no dependency-zone preload, original write/copy, game launch or external asset
   container access is requested. The pinned stock LIST guard still skips raw OBJ
   containers and dumping; its normal signed-PC loader and rejection path remain
   intact, as documented in findings 0004.
2. Hash the original through one opened read-only handle before and after the
   observation pair; compare SHA-256, length, write time and file identity. Verify
   the instrumented binary and official release binary before/after as well.
3. The stock native decoder reads/allocates the entire zone, including other
   families in memory. This is explicitly not a bounded-prefix Rust parser or a
   hard memory cap. The metadata instrumentation inspects only the selected
   world's scalar counts, surface range/offset descriptors, material pointers,
   bounded index elements and material pool entry pointer/reference metadata.
   It never reads vertex coordinates/attributes or other asset bodies for output.
4. Before scanning, apply local **policies**, not claimed format maxima: 100,000
   total pool entries; 100,000 surfaces; 20,000 material entries; 10,000,000 logical
   vertices/index elements; 512 MiB per declared vertex stream; 50,000,000 total
   per-surface index-element visits. Interval/material collections are bounded by
   those admitted counts. Invalid/null counts and policy-limit rejection are
   distinguished. Stream bodies are not scanned.
5. Run each decode from a fresh isolated external process directory using only
   `--list --no-color <single-input>`. Supervise a 120-second wall deadline and
   256-KiB combined stdout/stderr cap; terminate and report incomplete on either
   limit. Decode memory is observed, not forcibly capped. The adapter replaces
   the full directory listing with one aggregate record capped at 8,192 bytes.
6. Emit only counts, presence flags, aggregate arithmetic/coverage/candidate
   checks and material pointer-match categories, plus the selected map identifier.
   Never emit addresses, index/vertex values, coordinates, complete material names,
   scripts, crypto values or raw map dumps. Unmatched pointers are never dereferenced.
   A pointer matching both reference-marked and non-reference-marked entries is
   explicitly ambiguous; null, non-reference-marked-entry, reference-marked-entry,
   unmatched and ambiguous counts are separate. Internal helper keys use
   `loaded`/`placeholder` for those two marker categories only; they never mean
   material completeness, external ownership or unresolved dependency status.
   Unique counts omit null.
7. Require exactly one aggregate record, successful diagnostics, empty stderr and
   no other created files/directories besides the supervisor logs. Check category
   sums, repeat aggregates and source/tool preservation. Retain raw process/build
   logs, helpers, source identities, synthetic fixtures and metadata externally.

Reproduction uses external `0009-gfxworld-surface-boundary/observe.py` with the
local configuration path and a fresh optional output tag. The separately built
tool is below external `tooling/OAT/surface-7d027e8/`; `source-identities.json`,
`instrumentation.patch`, submodule identities, build/budget logs and native
synthetic checks are retained there. Actual roots never enter tracked documents.

## Direct observation outcome

Two successful loads on the stated original input produced identical aggregate
records. No capture/deadline/count policy was exhausted. Both runs exited 0,
reported zero warnings/errors and had empty stderr. Each process directory
contained exactly its two supervisor logs and no dumps or other directories.

| Observation | Run 1 | Run 2 |
| --- | ---: | ---: |
| Elapsed wall seconds | 1.172 | 0.671 |
| Combined captured bytes | 2,541 | 2,541 |
| Observed peak working set bytes | 391,979,008 | 391,254,016 |

These are local access measurements, not gameplay performance. Original SHA-256,
length, last-write time and same-handle file identity matched before/after the
pair. Instrumented and official binaries also remained unchanged. Selected
loader sources still match the pinned checkout; the signature path was neither
modified nor bypassed. Success is OAT's signature acceptance on its supported
path, not independent authenticity proof.

| Direct aggregate | Value |
| --- | ---: |
| Loaded pool entries / selected world entries | 7,549 / 1 |
| World `surfaceCount` | 6,469 |
| Draw logical `vertexCount` / index-element `indexCount` | 191,259 / 449,721 |
| Vertex stream 0 / 1 declared bytes | 6,886,816 / 489,788 |
| Sum of surface `triCount` fields | 149,907 |
| Sum of surface `vertexCount` fields | 175,268 |
| Surfaces with zero triangles / zero vertex count | 0 / 6,236 |
| Surfaces with negative `baseIndex` / `firstVertex` | 0 / 0 |

All four native array pointers were nonnull, and the root signed/count/presence
checks passed. All 6,469 index candidate ranges fit, with zero invalid or
policy-limited ranges. Their union covers all 449,721 index elements exactly:
zero repeated elements and zero gaps; the triangle-count sum times three is the
same 449,721. This establishes a coherent observed **index-range boundary** for
this sample under the tested three-elements-per-triangle interpretation.

All 6,469 logical vertex candidate endpoints also fit arithmetically, with zero
invalid or policy-limited endpoints. **That does not establish a universal
geometry slice:** 6,236 ranges are empty while their triangle counts are nonzero.
The candidate union covers only 3,188 logical positions, with 172,080 repeated
positions and 188,071 gaps relative to draw `vertexCount`. These are measures of
the tested numeric intervals, not observed counts of unique geometric vertices.

The bounded scan read 449,721 index elements internally and emitted none. Only
233 surfaces satisfy the surface-local candidate; the other 6,236 fail because
their declared vertex count is zero despite nonempty triangles. No surface
satisfies the already-global candidate. All candidates were evaluated without
an iteration cap or unread range; the **semantic mapping from surfaces to draw
vertices remains unestablished**, not a decoder corruption conclusion.
`firstVertex`/`vertexCount` cannot yet be assumed to describe every surface's
required vertex slice.

Starting-offset candidates fit stream 0 for all 6,469 surfaces. Stream 1 fits
for 6,457 and fails for 12. The aggregate did not distinguish negative sentinels,
optional layouts or other reasons for those failures. Stream bodies were not read
by the adapter, and complete byte spans/required streams remain unknown. These
failed candidates are evidence against silently assuming uniform layouts, not
proof that the original zone is corrupt. Complete per-surface byte-span validity
is unestablished for all 6,469 surfaces because their vertex encoding is unknown.

The loaded pool has 1,686 material entries: 1,564 non-reference-marked and 122
reference-marked under OAT's convention; no entry pointer is null. All 6,469
surface material pointers match the non-reference-marked category, representing
280 unique pointer identities. Null, reference-marked-only, unmatched and
ambiguous surface/unique categories are all zero. This is a same-loaded-pool
pointer relationship only. It establishes neither texture/shader completeness,
readiness for rendering, dependency-zone ownership nor resolution of the other
122 reference-marked entries.

## Validation and limits

The same external arithmetic/reference/coverage helper used by the adapter passed
26 user-created native synthetic checks before any original observation. These
cover checked addition/multiplication and overflow; exact-end and truncated ranges;
negative signed values; positive/null and zero/null counts; out-of-capacity empty
ranges; oversized policy counts; all five reference categories; and complete,
overlapping, contained and gapped interval unions. These are native tooling checks,
not Rust/game tests or fidelity comparisons.

The adapted supervisor passed user-created cap/deadline children without opening
originals: combined output beyond 256 KiB was terminated at the capture cap;
a sleeping child was terminated at a 0.1-second synthetic deadline. Both reported
incomplete/unsuccessful exits. The same checks passed again before the successful
observation pair. Aggregate category/range sums, exact repeat equality, admitted
index reads, source/binary preservation and no-extra-output checks passed.
There is no hard native-decoder memory cap,
independent authentication check, negative-signature experiment, geometry visual
review or original-game comparison. The four safe Rust crates, Bevy ownership,
workspace manifests and 25 previously accepted tests are unchanged; Cargo gates
are intentionally not repeated for documentation-only repository changes.

Outer-agent review independently reran all 26 native synthetic checks and both
supervisor selfchecks, followed by two further observations (0.719/0.688 seconds,
2,541 bytes each). All four aggregate records agree exactly. Originals and both
binaries retain their recorded identities; no unexpected process outputs were
created. Review also verified the pinned checkout, nine clean submodules, eleven
pristine source files, scoped instrumentation patch/header and helper hashes.
Working-tree CRLF was normalized only when comparing pristine files with Git
blobs; the recorded file hashes matched exactly. Documentation/content boundaries
and whitespace checks passed. These checks accept the observation evidence,
without establishing vertex encoding or gameplay/visual fidelity.

## Next bounded work

Investigate the selected world's **per-surface vertex-stream interpretation**:
establish the meaning of the zero `vertexCount` cases, `firstVertex`, both byte
offsets and material-dependent stream requirements before deriving complete byte
spans or a safe geometry adapter. Use the same one-zone external access boundary
and aggregate-only output. The index/material observations constrain that task;
they do not justify inventing a uniform logical vertex slice or fixed stride.
Coordinate units, axes, winding, textures, lighting,
collision and comparison with original appearance remain separate unknowns.
No Rust/Bevy geometry API is approved by this observation task.
