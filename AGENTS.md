# Project rules

## Documentation working set

Start with `docs/project-state.md`, then the source/tests and evidence relevant to
the next implementation. Dated history snapshots and old handoffs are lookup
references, not a mandatory full reading list. Keep project state concise and
replace stale status instead of appending repeated audits. Record one short
handoff per bounded task; link detailed evidence rather than duplicating it across
indexes. Documentation supports executable Rust progress and does not substitute
for implementing and validating the game.

## Goal and specification

Reimplement Black Ops II Zombies exclusively for Origins in idiomatic stable Rust
with Bevy. Preserve observable original behavior and appearance. The original
licensed game and its locally available data are the specification; do not
reinterpret, modernize, or silently approximate them.

The eventual scope includes visuals, movement, weapons, zombies and AI,
characters, audio, HUD, physics, rounds, perks, generators, robots, staffs,
Pack-a-Punch, Mystery Box, buildables, drops, quests/Easter egg systems, and all
Origins-specific mechanics. This is the overall goal, never one delegated task.

Prioritize correctness, memory safety, reusable architecture, fidelity,
deterministic gameplay, performance, and maintainability, in that order.

## Parallel movement phase delegation

> For the full-map Origins movement phase, use three concurrent subagents plus the primary integration agent. This user instruction replaces the earlier sequential policy. Each bounded task belongs to one worker with exclusive file ownership. The primary remains responsible for shared contracts, sequencing dependencies, review, validation, architecture coherence, and integration.

The outer agent owns the goal. Inspect repository state before planning, decompose
work into small bounded tasks, assign each task to one worker, and release ready
tasks as prerequisites land. Review each handoff and validate its result before
integration or its dependent assignment. Keep the available lanes on ready work;
blocked investigations identify precise missing evidence and release their slot.
Workers must not spawn subagents or combine unrelated objectives. The primary
owns manifests, lockfile, exports, boot dispatch, integration tests, project state,
shared contract changes and reviewed local commits. One numerical motor owner
implements all locomotion modes; collision families own separate modules and
reuse established consumers. No original-game play or capture during this phase.

The outer agent may perform lightweight coordination, inspection, validation,
documentation, and integration. Substantial investigation or implementation should
normally be delegated. Before each assignment, inspect Git status/diff, relevant
architecture documents, the previous handoff, existing abstractions, and relevant
tests. Review every handoff before starting the next subagent.

Every assignment must state one primary objective, scope, relevant files or
directories, required investigation, deliverables, validation criteria, exclusions,
and precise completion conditions. Split broad requests until one subagent can
deeply understand their entire scope. Keep investigation and implementation as
separate tasks when that helps establish evidence first.

Subagents inspect relevant existing work, investigate the original game/data when
needed, avoid duplicate abstractions, implement only the assigned scope, add or
update meaningful tests where applicable, document evidence and uncertainty, and
leave a coherent repository. Report adjacent work instead of implementing it
unless it is required to finish the assigned task. Do not revert another agent's
work. Return the handoff below and leave commits to outer-agent review.

## Architecture

Bevy supplies the engine/application lifecycle, ECS, scheduling, rendering, and
available engine facilities. Use Bevy composition, components, systems, resources,
and plugins where appropriate. Do not introduce a duplicate custom ECS, entity
registry, scheduler, renderer, or game-engine foundation. Investigate Bevy's
facilities before adding an engine adapter or subsystem.

Separate engine integration, reusable Zombies systems, Origins-specific
configuration/content behavior, tooling, tests, and documentation. Dependencies
point from assembly and map layers toward reusable lower layers, never back
toward a specific map. Avoid circular crate/module dependencies. See
`docs/architecture/README.md` for directory responsibilities and current members.

Reconstruct shared conceptual abstractions observable in original data or
behavior. Prefer traits, composition, enums, Bevy ECS, useful generics, and
data-driven definitions; do not mechanically translate original structures or
force artificial inheritance. Names such as weapon, light, trigger, zombie
controller, perk, inventory, and scripted event are investigative leads, not
approved APIs. Investigate before finalizing abstractions.

> If recurring behavior or structure appears in multiple original game systems, investigate whether it represents a shared abstraction before implementing separate versions.

Reject or revise integration that duplicates abstractions, unnecessarily couples
lower layers to Origins, lacks evidence, violates boundaries, introduces
unjustified unsafe code, breaks determinism, includes proprietary content,
silently approximates measurable behavior, or materially exceeds its scope.
Refactor early when evidence disproves an abstraction.

## Rust and unsafe

Use stable, idiomatic Rust and a compiler supported by the selected Bevy release.
Investigate current compatibility before pinning versions. Keep dependencies
small and justified. Keep modules focused, use explicit data and error types,
propagate recoverable errors, and avoid panics for normal runtime failures.

New Rust crates default to `#![forbid(unsafe_code)]`. The current core and boot
crates forbid unsafe code. Any exception requires a documented architectural
decision and outer-agent review. Unsafe code is permitted only when justified by
profiling, low-level integration, or FFI; isolate it, expose a safe API where
practical, document invariants, explain why safe Rust is insufficient, and include
focused tests and validation. Do not weaken a safe crate's policy casually.

Separate fixed-step gameplay simulation from rendering where practical. Define
timing, ordering, random seeds, and replay inputs explicitly when relevant to
determinism. Prefer data-oriented, cache-friendly structures for zombie
populations. Avoid unnecessary allocations and dynamic dispatch in measured hot
paths, without prematurely micro-optimizing.

## Evidence and fidelity

Investigate before approximating. Never invent behavior when it can reasonably
be measured. For every major system:

1. Inspect original implementation/data.
2. Identify shared abstractions.
3. Document observed structure.
4. Measure behavior.
5. Implement the reusable abstraction using Bevy where appropriate.
6. Implement Origins configuration.
7. Compare against the original.
8. Correct discrepancies.
9. Add regression tests.

Keep observations distinct from inferences, hypotheses, and implementation
choices. Record reproducible procedures, relevant version/build identifiers,
permitted hashes/identifiers, units, conditions, measurements, evidence limits,
and open questions. Do not copy original scripts or other proprietary content
into evidence documents. Abstract schemas and behavioral descriptions are allowed.
Implementation tasks reference supporting evidence.

Store findings in `docs/reverse_engineering/`, conclusions and boundaries in
`docs/architecture/`, important decisions in `docs/adr/`, and measurements and
comparisons in `docs/fidelity/`. Keep project state in `docs/project-state.md`
and task handoffs in `docs/handoffs/`.

Track fidelity as `UNKNOWN`, `INVESTIGATING`, `APPROXIMATE`, `MATCHED`, or
`VERIFIED`. Never claim `MATCHED` or `VERIFIED` without comparison evidence.
Approximation must be explicit, justified, bounded, and accompanied by the
measurement needed to resolve it. Missing evidence means uncertainty, not success.

## Testing and performance

Run validation appropriate to the assigned change and record actual results.
The workspace baseline is `cargo metadata --no-deps --format-version 1`,
`cargo test --workspace`, `cargo fmt --all -- --check`, and
`cargo clippy --workspace --all-targets -- -D warnings`. Use relevant focused
checks during iteration and the applicable workspace checks before integration.
Document unavailable tooling and skipped checks; do not imply they passed.

Use user-created or synthetic fixtures. Test observable behavior, error paths,
deterministic replay, and documented comparison cases where applicable. Keep live
original-game comparison distinct from unit tests, compile validation, visual
review, and performance measurements. A scaffold has no gameplay fidelity proof.

Establish correctness and fidelity, then profile. Optimization should normally be
a dedicated bounded task backed by profiling evidence. Pay attention to zombie
updates, navigation, animation, physics/collision, weapons, visibility, particles,
audio emitters, script/event dispatch, and asset streaming. Record workload,
hardware class, build mode, measurement method, and before/after results without
committing local machine paths or raw proprietary reports.

## Assets and local data

Use licensed local originals for personal use only. Original assets remain
external. Never commit, redistribute, or embed BO2 assets, proprietary binaries,
extracted copyrighted content, original scripts, local installation paths, or
proprietary generated dumps.

Local paths belong only in ignored `config/local_assets.toml`. The tracked
`config/local_assets.example.toml` contains placeholders, never real paths. Put
inspection outputs outside the repository; ignored output directories provide a
second guard, not permission to copy data into tracked files.

Repository output may contain only code, abstract schemas, non-copyrightable
measurements, behavioral/architectural documentation, appropriate
hashes/identifiers, and user-created or synthetic fixtures. Inspectors must
document what they read and emit. Review tracked candidates before adding files;
ignores alone do not establish that content is safe to commit.

## Git and execution sequence

Work locally. Use frequent, meaningful commits for coherent reviewed tasks. The
outer agent reviews the diff and validation before committing subagent work.
Never push to a remote unless the user explicitly instructs it. Do not add
secrets, machine-specific paths, generated builds, or prohibited assets to Git.
Retain the application's Cargo.lock once generated by Cargo.

Progress through bounded tasks: repository foundation; required agent skills;
locating/cataloging external assets; observable engine/game abstractions; Bevy
integration and evidence-backed entity/component and asset-loading architecture;
representative geometry; movement investigation and implementation; shared
weapon investigation/model and one weapon; shared zombie investigation/model and
one zombie; damage/death; spawning/round investigation and one round; comparison
of the vertical slice; abstraction corrections; remaining Origins systems.
Resolve toolchain availability before claiming executable validation. Do not rush
into broad feature implementation before shared abstractions are understood.

Required local skills under `.agents/skills/` are `bo2-reverse-engineering`,
`architecture-extraction`, `asset-inspection`, `origins-analysis`,
`rust-performance`, `gameplay-fidelity`, `zombie-ai`, `weapon-analysis`,
`rendering-analysis`, and `validation`. Each must define when to use it, one
bounded task's contents, investigation, required evidence, implementation
expectations, validation, handoff, and escalation conditions. Skills guide their
assigned responsibility and must not expand scope.

## Required handoff

Return this concise format and record it under `docs/handoffs/`:

```text
TASK:
<assigned objective>

STATUS:
COMPLETE | PARTIAL | BLOCKED

FINDINGS:
- ...

EVIDENCE:
- files inspected
- game data inspected
- measurements
- scripts/entity definitions
- observations

CHANGES:
- files created/modified
- architectural decisions

VALIDATION:
- tests
- comparisons
- observed results

FIDELITY STATUS:
UNKNOWN | INVESTIGATING | APPROXIMATE | MATCHED | VERIFIED

UNRESOLVED:
- ...

RECOMMENDED NEXT WORK:
- ...

SCOPE NOT IMPLEMENTED:
- ...
```

The outer agent must review the handoff, scope, evidence, diff, and validation
before beginning another delegated task.
