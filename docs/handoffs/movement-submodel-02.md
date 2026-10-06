TASK:
Implement the reviewed raw translation/supplied-matrix classic model query adapter.

STATUS:
COMPLETE

FINDINGS:
- Explicit supplied pose and family closure preserve native recenter/translation, mixed row-dot order and strict-fraction transpose normal promotion.
- Centered hulls remain unrotated; normals retain scale, inside retained-normal behavior and native2c state. Translation/identity-matrix branches remain distinct.
- Typed callback contact/error values propagate with transactional authoritative result publication; callback workspace side effects are outside this transaction.

EVIDENCE:
- [Source/implementation contract](../reverse_engineering/transformed-submodel-trace-contract.md), eight source-qualified native bodies from submodel01 and existing shared TraceResult/TraceError.
- Independent external submodel02/reference.py fixture rounds inputs to f32 and supplies canonical non-axis endpoint/hull/normal words; no original invocation.

CHANGES:
- crates/physics/src/transformed_model_trace.rs and tests/transformed_model_trace.rs; brief contract notes and this handoff.
- Primary supplied crate export; no other shared files edited.

VALIDATION:
- cargo test --locked -p bo2-physics --test transformed_model_trace:9 passed.
- cargo clippy --locked -p bo2-physics --all-targets -- -D warnings:passed after preserving operand selection through ordered tuple inputs.
- Owned-file rustfmt check, document links/path guard and whitespace checks passed.
- Workspace integration belongs to primary; no game/window/original comparison performed.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Scene instance pose/activation and geometry identity, selector/family aggregation, complete terrain hull queries and native player receiver binding.
- Angle-to-matrix/x87 trigonometry and original comparison remain separate.

RECOMMENDED NEXT WORK:
- Review and integrate the adapter, then connect qualified ordinary-model consumers with explicit scene pose/activation admission.

SCOPE NOT IMPLEMENTED:
- Angle basis, scene defaults/generation, world/selector dispatch, new families, dynents, receiver, motor, camera, original access or commits.
