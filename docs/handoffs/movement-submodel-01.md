TASK:
Qualify classic transformed cmodel adapter, query dispatch and runtime pose ownership.

STATUS:
COMPLETE

FINDINGS:
- Qualified angle/matrix adapters recenter hulls, subtract origin, row-transform endpoints and transpose-promote strictly closer normals without normalization.
- Hull extents stay unrotated; world, ordinary model and reserved box selectors remain distinct; stationary versus moving follows centered endpoint equality.
- Classic entity cmodel path supplies selector+dc/current origin+134/angles+140 gated by f9, contents/exclusions and absolute bounds. This caller supplies point hulls only.
- Active scene pose/activation/update and alternate player receiver binding remain unresolved; no universal zero pose or active-model assumption.

EVIDENCE:
- [Transformed contract](../reverse_engineering/transformed-submodel-trace-contract.md), ownership04 and existing world/brush/terrain/static contracts.
- Licensed disk identity, eight bounded bodies/1,368 instructions/5,662 bytes and independent transform checks retained externally.

CHANGES:
- Contract and this handoff only; no Rust/shared/scene edits, commits or original-file mutation.

VALIDATION:
- Before/after disk hash and exact instruction mapping pass;12 independent synthetic source-equation cases pass.
- Document links and whitespace checks pass. No original comparison, original invocation/process access or Rust integration claimed.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Runtime instance/candidate linkage, pose producers/activation/filter state, addon namespace and special box/thread provider.
- Complete non-point terrain and native player collision binding; x87 angle basis bit policy.

RECOMMENDED NEXT WORK:
- Review the supplied-pose ordinary-model adapter boundary, then qualify a scene pose/activation provider and non-point caller ownership before full-map integration.

SCOPE NOT IMPLEMENTED:
- Rust adapter, scene admission, dynents, original comparison, motor or full-map coverage.
