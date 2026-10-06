# Global collision storage and declared model admission

Collision lane 04, 2026-10-05. **PARTIAL**: native storage references and retained
model payload coverage are qualified; final asset-manager placement into global
storage remains unresolved. Source interpretation **INVESTIGATING**, full-map and
player fidelity **UNKNOWN**. No original execution/process reads/input/capture.

## Source and bounded procedure

Same licensed disk identity as [stationary findings](stationary-world-contract.md):
SHA256 `2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
Scanned only text for exact address-word references, then decoded bounded candidate
contexts and nine complete selected functions (3,048 bytes). The complete selected
spans and direct branches are rechecked against a separate disk reread. Raw source,
disassembly, address discovery and model ledger remain external. Partial discovery
contexts are not interpreted as complete functions.

Rechecked identity-bound schema33 membership and schema34 geometry. Inspected
their retained producer and pinned OAT T6 declarations/loader rules. The producer
asserts 72-byte ClipInfo, 20-byte leaf node, 44-byte leaf and 76-byte cmodel layouts;
it retains actual serialized provider presence and cannot invent runtime providers.
No native decoder or observation was executed this task.

## Qualified storage graph

The native accessor RVA `0x251c50` returns the fixed collision object. Reset routine
`0x1da220` zeros exactly 332 bytes of it while preserving its first name pointer.
Native consumer globals align with this object's **embedded** ClipInfo at offset8:

| Field relative to collision object | Offset | Consumer use |
| --- | ---: | --- |
| Embedded ClipInfo materials | 20 | Moving/stationary terrain entry references |
| Embedded leaf-node array | 36 | Moving/stationary leaf roots and relative node traversal |
| Embedded brush count / array | 64 / 68 | Global u16 terminal indices into 96-byte records |
| BSP node array / leaf array | 96 / 104 | Native world traversal and 44-byte leaves |
| AABB array | 140 | Terrain references to 32-byte records |
| cmodel count / array | 144 / 148 | Selected model ordinal, 76-byte records and leaf at offset32 |

This is a coherent storage-layout observation, not evidence that a particular
paused runtime has a particular loaded map. Native brush/node consumers do not
read the selected cmodel's serialized ClipInfo pointer before resolving terminals.
All selected model leaves use the same native global node/brush namespaces.
Provider-null in schema33 therefore does **not** imply absent model geometry.

Complete serialized clipmap loader RVA `0x0c90f0` reads a 332-byte header and loads
embedded ClipInfo first. For inline pInfo sentinel branches only, it loads the
referenced ClipInfo then overwrites pInfo with the native global embedded ClipInfo
address, through setter `0x2222b0`. For serialized null pInfo it skips that branch;
for non-inline references it takes the alias conversion route. The retained map's
pInfo is null, so the inline rewrite cannot be asserted as its actual path.

Complete cmodel array loader `0x252fd0` separately loads 76-byte records and optional
reusable ClipInfo pointers. It preserves null pointers. Runtime map admission
`0x236010` invokes a selected asset lookup through `0x481290`, then initializes
collision working storage in `0x481120` and sets the global object's in-use field.
The lookup requests the pinned T6 clipmap-PVS asset category; its returned pointer
is not assigned by this caller. Final placement/copy into the global object is
therefore **not established** by the admitted caller.

Complete addon model routine `0x4812b0` looks up the pinned addon-map category,
conditionally copies the existing cmodel array into addon storage and replaces
global model count/array. An active addon changes the model namespace. Its presence,
matching selected name and payload are not measured here; no addon is silently
assumed absent. This is distinct from shared immutable brush payload storage.

## Every declared model and reference closure

All ordinals0 through264 have an external per-ordinal ledger. Native structural
closure uses the admitted global namespaces, node contents gates, signed counts,
adjacent negative-node processing and relative child references. An independent
recursive replay agrees with the iterative replay for all265 model leaves.
This enumerates possible references, not a spatial/timing/activation query.

| Model disposition | Count | Scope |
| --- | ---: | --- |
| World selector | 1 | Ordinal0 takes world dispatch rather than its empty cmodel leaf. |
| Brush only | 223 | Nonzero brush root; no terrain contents. |
| Brush and terrain | 13 | Nonzero brush root and participating terrain metadata. |
| Terrain only | 27 | Root zero; nonzero terrain contents and AABB range. |
| Empty leaf | 1 | Ordinal5: brush/terrain contents and AABB count all zero. Excluded only from this cmodel leaf query, not arbitrary entity/dynent behavior. |

The 27 terrain-only ordinals are9-19,32-39,46 and191-197. They contain28 AABB
references; all40 terrain-bearing models contain41 references. All236 brush-bearing
model closures resolve583 unique brush IDs. **Every ID is already present in
schema34**; no model-only brush payload is missing. No brush ID is shared between
distinct cmodel owners in this retained map, although every owner uses global
storage. Null serialized provider associations are retained as source metadata.

Native root-zero means indexing real node0 when the brush contents gate permits;
it is not a native empty-root sentinel. Retained embedded node0 has contents0,
count0 and self child offsets, so its mask gate excludes traversal before a cycle.
This establishes the bounded retained brush-root exclusion. It does not remove
terrain: 214 root-zero map leaves and27 root-zero model leaves participate there.

The structural map-leaf union remains8,417 brush IDs and includes all583 model
brush IDs. Consequently pool/union membership does not prove exclusive static
world ownership. Do not subtract the583 IDs from world traversal or mark all8,417
as independent world-space statics. Preserve owner roots, native mask/order and
model-instance transforms instead.

## Required scene inputs and remaining exact gap

Assets scene admission should bind the independently checked selected map identity,
embedded global payload identity, ordered BSP/leaves/nodes, cmodel ordinals and
possible addon identity/count replacement. Geometry schema34 can be reused directly
for all current model brush IDs. A new bounded producer must retain map-leaf and
cmodel/leaf bounds, model radius, material metadata, terrain references and pointer
sharing as identities. Terrain lane owns terrain arrays and these common leaf/model
headers; no duplicate producer is introduced by this investigation.

Scene owner instances additionally require model selector, transform and activation
with revision identity. Static model collision and dynent providers/poses remain
separate families. Definition-only placement cannot stand in for current pose.

The precise native binding gap is the asset-manager copy/placement from the
selected clipmap-PVS header into the fixed global collision object, including any
addon replacement association. The selected lookup helper reaches generic asset
registry logic, but that full final assignment is not qualified. The slot is
released with this named gap; null model pointers are no longer a reason to block
all brush readers or invent substitute providers.

## Validation receipts

Nine complete native spans, direct branch boundaries and before/after disk identity
pass; exact hashes retained externally. Two schema identities pass. All265 model
closures agree between independent traversals; all583 model brush IDs resolve to
existing payloads. Root-zero contents gating and full map/model union checks pass.
No Rust changed, so Cargo gates are deferred to primary integration. This is source
and reference qualification, not native runtime/world/movement comparison.
