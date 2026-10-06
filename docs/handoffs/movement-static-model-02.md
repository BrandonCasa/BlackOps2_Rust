TASK:
Recover and safely admit the complete declared classic static point-collision payload.

STATUS:
COMPLETE

FINDINGS:
- All 10,837 ordered instances / 420 shared providers, 510 surfaces and 78,296
  collision triangles recovered and safely admitted. Native array sharing was
  checked: zero exact/partial overlaps in this retained set.
- Two collmaps retain only unresolved PhysGeomList presence/count/contents.
  Point payload completeness does not establish motor provider completeness.

EVIDENCE:
- [Payload admission](../reverse_engineering/static-model-payload-admission.md)
  and [source contract](../reverse_engineering/static-xmodel-collision-contract.md).
- Owned external movement-static-model-02 decoder adapter/producer/supervisor/
  recovery receipts; source, terrain exchange and previous producer hashes
  checked unchanged. Every static01 association and pose/metadata checked.

CHANGES:
- Added assets static_model_snapshot reader and synthetic/opt-in tests.
- Project-owned binary exchange v1 preserves exact float words, IDs, signed
  metadata, ownership/order and unresolved collmap metadata; no runtime pointers.
- Primary owns the module export. No dependencies or shared contracts changed.

VALIDATION:
- Owned adapter MSVC /W3 /WX compile/link and supervised recovery passed.
- Six focused locked synthetic tests passed; one opt-in retained payload test
  passed: 4,760,040 bytes, SHA256 a4f4024660d26cf564012c629099d64941b5b47ce77701aa89da62fe50b8fce0.
- Owned Rust formatting, locked assets all-target warning-denying Clippy,
  documentation relative links and Git whitespace checks passed.
- Primary owns final locked workspace and headless/client integration gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Runtime sector root/free-list setup/order, stationary/swept player hull and
  player motor backend/provider binding; collmap per-geom data and consumers.
- Identity declarations require independent caller authentication.
- No native invocation or original comparison evidence.

RECOMMENDED NEXT WORK:
- Implement independently qualified point consumer against admitted shared
  providers, keeping motor/backend and runtime sector work separate.
- Qualify remaining participating hull/collmap provider semantics for full map.

SCOPE NOT IMPLEMENTED:
- Numerical queries, scene construction, movement/ECS/host/camera, original
  process access/play/capture, native execution, full-map coverage claim or commits.
