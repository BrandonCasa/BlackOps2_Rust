# Resolved damage and life state

This increment follows [licensed script evidence](../reverse_engineering/damage-life-state-01.md).
Fidelity is APPROXIMATE: player lethal routing/downed assignment are source
observations; player/actor health subtraction and actor zero-clamping remain inferred
native behavior. Positive integer player damage is an implementation admission restriction,
not a recovered VM conversion rule. Original comparisons are still required.

## Ownership and counterexamples

`bo2-gameplay` owns a Bevy `Vitality` component, validated `ResolvedDamage`, the
resolved application system, and request/result resources. `bo2-origins` depends
on gameplay and exposes the observed unperked maximum100/downed1/actor fallback150
configuration. The headless demonstration depends on both and the existing core
composition. Gameplay has no Origins dependency; no existing engine crate changes.

One private-invariant component holds current/max health, route and life state.
An enum captures the meaningful human/ordinary-actor variation without inheritance
or duplicate health components. Public constructors require positive current
health no greater than maximum. Actor terminal state is the only zero-health
state; player downed health remains positive. This prevents treating health0 as a
universal death/downing signal. Special actors with cannot-die flags are explicitly
outside this ordinary-actor constructor's admission contract.

Resolved positive damage below current health subtracts without underflow.
Equality/greater damage downs an admitted coop human or kills an ordinary actor.
Downed/dead repeats return `Ignored` without state mutation. Solo/last-player
lethal policy returns `UnsupportedPlayerPolicy` atomically; nonlethal damage still
applies. Session assembly must refresh teammate policy when eligibility changes.
No solo game-end behavior is fabricated. Current health is caller-supplied in
Origins constructors: neither100 nor150 proves actual spawn/round initialization.

The alternative of separate player/zombie health systems duplicates numeric
validation and event ordering. A universal terminal-death system contradicts the
observed positive-health last-stand and session-dependent route. Native conversion,
negative/healing damage, new actor archetypes, or engine comparison findings may
require refining this boundary rather than extending it silently.

## Bevy scheduling and deterministic admission

Pinned Bevy0.19.1 `FixedUpdate`, `TimeUpdateStrategy`, resources, query-by-Entity
and system sets were inspected before implementation. `ResolvedDamagePlugin`
registers one application system in `ApplyResolvedDamage` on `FixedUpdate`.
Bevy owns entity identity, ECS lifecycle and fixed scheduling. The plugin sets no
rate; this untimed boundary does not establish original simulation frequency.

Callers submit requests with strictly increasing u64 IDs, including across fixed
steps. Duplicate/reordered IDs fail before admission. No counter increment can
overflow; u64::MAX is a valid final ID. Accepted requests retain caller order and
are applied on the next fixed step, using direct entity lookup rather than query
iteration. Failed target lookup returns a result instead of panicking. A despawned
entity cannot alias a newly spawned entity because Bevy IDs include generations.

Producer systems must run before `ApplyResolvedDamage` and define their mutual
order; result consumers run after it. The queue does not invent a global producer
ordering or a general replay framework. Submit the same ordered inputs at the
same simulation boundaries for equivalence; differing render pacing must not
alter which inputs are admitted. Queue/result storage is caller-drained and no
capacity/drop policy is silently selected. Results persist across empty steps.

## Executable demonstration and limits

Run `cargo run -p bo2-damage-life-state-demo --locked`. It explicitly selects
initial player100 and actor150, assumes eligible teammate policy, submits resolved
nonlethal/lethal/repeated impacts, drives Bevy's fixed loop through `App::update`,
and checks positive-health downing and zero-health actor death before exit0.
Its resolved60 impact is synthetic input; it does not model every original melee
hit, impact filtering or timing. No original files are read.

Tests cover h-1/h/h+1, u32 limits, invalid values, atomic unsupported policy,
terminal repeats, missing/despawned components/entities, duplicate/reordered IDs,
and repeated replay across different spawn order and zero/multiple fixed steps
per application update. This is synthetic executable correctness and Bevy
integration, not original-game fidelity. Revival, bleedout, regen, shield,
same-attacker filtering, weapons, rounds, raw impact and game-end rules remain
outside this boundary. Resolve those in bounded evidence/implementation tasks.
