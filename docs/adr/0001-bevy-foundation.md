# 0001: Bevy foundation

Status: accepted engine direction; implementation pending.

## Context and evidence

The user explicitly requested Rust with Bevy. The repository has no existing
engine implementation. Original game architecture has not been investigated,
and Rust tooling is currently unavailable.

## Decision

Use Bevy as the engine/application, ECS, scheduling, and rendering foundation.
Build reusable Zombies systems and an Origins-specific layer using its facilities.
Do not create competing ECS or renderer foundations. The initial diagnostic
workspace stays dependency-free until a bounded integration task can investigate
compatible stable Rust/Bevy versions and validate an actual build.

## Consequences and validation

Future engine-facing tasks first inspect Bevy's relevant facilities and original
game evidence. Add adapters only when a documented gap requires one. Selecting
Bevy does not demonstrate original-game fidelity or working Bevy integration.
No Bevy version has been selected, no dependency has been added, and no Bevy build
or runtime validation has occurred.

Unresolved: toolchain availability, version compatibility, application integration,
and evidence-backed component/system ownership. Resolve these in bounded tasks.
