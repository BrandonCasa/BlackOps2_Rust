---
name: gameplay-fidelity
description: Design or execute a controlled comparison between one implemented behavior and licensed original Origins, recording discrepancies and scoped fidelity status. Use for measurement and regression evidence, not new feature implementation or performance tuning.
---

# Gameplay fidelity

## Bounded task

Compare one observable behavior under a named set of conditions. State original
and implementation builds, scenarios, observables, capture method and completion
criteria. Read [project rules](../../../AGENTS.md), relevant investigation,
[fidelity status](../../../docs/fidelity/README.md), tests and previous handoff.
Do not spawn agents, commit or expand the comparison into unrelated features.

## Investigation

1. Inspect the claimed behavior contract and implementation/test assumptions.
   Identify unknown original conditions before designing the comparison.
2. Make scenarios reproducible: inputs, player/map/round state, equipment, seed
   where available, fixed-step settings and timing reference. Document uncontrolled
   original randomness rather than assuming it can be replayed exactly.
3. Select measurable observables and units. Set justified tolerances from original
   repeatability and measurement resolution before judging implementation results.
4. Repeat original and implementation trials under equivalent conditions. Separate
   capture/observer error from behavior differences and show sample variability.
5. Trace discrepancies to evidence; classify observations, hypotheses and next
   experiments without silently changing the expected behavior to fit results.

## Required evidence

Write permitted methods, build identifiers, conditions, measurements, tolerances,
discrepancies and scoped conclusions in `docs/fidelity/`, linking source findings
and regression coverage. Keep original recordings, screenshots, scripts, assets,
raw proprietary outputs and local paths external to tracked documentation.

## Implementation expectations

Comparison tasks may add assigned measurement tooling or synthetic regression
cases, using safe Rust and existing Bevy scheduling/input facilities. They do not
authorize feature changes or corrections unless explicitly included in the task.
Report architectural causes and required follow-up instead of adding workaround
rules. Keep simulation timing separate from rendering and measurement timing.

## Validation

Re-run the procedure and confirm it can expose an intentionally different
synthetic result where useful. Check arithmetic, units and case coverage; run
applicable workspace gates for tooling/test changes. Distinguish live original
comparison from unit, compile, visual and performance evidence.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff).
Use `UNKNOWN` for absent evidence, `INVESTIGATING` for ongoing measurement and
`APPROXIMATE` for explicit implemented limits. `MATCHED` requires documented
comparisons; `VERIFIED` requires reproducible checks for stated cases/tolerances.
Neither implies untested cases match. Report unavailable original access, ambiguous
conditions, inadequate resolution or contradictions to the outer agent; do not
relax tolerances after a failure to manufacture success.
