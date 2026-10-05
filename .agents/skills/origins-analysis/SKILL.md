---
name: origins-analysis
description: Investigate one Origins-specific mechanic, configuration or quest transition and identify its boundary with reusable Zombies systems. Use for map-specific behavior, not generic weapons, zombie control or engine architecture.
---

# Origins analysis

## Bounded task

Select one mechanic or transition with named initial conditions, observables and
completion criteria, such as one generator activation transition. A whole quest,
all staffs or the entire map is too broad. Read
[project rules](../../../AGENTS.md), existing Origins findings, relevant reusable
systems, [architecture](../../../docs/architecture/README.md) and previous handoff.
Do not spawn agents, commit or implement adjacent mechanics.

## Investigation

1. Inspect the relevant licensed map data, entity relationships or script behavior
   read-only. Record build/mode and describe semantics without copying content.
2. Identify preconditions, participant roles, input/event sequence, state changes,
   visible/audio feedback, interruption and repeatability for the selected case.
3. Compare the pattern with existing interactable, inventory, event or progression
   abstractions. A map-specific name does not justify a separate generic subsystem.
4. Measure one controlled case and relevant boundary/failure cases. Record units,
   timing resolution, player count, round/progression state and randomness when
   they affect interpretation; distinguish observations from hypotheses.
5. Classify observed rules as reusable semantics or Origins configuration. Report
   neighboring quest/mechanic dependencies without expanding the assignment.

## Required evidence

Record procedures, permitted identifiers, abstract transition descriptions,
measurements, dependencies, uncertainty and open questions in
`docs/reverse_engineering/`; place original comparisons in `docs/fidelity/`.
Original scripts, assets, raw captures/dumps and installation paths stay external.

## Implementation expectations

Implement only the assigned transition/configuration once evidence supports it.
Place Origins rules in `game/zombies/origins`; reusable simulation belongs below
the map layer. Use Bevy components, resources, systems and available scheduling
facilities. Define fixed-step ordering and replay inputs when timing matters.
Do not hard-code guessed quest conditions or create a duplicate event engine.

## Validation

Use synthetic fixtures for observed preconditions, success, interruption and
repeat-input behavior that fall within scope. Check deterministic replay and
dependency direction. Run applicable workspace gates for implementation changes
and compare the stated cases against the original separately from unit tests.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff) with
linked evidence and actual results. Use `APPROXIMATE` only for explicit bounded
limits; `MATCHED`/`VERIFIED` require scoped comparison evidence. Report missing
progression state, multiplayer-dependent uncertainty, contradictions or a needed
shared abstraction to the outer agent. Uninvestigated mechanics remain `UNKNOWN`.
