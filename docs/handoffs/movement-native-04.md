TASK:
Qualify ordinary Origins movement parameter initialization and the C96 baseline
movement/sprint modifier dependency without sampling live objects.

STATUS:
PARTIAL

FINDINGS:
- Qualified script scale1 -> client54e0 -> state50c initialization/update copy.
- Qualified C96 baseWeaponDef ordinary movement1 and sprint-duration1; native
  getter/variant/base-definition association corroborates decoder field layout.
- Raw ADS field2 is not an effective movement claim. Runtime weapon ID/name table
  assignment and complete profile defaults remain unresolved.
- Base speed/gravity consumer and override ownership recovered; named parameter
  registration/type/range/default associations not recovered.

EVIDENCE:
- Current diffs, prior movement contracts/handoff03 and relevant source reviewed.
- Qualified disk bodies and static pointer/base/displacement candidates read.
- Pinned OAT weapon-only extraction of accepted zm_tomb.ff; c96_zm record and
  field layout/consumer checks; original assets/results remain external.
- Detailed findings: docs/reverse_engineering/movement-baseline-parameters.md.
- External movement-native-04 receipts/scripts retain hashes and reproduction.

CHANGES:
- Added baseline parameters contract and this handoff; no Rust/shared edits.

VALIDATION:
- Repeat weapon extraction reproduced identical c96_zm hash/fields; decoder
  completed without warnings/errors. Source fastfile hash rechecked.
- Bounded layout check matched native588/58c/590 and variant getter+8.
- git diff --check and owned-doc prohibited-path/content review passed.
- No original comparison or live-object sample; Cargo gates not run doc-only.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Exact registration/default/type/range mapping for named dvar slots, base speed
  and gravity; no proof of absent dynamic-module ownership.
- Weapon ID/name population and x87 startup precision/control word/conversions.

RECOMMENDED NEXT WORK:
- Implement independently qualified SSE coefficient kernels and explicit timing
  with supplied inputs, preserving profile and full-motor errors for unresolved
  evidence. Do not adopt guessed executable Origins defaults.
- Bounded registration/initializer mapping and startup floating-point policy.

SCOPE NOT IMPLEMENTED:
- Numerical motor, executable Origins profile, ADS or other weapon behavior,
  locomotion-mode transitions, collision backend and application assembly.
