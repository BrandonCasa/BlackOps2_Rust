---
name: rendering-analysis
description: Work on one appearance or rendering integration in the Origins game application.
---

# rendering-analysis

Use for one appearance or rendering integration; do not expand it into unrelated engine or game work.

Work sequentially in this chat. Never spawn subagents or delegate to another chat.
Read `docs/project-state.md`, applicable source/tests and the retained reference relevant to the task.

Use Bevy scenes, StandardMaterial, lighting, visibility, camera and post-processing first. Captures explain rendering; they never select game geometry or freeze the player's camera. Document material approximations.

Practical reconstruction is authorized. Missing exact native behavior does not block useful implementation; label approximations and preserve the complete systems roadmap. Do not create another viewer, demo, custom renderer, ECS or scheduler.

Deliver a coherent implementation or reproducible finding, with meaningful validation appropriate to the change. Keep evidence concise and external proprietary content out of Git. Update the one current handoff with findings, changes, actual validation, limitations, fidelity status and next work. Escalate only a concrete missing input or failed dependency; continue independent work without routine confirmation.
