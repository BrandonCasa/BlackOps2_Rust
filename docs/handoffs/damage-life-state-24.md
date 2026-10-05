TASK:
Qualify ordinary unperked solo/last-eligible lethal routing before implementation.

STATUS:
COMPLETE

FINDINGS:
- Solo player intermission blocks the synchronous laststand callback before health 1/trigger setup; last-eligible multiplayer can reach that setup.
- Session end is distinct from coop down/revive. Later intermission directly assigns current 100 without assigning maximum; terminal zero is unsupported.
- Remaining-player eligibility differs from literal count and full revive interaction eligibility. Available solo Quick Revive is excluded from the proposed no-perk loss route.

EVIDENCE:
- Findings24 links named source functions and Task01 identities; source03/06, architecture, API/tests and handoff23 inspected.
- External Task24 identity receipt: 10 existing compiled/decompiled/disassembled artifacts match lengths/hashes, Steam build 24784239. No runtime activity.

CHANGES:
- docs/reverse_engineering/damage-life-state-24.md and this handoff only. No architecture/API changes.

VALIDATION:
- Read-only identity verification and opcode corroboration of synchronous callback, indefinite wait and direct intermission health store.
- Diff/permitted-content/UTF-8 review; no proprietary text, local paths or original payloads tracked. Cargo skipped for documentation-only changes; no helper added.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Native fake-damage effects, cross-thread timing, loaded r5354 equivalence and controlled multiplayer comparison. No blanket trigger/countdown cleanup guarantee.

RECOMMENDED NEXT WORK:
- Implement explicit caller-admitted session-loss disposition separate from player downing; review numeric health semantics before selecting its API. Compare solo and last-eligible multiplayer separately.

SCOPE NOT IMPLEMENTED:
- Rust, session engine/UI, Quick Revive, native tracing, gameplay controls, commits or pushes.
