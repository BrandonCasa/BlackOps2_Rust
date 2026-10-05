# Origins mechanics coverage

Recorded 2026-10-05. Research uses licensed Steam compiled scripts, manifest build
24784239, with retained external OAT v0.33.0 extraction and GSC Tool1.4.10.333
interpretation. Per-topic records pin input/tool identities and named anchors.
Effective runtime patch precedence and Steam/Plutonium equivalence are unresolved.
Originals and generated views stay external; no production gameplay was added.

**O**: decoded original-data observation, sometimes cross-checked in same-tool
disassembly. **I**: source-derived inference. **M**: original runtime measurement.
**P**: proposed Rust/Bevy design. Synthetic checks validate proposals, not original
behavior. Records may use S/B for source/disassembly observations; those map to O
here. All researched topics remain **INVESTIGATING**, with zero M samples in this
research batch. Adjacent untouched mechanics are **UNKNOWN**.

| Lane / mechanic | Representative evidence | Limits and next discriminating check |
| --- | --- | --- |
| Rounds: normal completion and spawn debit | [Accounting](rounds/normal-round-accounting.md), O/I | Resolve active Origins enemy callback consumer; compare exhausted pending budget with one surviving counted enemy |
| Rounds: roster budget scaling | [Configured budget](rounds/player-count-budget-02.md), O/I; 12 synthetic hypothesis/replay checks | Table operands/arithmetic order established; resolve VM divide/int/float and active roster/callback before claiming exact counts |
| Economy: perk admission/debit/acquisition | [Juggernog purchase](economy/purchase-boundary.md), O/I; 16 synthetic cases | Verify power-loss/intermission/same-frame event order, effective modifiers/cap and native acquisition timing |
| Economy: upgrade and shutdown | [MP40 Pack-a-Punch](economy/pack-a-punch-02.md), O/I; 15 synthetic cases | Compare taken notification/flag cleanup versus completed grant; resolve generic destructive power-off dispatch |
| World: generator activation/capture | [Generator](world/generator-activation-01.md), O/I; 10 synthetic checks | Bind life-validity/score helpers and mode rates; measure partial occupancy, zero-progress interruption and conditional refund |
| World: excavation | [Shovel/site lifecycle](world/excavation-02.md), O/I; 12 synthetic checks | Measure native use admission/readiness, simultaneous use, actual refill count ceiling and RNG/permutation semantics |
| Quests/staffs: build versus possession | [Air staff recipe/claim](quests/staff-build-ownership.md), O/I | Compare completed construction followed by separate pickup; verify side-effectful swap rejection and ammo/ownership return |
| Quests/staffs: upgrade prerequisite/event | [Air unlock/charge/pickup](quests/staff-upgrade-02.md), O/I | Bind actual map prerequisite flags and effective death-credit callback; compare count-before-soul-arrival and retrieved-without-grant cases |

The [shared-boundary proposal](../../architecture/origins-mechanics-proposal.md)
walks representatives and counterexamples through candidate data ownership,
Bevy facilities, partial scheduling order and deterministic replay.
The [implementation backlog](implementation-backlog.md) lists evidence gates,
expected files, synthetic tests and original comparison acceptance per small task.
Both remain proposals for implementation review.

## Explicit gaps

| Family | Evidence currently available | Still UNKNOWN / unmeasured |
| --- | --- | --- |
| Spawn eligibility and special populations | Shared spawn gates and capture-enemy callback/config leads | Native actor admission, zone/barrier eligibility, capture attacker ledger/scheduling, full special-round interactions |
| Box | Original local-zone binding/lock and shared interaction helper | Paid random selection, distribution, claim/expiry/refund, relocation, multiplayer race |
| Perk persistence and free/random grants | Active/paused ownership and pause-helper leads | Complete generator-loss/life-state/session persistence, Wunderfizz distribution/caps; coordinate damage owner |
| Robot | Original script availability and assembly calls | Cycle timing/selection, foot-entry and quest contact; lethal contact belongs to damage/collision owners |
| Buildables | Original shared/map craftable families; one staff recipe representative | Shield/other recipe ownership/assembly differences before extracting common API |
| Tank / environmental traversal | Original tank/teleporter/chamber initialization leads | Paid route/cooldown, boarding/exit, destinations and native traversal; coordinate movement owner |
| Main quest | Staff built/all-crafted flags and elemental quest family leads | Full dependency graph, resets, cancellation, multiplayer progression and final completion |
| Presentation/audio | Source feedback/animation requests | Actual duration, assets, HUD/sound correspondence; appearance owner retains integration |

Missing evidence is a backlog entry, not a fidelity pass. Do not copy generic
Zombies dog-round rules into Origins from a shared branch. Do not turn source
waits into measured elapsed time or hard-code fallback config values without
following data-table overrides. Preserve source counterexamples such as a staff
swap mutation before later rejection and a no-debit score-helper return with
continued purchase staging.

## Execution and validation

Four distinct workers were explicitly configured as GPT-6.1 Sol Medium, with clean
history at creation. The fourth was initially rejected by the tool's thread limit
and started on retry after a completed task; the rounds worker covered the first
staff task while waiting. Workers received one bounded task per turn, exclusive
record/handoff ownership and no nested delegation. Coordinator owns shared state,
index, proposal, backlog, review, commits and messages to the other three chats.
No production work was delegated to research workers.

Document/privacy/link checks and the tracked synthetic checks are the applicable
gates. Cargo gates were not repeated because no Rust/Cargo production files changed.
The coordinator handoff records final counts and reviewed revisions. Original
comparison procedures require shared-session coordination, verified pause/speed,
separate wall/game time and verified normal speed for baseline timing.

Bounded handoffs: [round01](../../handoffs/mechanics-rounds-01.md),
[round02](../../handoffs/mechanics-rounds-02.md),
[economy01](../../handoffs/mechanics-economy-01.md),
[economy02](../../handoffs/mechanics-economy-02.md),
[world01](../../handoffs/mechanics-world-01.md),
[world02](../../handoffs/mechanics-world-02.md),
[quest01](../../handoffs/mechanics-quests-01.md),
[quest02](../../handoffs/mechanics-quests-02.md),
[boundary review](../../handoffs/mechanics-boundary-review.md).
