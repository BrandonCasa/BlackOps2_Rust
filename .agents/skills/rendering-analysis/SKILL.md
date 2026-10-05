---
name: rendering-analysis
description: Investigate one Origins visual feature or integrate one evidence-backed geometry, material, lighting or camera adapter with Bevy. Use for appearance and rendering semantics, not a replacement renderer, full map conversion or general asset cataloging.
---

# Rendering analysis

## Bounded task

Select one visual feature or representative geometry/lighting sample, with named
source/build, camera conditions, owned files and completion condition. Read
[project rules](../../../AGENTS.md), relevant asset/visual findings,
[architecture](../../../docs/architecture/README.md), tests and previous handoff.
Do not spawn agents, commit or attempt the whole map or rendering pipeline.

## Investigation

1. Inspect licensed geometry/material/light/entity metadata read-only for the
   selected sample. Identify observed references and repeated structures without
   copying assets, shader source or proprietary definitions into findings.
2. Establish coordinate basis, units, transforms and camera projection from
   evidence before adapting geometry. Label unconfirmed conventions explicitly.
3. Capture controlled original observations externally: build, location/camera,
   resolution, display settings and relevant game state. Distinguish geometry,
   lighting, material, animation and post-processing explanations for differences.
4. Measure the selected observable with a repeatable method. Document color-space,
   exposure or capture uncertainty where relevant; avoid guessed transfer functions
   or declaring a similar screenshot equivalent.
5. Inspect the selected Bevy release's relevant meshes, transforms, assets,
   materials, lights and camera facilities before introducing any adapter.

## Required evidence

Record permitted identifiers, abstract metadata relationships, coordinate/unit
findings, reproducible procedure, measurements, hypotheses and unknowns in
`docs/reverse_engineering/`; visual comparisons belong in `docs/fidelity/`.
Original screenshots, meshes, textures, shaders, raw dumps and paths stay external;
repository fixtures and illustrative images must be user-created or synthetic.

## Implementation expectations

Implement only the assigned adapter/configuration using Bevy rendering, ECS and
asset facilities. Keep generic adapters independent of Origins. Use safe Rust,
external originals and explicit unsupported-data errors; avoid duplicate renderers
or compensating guessed values that conceal uncertain coordinates/material rules.
Bound and justify any `APPROXIMATE` visual choice and its resolving measurement.

## Validation

Test synthetic transforms, geometry bounds and confirmed format/material mappings
where relevant. Run applicable workspace gates and an actual Bevy render check
when available. Compare equivalent controlled camera cases; report visual review,
numeric comparisons, GPU/runtime checks and compile results separately.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff) with
evidence links and actual results. Use `UNKNOWN`/`INVESTIGATING` without evidence;
`MATCHED`/`VERIFIED` require scoped original comparisons. Report unsupported
formats, uncertain axes/color semantics, absent rendering tooling or a suspected
Bevy facility gap to the outer agent; do not build a new engine to bypass it.
