TASK:
Implement the source-qualified raw per-instance classic static point consumer.

STATUS:
COMPLETE

FINDINGS:
- Endpoint column transform, ordered surface/triangle masks/bounds, strict
  fraction ties, allowance/tolerance and world-normal row promotion are executable.
- Accepted hits clear both inside flags, preserving opaque +2c; all misses,
  ties and errors leave incoming state unchanged, including late arithmetic errors.
- Negative final bones have explicit atomic safe-adapter rejection. Native can
  write a local result before skipping promotion; all 510 licensed bones are
  nonnegative, so this restriction excludes no retained participating surface.

EVIDENCE:
- [Source and implementation contract](../reverse_engineering/static-xmodel-collision-contract.md)
  and [admitted payload](../reverse_engineering/static-model-payload-admission.md).
- Complete admitted 29acc0/00dfd0/1831e0/275950 bodies and retained bone metadata.
- Owned external movement-static-model-03 Python source-equation fixtures with
  explicit f32 stores; non-axis reassociation changes expected fraction/normal bits.

CHANGES:
- Added physics static_point_trace module and synthetic tests.
- Reused shared TraceResult/TraceError; added narrow request/effect/contact/error.
  Contacts expose source ordinals and bone rather than native runtime pointers.
- Primary owns export. No shared result fields, dependencies or other queries changed.

VALIDATION:
- Ten focused locked physics tests passed: independent exact-bit front/non-axis
  cases, transforms/shared providers, surface/bounds/masks/LOD, on-plane and
  stationary points, ordered ties, tolerance boundaries, fraction0 flags,
  normalization guard, invalid inputs/references/negative bones and atomic errors.
- Focused locked physics all-target warning-denying Clippy, owned formatting,
  documentation links and Git whitespace checks passed.
- No original/native invocation or comparison; primary owns workspace integration.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Runtime sector traversal/order, stationary/swept player hull, collmap geometry
  and player motor backend/provider binding remain separate tasks.
- Negative-bone local-write behavior is deliberately not published by this adapter.

RECOMMENDED NEXT WORK:
- Assemble qualified classic point-family aggregation only after runtime ordering
  evidence; retain separate motor and player-hull provider qualification.

SCOPE NOT IMPLEMENTED:
- PlayerCollisionWorld/full-map coverage, sector construction, hull/capsule,
  collmaps, ECS/controller/camera/host, native execution, original capture or commits.
