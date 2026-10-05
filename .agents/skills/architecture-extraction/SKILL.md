---
name: architecture-extraction
description: Extract one reusable semantic boundary from observed BO2 structures and map it to idiomatic Rust and Bevy ownership. Use for evidence-backed abstraction or dependency decisions, not general engine construction or asset parsing.
---

# Architecture extraction

## Bounded task

Investigate one candidate abstraction or dependency boundary using a named set of
existing findings and representative original structures. Define the decision to
resolve, owned documents/modules and completion condition before changing APIs.
Read [project rules](../../../AGENTS.md), the
[architecture](../../../docs/architecture/README.md) and previous handoff.
Do not spawn agents, commit or take ownership of neighboring systems.

## Investigation

1. Inspect the actual Cargo/module graph, current components, resources, systems,
   asset definitions and tests. Reserved directories are not approved APIs.
2. Compare the selected recurring structures: ownership, lifetime, identity,
   configuration, state transitions, event flow and variation. Trace observations
   to source/build identifiers; distinguish evidence from inferred relationships.
3. Look for counterexamples before treating a shared field/name as shared behavior.
   Explain whether composition, an enum, data, a trait or a generic serves the
   observed semantics; avoid translating original inheritance mechanically.
4. Inspect the selected Bevy release's relevant facilities before proposing an
   adapter. Prefer its ECS, entities, scheduling, rendering and asset facilities.
5. Describe the smallest boundary, dependency direction and evidence that could
   disprove it. Keep reusable systems independent of Origins and boot assembly.

## Required evidence

Link permitted observations in `docs/reverse_engineering/`. Record conclusions,
alternatives, dependency direction and unresolved semantics in `docs/architecture/`;
use an ADR for an important decision. Include abstract examples, not copied
scripts, assets, dumps or local installation paths.

## Implementation expectations

Implement only an assigned, evidence-backed boundary or migration. Reuse existing
definitions and Bevy composition; do not create a second entity registry, ECS,
scheduler or renderer. Preserve stable Rust and existing unsafe prohibitions.
Document uncertainty rather than exposing speculative APIs as established facts.

## Validation

Check the dependency graph for cycles and lower-layer Origins imports. For an
implementation, exercise the documented semantic contract and preserve observable
behavior with relevant tests, then run applicable workspace gates. For a proposal,
walk representative examples and counterexamples through the model. Static graph
or compile success does not prove correspondence to the original architecture.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff) with
the evidence-to-boundary rationale and actual validation. Fidelity remains
`UNKNOWN`/`INVESTIGATING` without scoped comparison evidence; never invent
`MATCHED` or `VERIFIED`. Report conflicting evidence, duplicated abstractions,
unavoidable boundary changes, unsafe requirements or missing Bevy facilities to
the outer agent; adjacent migrations remain recommendations.
