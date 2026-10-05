# Origins candidate decoded asset directory

Recorded: 2026-10-04. Question: can a pinned external decoder list the actual
asset directory of the single licensed `zone/all/zm_tomb.ff` without dumping
assets? **Result:** two OAT LIST runs succeeded with the same 7,549 entries across
31 types. Asset availability remains **INVESTIGATING**; gameplay and visual
fidelity remain **UNKNOWN**. This establishes a decoded directory, not complete
map dependencies, usable geometry, or a comparison with the original game.

## Source and verified tool identity

The source is the unchanged candidate from [catalog 0001](0001-local-asset-catalog.md),
Steam Zombies manifest build `24784239`, English labels. The game was not launched.

| Identity | Value |
| --- | --- |
| Relative input | `zone/all/zm_tomb.ff` |
| Input length | 78,753,344 bytes |
| Input SHA-256 | `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5` |
| Official release | OAT `v0.33.0`, commit `7d027e8f89118196713e955b0e11f8404149c54d` |
| Download archive | `oat-windows.zip`, 3,636,251 bytes |
| Archive SHA-256 | `4bf366cec03558a4b0f20bc8f197a51f4b9dad0aa3edc52b0e40fef2dd95a4f5` |
| Executed binary | `Unlinker.exe`, 4,756,480 bytes |
| Binary SHA-256 | `9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63` |
| Reported version | `OpenAssetTools Unlinker v0.33.0` |

The archive was obtained only from the official
[release](https://github.com/Laupetin/OpenAssetTools/releases/tag/v0.33.0).
Its length/digest matched the pinned
[release API metadata](https://api.github.com/repos/Laupetin/OpenAssetTools/releases/tags/v0.33.0)
before unpacking or executing. Archive member names, resolved destination
containment, symlink absence and expanded-size bounds were checked. All package
files remain below external `inspection_output/tooling/OAT/v0.33.0`.
The package's bundled support files are tool files, not extracted local BO2 assets.
No global installation, PATH change, source build or additional runtime was needed.
Public OAT source is GPL-3.0; no implementation or cryptographic constants were
adopted into this Rust repository.

## Read/emit procedure and execution limits

1. Resolve both external roots from ignored `config/local_assets.toml`; assert
   they are outside the repository. Open only the stated fastfile for identity
   hashing, checking length/write time/file identity from the same handle.
2. Run verified `Unlinker.exe --version` and `--help` in isolated external
   directories. Both exit 0; help confirms `--list` and `--no-color`.
3. In a new empty external working directory, invoke exactly
   `Unlinker.exe --list --no-color "<installation_root>/zone/all/zm_tomb.ff"`.
   Supply one explicit input, with no preload, search-path, dump, conversion,
   include/exclude or zone-definition options.
4. Supervise with a 120-second process deadline and 8,388,608-byte combined
   stdout/stderr capture limit. Concurrent pipe readers terminate the child on
   output exhaustion; the supervisor polls without blocking the coordinating
   shell. Either limit means incomplete observation. Sample the Windows-reported
   peak working set; this is observation, not a hard memory limit.
5. Keep raw stdout/stderr externally. Parse only the section between the exact
   zone/`Content:` framing and its blank terminator. Split the first type/name
   separator, validate types against the release's T6 type table, and exclude
   diagnostics. Emit aggregate counts and the five selected map identifiers only
   into tracked findings. Do not retain a complete asset-name list in Git.
6. Repeat the same listing once because the first run was small and successful.
   Compare ordered type/name pairs, totals and directory digest. Rehash and
   compare source length/write time/file identity; review process directories
   and the executable identity for unexpected files or mutation.

The stock tool reads/decrypts/decompresses the full fastfile and allocates decoded
zone data. This is not the existing inspector's 312-byte metadata read guarantee;
no hard decoder memory or expanded-byte bound was investigated. Original data is
loaded in memory, while no asset dump/conversion is requested. Raw names/logs and
public-source copies remain in external `0008-origins-asset-directory/`.
The external helper `0008-origins-asset-directory-observe.py` accepts the local
configuration path and an optional unused output tag for reproduction. Metadata
records are `0008-origins-asset-directory.json` and `0008-validation.json`; raw
process streams and complete pair records remain beside them. The helper is
local evidence tooling, not a shipped Rust loader or game dependency.

| LIST run | Exit | Elapsed seconds | Combined captured bytes | Observed peak working set bytes | Warnings/errors |
| --- | ---: | ---: | ---: | ---: | --- |
| 1 | 0 | 0.765000 | 300,766 | 391,651,328 | 0 / 0 |
| 2 | 0 | 0.781000 | 300,766 | 391,491,584 | 0 / 0 |

Stderr was empty. Neither budget was exhausted. These times include process
startup/capture/supervision and are local access observations, not gameplay
performance measurements. Each process working directory contained only the
supervisor's two log files afterward, with no extra files or directories. Source
SHA-256, length, last-write time and file identity agreed before/after; binary
SHA-256 also remained unchanged. No originals were copied or written.

## Direct directory observations

Both ordered listings contain 7,549 entries and 7,549 unique raw `(type, name)`
pairs. There are 7,447 unique raw names across types; a shared name is not an
observed dependency. The canonical ordered type/name stream SHA-256 is
`e615e82d6225419c70bc1887b60855aa5eaccc403dfe9cfc6b37d43080618b80`.

| Listed type | Entries | Leading-comma names |
| --- | ---: | ---: |
| attachment | 14 | 1 |
| attachmentunique | 56 | 0 |
| camo | 34 | 0 |
| clipmap | 1 | 0 |
| comworld | 1 | 0 |
| footstepfxtable | 2 | 0 |
| footsteptable | 8 | 0 |
| fx | 463 | 62 |
| fximpacttable | 1 | 0 |
| gameworldmp | 1 | 0 |
| gfxworld | 1 | 0 |
| image | 1,985 | 61 |
| keyvaluepairs | 1 | 0 |
| lightdef | 5 | 0 |
| mapents | 1 | 0 |
| material | 1,686 | 122 |
| menu | 3 | 0 |
| menulist | 4 | 0 |
| physpreset | 12 | 1 |
| rawfile | 53 | 5 |
| script | 60 | 0 |
| skinnedverts | 1 | 0 |
| soundbank | 1 | 0 |
| stringtable | 1 | 0 |
| techniqueset | 254 | 42 |
| tracer | 15 | 0 |
| vehicle | 4 | 0 |
| weapon | 129 | 0 |
| xanim | 1,735 | 22 |
| xmodel | 1,015 | 16 |
| zbarrier | 2 | 0 |
| **Total** | **7,549** | **332** |

One entry each of `gfxworld`, `clipmap`, `comworld`, `mapents` and `gameworldmp`
has identifier `maps/mp/zm_tomb.d3dbsp`; none has the comma marker. This is direct
decoded directory evidence for the `zm_tomb` map-world families. Directory
presence alone does not establish the payload's properties or renderability.
The 129 weapon entries and related families are future investigation leads;
their mechanics/ownership/relationships were not inspected. Script/rawfile names
were counted only, with no script text emitted into tracked work.

## Public-source interpretation and limits

All links below target the exact release commit; anchors use actual raw-file
line numbers. The release's
[arguments](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Unlinking/UnlinkerArgs.cpp#L324-L325)
select LIST. Its
[object-loading guard](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Unlinking/Unlinker.cpp#L120-L123)
skips raw external container loading in LIST; the
[listing branch](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Unlinking/Unlinker.cpp#L218-L224)
is separate from dumping. The
[content printer](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Unlinking/ContentLister/ContentPrinter.cpp#L13-L23)
prints only pool entry types/names. It does not expose dependency edges, decoded
field values, geometry buffers, external image resolution, patch order or asset
completeness. The parser independently accepted all observed types against the
[T6 type table](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/GameT6.cpp#L11-L73).

There are directly observed 332 leading-comma names and 7,217 other entries.
OAT's [reference predicate](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCommon/Pool/XAssetInfo.cpp#L96-L103)
interprets that marker as an asset reference. Thus these are reference entries
under OAT's convention, not 332 proven unresolved dependencies or known owning
zones. Listing without preloads/raw containers cannot establish resolution.

For the supported signed little-endian PC path, the release
[loader factory](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneLoading/Game/T6/ZoneLoaderFactoryT6.cpp#L347-L366)
adds signature verification after content loading. The
[verification step](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneLoading/Loading/Steps/StepVerifySignature.cpp#L21-L38)
throws on rejection, and the
[step executor](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneLoading/Loading/ZoneLoader.cpp#L76-L98)
returns load failure on that exception. **Source-based interpretation:** the
successful LIST loads indicate acceptance by this tool's signature path; no
authentication bypass was requested. This is not independent cryptographic
verification, a negative-signature experiment, or all-installation integrity
proof. No keys, signatures or hash-state values are retained in tracked output.

## One actionable next investigation

Observe the one listed `gfxworld`'s **surface triangle-range to world-draw buffer
and material-reference boundary**. On this same input only, measure aggregate
surface/vertex/index counts and whether every surface's declared triangle/index
and vertex ranges fit the corresponding draw buffers; record material-reference
resolution counts within the loaded zone. Keep vertices, indices, coordinates,
complete material-name lists and map dumps external/unemitted. Do not implement
rendering or assert reference ownership based on this listing.

This question is grounded in the public
[world-draw declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1231-L1247),
[surface-triangle declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3901-L3912)
with their [surface material field](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3914-L3923),
and [loader surface-count schema](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/GfxWorld.txt#L144-L178).
These are source leads, not observed values or an approved Rust schema.
Stock LIST cannot emit these fields, and
[T6 dumper registration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ObjWriting/Game/T6/ObjWriterT6.cpp#L52-L57)
does not register a `GfxWorld` dumper. A separately assigned bounded **external
metadata adapter** around the pinned decoder is therefore the next access route;
first corroborate the field/range/material semantics and its read/emit contract.
It must emit permitted measurements only, with resource limits, source-preservation
checks and synthetic range/reference validation. It is not permission to dump
other families or copy the GPL loader into `bo2-assets`.

The observations will constrain a later safe, reusable Bevy asset-access boundary
before a representative surface is adapted. Coordinate/unit/winding/vertex
encoding, textures, lighting, collision, geometry completeness and original-game
visual comparison remain separate unknowns.

## Validation

Ordered pair comparison and directory summaries agree exactly across both runs;
all table counts sum to 7,549 and marker counts to 332. The parser's external
synthetic checks accept a reference marker/name containing a comma separator and
reject unknown types, a diagnostic inside the asset section and malformed framing.
The same supervisor was checked with two user-created Python children, without
opening originals: a combined stdout/stderr stream exceeding 8 MiB was killed at
the capture cap and reported incomplete; a sleeping child was killed at a short
0.1-second test deadline and reported timeout. Both exited unsuccessfully and
left only supervisor logs. The external self-check helper and results are retained.
Input/binary identity, budget and process-directory checks passed. Documentation
links, path/content boundaries, handoff format and `git diff --check` passed.
Outer review independently verified all 162 unpacked package files against the
verified archive, compared immutable primary sources, reproduced two matching
LIST runs and both synthetic supervisor limits, and confirmed unchanged identities.
All 7,549 ordered entries agreed exactly with the original two observations.
Parent runs took 0.687/0.735 seconds and captured 300,766 bytes each; their records
remain in external `0008-origins-asset-directory/parent-review/`.
Unchanged Cargo gates were not rerun for this documentation-only task. No gameplay,
rendering, decoded-field or live original-game validation is claimed.
