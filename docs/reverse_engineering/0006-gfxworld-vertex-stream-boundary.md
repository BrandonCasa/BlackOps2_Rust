# GfxWorld vertex-stream boundary

Recorded: 2026-10-05. Question: what explains nonempty-triangle surfaces with
zero `vertexCount`, and which per-surface vertex byte spans are supported before
geometry work? Structure understanding remains **INVESTIGATING**. Overall,
gameplay and visual fidelity remain **UNKNOWN**. This is an external metadata
investigation; no Rust adapter, decoded attribute output or rendering is added.

## Sources and tested interpretation

The only original is the unchanged source from [findings 0005](0005-gfxworld-surface-boundary.md):
`zone/all/zm_tomb.ff`, Steam Zombies build `24784239`, English, 78,753,344 bytes,
SHA-256 `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
The selected world remains `maps/mp/zm_tomb.d3dbsp`. No other original input,
preload, game process, executable inspection or raw shader/script inspection is
used.

OAT remains pinned to `v0.33.0`, commit
`7d027e8f89118196713e955b0e11f8404149c54d`. Its normal signed LIST loader is
unchanged. The generated T6 loader reads each 80-byte surface and resolves its
material pointer without rewriting its scalar triangle descriptor. This is
source corroboration of the access method, not proof of the original draw call.

| Primary public source | Supported fact and limit |
| --- | --- |
| [T6 surface descriptor](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3901-L3923) and [surface loader](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/GfxWorld.txt#L161) | Two signed byte-offset fields, signed `firstVertex`, unsigned 16-bit vertex/triangle counts, signed index base, and a material pointer. Names do not settle draw semantics. |
| [T6 world formats/technique set](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L790-L808) | Nine TEX/NRM format categories are attached to a material's technique set; the enum supplies no byte strides. |
| [Material-to-technique-set relation](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L689-L704), [passes](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3047-L3089) and [routing declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L5779-L5872) | A pass has a vertex declaration containing up to sixteen semantic routes. Its `streamCount` counts routes; it must not be equated with the world's two physical byte streams. |
| [Technique/pass loading rules](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/MaterialTechniqueSet.txt#L10-L30) | Pass arrays use `passCount`; runtime Direct3D input layouts are not loaded. Routing metadata therefore does not supply an observed GPU byte layout. |

A separate public Black Ops II PC runtime extractor supplies a discriminating
addressing candidate. C2M is pinned to commit
`9a3accfb75c9a2f9780446c2b7bef040570357cf`; its
[surface declaration](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L148-L200)
matches the relevant field placement, and its
[face addressing](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L454-L485)
uses stream-0 byte offset plus stored local index times 36, without consuming
`firstVertex` or `vertexCount`. Its
[36-byte record declaration](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L205-L257)
and [unpacking function](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L597-L610)
are candidate attribute evidence only. No attribute bytes were read here.

This is the extractor author's implementation evidence, not original engine
source. It targets recognized PC executables, does not certify Steam build
`24784239`, uses one stream-0 record layout without technique-format dispatch,
and does not establish stream-1 consumption. Its unit conversion, UV conversion,
normal unpacking and winding choices are not adopted. No other game's layout is
substituted, and no GPL source is copied into this repository.

For each admitted surface index interval, let `E_surface` be one plus its largest
stored local index, computed internally without emitting index values. Group
surfaces by equal stream-0 starting offset; let `E_group` be the maximum of those
extents. Test the candidate prefix
`[offset0, offset0 + 36 * E_surface)` against stream capacity, and the larger
group prefix against the next distinct starting offset, or capacity for the last
group. Checked arithmetic precedes each range admission. The prefixes cover
every referenced candidate record, even if a group contains unreferenced records;
they are not counts of unique referenced vertices.

## Direct observations

The final two observations agree exactly. All 6,469 surfaces and all 449,721 index
elements are retained; no zero-count surface, index or material-format category
is dropped. Existing triangle/index partition and material-pointer results agree
with findings 0005.

| Aggregate | Observed result |
| --- | ---: |
| Distinct stream-0 offset groups | 320 |
| Groups shared by multiple surfaces | 236 |
| Surface candidate prefixes fitting stream-0 capacity | 6,469 / 6,469 |
| Zero-count surface candidate prefixes fitting capacity | 6,236 / 6,236 |
| Group candidate prefixes fitting next offset/capacity | 320 / 320 |
| Groups whose candidate end rounds to the next boundary at 16-byte alignment | 320 / 320 |
| Groups with one consistent `firstVertex`, stream-1 offset and format | 320 / 320 for each relation |
| Sum of group indexed extents | 191,259 |
| Indexed candidate record bytes | 6,885,324 |
| Intergroup/final alignment padding bytes | 1,492 |
| Leading stream-0 bytes before first group | 0 |
| Total stream-0 accounting | 6,886,816 bytes |

Thus `191259 * 36 + 1492 = 6886816` accounts for the entire declared stream-0
capacity. Each group has a complete candidate byte prefix and an observed
alignment boundary. This is substantially stronger than merely finding starting
offsets inside capacity, but remains a tested byte-addressing interpretation
without vertex-attribute or original draw comparison evidence.

The `firstVertex` group candidate is also coherent: all 320
`[firstVertex, firstVertex + E_group)` intervals fit the draw's logical vertex
capacity, and their union partitions all 191,259 positions with zero gaps or
repeats. Each group has a distinct `firstVertex`. No group's `firstVertex` equals
the cumulative indexed extent in sorted stream-0 byte order, and the values are
not dense group IDs from zero through 319. Logical interval order and byte-buffer
order therefore differ under this interpretation. This supports a group logical
base candidate; it does not prove which fields the original draw path uses.

### What the zero counts distinguish

There are 285 groups containing only zero `vertexCount` fields, accounting for
all 6,236 zero-count surfaces. The remaining 35 groups contain only positive
counts and account for all 233 positive-count surfaces; twenty have one surface
and fifteen have multiple positive-count surfaces. In those 35 groups the maximum
declared count equals the indexed extent. Their maximum-count sum is 3,188,
whereas summing repeated per-surface declared fields gives 175,268.

No zero-count surface shares its stream-0 offset with a positive-count surface.
This rejects the simple hypothesis that a group's first/owning surface records
the count and its other surfaces use zero as a sharing sentinel. Zero also cannot
mean an empty required candidate prefix: every such group has nonempty triangles
and a fitting indexed byte extent. All seven observed material-format categories
contain zero-count surfaces, so zero counts do not select a single format.

The supported practical explanation is that the declared per-surface count is
not a universal vertex-access bound in this decoded sample. Local indices and
stream-0 group offsets supply a coherent alternative without that field; the
public extractor uses the same alternative. Why the original data retains
positive counts for only 35 groups, whether the field is legacy/unused in a
particular draw path, and the exact original command remain unknown. Neither
corruption nor a semantic field replacement is established.

### Material categories and stream 1

Every surface material joins to a non-reference-marked same-pool entry, and every
inspected technique set joins to the same category. These are OAT pointer/marker
relations, not proofs of dependency ownership or material completeness.

| OAT world-format category | Surfaces | Zero-count surfaces | Unique technique sets |
| --- | ---: | ---: | ---: |
| 0: TEX 1 / NRM 1 | 4,154 | 4,105 | 23 |
| 1: TEX 2 / NRM 1 | 313 | 297 | 5 |
| 2: TEX 2 / NRM 2 | 1,679 | 1,518 | 10 |
| 3: TEX 3 / NRM 1 | 8 | 8 | 3 |
| 4: TEX 3 / NRM 2 | 105 | 105 | 8 |
| 5: TEX 3 / NRM 3 | 209 | 202 | 6 |
| 7: TEX 4 / NRM 2 | 1 | 1 | 1 |
| Total | 6,469 | 6,236 | 56 |

Categories 6 and 8 are absent from this sample; no support is claimed for them.
All seven observed categories pass the stream-0 addressing/extent tests.
Across 56 technique sets, the adapter inspects 1,120 unique techniques/passes,
fifteen unique declarations and 75 routes. No route source/destination exceeds
the public enum ranges. Five declarations mark optional sources; ten inspected
routes use the optional-source range. All fifteen declaration `isLoaded` fields
are false. This does not reveal the byte sizes or required physical streams of
those semantic attributes.

All 6,469 stream-1 offsets are nonnegative. None exceeds the 489,788-byte
capacity, and none is a negative sentinel. Of them, 6,454 are below capacity and
fifteen equal capacity, all in category 0. Twelve of those fifteen have positive
`vertexCount`; three have zero. These are exactly the twelve previous failures:
the old starting-offset candidate required strict `offset < capacity` when the
surface count was nonzero, while allowing an empty span at capacity for zero
counts. The failures thus reflect that tested requirement, rather than observed
negative/out-of-capacity offsets. Category 0 and optional routing are compatible
with absent additional data, but do not prove that interpretation. A positive
surface count does not establish that stream 1 is consumed.

**Complete required stream-1 spans and attribute encodings remain unknown for
all surfaces.** No stride is inferred from names, divisions of total capacity,
the aligned-char alias, or another game's declaration. Required-stream/layout
mapping must be established separately before a complete geometry/material import.

## Read/emit, bounds and reproduction

The native decoder still reads and allocates the whole same zone through normal
signed loading. It is not a bounded-prefix parser or a hard memory-capped
decoder. LIST runs with only `--list --no-color <single-input>` from fresh
external directories; raw containers, dumping and preloads remain disabled.

The adapter reads descriptors, checked index intervals, same-pool material and
technique-set pointer/marker metadata, format scalars, bounded pass arrays and
fixed declaration routing scalars. Index elements are scanned twice: 449,721
visits for the accepted task-0009 checks plus 449,721 for group extent checks,
899,442 visits per observation. Neither byte stream's body, runtime input-layout
objects, shaders, texture tables nor script bodies are read by the adapter.
No pointer address, name list, individual index, vertex/coordinate or proprietary
content is emitted. Native allocation extents are trusted to the pinned decoder.

Existing admission policies remain 100,000 pool entries/surfaces, 20,000 material
entries, 10,000,000 logical vertices/index elements, 512 MiB per stream and
50,000,000 index visits per scan. Additional policies bound technique-set pool
entries/visited sets to 20,000; unique techniques/declarations and total passes
to 100,000; passes per technique to eight; routing to its fixed sixteen entries
per declaration and 1,600,000 total visits. Pointer joins precede material and
technique-set dereferences; null/reference/unmatched categories are not followed.
Nested technique/declaration allocation validity remains the native loader's
responsibility. Group/interval collections are bounded by admitted surface counts.
These are inspection policies, not claimed format maxima.

The supervisor preserves the 120-second decode deadline, 256-KiB combined output
cap and one aggregate record of at most 8,192 bytes. Synthetic cap/deadline
children run before each observation pair. Successful processes must exit 0,
have empty stderr and zero warning/error diagnostics, emit the expected selected
world and revised key set, and create only the supervisor's two logs. The original
is hashed before/after through read-only handles with SHA-256, length, write time
and file identity comparisons. Official, accepted task-0009 backup and task-0010
binary identities are checked before/after; final task-0009 restoration is checked
separately.

Everything needed to reproduce is external under
`0010-gfxworld-vertex-stream-boundary/`: final `observe.py` with configuration
path and a fresh tag; standalone `Unlinker.exe`; source/header snapshots and patch;
native fixtures; public source identities; actual build commands/logs; preliminary,
stale-key and final observations; source checks and restoration records. The final
authoritative pair is `observations-v3/observation.json`. Actual roots remain
only in ignored configuration/external records.

| Final artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Task-0010 `Unlinker.exe` | 4,793,856 | `0d6884ddd1d10748501b26bafc10de53de299ea65b29ce124ef379b29fff8dee` |
| Final `ContentPrinter.cpp` | 11,122 | `0fd48072c5730f4222d625385d2c9f4f4f1d4128dbfb2530780291386ecf2fb8` |
| `VertexObservation.h` | 8,795 | `f0beb458fb7a766ada256c9c32c1e1077588fbb9ffb8ef982b222036e9f60185` |
| `VertexObservationChecks.h` | 737 | `a06b06106b5336a6f898cfc0a28c3a6dae4e1f7c9f9f89f369709ec3fe074512` |
| Final `observe.py` | 9,780 | `7a92a36642c120c4d0c7479b060b1d4364639612cbbbab7875e08fa1d3cfeb6a` |

## Actual validation and limits

All 22 new native synthetic checks and the accepted 26 checks passed before
original observations and on subsequent builds. They cover signed/null/zero
cases, truncation, checked arithmetic and overflow, policy rejection, zero-stride
rejection, alignment, sentinel categories and overlapping interval coverage.
These are tooling tests, not gameplay or encoding comparisons.

The initial Release/Win32 build of the same `Tools\UnlinkerCli` target completed
in 224.781 seconds under its 900-second bound. A brief check found accepted
task-0009 compiler host `HostX86` versus task-0010 `Hostx64`. Changing the tracked
compiler path is a supported explanation for broad dependency recompilation,
not a proven sole cause; preserve the same host/compiler path for incremental
work. No global environment or toolchain changes occurred.

A header-only follow-up build exited 0 in 11.984 seconds but did not compile
`ContentPrinter.cpp`; its pair still emitted the earlier key set. That attempt
is retained as stale-key evidence and is not accepted as follow-up validation.
Changing the production translation unit forced recompilation: revision 2 built
in 12.406 seconds and passed two fresh-key observations; the final group-interval
revision built in 14.297 seconds and passed its own fresh-key pair. No build
deadline extension was needed. Source snapshots, all binaries and logs for the
different attempts remain separate externally.

| Authoritative final observation | Run 1 | Run 2 |
| --- | ---: | ---: |
| Exit code | 0 | 0 |
| Elapsed seconds | 1.203 | 0.719 |
| Combined captured bytes | 6,163 | 6,163 |
| Observed peak working set bytes | 391,036,928 | 391,303,168 |

The preliminary, stale-key, revision-2 and final pairs total eight successful
same-input loads; each pair preserves original/binary identities and has no
unexpected outputs. Only matching records from the same instrumentation revision
are compared as complete records. The final pair matches the earlier shared
aggregates and adds the group-interval discriminator. All supervisor synthetic
limit checks passed. No vertex-byte reads or live game comparison occurred.

Task-0009 `ContentPrinter.cpp`, `SurfaceObservationChecks.h` and `Unlinker.exe`
were backed up before changes, restored byte for byte, and verified against their
accepted hashes from findings 0005. Its instrumentation patch also matches
exactly. The two task-0010 headers were removed from the reused checkout after
separate preservation. Eleven recorded upstream source hashes and the official
release binary remain unchanged. The source/signature/crypto/container/dump paths
are pristine. This establishes tooling/source preservation, not independent
authentication, original draw truth or rendering fidelity.

Outer-agent review independently inspected the final adapter/checks/supervisor,
reran all 22 new and 26 accepted native cases and both supervisor limit children,
then reproduced two final observations in 0.656/0.672 seconds with 6,163 bytes
each. All four final-revision aggregates agree exactly; originals and tools were
unchanged across the parent pair. Records remain external under
`parent-review-050831`. Final outer-agent review verified nine clean pinned
submodules, thirteen recorded source identities (eleven upstream files and two
generated loaders), nine helper/artifact hashes, the C2M blob against its pinned
tree, and restored task-0009 source/header/binary/exact patch. Documentation,
content boundaries, source links and whitespace passed. The scoped observation
evidence is reviewed and accepted; attribute encoding and draw truth remain
unverified.

All five owned documents passed checks for 41 relative links, eleven immutable
public-source anchors, the ten required handoff headings, local-root/content
boundaries and whitespace, including both new files. `git diff --check` passed.
Rust, Cargo manifests/lockfile, unsafe policies and Bevy ownership are
unchanged; Cargo gates are intentionally not rerun for documentation-only work.

## One next bounded task

Validate the candidate stream-0 position prefix on the same selected world before
position decoding/import: internally read the three candidate float32 position
components at the source-supported 36-byte addresses, after span admission;
aggregate finite-value and per-surface declared-bounds consistency across all
seven observed formats, including every zero-count case. Establish comparison
tolerances and synthetic NaN/infinity/truncation cases before originals. Emit no
coordinates, indices or asset content. This can discriminate the proposed
attribute encoding without rendering or silently omitting unsupported surfaces.

Complete stream-1/layout mapping, normal/UV/color encoding, units/axes/winding,
the original reason for selective positive counts and actual draw semantics
remain separate unresolved prerequisites for full geometry/material import.
No geometry API or Bevy rendering integration is approved by this task.
