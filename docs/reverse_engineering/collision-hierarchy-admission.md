# Collision hierarchy exchange v1

Source/data fidelity: INVESTIGATING. Complete declared hierarchy admission is
implemented; scene ownership, original query numerics and movement are not.
Read [global ownership](collision-global-ownership.md),
[stationary traversal](stationary-world-contract.md) and
[terrain admission](terrain-payload-admission.md).

## Recovery and provenance

An exclusively owned external OAT adapter copy reads the same decoded Origins
clipmap as retained schemas33/34 and terrain v1. It uses pinned OAT revision
`7d027e8f89118196713e955b0e11f8404149c54d`. Only the owned decoder executes;
the original game/process is never invoked or inspected. The producer emits
exact collision words and ordered pointer-to-index associations, not pointers,
material names, scripts or other assets. Existing producers and payloads were
reused read-only. External recovery/build/supervisor/identity receipts remain in
the configured inspection root's `movement-collision-07` directory.

Input: 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
Same-handle identity before/after each decode agrees. Schema33 membership:
2,634,532 bytes, SHA256
`6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3`.
All 14,968 node scalar fields, split words and terminal sequences agree with it.
The first 25,453 index words agree exactly with its original global index pool.

New hierarchy: **689,822 bytes**, SHA256
`228eef5911ea7aec162bc797b0a1169e1b4775798b678d1febb71664612c81e7`.
Two fresh decoder runs produce identical bytes. Complete arrays: 12,105 planes,
14,968 brush nodes, 26,037 terminal index words, 4,525 BSP nodes; associated declared
counts are 8,417 global brushes and 4,791 leaves. Leaves/models remain exclusively
in TerrainSnapshot, and geometry remains in schema34.

341 terminal lists are outside the original global index pool. Native consumers
follow their pointers, so they cannot be discarded. The producer unions overlapping
external address intervals, appends their exact ordered words after the original
pool and remaps each terminal to a checked span. This preserves sharing without
sorting brush IDs. The retained map has 584 appended words and no overlapping
external terminal-span pairs; every appended word is referenced. Shared original
pool spans are retained unchanged. External intervals are ordered by source address
within the admitted decode; byte-identical repeat recovery validates this source's
remapping, not an arbitrary future decoder's allocation policy.

## Binary layout and immutable reader

`read_collision_hierarchy(reader,length)` returns a private-construction
CollisionHierarchy with immutable ordered slices. Little-endian header216:

| Offset | Bytes | Field |
|---|---:|---|
|0|8|`BO2HIER` plus zero|
|8|4|Version1|
|12|4|Reserved zero|
|16|8|Exact exchange length|
|24|40|Source byte count and SHA256|
|64|40|Membership byte count and SHA256|
|104|40|Schema34 payload byte count and SHA256|
|144|40|Terrain payload byte count and SHA256|
|184|24|Plane/node/index/BSP/brush/leaf counts, six u32|
|208|4|Original index-pool count|
|212|4|Reserved zero|

Body arrays follow in header order, excluding the final brush/leaf count metadata:

- Plane20: normal f32 bits3, distance bits, kind u8, signbits u8, reserved u16 zero.
- BrushNode24: axis u8, flags u8, signed count i16, contents u32, distance/range
  words, relative child offsets u16x2, terminal-first u32. Positive count selects
  an index span; its union-only split fields are schema-owned zeros. Nonpositive
  count selects split metadata; terminal-first is zero. Flag1 marks external
  terminal storage, flag2 the narrowly admitted inert box association.
- Terminal indices: ordered u16 words, original pool followed by external regions.
- BSP8: global plane ordinal u32 and signed child i16x2; nonnegative child selects
  a node, negative child selects leaf `-1-child`. Children/order and sharing persist.

Native node gating occurs before children. Retained real node0 has contents0,
count0 and self offsets; it is admitted metadata, not an empty-root sentinel.
Every nonzero-content split contributes both children to graph admission; negative
count additionally contributes adjacent node `index+1`. Contents0 contributes no
traversal edge because every native mask excludes it. Structural child bounds are
still checked for all nodes. Iterative graph validation rejects every participating
cycle, including otherwise unreachable records, and all BSP cycles. No numerical
plane classifier or spatial traversal is implemented.

Node14967 is the retained special-box terminal: last node, count1, external storage,
contents0 and index8417 equal to global brush count. Its explicit marker requires
exactly this shape; other markers or ordinary out-of-range terminal references
reject. Ordinary terminal spans cannot share its missing geometry sentinel.
The native contents gate excludes this retained node in ordinary masked traversal.
The separate runtime box selector remains unresolved and requires a dedicated
provider when selected; this admission does not exclude possible box requests.

Planes must be finite. Signbits above7 reject with InvalidPlaneMetadata because
the admitted stationary classifier `11520` has only eight valid sign dispatches
and traps invalid values. Plane kind remains an opaque u8: native kinds at least3
select the general route, so no guessed maximum3 is imposed. Split axes require
0..2; terminal axes are unused metadata. Signed zero and coefficient words are
preserved; normalization and x87 numerical qualification remain query work.

## Bounded rejection and scene binding

Input is capped at32MiB; requested retained/graph-validation capacities at64MiB.
Planes/nodes cap at1,000,000 each, indices at4,000,000, BSP nodes/leaves at32,768
and global brushes at65,535. Checked count/size arithmetic and exact header,
supplied and computed lengths precede body allocation. Vectors use fallible
reservation. Allocator metadata/rounding and the supplied Read implementation are
outside the requested-capacity bound. Graph stacks are iterative and bounded by
declared nodes. Invalid read counts, truncation, I/O kinds and more than1,024 total
interruptions produce typed redacted errors. No partially admitted mutable snapshot
is returned on failure.

`check_identity` compares declarations only. The caller must independently hash
the actual source and payload files; declarations do not authenticate themselves.
`validate_terrain` checks shared source/membership identity, leaf count and every
map/model brush root without duplicating common records. `validate_brush_collection`
checks shared identity, resolves participating provider0 terminal IDs without
assuming record sort order, and verifies every referenced side plane's exact words
against the global plane array. Actual brush/terrain payload hashes are separately
bound to header declarations by the retained opt-in test. Runtime global placement,
addon namespaces, poses, activation and the alternate movement receiver remain
explicitly separate scene prerequisites.

## Validation

Ten focused synthetic tests pass: exact words/order, shared/gated graphs, negative
adjacency, all truncated prefixes, length/count limits, invalid planes/axes/flags,
terminal/BSP references, narrow box metadata, short/hostile reads, mismatched roots
and identities, unsorted geometry binding, missing geometry and conflicting planes.
The opt-in full hierarchy test passes with independently supplied hierarchy SHA256
and actual terrain/schema34 hashing; all model/map roots and participating brush
references resolve. Fresh recovery separately validates full BSP references and
both graphs against schema33. Locked assets tests, assets all-target warning-denying
Clippy, formatting and diff checks pass. Workspace integration gates belong to the
primary. No original comparison or full-scene/player acceptance is claimed.
