TASK:
Establish original-data evidence for one normal round lifecycle/spawn accounting boundary.

STATUS:
COMPLETE

FINDINGS:
- Exhausted pending spawn budget alone does not end a normal round: counted enemies and holds must clear, unless an explicit wait override applies.
- Successful ordinary spawn consumes pending budget; undefined result bypasses that debit. Concurrent AI/actor capacity is separate from full-round work.
- Origins installs a capture-enemy filter, but the retained shared utility does not dispatch to it. Active callback binding remains unresolved.

EVIDENCE:
- Read project state, architecture, prior directory/source findings, existing external damage-life-state-01 receipt and compiled/decompiled/disassembled scripts.
- Freshly hashed two licensed zones and three retained scripts; freshly decoded five views with GSC Tool 1.4.10.333, all exit 0.
- Named symbols/opcode anchors, full hashes, abstract observations and reproducible read/emit procedure: [normal-round-accounting](../reverse_engineering/origins-mechanics/rounds/normal-round-accounting.md).
- Raw script views and identity receipt remain external in inspection_output/mechanics-rounds-01. No original runtime input, process injection or live measurements.

CHANGES:
- Added one bounded findings record and this handoff only; no production code or approved API changes.
- Proposed shared ownership separates pending budget, queried enemy membership, capacity and holds; Origins policy remains above reusable simulation.

VALIDATION:
- Current input/script hashes match prior receipt where available; fresh _zm and zm_tomb decompiled hashes match retained views.
- Cross-checked completion predicate, spawn debit and entity filtering in disassembly; recorded decompiler/loader limitations.
- Relative links and privacy checked; no copied proprietary text or machine-specific paths in tracked candidates.
- Cargo gates not run: documentation-only task. Synthetic cases/comparison procedures proposed, not executed.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Active map callback dispatch, zone/patch load order, exact runtime entity membership and timing remain unproved.
- Player-count formula/type semantics, active configuration, replacement ledger, special populations and restart ordering need separate tasks.

RECOMMENDED NEXT WORK:
- Resolve active enemy-query callback dispatch or replacement using a bounded original-data patch/consumer trace.
- Investigate player-count scaling and one special-population boundary separately, reusing retained originals.

SCOPE NOT IMPLEMENTED:
- Rust/Bevy round gameplay, damage/downing/revival, other Origins mechanics, shared indices/state, commits/push, live game control or capture framework.
