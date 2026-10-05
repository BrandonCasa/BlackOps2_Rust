# GfxWorld candidate position prefix

Recorded: 2026-10-05. Question: does the selected world's 36-byte stream-0
addressing candidate expose finite float32 positions consistent with each
surface's declared bounds, across every observed format and zero-count case?
Structure understanding remains **INVESTIGATING**; overall, gameplay and visual
fidelity remain **UNKNOWN**. This task adds external aggregate observations and
documentation, without geometry import or an approved Rust/Bevy API.

## Source identity and bounds evidence

The only original input is unchanged `zone/all/zm_tomb.ff`, Steam Zombies build
`24784239`, English, 78,753,344 bytes, SHA-256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
The only selected world is `maps/mp/zm_tomb.d3dbsp`. Access uses external OAT
`v0.33.0`, immutable commit `7d027e8f89118196713e955b0e11f8404149c54d`,
with the normal signed LIST loader and signature rejection unchanged.
No other original input, dependency preload, game process, executable inspection,
raw container, copied original, shader or script inspection is used.

[Findings 0006](0006-gfxworld-vertex-stream-boundary.md) establishes the admitted
36-byte candidate addressing and 320 complete offset-group prefixes. This task
tests only the first twelve bytes of each candidate record as three little-endian
IEEE-754 binary32 components at offsets 0, 4 and 8. The candidate comes from
public C2M, pinned to `9a3accfb75c9a2f9780446c2b7bef040570357cf`:
its [record declaration](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L205-L257)
and [face addressing](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L479-L481).
Its [2.54 position conversion](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L597-L610)
is **not adopted**. C2M is third-party PC-runtime/version-limited implementation
evidence, without format dispatch or stream-1 support, rather than original-engine
truth or proof of this Steam build's rendering behavior.

The pinned [T6 declaration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3901-L3923)
places two vector fields named `mins`/`maxs` inside `srfTriangles_t`, and a
separate two-vector `bounds` array in `GfxSurface`. The
[world count](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1344-L1350)
and [loading rule](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/GfxWorld.txt#L159-L178)
count the surfaces by `GfxWorld::surfaceCount`. The preserved generated T6 loader
loads that many complete 80-byte records and subsequently resolves only the
material pointer; it does not rewrite these scalar bounds. Native compile-time
checks confirm 12-byte `vec3_t` and 80-byte `GfxSurface` for this Win32 build.

Neither inspected public source supplies an original-engine bounds consumer.
Names therefore do not settle minimum/maximum versus midpoint/half-size semantics.
Before originals, the measurement declares **two ordered-endpoint candidates**:
`tris.mins`/`tris.maxs`, and `bounds[0]`/`bounds[1]`. It independently checks each
pair for finite, non-descending components, compares the pairs for equality, and
compares only each surface's referenced positions to its own pair. A shared
group's complete prefix is not required to lie inside each member surface.
The outcomes below support one candidate and contradict interchangeability;
they do not silently establish the original purpose of either field family.

## Predeclared numeric and admission contract

External `predeclared-policy.md` and the tested helper precede any original
position/bounds reads. The exact comparison promotes decoded binary32 values to
binary64 without conversion and uses inclusive component-wise containment.
The separate numerical diagnostic expands each endpoint outward by exactly
**four binary32 `nextafter` steps**. This fixed narrow allowance tests limited
binary32 rounding at stored extrema; it is an investigation policy rather than
a measured original tolerance or permission to discard discrepancies. It is not
widened after observations. Exact and tolerant violation counts are independent.
Maximum exceeded-endpoint ULP distance is aggregate-only; its ordering coalesces
signed zero and is not a distance in world units.

Unsupported endian/float representation rejects before bytes are read. The helper
requires native little-endian IEEE-754 binary32, 4-byte floats, 24 significand
bits, and a known-pattern representation selfcheck. Every coordinate component is
copied with `memcpy`, including unaligned synthetic sources; no unaligned cast
or record-struct dereference is used. NaN and either infinity are counted rather
than silently treated as positions. Nonfinite or descending endpoints cannot
establish containment. Four-step expansion reaching infinity makes the tolerant
comparison unavailable, rather than admitting an unbounded endpoint. Signed-zero,
subnormal, exact-boundary and four-versus-five-step behavior is synthetically tested.

Admission precedes every vertex-byte read: signed/null/count/capacity/policy checks,
checked multiplication and addition for the whole 36-byte record span and its
12-byte prefix, index inside the surface's derived extent, and complete group end
inside its next offset/capacity. All groups are admitted before the first stream
body read. Surface-array admission covers a complete 80-byte record before copying
its twelve bounds floats. Native allocation extents remain trusted to the pinned
decoder; this adapter does not independently reconstruct decoded allocations.

Policies remain 100,000 pool entries/surfaces/groups; 20,000 material and
technique-set entries; 10,000,000 logical vertices/index elements/retained marker
bytes; 512 MiB stream capacity; 50,000,000 index visits per scan and position
visits per pass; 150,000,000 float visits per position pass; and 1,200,000 bounds
float reads. Two position passes imply at most 100,000,000 total position visits
and 300,000,000 position float visits. Surface metadata is bounded by 100,000
records; unique-reference markers by 10,000,000 bytes. Positions are temporary
triples, not retained geometry. These are inspection policies, not format maxima.
Material/technique-set same-pool joins precede dereferences; ambiguous, reference,
null or unmatched pointers reject. No technique/pass/routing body scan is needed
for this task; the accepted task-0010 routing limits and evidence are unchanged.

## Direct aggregate observations

Two fresh complete schema-11/revision-1 observations agree exactly. Both exit 0
with zero warnings/errors, empty stderr, no exhausted policy/deadline/capture cap,
and only the supervisor's stdout/stderr logs in each fresh process directory.

| Coverage or consistency measure | Result |
| --- | ---: |
| Surfaces / zero-count surfaces / offset groups | 6,469 / 6,236 / 320 |
| Zero-only groups | 285 |
| Complete candidate group records / finite records | 191,259 / 191,259 |
| Unique **addressed** referenced records / finite unique records | 191,259 / 191,259 |
| Referenced index/position visits / finite visits | 449,721 / 449,721 |
| Group records never referenced by any surface | 0 |
| NaN / infinity components in complete group scan | 0 / 0 |
| Surface bounds floats copied / bytes copied | 77,628 / 310,512 |
| Candidate position float visits / prefix bytes read | 1,922,940 / 7,691,760 |
| Index visits in extent scan / reference scan | 449,721 / 449,721 |
| Candidate record bytes / padding / stream-0 capacity | 6,885,324 / 1,492 / 6,886,816 |
| Group logical intervals: covered / repeated / gaps | 191,259 / 0 / 0 |
| Finite ordered endpoint pairs, each representation | 6,469 / 6,469 |
| Equal / unequal bounds pairs | 0 / 6,469 |
| Tolerance unavailable, either representation | 0 |

"Unique addressed" means distinct `(group offset, local index)` records, not
distinct coordinate triples or deduplicated mesh vertices. Group scanning visits
every candidate record; surface comparison follows every reference, including
duplicates, against that surface's own bounds. The separate unique marker totals
show that the group prefixes contain no unreferenced records in this sample.

| Format | Surfaces | Zero-count | Groups | Candidate/unique records | Reference visits | `bounds[0..1]` exact/tolerant outside visits | `tris` exact/tolerant outside visits |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 4,154 | 4,105 | 144 | 132,709 | 281,460 | 0 / 0 | 281,447 / 281,447 |
| 1 | 313 | 297 | 27 | 3,618 | 6,972 | 0 / 0 | 6,972 / 6,972 |
| 2 | 1,679 | 1,518 | 94 | 49,724 | 147,819 | 0 / 0 | 147,819 / 147,819 |
| 3 | 8 | 8 | 4 | 54 | 102 | 0 / 0 | 102 / 102 |
| 4 | 105 | 105 | 17 | 1,343 | 3,186 | 0 / 0 | 3,186 / 3,186 |
| 5 | 209 | 202 | 33 | 3,806 | 10,173 | 0 / 0 | 10,173 / 10,173 |
| 7 | 1 | 1 | 1 | 5 | 9 | 0 / 0 | 9 / 9 |
| Total | 6,469 | 6,236 | 320 | 191,259 | 449,721 | 0 / 0 | 449,708 / 449,708 |

Every format's reference visits are finite. Formats 6 and 8 are absent; no support
claim is made for them. Every surface, including all 6,236 zero-count surfaces,
has zero exact or tolerant `bounds[0..1]` violations. No tolerance rescues an
exact failure. The maximum exceeded-endpoint ULP distance for this pair is zero.

Conversely, every one of the 6,469 surfaces has at least one position outside
its embedded `tris` endpoint candidate, under both comparisons; all 6,236
zero-count surfaces also fail that candidate. The aggregate failures are substantial,
not a small boundary discrepancy: 449,708 of 449,721 visits fail, and four ULP
steps rescue none. Thirteen format-0 reference visits are inside that candidate.
Finite/ordered endpoints alone therefore cannot validate a field's world-bounds
role. The embedded and outer bounds must not be substituted for one another.
Why the embedded fields differ, and what the original uses them for, is unknown;
this is not a corrupt-original conclusion.

**Inference:** the raw, unconverted stream-0 float32 prefix is numerically
consistent with the separate surface `bounds[0]`/`bounds[1]` ordered-endpoint
interpretation across the complete sample. Together with findings 0006, this
supports a candidate position prefix for all seven observed formats and every
zero-count case. It does not prove an original GPU declaration, byte layout for
other attributes, units, axes, winding, draw semantics or appearance fidelity.

## External reproduction and preservation

All tooling/public source, fixtures, patches, binaries, build logs and observation
records stay external under `0011-gfxworld-position-prefix/`, resolved through
ignored local configuration. Reproduce with external `observe.py`, the absolute
configuration argument and a fresh alphanumeric tag. The authoritative pair is
`observations-v1/observation.json`; `reproduction.md`, `tooling-identities.json`
and `pristine-source-identities.json` preserve exact procedures and hashes.
Actual installation/output roots are absent from tracked findings.

The native decoder still reads/allocates the entire one-zone input. This is
neither a bounded-prefix parser nor a hard memory-capped decoder. Instrumentation
reads only pool/surface descriptors, admitted index elements, joined material/
technique-set format scalars, candidate first-twelve-byte prefixes and the required
surface bounds fields. It reads no other vertex attributes, stream-1 bodies,
runtime layouts, texture/shader bodies or scripts. Output is bounded aggregate
counts/categories, identifiers and hashes: no individual coordinate/index values,
original buffers, names lists, pointers or proprietary dumps.

Each isolated invocation uses only `--list --no-color <single-input>`, a
120-second deadline, 256-KiB combined stdout/stderr cap and one aggregate record
at most 8,192 bytes. Before/after original hashing uses read-only opened handles
with SHA-256, length, last-write time and same-handle file identity comparisons.
Official, accepted task-0009 backup, task-0010 and task-0011 binary identities
are checked around the pair; all remain unchanged. The normal loader's signature
acceptance is not independent authenticity verification.

| Observation | Run 1 | Run 2 |
| --- | ---: | ---: |
| Elapsed wall seconds | 7.797 | 7.407 |
| Combined captured bytes | 5,751 | 5,751 |
| Observed peak working set bytes | 391,479,296 | 390,627,328 |

These are local instrumentation timings, not gameplay/performance measurements.
Repeated `nextafter` diagnostics explain additional inspection work; no performance
optimization is attempted.

| Final external artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Unlinker.exe` | 4,792,832 | `4f94d5e8c2c76bc82f02477026f1da063bb4d00d80568d9f73d6926b5351bde3` |
| `ContentPrinter.cpp` | 2,274 | `d46501a7969dbf1ef8bfb6e43dc5c0322b9d0799a18537fd0c2f073dbcf0028f` |
| `PositionObservation.h` | 9,668 | `1f95a965904398c91aa0d84c5f6b2fda5196f6860e387586cbf2c5312c45db46` |
| `PositionObservationChecks.h` | 3,782 | `ac1fcca65951cacdb1249afefdcacbfb296c2f128dc7ef5ab38975d14dea922d` |
| `observe.py` | 10,320 | `0f495887ce6a6be76dba9697a3a8cbecc1a3e1744258f69e9dbd0565c39456e8` |

The same Hostx64 process-local Visual Studio environment and actual `.slnx`
`Tools\UnlinkerCli` Release/Win32 target were reused. Initial compilation found
typed `vec3_t` address and bounded Win32 marker-index cast corrections; both
failed logs are preserved. The final build exited 0 with no warnings in 14.625
seconds; cumulative build time was 46.312 seconds within one 900-second allowance.
Production `ContentPrinter.cpp` changes forced compilation, and the supervisor
required new schema 11/revision 1, preventing acceptance of a stale binary.
No global runtime/environment changes were made.

Accepted task-0009 source/header/binary and exact instrumentation patch were
backed up, restored byte for byte and checked against their accepted hashes.
Only the two task-0011-added headers were removed from the reused checkout after
external preservation. Thirteen recorded pristine source identities, nine clean
pinned submodules, the official binary and separate task-0010 artifacts remain
unchanged. Decoder/signature/container/dumping source was not modified.

## Validation, remaining limits and next work

Before originals, the same helper used by the adapter passed **46 new native
synthetic checks**, plus the accepted 22 vertex-boundary and 26 surface checks.
Cases cover valid, negative, null, zero, exact-end, truncated, overflow/oversize,
unsupported endian, unaligned memcpy, known IEEE representation, NaN/infinities,
invalid/reversed/nonfinite bounds, signed zero, subnormal/large/exact extrema,
one/four/five-ULP boundaries and finite-limit expansion rejection. Each build
attempt reran them. Supervisor synthetic output-cap and deadline children passed
before the original pair without opening originals. Aggregate category/accounting,
complete coverage, repeat equality, no-extra-output and identity checks passed.

Outer review independently reran all 46+22+26 native checks, the supervisor's
cap/deadline children and two fresh observations, retained externally in
`parent-review-014230/`. Runs completed in 5.375/5.406 seconds with 5,751 captured
bytes each and observed peaks 390,979,584/391,356,416 bytes. All four final records
agree exactly, and original/four-binary identities and output boundaries hold.
Review verified nineteen task artifacts, thirteen pristine source identities,
eleven upstream Git blobs, nine clean pinned submodules and exact task9 restoration.
The production project has no fast floating-point override. All 46 relative links,
six immutable source anchors, ten handoff headings, configured-root exclusions
and all-file whitespace checks passed. This bounded task is accepted.

Tracked Rust, manifests, dependencies, unsafe policies, Bevy ownership and the
previously accepted Cargo gates are unchanged; Cargo gates are intentionally not
rerun for this documentation-only task. No geometry render, original-game
comparison, independent authentication or absent-format validation occurred.
No `MATCHED`/`VERIFIED` status is claimed.

One next bounded investigation should determine whether **format 0 consumes
stream 1**, using its at-capacity surface cases and public input-layout evidence
to establish a required-stream/span rule before import. This task's position
prefix does not settle physical-stream consumption or normal/UV/color encoding.
Units, axes, winding, embedded-`tris` bounds meaning, selective counts, draw
commands, materials/textures, collision/lights/models and all gameplay remain
separate unknowns. No geometry exporter/importer, Bevy asset loader or architecture
API is implemented or approved here.
