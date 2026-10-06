---
name: origins-analysis
description: Work on one Origins mechanic in the Origins game application.
---

# origins-analysis

Use for one Origins mechanic; do not expand it into unrelated engine or game work.

Work sequentially in this chat. Never spawn subagents or delegate to another chat.
Unit tests and Clippy are banned; do not run Clippy; do not add or restore test suites or test fixtures.
Read `docs/project-state.md`, applicable source and validation tools and the retained reference relevant to the task.

Find its source configuration/transitions and separate reusable behavior from Origins parameters. Put it in the game application and update the systems roadmap.

Practical reconstruction is authorized. Missing exact native behavior does not block useful implementation; label approximations and preserve the complete systems roadmap. Do not create another viewer, demo, custom renderer, ECS or scheduler.

Deliver a coherent implementation or reproducible finding, with meaningful validation appropriate to the change. Keep evidence concise. Only validated prepared runtime files under `assets/origins/` may enter Git (glTF, mesh buffers and PNGs via LFS; `game.json` in ordinary Git). Keep originals, intermediates, captures and reports external. Update the one current handoff with findings, changes, actual validation, limitations, fidelity status and next work. Escalate only a concrete missing input or failed dependency; continue independent work without routine confirmation.
