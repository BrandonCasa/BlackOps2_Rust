---
name: zombie-ai
description: Investigate or implement one evidenced zombie controller, targeting or navigation transition for one archetype. Use for shared zombie AI semantics, not complete populations, rounds, spawning or all Origins enemies.
---

# Zombie AI

## Bounded task

Choose one archetype and one controller/target/navigation question or documented
transition. Specify initial state, stimuli, observables, owned files and completion
condition. Read [project rules](../../../AGENTS.md), existing AI/zombie/animation
findings, [architecture](../../../docs/architecture/README.md), tests and handoff.
Do not spawn agents, commit or implement spawning/round systems incidentally.

## Investigation

1. Inspect relevant licensed controller/entity data and behavior read-only.
   Describe state, lifetime, targeting and movement relationships abstractly;
   avoid copying scripts or assuming entity names define reusable APIs.
2. Identify selected transition triggers, target eligibility, tie-breaking,
   interruption and animation/collision dependencies. Compare with other observed
   controllers only as needed to test whether the abstraction is shared.
3. Measure controlled stimuli such as target removal or an obstacle, limited to
   the assigned question. Record build, map region, player count/state, round,
   timing, distance units and uncontrolled variability where relevant.
4. Separate navigation intent, path execution, movement/collision and animation
   observations. Do not infer the entire navigation algorithm from one path.
5. Inspect current Bevy queries/scheduling and existing movement/navigation
   facilities before proposing controller components or an adapter.

## Required evidence

Document procedures, permitted source identifiers, abstract state/transition
descriptions, measurements, cross-controller similarities, competing hypotheses
and unknowns in `docs/reverse_engineering/`; comparisons belong in `docs/fidelity/`.
Original assets, scripts, raw captures/dumps and local paths remain external.

## Implementation expectations

Implement only the assigned, supported transition with Bevy components, resources
and fixed-step systems. Keep shared AI below reusable Zombies assembly and Origins
configuration; use explicit deterministic input/order/seed rules. Do not invent
states, distances, attack timing or targeting constants. Preserve safe Rust and
data-oriented population access without premature optimization or a second ECS.

## Validation

Use synthetic cases for the observed trigger, boundaries, target loss and
interruptions within scope. Test repeatable transitions and replay ordering;
avoid assuming incidental ECS iteration order. Run applicable workspace gates
and compare the measured cases against the original separately from unit tests.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff) with
actual results and evidence links. Unknown behavior stays `UNKNOWN` or
`INVESTIGATING`; explicit approximations remain bounded. Never claim
`MATCHED`/`VERIFIED` without comparison evidence. Report missing navigation,
collision or animation dependencies, multiplayer ambiguity, contradictions or a
shared-controller boundary issue to the outer agent; do not build adjacent systems.
