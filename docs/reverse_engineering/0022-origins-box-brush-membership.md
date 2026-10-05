# Separate box owner identifies the one-past terminal

Task0032, 2026-10-05. **COMPLETE** for distinguishing the rejected terminal's
decoded box-owner association; data interpretation **INVESTIGATING**, original
collision/gameplay **UNKNOWN**. No world membership or Rust implementation is
admitted by this task. The ordinary `brush_index` guard remains unchanged.

## Exact observed relationship

One schema32 diagnostic, immediately before that guard, reports:

| Relationship | Observation |
| --- | --- |
| Embedded provider / ordinary brush count | 0 / 8,417 |
| Rejected node / node count / last node | 14,967 / 14,968 / true |
| Node signed count / terminal ordinal / value | 1 / 0 / 8,417 |
| Separate box-model leaf root | 14,967, equals rejected ordinal |
| Node at that root in declaring provider | In range; count 1; identical terminal-list pointer |
| Box leaf opaque brush / terrain contents | 4,294,967,295 / 0 |
| Box-model provider pointer | Absent; no substituted provider |
| Box-brush pointer | Present; outside counted brush pool; not equal to pool end |
| Rejected terminal-list pointer | Outside counted index pool; not equal to pool end |
| Map leaves / direct matching roots | 4,791 / 0 |
| Models / raw matching roots / explicit-provider matching roots | 265 / 0 / 0 |

Both declaring/embedded brush counts remain 8,417 and both ranges reject value
8,417, as in [findings0021](0021-origins-leaf-brush-index-namespace.md). The counted
index pools are identical, each 25,453 entries. All 265 model providers are absent.
The box root's lookup is explicitly **in the declaring provider**, a measured
ordinal relationship; absent box.info is not evidence of runtime provider fallback.
Direct-root counts do not establish transitive reachability or absence of overlap.
No box brush or rejected ordinary brush identifier is dereferenced.

This is a concrete counterexample to admitting every positive terminal as an
ordinary brush reference: the rejected node is explicitly named by the separate
box owner. Its exact root/count/list/value relationship distinguishes it from an
unidentified empty sentinel. It does not establish the runtime mapping from value
8,417 to box_brush, or prove that ordinary owner traversal cannot reach this node.

## Source and interpretation

Pinned OAT revision `7d027e8f89118196713e955b0e11f8404149c54d` declares separate
[box_brush and box_model fields](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1092).
The retained generated loader lines756-772 allocates or aliases box_brush and loads
box_model through the ordinary cmodel loader. Positive terminal lists can likewise
be allocated separately (lines395-438). Thus independent decoded allocations are
supported; neither observed false pool-end equality invalidates the root/list join.
The inspected T6 source contains no authentic producer/consumer convention.

The related [CoD4x producer](https://raw.githubusercontent.com/callofduty4x/CoD4x_Server/master/src/cm_load_obj.cpp)
in CM_InitBoxHull creates a box brush at the brush-pool end, a positive-one node
named by the box-model root, and a terminal at the index-pool end containing the
ordinary brush count. CMod_LoadBrushRelated appends this node last. This is an
explicit **other-engine analogy**, not T6 runtime proof. The primary page was read
at master; external byte retention failed with URLError, so no pinned revision or
byte hash is claimed for that analogy.

## Narrow next admission proposal

Preserve a separate box-owner identity and raw terminal value. A next diagnostic
membership adapter may recognize only the measured box signature: embedded
declaring provider, explicit box root equal to the final counted node, count one,
sole value equal to ordinary brush count, identical root/list relationship, and a
present box brush outside the ordinary pool. Retain null providers, opaque words
and list identity distinctions; never dereference the one-past value as ordinary.

Keep `value < numBrushes` for every ordinary terminal. Owner collection should start
from counted map-leaf roots, retain the separate box owner, and reject a selected
ordinary closure that reaches the classified box node instead of silently skipping
it. A box-signature mismatch remains a named rejection. This is a precise proposed
classification/admission change, not a change implemented here or an authentic
T6 trace rule. Signed branches/root-zero conventions and original owner reachability
remain explicit questions for the next bounded membership increment. Model providers
must not be filled in merely because their raw root ordinals are valid.

## Reproduction and validation

Only `zone/all/zm_tomb.ff`, 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`, was read;
selection `maps/mp/zm_tomb.d3dbsp`, normal signed single-zone LIST. The copied native
adapter emits scalar/equality metadata only, with a 16 KiB serialization cap,
bounded direct-root scans/prefixes and existing counted-span/alignment guards.
No graph, geometry, addresses, materials, scripts or live game input is emitted.

Compile, production relink and fixture link pass with warnings as errors. Eighteen
box cases and twelve inherited actual adapter cases pass; twenty-four validator
rejection cases and four Python syntax checks pass. Cases cover exact/mismatched
joins, null/distinct providers, wrapped/misaligned pointers, negative/out-of-range/
nonterminal roots, null lists, pool-end overflow, capacities and ordered/truncated
direct-root references. Pool overflow is rejected by the existing stage2 header
guard before any new scan. Outer review preceded the sole original read.

Original result: **DIAGNOSTIC_CAPTURE**, `admitted_membership=false`, native exit 123
at stage 3/brush_index in 1.297s. Record 1,516 bytes, SHA256
`69d3174a856b40bbb2862b1625b4d42bac9525b13b34009d7a14c3b1d221dd03`.
Captured output 510 bytes; no 120s/2MiB capture/16KiB record cap or I/O failure.
The child is reaped and capture threads joined. Before/after full hashes on the
same held source handle agree with file identity; the decoder separately name-opens
the input. No retry. External source/tools/receipts and twelve reused task0031
files are preserved. No Rust/Cargo change; unchanged workspace gates skipped.

World geometry selection, zero-side admission, transforms, masks/player hulls and
triangle/terrain/static-model/dynamic collision remain outside this evidence.
