# Leaf-brush rejection: one-past embedded pool identifier

Task0031, 2026-10-05. The bounded diagnostic objective is **COMPLETE**; membership
access remains **INVESTIGATING**, original collision/gameplay fidelity **UNKNOWN**.
The strict `value < declaring_provider.numBrushes` guard remains unchanged.
No static-world membership record or Rust collection is admitted.

## Observed tuple

The single normal signed LIST observation of `zone/all/zm_tomb.ff` selects
`maps/mp/zm_tomb.d3dbsp`. Input identity: 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.

| Field | Observed value |
| --- | --- |
| Declaring provider | 0, embedded ClipInfo |
| Declaring / embedded brush count | 8,417 / 8,417 |
| Brush pointers equal / counts equal | true / true |
| Node ordinal / signed terminal count | 14,967 / 1 |
| Axis / opaque contents | 0 / 0 |
| Terminal ordinal / raw u16 value | 0 / 8,417 |
| Declaring / embedded range compatible | false / false |
| Declaring / embedded counted index-pool size | 25,453 / 25,453 |
| Index-pool pointers equal | true |
| Terminal-list span admitted into either index pool | false; both first-index fields null |
| Provider matches separately loaded pInfo | false |
| Model count / absent model providers | 265 / 265 |
| Explicit model references to provider0 | 0; empty, untruncated list |

The rejected value is exactly one past the last admissible zero-based brush index.
It is not merely invalid in a smaller provider while valid in the embedded pool.
The positive terminal list passes its own count/null/alignment/arithmetic admission,
but does not join either counted index pool. Complete decoder allocation extents
remain trusted. A separate list is permitted by the loader; its separate location
does not itself imply malformed data.

The adapter scans counted nodes before deriving any owner closure. Therefore this
tuple establishes neither node 14,967's reachability from a map/model root nor its
runtime use. Null model providers remain absent; they are not replaced with provider 0.
Contents 0 is an opaque observed word, not proof of an empty collision owner.

## Source-backed alternatives and limits

Pinned OAT revision `7d027e8f89118196713e955b0e11f8404149c54d` supplies
[T6 ClipInfo/leaf/model declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1021)
and [count/reuse rules](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/clipMap_t.txt#L40).
The retained generated loader lines395-438 loads positive-count terminal lists
independently; lines515-543 loads reusable model providers. Neither establishes
the consumer meaning of this one-past identifier. See
[findings0020](0020-origins-world-brush-membership.md) for the ownership boundary.

An empty/special terminal sentinel is a candidate explanation for the equality and
zero contents. An unused or malformed node, or loader/runtime bookkeeping, also
remains possible. None is demonstrated by this scalar observation. T6 additionally
declares separate `box_brush` and `box_model` fields
([declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1092),
[reusable pointer rule](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/clipMap_t.txt#L19)).
Those fields are a bookkeeping investigation lead; they do not prove that index
8,417 refers to either field. No T6 producer/consumer convention, sentinel removal,
bit stripping, remapping or range relaxation is justified here.

## Procedure and validation

A copy of the qualified task0030 adapter emits schema31 immediately before its
unchanged `brush_index` rejection. The record contains only the scalar tuple,
pointer/count equality booleans, nullable list joins and at most 32 ordered model
references with total/truncation metadata. It emits no addresses, geometry,
materials, scripts or full graph. Serialization is bounded during writes to 16 KiB;
create/write/flush/close failures are typed and cannot count as diagnostic capture.

Native compile, production relink and fixture link pass with warnings as errors.
Twelve actual translation-unit cases pass: own/distinct/shared providers, a value
invalid locally but valid globally, invalid in both, absent/truncated model refs,
global index-pool alias, output errors and exact/exceeded serialization capacity.
Twelve independent validator rejection cases pass, including range/nullable/type/
order/truncation consistency, duplicate keys, oversized and truncated JSON.
Python syntax checks pass. The outer agent reviewed source, fixtures and controlled
framing before the one original read.

The native exits 123 in 1.172s with stages 1-3 and exact `brush_index` rejection.
The validator accepts this only as **DIAGNOSTIC_CAPTURE**, with
`admitted_membership=false`. The 735-byte record SHA256 is
`f9f341eec53fb5a43c99c4017ffca65d28e198d5fa49b6b79b000a7ec6a69d04`.
Captured stdout/stderr total 509 bytes; no 120s/2MiB capture/16KiB record limit,
I/O or supervisor failure occurs. The child is reaped and capture threads joined.
Independent before/after full hashes on the same held source handle agree with
size/mtime/device/file identity. The decoder separately name-opens the source;
held-handle decoder binding is not claimed. The one-attempt allowance is consumed.

Source, executable, raw record, native/fixture/validator receipts and identities
remain external. Limited preservation checks cover seven reused task0030 files
and seven qualified task0031 source/tool files. No Rust/Cargo changes were made;
unchanged workspace gates were skipped.

The next missing relationship is the original producer/consumer convention for
this one-past value together with the rejected node's owner reachability. Establish
those before admitting membership. Static-world queries, model transforms, masks,
player hulls, zero-side geometry and triangle/terrain/dynamic collision remain
unsupported by this diagnostic.
