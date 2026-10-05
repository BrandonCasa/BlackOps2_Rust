---
name: bo2-reverse-engineering
description: Investigate one unfamiliar BO2 data structure or observable behavior and document reproducible findings before implementation. Use for general evidence questions; specialized weapon, zombie, rendering or Origins mechanics use their focused skills.
---

# BO2 reverse engineering

## Bounded task

Resolve one named question about one source family or behavior, with specified
build, permitted sources, deliverable and completion condition. Read the
[project rules](../../../AGENTS.md), relevant findings and previous handoff.
Own only the assignment; do not spawn agents, commit or expand into adjacent systems.

## Investigation

1. Record which licensed sources are available and which questions they can answer.
   Keep real paths in ignored local configuration and raw outputs external.
2. Inspect relevant implementation/data read-only. Describe relationships,
   identifiers and recurring patterns without copying original scripts or content.
3. Separate direct observations, inferred semantics, competing hypotheses and
   implementation choices. Names alone do not establish an abstraction.
4. Design a discriminating measurement for the unresolved question. Control build,
   game state, inputs and timing; repeat enough to reveal variability.
5. Compare repeated structures against existing abstractions and identify evidence
   needed before recommending a reusable model. Report adjacent questions separately.

## Required evidence

Write permitted findings in `docs/reverse_engineering/`: question, source/build
identifiers or appropriate hashes, read/emit procedure, units, conditions, samples,
observed relationships, contradictions, limits and open questions. Describe
script/entity semantics abstractly; keep original text and raw dumps external.

## Implementation expectations

Investigation does not authorize gameplay implementation. If the assignment
includes a small inspection tool, document its input/output contract and use
safe Rust, explicit errors and bounded reads. Any proposed model must fit the
[architecture boundaries](../../../docs/architecture/README.md) and Bevy ECS,
scheduling and engine facilities; do not invent a replacement engine.

## Validation

Reproduce the procedure against the stated source/build and check whether the
evidence distinguishes the hypotheses. Test any assigned inspector with synthetic
valid, truncated and unsupported inputs, then run applicable workspace checks.
Record unavailable measurements separately from tests; compilation is not fidelity.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff), linking
findings and actual validation. Use `UNKNOWN` or `INVESTIGATING` until comparison
evidence supports a scoped status; never fabricate `MATCHED` or `VERIFIED`.
Report missing sources, ambiguous builds, contradictory measurements, unsafe
integration needs or a question too broad for one task to the outer agent.
Continue independent in-scope evidence work; do not fill gaps with invented behavior.
