# Documentation

Start with [project-state.md](project-state.md). It is the concise current brief:
implemented code, pending work, next executable milestones, and validation.
Read source/tests next; retrieve historical evidence only for a specific question.

- [Architecture](architecture/README.md): Bevy ownership and crate boundaries.
- [Snapshot format](architecture/diagnostic-geometry-snapshot.md): current Rust input contract.
- [Geometry rendering](architecture/diagnostic-geometry-render.md): existing executable diagnostic.
- [Pending collision handoff](handoffs/0026-origins-collision-brush-boundary.md): incomplete investigation.
- [Fidelity](fidelity/README.md): comparisons still missing.
- [Reverse-engineering index](reverse_engineering/README.md): evidence lookup by subject.
- [Decisions](adr/README.md): architecture decisions.
- [Restart handoff](handoffs/2026-10-05-chat-restart.md): pause and consolidation scope.

Historical records remain available in the [full index](history-index-2026-10-05.md)
and [prior project-state snapshot](project-state-history-2026-10-05.md).
They are preserved evidence, not instructions to repeat every previous check.
Maintain one current brief and concise task handoffs. Do not append the full
validation history to this index or the working brief after each task.
