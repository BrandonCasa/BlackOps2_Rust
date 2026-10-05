---
name: weapon-analysis
description: Investigate one shared weapon-definition pattern or measure and implement one evidenced weapon transition. Use for weapon configuration and firing/ammunition semantics, not all weapons, global damage, upgrades or complete combat.
---

# Weapon analysis

## Bounded task

Choose one definition-family question or one representative weapon transition,
such as a reload interruption. Specify weapon/build/variant, source set, observed
contract, owned files and completion condition. Read
[project rules](../../../AGENTS.md), existing weapon/inventory/damage findings,
[architecture](../../../docs/architecture/README.md), tests and previous handoff.
Do not spawn agents, commit or combine all weapon mechanics into one assignment.

## Investigation

1. Inspect licensed definition/animation data read-only. Identify recurring fields,
   references, configuration versus runtime state and variant relationships;
   summarize semantics without copying definitions or scripts.
2. Compare representative definitions only to resolve the selected shared-pattern
   question. Names such as hitscan/projectile are investigative leads, not APIs.
3. Measure the chosen transition with controlled weapon, ammo, player state and
   inputs. Record build, timing reference, units and relevant perk/upgrade effects
   as conditions; do not accidentally attribute modified behavior to the base gun.
4. Trace the observed event order between input, eligibility, ammunition change,
   shot/hit generation, animation and feedback, limited to the assigned case.
   Separate observations, timing uncertainty and inferred implementation choices.
5. Inspect existing inventory, damage and Bevy fixed-step facilities before
   proposing reusable weapon components/configuration. Report missing dependencies.

## Required evidence

Record permitted source identifiers, abstract field/relationship summaries,
procedures, timing/ammo observations, boundary cases and unknowns in
`docs/reverse_engineering/`; put original comparisons in `docs/fidelity/`.
Original assets, definitions, scripts, raw dumps/captures and paths stay external.

## Implementation expectations

Implement only the assigned evidence-backed definition model or transition.
Separate reusable weapon state/simulation from Origins configuration and render/
audio presentation. Use safe Rust and Bevy composition, explicit fixed-step event
ordering and seeded randomness when applicable. Do not invent damage, cadence,
spread, ammo or reload constants, or create a parallel combat/event foundation.

## Validation

Test observed preconditions, boundary ammo/input states and interruptions within
scope using synthetic definitions. Check deterministic replay/event ordering and
explicit unsupported/invalid configuration errors. Run applicable workspace gates;
compare the stated weapon cases against the original separately from unit tests.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff) with
linked evidence and actual results. Preserve `UNKNOWN`/`INVESTIGATING` for missing
evidence and explicit `APPROXIMATE` limits; `MATCHED`/`VERIFIED` need scoped original
comparisons. Report ambiguous units/variants, contradictory timing, missing shared
damage/inventory semantics or unsupported asset relationships to the outer agent.
