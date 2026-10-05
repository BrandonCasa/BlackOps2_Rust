# Origins mechanics: proposed shared boundaries

Recorded 2026-10-05. **Proposal for implementation review**, not an approved API,
production implementation, or fidelity claim. Source interpretation is
INVESTIGATING; original gameplay comparisons remain absent. Start with the
[coverage index](../reverse_engineering/origins-mechanics/README.md) and use the
[implementation backlog](../reverse_engineering/origins-mechanics/implementation-backlog.md)
for bounded next work.

## Existing ownership and evidence

This worktree's Cargo graph still has seven members. `bo2-core` composes Bevy
`MinimalPlugins`; boot is a headless smoke check. Geometry tools do not provide
gameplay identity, inventory, interaction or round APIs. Reserved directories in
[the architecture](README.md) are responsibilities, not established abstractions.
Do not create all reserved crates to accommodate this proposal.

The neighboring damage chat's `ca51fa5` foundation owns resolved damage through
`DamageQueue`, `DamageResults`, `ApplyResolvedDamage` and `ResolvedDamagePlugin`.
A later source snapshot was inspected read-only at HEAD `f269bb1` **with uncommitted
revive changes**: `crates/gameplay/src/lib.rs` includes `Vitality`, `LifeState`,
episode-bound revive tokens, `LifeCommandQueue`, drainable persistent results,
`ApplyLifeCommands` and `LifeStatePlugin` in `FixedUpdate`. Those later API names
are a working-tree observation, not APIs attributed to `ca51fa5` or an acceptance
receipt. None are integrated here. Reuse the owning chat's final reviewed boundary;
do not add a second player-validity or damage state machine. The original
`is_player_valid` helper still needs explicit mapping to admitted life states;
its name does not establish the mapping.

Representative original evidence supports separations, not a universal transaction:

- [Normal rounds](../reverse_engineering/origins-mechanics/rounds/normal-round-accounting.md)
  distinguish pending spawn work, queried enemy membership, capacity and holds.
- [Configured budget](../reverse_engineering/origins-mechanics/rounds/player-count-budget-02.md)
  establishes table operands and arithmetic order while leaving exact VM numeric
  results unresolved; replacement admissions need separate ledger accounting.
- [Perk purchase](../reverse_engineering/origins-mechanics/economy/purchase-boundary.md)
  separates eligibility, ordered refusal, debit, pending acquisition and ownership.
- [Pack-a-Punch](../reverse_engineering/origins-mechanics/economy/pack-a-punch-02.md)
  separates payment, weapon removal, ready return, taken notification, cleanup and
  inventory grant. Map availability loss is distinct from destructive power-off.
- [Generator capture](../reverse_engineering/origins-mechanics/world/generator-activation-01.md)
  separates a global admission lock, ownership, participant-dependent progress,
  defender recapture and linked-service eligibility.
- [Excavation](../reverse_engineering/origins-mechanics/world/excavation-02.md)
  separates personal capability, disappearing world pickup and globally depleted
  sites, including emerging sites counted before their triggers are ready.
- [Staff construction and claim](../reverse_engineering/origins-mechanics/quests/staff-build-ownership.md)
  separates shared recipe parts, carried crystal, team construction, player weapon
  grant and owner/presentation state. Staff swap validation can remove a different
  staff before the later target-claim rejection; admission is not universally pure.
- [Initial staff upgrade](../reverse_engineering/origins-mechanics/quests/staff-upgrade-02.md)
  separates unlock, insertion, death credit, delayed soul presentation, charge
  completion and pickup. Retrieved notification can occur before eligibility
  refusal; actual crypt prerequisite struct binding and death dispatch remain gaps.

More source-backed examples and counterexamples are linked in the coverage index.
Missing callback dispatch, native helpers, patch precedence and thread scheduling
are implementation gates, not permission to hard-code plausible behavior.

## Candidate data and responsibility

Names below describe proposed Rust shapes. Widths, persistence and serialization
must be selected by a bounded implementation; script numbers do not imply Rust
integer widths. Bevy `Entity` remains the live identity; stable replay keys map to
it at assembly, rather than replacing Bevy with another entity registry.

| Candidate | Shape and owner | Evidence / review limit |
| --- | --- | --- |
| Point balance | Player `Component`, checked point amount and typed debit/credit result in reusable Zombies | Purchaser and score mirror; modifier rules and net refund must be separately bound |
| Perk ownership | Player component with active and paused sets, plus a distinct pending acquisition episode | Both ownership forms reject duplicate machine purchase; pending is not grant |
| Interaction request | Tick/sequence, player, target and typed action; input adapter submits, gameplay validates | Trigger notifications/actions recur; request serialization is an implementation choice |
| Purchase episode | Component/state enum for admitted, paid/pending, acquired or canceled plus effective price and outcome | Preserve observed stages; do not promise automatic refund or erase same-frame races |
| Round state | Resource containing round identity, phase, pending budget and typed holds/override | Source completion predicate; no separate cached living count without reviewed membership contract |
| Counted enemy membership | Component/tag queried from entities, with reusable policy input | Origins callback consumer unresolved; a generic dog branch does not imply Origins dog rounds |
| Generator state | Origins component: zone key, ownership, contested progress and defender state; separate global capture resource | A single power Boolean cannot encode admission/progress/recapture |
| Eligibility dependency | Origins configuration mapping target to zone and a reusable typed admission result | Local-zone ownership gates Juggernog; service presentation is not authoritative admission |
| Inventory ownership | Small typed item/category state with explicit player, team and world scopes | Review staff, excavation and craftable counterexamples before committing a unified API |
| Progression condition | Origins typed condition/transition data with persistent completion state and typed domain notifications | Do not translate thread names into a general script VM or assume every notification is durable |
| Staff charger | Origins component for shared charger identity/count/inserted/charged/full, with base/upgraded variants in configuration | Initial charge uses accumulated credit before soul arrival; runtime callback and array tie order need admission |

Keep shared point arithmetic, perk ownership and admission below map assembly.
Keep named generator bindings, staffs, dig rewards and quest conditions in Origins.
Only extract capture progress into a reusable layer after a second independently
supported use needs the same semantics; ordinary occupancy and recapture already
provide a counterexample to one unconditional rate calculation.

Proposed dependency direction: boot -> Origins/common assembly -> reusable
Zombies/gameplay/weapon facilities -> Bevy and justified engine adapters. Tools
depend on format/diagnostic helpers, never on map gameplay. Shared Zombies owns
simulation; `game/zombies/common` assembles plugins/configuration. No reverse
Origins import, duplicate ECS, renderer, scheduler or original scripting runtime.

## Bevy facilities inspected

The workspace pins Bevy 0.19.1 and stable Rust 1.99.0; no version change is proposed.
The installed pinned source was inspected directly, including these relative
crate sources (SHA-256 of the inspected file):

| Source | SHA-256 |
| --- | --- |
| `bevy_time/src/fixed.rs` | `c3e4e4bee5d4cdde83d23e3016d7c8532b2ca00f98fbea93f34c23d316bfb963` |
| `bevy_ecs/src/message/messages.rs` | `32186c29d0c71e82420d631e779ca1309418d476bfb9a3ca0da01b78e21efc15` |
| `bevy_ecs/src/schedule/config.rs` | `8edc37532dfd3c08dae0677007f1e6a7765760d1c1cdea0bcce070ddaf4b9f41` |
| `bevy_app/src/main_schedule.rs` | `b3d5c7feb138dd2ab0f8aa47b10bef29974bf286eacf451edd963ca959ff0f56` |

`MinimalPlugins` already includes `TimePlugin`. `Time<Fixed>` supplies an engine
fixed clock and `FixedUpdate` can run zero or multiple times per rendered update.
Its default 64 Hz is a Bevy default, **not evidence of original simulation rate**.
Source script waits of 0.1 or 1 seconds are requested script delays, not an
independently measured simulation tick or wall-clock duration. Admit an explicit
duration only after the timing task in the backlog; synthetic runs label their
chosen duration as a test configuration.

`SystemSet`, `.before/.after/.chain()` and engine `ApplyDeferred` supply ordering
and deferred entity visibility. `.chain()` can insert deferred application along
dependency edges; `.chain_ignore_deferred()` does not. Budget debit and entity
admission need one reviewed commit boundary before completion queries. Blindly
chaining every system would serialize unrelated work and would not discover
original ordering.

Bevy `MessageWriter`/`MessageReader` provide transient typed notifications; the
inspected `Messages` implementation retains messages across its last two updates.
Readers that skip those updates can lose them. Use messages for same-step ordered
domain notification/presentation; persistent quest state and queued input must
survive empty fixed steps and gated consumers in resources/components. Reuse the
neighboring life queue pattern where appropriate. This is a data resource inside
Bevy, not a custom scheduler or universal event bus. Message IDs and live Entity
indices are not durable replay identity.

## Proposed fixed-step order and replay boundary

The following is a reviewable partial order, **not an observed original total
order**. Especially damage versus purchase, generator loss versus acquisition,
and simultaneous quest notifications need comparison before choosing precedence.

1. Before the fixed loop, collect input into persistent requests with stable
   player/target keys, sequence numbers and an explicit target tick.
2. In `FixedUpdate`, admit the tick's requests in specified order; apply reviewed
   life commands before any gate that explicitly consumes their resulting state.
   A different observed ordering requires a rule-specific change, not silent
   rescheduling.
3. Resolve each rule's ordered eligibility, mutations and debit/staging outcomes.
   Preserve source denial priority and side effects: staff validation can remove
   another staff before claim rejection; the perk score helper can return during
   intermission without debit while its caller continues staging. Do not force all
   requests through pure validation or an all-or-nothing payment transaction.
   Rust mutation/episode boundaries require review against these counterexamples;
   original comparison must determine observable simultaneous-event outcomes.
4. Advance map progress and explicit timers; update ownership and authoritative
   eligibility dependencies. Revalidate only at stages supported by the source.
5. Commit successful spawn admission/budget changes and retire/remap entity
   membership with Bevy deferred synchronization where needed.
6. Evaluate round completion against committed membership, pending work and holds;
   apply typed progression transitions after their prerequisite outcomes.
7. Publish typed outcomes, drainable receipts and presentation notifications;
   rendering/UI interpolate/read state outside authoritative simulation.

Candidates for plugins are a small shared `ZombiesRoundPlugin`, economy/inventory
plugin only as the first evidenced transitions warrant it, and an
`OriginsMechanicsPlugin` in map assembly. Use Bevy facilities rather than adding
a scheduler. Do not create these plugins in this documentation task.

Replay captures source/config digest, admitted build/mode, initial state, explicit
tick duration, stable keys, ordered requests, spawn outcomes, randomized decision
inputs and cancellation/ownership events. Record draws or deterministic seeded
streams with named consumers; do not assume unrelated systems can share one RNG
without changing draws. Define checked arithmetic and quantization explicitly;
fraction-based synthetic checks do not prove original VM float behavior.
Entity/query iteration order, frame count, wall-clock time, asset readiness and
thread races must not silently choose gameplay outcomes. Compare canonical
semantic state/outcome receipts, never allocator-dependent Entity IDs.

## Review examples, counterexamples and falsifiers

| Example | Proposed walk-through | What can disprove or require refinement |
| --- | --- | --- |
| Pending zero, one living counted enemy | Round query retains active phase | Active patch routes membership through a different consumer/filter |
| Unpowered Juggernog, sufficient score | Origins eligibility rejects before debit | Effective load precedence removes or replaces this callback |
| Paid bottle canceled | Pending state ends; no automatic refund invented | Another refund owner or native path establishes a different scoped result |
| Generator at zero with no participants | Raw negative change reaches clamped-zero termination | VM comparison/float or override differs from retained interpretation |
| Zero-rate generator recapture | No changed-progress endpoint branch | Runtime defender state/update replaces retained rate path |
| Service loses power during acquisition | Retain separate ownership and eligibility state | Original comparison resolves ordering/paused state differently |
| Staff recipe completed | Team built state exposes object; distinct later pickup grants weapon | Native trigger pipeline automatically submits another event |
| Staff swap target already claimed | Preserve earlier other-staff removal separately from target grant refusal | Runtime rollback/helper differs from decoded order |
| One normal dig succeeds | Personal shovel stays; shared site depletes before reward selection | Reward failure restores state through another owner |
| Refill starts at count15 | Serial immediate-increment model can reach16; readiness remains separate | Thread interleaving/override imposes a different actual ceiling |
| PaP taken notification precedes grant | Reservation cleanup and deferred trigger-off need not wait for inventory grant | Native thread order or callback adds a completion barrier |
| Same initial budget with replacement credit | Replay can admit extra actors while preserving pending work | An untraced ledger consumer changes the compared population metric |
| Charge20 precedes soul arrival | Checker consumes accumulated count; presentation event is separate | Effective actor callback/patch changes credit routing |
| Retrieved notification with ineligible pickup | Notification is not an authoritative acquired result | Another consumer mutates state before the refusal |

Inventory and quest-specific examples must be reviewed against their bounded
records before implementation. Staff possession, team build flags, dig-site
availability and random reward selection need distinct ownership, rather than
forcing all into a player item count.

No original live comparisons or production tests were performed by this proposal.
Document links, content privacy and source-to-boundary grounding are validated in
the coordinator handoff. Cargo gates are reserved for executable Rust changes.
