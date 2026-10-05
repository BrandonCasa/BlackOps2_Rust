# Replayed map-leaf brush membership candidate

Task0033, 2026-10-05. **COMPLETE** for the bounded diagnostic collection evidence;
data interpretation **INVESTIGATING**, original collision/gameplay **UNKNOWN**.
Native and independent replay select the same 8,417 ordinary brush IDs from map
leaves without reaching the separately tagged box node. This establishes a concrete
candidate collection, not authentic T6 traversal or exclusive static ownership.

## Observed collection and limits

| Measurement | Result |
| --- | --- |
| Map leaves / ordinary brush pool / model count | 4,791 / 8,417 / 265 |
| Candidate status / unique IDs / reached nodes | admitted / 8,417 / 14,551 |
| Candidate IDs | Complete ordinary range 0 through 8,416 |
| Declared nodes / positive / zero / negative | 14,968 / 10,872 / 1,620 / 2,476 |
| Negative tags / unsupported negative nodes | All -1 / 0 |
| Terminal values / lists outside counted index pool | 26,037 / 341 |
| Tagged box node / preserved raw terminal / reached by map | 14,967 / 8,417 / false |
| Root-zero map leaves / nonzero brush contents / nonzero terrain contents | 727 / 0 / 214 |
| Shared replay work / retained candidate IDs | 40,004 visits / 8,417 |
| Explicit model providers | All 265 absent |
| Selected zero-side brushes / invalid metadata | 3,178 / 0 |

The candidate equals the entire ordinary pool **as a measured traversal result**;
it was not selected by assuming every pooled brush static. Null model providers
remain absent, each candidate `provider_absent`. Model sharing, transforms,
model-only IDs and actual world/model overlap are therefore unavailable. Empty
known-model unions do not prove absence of submodel collision.

All ordinary brush metadata has finite positive endpoint bounds and valid counted
side/vertex spans. Side counts range 0-24, vertex counts 4-32. The 3,178 zero-side
records have 6-12 stored vertices. These metadata facts do not establish an axial-only
shape or agreement of source vertices with six endpoint halfspaces. Geometry bodies
were not emitted. Existing schema28/ConvexBrush requires 1-64 sides; zero-side
admission remains a separate explicit revision. No ordinary record exceeds the
existing 64-side/256-vertex upper limits.

## Exchange and interpretation

Schema33 retains ordered providers, counted index pools/nodes, signed node tags,
raw branch offsets/split float words, opaque contents, map/model roots, absent
providers, bounded brush metadata and candidate IDs/status/reached counts. It
retains the separate box owner and raw out-of-ordinary-range value. The exact
[task0032 association](0022-origins-box-brush-membership.md) is recognized only for
the embedded provider's final node named by the explicit box root, count one,
sole value equal to ordinary brush count, external terminal list and a present
box brush outside the counted pool. The root selects that admitted declaring
node/list; no independent pointer-join proof or runtime provider fallback is claimed.

Every ordinary terminal still requires `value < numBrushes`; a box-signature
mismatch does not relax it. A closure reaching the tagged node returns
`box_association_reached` before brush indexing. Relative child offsets, negative-1
adjacent-node handling and root-zero-empty remain named **other-engine diagnostic
hypotheses**, as documented in [findings0020](0020-origins-world-brush-membership.md).
Selected root zero with nonzero opaque brush contents returns
`zero_root_nonzero_brush_contents`, retaining raw evidence. Terrain contents alone
do not select brushes. No mask interpretation is performed.

Limits remain 16 providers, 100,000 aggregate nodes, 65,535 brush metadata entries,
1,000,000 counted index values/terminal visits, 65,535 leaves, 4,096 models,
131,072 retained candidate IDs and 2,000,000 traversal node/terminal visits.
The independent checker replays source order, statuses, partial reached counts,
sorted IDs and the shared work/retention budget for map then explicit model owners.
It does not admit any null-provider alternate. Record serialization is capped at
8MiB **during writes**; captured stdout/stderr remain separately capped at 2MiB.
The larger record cap was chosen before the read from the measured graph workload.
Complete decoder allocation extents remain trusted.

## Single original read and validation

Only `zone/all/zm_tomb.ff`, 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`, was read;
selection `maps/mp/zm_tomb.d3dbsp`, normal signed single-zone LIST. Input and raw
record/source/executable/receipts remain external; no geometry planes/vertex bodies,
addresses, materials, scripts or live game/session input were emitted.

Native exits 0 in 1.344s, stages 1-7, no rejection predicate. Record 2,634,532 bytes,
SHA256 `6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3`.
Candidate-ID SHA256 is
`f0adad0887fced6fe965be0e7574d5297f5714fedfa958aa05dcc231b1297ddc`
for ascending uint32 little-endian values with no header. Complete native summary
agrees with record counts and independent replay. Captured output 1,026 bytes;
no 120s/record/capture cap, I/O or supervisor failure. Child reaped, threads joined.
Before/after full source hashes on the same held handle agree with file identity;
the decoder separately name-opens it. One attempt consumed; no retry.

Native compile/production relink/fixture link pass with warnings as errors.
Thirty-nine actual adapter cases and sixteen checker tampering/framing cases pass,
including box signature mismatches/overlap, ordinary range, shared/null owners,
zero-root contradiction, branch/cycle/range/work/aggregate/serialization limits.
Five Python syntax checks pass. Outer review preceded original execution; closure
replay on the retained original record passes again without decoding. Nine reused
source/tool files and nine qualified task0033 files are preserved. Docs checks pass;
no Rust/Cargo change, unchanged workspace gates skipped.

The next executable boundary can admit this bounded metadata into an owned
diagnostic collection retaining `(provider, brush ID)` and explicit supported
geometry coverage, then reuse existing point-segment queries. Preserve the named
interpretation/box-overlap failure and unknown model owners. Geometry acquisition
and zero-side/source-vertex validation require their own bounded admission step;
this graph contains no query hulls. Player masks/hulls/movement and triangle,
terrain, collision-AABB, static-model and dynamic collision remain unsupported.
