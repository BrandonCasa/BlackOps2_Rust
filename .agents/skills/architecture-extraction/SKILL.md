---
name: architecture-extraction
description: Work on one reusable system boundary in the Origins game application.
---

# architecture-extraction

Use for one reusable system boundary; do not expand it into unrelated engine or game work.

Work sequentially in this chat. Never spawn subagents or delegate to another chat.
Read `docs/project-state.md`, applicable source/tests and the retained reference relevant to the task.

Read the relevant Rust modules and original-data observations. Choose components, resources and systems that compose with Bevy; avoid duplicate engine infrastructure.

Practical reconstruction is authorized. Missing exact native behavior does not block useful implementation; label approximations and preserve the complete systems roadmap. Do not create another viewer, demo, custom renderer, ECS or scheduler.

Deliver a coherent implementation or reproducible finding, with meaningful validation appropriate to the change. Keep evidence concise and external proprietary content out of Git. Update the one current handoff with findings, changes, actual validation, limitations, fidelity status and next work. Escalate only a concrete missing input or failed dependency; continue independent work without routine confirmation.
