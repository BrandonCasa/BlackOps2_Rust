TASK:
Recover and safely admit a complete external terrain payload through retained OAT
decoder access and a bounded Rust reader, preserving accepted schemas33/34.

STATUS:
COMPLETE

FINDINGS:
- Complete external terrain exchange recovered: 509 materials, 51,129 AABBs, 18,968
  partitions, 67,196 triangles, 40,025 vertices, 25,200 packed bytes, 4,791 leaves,
  all 265 model ordinals with complete source bounds/radius/embedded-leaf metadata.
- All graph references/cycles and membership leaf fields admit; 214 root-zero map
  leaves/253 references and 27 root-zero models/28 references retained, not excluded.
- Serialized-info presence remains metadata, distinct from runtime ownership.

EVIDENCE:
- Qualified terrain contract, T6 declarations/loader, existing reader/test patterns,
  retained producer/decoder/helpers, source/membership/payload identity receipts.
- [Payload/layout/admission details](../reverse_engineering/terrain-payload-admission.md).
- External source SHA256 `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`;
  exchange 3,084,512 bytes SHA256 `e5ecb51a076898aeae66cfb6fd38ef79218768c29d3089279303e39801315a67`.
- Owned external decoder run only; no original game/process/code invocation.

CHANGES:
- assets terrain_snapshot module and synthetic/opt-in tests. Shared export owned
  by primary. Safe private construction, immutable slices, explicit identity check,
  checked binary layout/caps/references, iterative graph validation and typed errors.
- Linked format/admission documentation and this handoff. External producer/helpers/
  payload remain external; old producer/decoder/schemas retained unchanged.

VALIDATION:
- External C++ producer compile/link passes; first decode and independent binary/
  membership checks pass. Source hash/same-handle identity and old helper hashes
  agree before/after; no proprietary candidates added to Git.
- Six focused synthetic Rust tests pass; explicit full retained opt-in passes.
- Focused warning-denying all-target assets Clippy passes; initial test-style warning
  fixed. Owned Rust formatting/document links/diff whitespace checks pass.
- Final workspace gates deferred to primary integration; numerical queries not run.

FIDELITY STATUS:
INVESTIGATING for data admission; player/full-map UNKNOWN.

UNRESOLVED:
- Runtime provider/scene binding, transformed poses/activation and brush-root refs.
- Full capsule/edge/vertex/stamp numerics and packed-edge interpretation, as already
  recorded in terrain contract. Recovery does not establish native query behavior.

RECOMMENDED NEXT WORK:
- Scene owner binds this exact payload and recovered model/leaf records; separately
  qualify/implement terrain numerical increments against retained complete bodies.

SCOPE NOT IMPLEMENTED:
- Terrain numerical queries, motor/camera defaults, runtime scene ownership,
  original gameplay/capture/process access, broad extraction, commits or pushes.
