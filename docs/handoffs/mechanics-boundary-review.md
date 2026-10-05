TASK:
Review the Origins mechanics boundary proposal against retained source findings,
existing ownership and pinned Bevy facilities; report corrections and falsifiers.

STATUS:
COMPLETE

FINDINGS:
- Existing seven-member graph has no production mechanics APIs. Proposed reusable
  simulation below Origins/common assembly preserves the documented dependency
  direction and introduces no second ECS, scheduler, renderer or registry.
- Correction found and coordinator addressed: initial proposal attributed LifeCommandQueue, revive tokens,
  ApplyLifeCommands and LifeStatePlugin to ca51fa5. That revision actually exposes
  DamageQueue, DamageResults, ApplyResolvedDamage and ResolvedDamagePlugin.
  Revised proposal separates those APIs as an uncommitted neighboring snapshot,
  requiring owner review, and identifies ca51fa5's damage boundary accurately.
- Initial proposed fixed-order step 3 needed refinement: eligibility, debit and staging are not one
  established universal atomic transition. Perk debit can return at level
  intermission while the caller continues staging; callbacks may change inputs.
  Coordinator's revised step 3 and M05 preserve this exception. Keep the
  normal-case synthetic contract explicitly scoped and this race open.
- Inventory/admission counterexample, now explicit in revised step 3: staff swap validation removes other staffs
  before the later unclaimed-target check. A rejected interaction can have earlier
  effects; do not generalize side-effect-free rejection or automatic rollback.

EVIDENCE:
- Read project state, architecture, architecture-extraction/validation skills,
  proposal, backlog, Cargo.toml/core composition and relevant previous handoffs.
- Reviewed committed [round](../reverse_engineering/origins-mechanics/rounds/normal-round-accounting.md),
  [perk](../reverse_engineering/origins-mechanics/economy/purchase-boundary.md),
  [generator](../reverse_engineering/origins-mechanics/world/generator-activation-01.md),
  [excavation](../reverse_engineering/origins-mechanics/world/excavation-02.md) and
  [staff](../reverse_engineering/origins-mechanics/quests/staff-build-ownership.md)
  records; HEAD at final inspection was 740e9b8.
- Read neighboring gameplay source using git show ca51fa5:crates/gameplay/src/lib.rs.
- Inspected pinned Bevy 0.19.1 fixed.rs, messages.rs, schedule/config.rs and
  main_schedule.rs; fresh SHA-256 values match all four proposal digests.
- No original binary/script re-decoding, runtime input or new measurements.

CHANGES:
- Added this handoff only. Proposal, backlog, findings, state and production files
  remain owned by their respective workers/coordinator; no architectural API approved.

VALIDATION:
- Walk-throughs: pending0/counted1 retains a round; pending1/counted0 also retains
  it; failed ordinary spawn does not consume budget. Custom Origins membership
  remains gated by unresolved callback dispatch, rather than a new cached count.
- Owned/paused Juggernog refusal precedes local-zone power, affordability and cap.
  Payment remains distinct from acquisition and cancellation has no invented refund.
- Generator empty-at-zero uses raw negative change before clamp and terminates;
  zero-rate recapture skips that changed-progress endpoint. Ownership, contested
  progress, global admission and service eligibility therefore remain separate.
- Staff build completion exposes world/quest state without granting ownership;
  later handled pickup grants selected variant and records base owner. Stale-owner
  clear guards and actual inventory checks disprove one interchangeable owner bit.
- Excavation consumes a shared mound, preserves personal shovel and counts an
  emerging mound before use readiness. Snow has a staff exception; <=15 admission
  can yield16 in a serial model. No universal capacity or respawn timer inferred.
- Bevy review confirms fixed0/multiple updates, default64Hz only as engine default,
  chain deferred synchronization and two Messages::update buffer retention.
  Consumers need an explicit update/ordering contract; persistent state/requests
  cannot rely solely on transient messages. No original tick frequency established.
- Relative-link scan of proposal and five findings records found one pending link:
  mechanics coverage README, owned by coordinator and not yet present at scan time.
  All other scanned relative file targets existed. Recheck pending link at integration.
- Re-read revised proposal ownership paragraph, order step3 and backlog M05:
  factual revision attribution and rule-specific counterexamples are addressed.
  Later uncommitted neighboring life APIs were not independently accepted here.
- Privacy regex scan found no absolute drive paths, common secret/key markers or
  private-key headers (rg exit1 means no matches); manual review found abstract
  descriptions/identifiers/hashes rather than original scripts or proprietary dumps.
- Cargo gates intentionally not run for this documentation-only review. Review
  examples are static semantic checks, not executed gameplay/model tests.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Effective patches/dispatch, native validity/trigger helpers, original numeric
  precision, same-frame ordering, timing and life-state eligibility remain gates.
- Persistent progression state is a candidate per-transition decision, not evidence
  that every original notification or quest stage is durable across restart/death.

RECOMMENDED NEXT WORK:
- Verify pending coverage link before integration; retain staff swap/claimed-target
  and intermission no-debit/staging cases in bounded implementation acceptance.

SCOPE NOT IMPLEMENTED:
- Rust gameplay, shared APIs, new decoding, original runtime comparison, commits,
  shared-document edits, nested delegation or adjacent mechanics investigation.
