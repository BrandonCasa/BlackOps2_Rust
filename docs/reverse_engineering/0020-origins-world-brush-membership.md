# Clipmap brush membership: unresolved terminal index namespace

Task0030, 2026-10-05. **INVESTIGATING** for data relationships; original runtime
collision/gameplay fidelity remains **UNKNOWN**. The source establishes separate
map-leaf and model-leaf references to shared collision data. The single original
observation rejects a terminal index join, so no static-world collection is admitted.

## Source-backed ownership boundary

The retained OAT revision is `7d027e8f89118196713e955b0e11f8404149c54d`.
[T6 declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1021)
and [clipmap rules](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/clipMap_t.txt#L40)
declare these relationships:

- ClipInfo has independently counted brush, leaf-node and u16 leaf-index pools.
- The clipmap has a counted map-leaf array. Each leaf has an integer node root,
  opaque brush/terrain contents and separate collision-AABB references.
- Each counted cmodel embeds its own leaf and references a ClipInfo through a
  reusable pointer. Generated loader lines515-543 permits allocation or aliasing;
  shared provider or brush storage does not make the owners equivalent.
- A positive signed leafBrushCount selects a terminal list of that many u16 values.
  Generated loader lines395-438 permits a separately allocated list or pointer
  conversion; the list need not be a span of the counted global leaf-index pool.
- Zero and negative node tags retain branch offsets and scalar split fields.
  The loader does not supply the runtime meaning of those branches or root zero.

Consequently a brush pool is a storage/identity boundary, **not a static-world
ownership list**. The 8,417 pooled brushes measured in task0028 cannot all be
classified as world-space statics from their pool membership alone. Model index
zero, transforms and original runtime provider selection remain unproved.

An [other-engine CoD4x producer](https://github.com/callofduty4x/CoD4x_Server/blob/master/src/cm_load_obj.cpp)
computes relative child offsets and uses negative -1 to add the adjacent node.
It also permits zero nonaxial sides after removing six axial sides. This is an
analogy for the shared layout, **not T6 consumer evidence**. That online source
was read at master; a separate pinned-byte retention attempt failed. The T6
declarations preserve endpoint triples and six axial flag pairs, but this task
does not establish an original zero-side brush's geometry or runtime use.

## Bounded proposed exchange and interpretation

The external schema30 candidate retains ordered provider IDs, map/model roots,
signed node tags, opaque contents, branch offsets/float words, terminal values
and nullable joins into the counted index pool. A null model provider remains
null. Brush metadata records side/vertex counts and finite-bound/span status;
no planes, vertex bodies or geometry coordinates are emitted by this task.
Parallel bounds/contents remain optional as established in
[findings0019](0019-origins-collision-brush-recovery.md).

The sole candidate interpretation is explicitly diagnostic: roots zero are empty,
child offsets are relative, and negative -1 adds node+1 as well as both children.
Other negative values, cycles and range failures retain an unavailable candidate
status. A separately labelled null-model/embedded-provider assumption is bounded
analysis, never a substitution for an observed provider. No masks are interpreted.

Limits: 16 providers, aggregate 100,000 nodes/65,535 brush metadata entries/
1,000,000 index-pool values/1,000,000 terminal visits, 65,535 map leaves and 4,096
models. Candidate traversal charges both nodes and terminal visits to a two-million
work budget and retains at most 131,072 aggregate candidate indices. Serialization
enforces 2 MiB **during writes**, before opening the record. Alternate analysis
uses a separate two-million work budget and one running union, rather than keeping
a membership list per null model. Counts, alignment, arithmetic and list/brush joins
are checked; complete native allocation extents remain decoder-trusted.

A future Rust exchange should preserve `(provider ID, brush ID)` separately from
ordered owner membership, retain ambiguous/absent provider relationships and all
raw axial/side flags and float words from the brush geometry contract. Reuse the
existing assets/physics boundary: assets admits source identities and references;
physics queries an explicitly selected owned collection; map assembly chooses its
owner/interpretation. This introduces no engine/entity registry or Origins dependency
in lower layers. Zero-side support needs an explicit geometry admission revision;
the current schema28 reader and ConvexBrush constructor require at least one side.
Triangle/terrain, collision-AABB, static XModel and dynamic-entity collision families
must remain explicitly unsupported. The schema30 candidate is **not admitted** for
original membership until the terminal namespace relationship below is resolved.

## Single original observation

Only `zone/all/zm_tomb.ff`, 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`, was read.
Selection remained `maps/mp/zm_tomb.d3dbsp`, through the existing normal signed
single-zone LIST route with no bypass, preload or extraction. Declared header
metadata and membership arrays were the scope; other collision-family bodies,
materials, scripts and live game/session input were excluded.

The observation exits123 in1.281 seconds, emits stages1-3 and the exact predicate
**`brush_index`**, captures430 bytes, and creates no record. At stage3 a positive
terminal list entry fails `value < declaring ClipInfo.numBrushes`. Header admission
had passed; candidate traversal stage4 was never reached. This does not identify
the provider/node ordinal or offending value because those scalars were not emitted.
The guard assumes that each terminal u16 directly indexes its declaring provider's
brush pool; declarations and the other-engine analogy do not prove that assumption
for this rejected original relationship.
It proves neither a corrupt asset nor a sentinel, alternate namespace, bad provider
selection or authentic world/submodel ownership. Original sign inventories,
zero-side statistics, membership counts and overlaps are therefore unavailable.

The 120-second deadline and 2 MiB capture/artifact caps are not hit. The owned child
is reaped and capture threads joined; no I/O/supervisor failure is reported. Two
independent before/after same-handle hashes, using 64 KiB blocks with a128 MiB/pass
ceiling, agree in bytes/SHA/mtime/device/file identity. The decoder separately
name-opens the checked input, so no held-handle binding or transient-name proof is
claimed. **The one-attempt allowance is consumed; no retry occurred.**

## Validation and smallest next relationship

Native compilation, production relinking against retained dependencies and fixture
linking pass with warnings treated as errors. Twenty-three actual translation-unit
cases cover normal/shared/submodel references, independent lists, missing providers,
positive/zero/negative tags, cycles/ranges/null/alignment/counts and real serialization,
candidate-index, terminal-work and aggregate-pool limits. Nine repaired checker cases
independently verify retained synthetic graph joins/sets and bounded null-model
alternates. These are synthetic results, not original membership evidence.

External `original-result.json`, `original-membership-run.json`, identity receipts,
source, executable and fixture/checker receipts preserve the failed observation.
No Rust/Cargo/production source changed; unchanged workspace gates were skipped.

The smallest missing relationship is **which provider/node/list is represented by
the rejected u16 terminal value and what namespace/range that value uses**. A later
separately bounded diagnostic needs the provider ID, node ordinal, signed count,
list-to-pool join status, offending ordinal/value, declared brush count, provider
brush-pool sharing relation and global-index-pool compatibility before
rejection. Resolve that relationship without dropping contradictions or assuming
all pooled brushes static, then admit owner membership and extend Rust collection
queries. No additional original observation is authorized by this handoff.
