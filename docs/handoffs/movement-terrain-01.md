TASK:
Qualify the terrain collision-family source contract and retained input availability,
especially the 214 root-zero terrain leaves and 253 AABB references.

STATUS:
COMPLETE (bounded source/availability investigation; player terrain remains unsupported)

FINDINGS:
- Seventeen complete reachable bodies qualify terrain gates, ordered traversal,
  stamp suppression, partition/triangle/vertex/packed-edge storage and result writes.
- Moving contacts require strict fraction improvement for material/partition writes;
  edge-flag-only writes and stationary incoming flags remain distinct.
- Stationary capsule helpers mix f32/f64. Packed-edge consumer uses local triangle
  ordinal despite global packed-buffer loader counts; interpretation remains open.
- Existing schemas retain no terrain payload arrays; a bounded external OAT producer
  can use the previously admitted decoded clipmap access. No producer run here.

EVIDENCE:
- Current Git diff/status, architecture/state and player-world/stationary contracts;
  assets/physics adapters, OAT T6 declarations/loader and task0034 producer/helpers.
- Disk identity, PE mapping, seventeen reachable bodies (10,511 instruction bytes),
  static mask initializer, constants and separately rehashed external schemas.
- Detailed abstract contract/availability:
  [terrain-trace-contract](../reverse_engineering/terrain-trace-contract.md).
- External task qualification, availability and independent-case receipts; original
  bytes/disassembly remain external. No original process, play/capture or invocation.

CHANGES:
- Added the linked contract and this handoff; external inspection/check helpers only.
- No Rust, shared contracts, manifests, exports, scene ledger or commits changed.

VALIDATION:
- Branch/instruction boundaries and separate disk-byte/identity checks pass for all
  seventeen bodies, including complete XYZ-mask initializer and stored constant.
- Schema33/34 identity/array-absence checks pass. All map-leaf AABB ranges valid;
  exactly 214 root-zero terrain leaves reference 253 distinct AABBs.
- Ten point-triangle f32 versus exact geometry cases and five strict sphere-face
  overlap/tangency cases pass; not native execution or complete capsule validation.
- Documentation links and whitespace checks pass. Cargo gates skipped for docs only.

FIDELITY STATUS:
INVESTIGATING for source contract; player/full-map UNKNOWN.

UNRESOLVED:
- Missing actual terrain arrays/material payload and runtime global-provider binding.
- Per-query stamp initialization/wrap policy, packed-edge interpretation and complete
  capsule/edge/vertex numerical transcription/regressions before implementation.
- No source-backed terrain exclusion, native runtime or original comparison proof.

RECOMMENDED NEXT WORK:
- Reviewed bounded external terrain producer/reader admission; numerical point subset
  and complete capsule transcription as separate tasks. Keep missing arrays explicit.

SCOPE NOT IMPLEMENTED:
- Terrain Rust implementation, full-map collision, render-mesh collision substitute,
  movement/camera defaults, original gameplay/process access, assets, commits/pushes.
