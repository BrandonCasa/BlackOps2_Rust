# Current handoff

TASK: Implement every facet of [renderer plan 3.0](../renderer-plan-3.md), qualify native 1080p/DDGI >=80 average FPS debug and 100+ FPS release, with ECS integration, subagent consultation, critic-approved atomic commits and rendering-logic tests.

STATUS: PARTIAL. Source audit and plan completed. Implementation begins with reliable frame attribution; no current-build FPS acceptance is established.

FINDINGS: The active user goal lifts the subagent and rendering-unit-test bans. Existing ray animation creates replacement meshes/BLAS; a changed provider can refresh the whole ray scene. Cube tracing/filtering runs every frame, GI cascades sample before ownership selection, and exact camera-height changes invalidate sky lighting. Native diagnostic latest-only storage, repeated paths, a 256-query pool and graph-late recorder initialization leave attribution incomplete. See the persistent plan for full scope and dependency limitations.

EVIDENCE: Current source audit against Bevy 0.19.1/wgpu 29.0.4. Earlier feature/capture evidence remains E:/Coding/Captures/origins-ddgi-default-off-20261008; prior matched release evidence remains E:/Coding/Captures/origins-debug-release-20261008-1791447061644. These do not establish performance of future changes. Source-backed consultation is in the attribution_audit and scene_audit agents of this task.

CHANGES: Saved full plan and explicit checklist under docs/renderer-plan-3.md. Updated AGENTS for renderer-only unit tests, ECS/dependency integration and critic-approved commits. Rendering implementation has not yet changed.

VALIDATION: Initial tracked/staged state clean. No compiler, tests, game run or performance measurements for this milestone yet. No Clippy, unrelated test suites, fixture creation or push.

UNRESOLVED: Complete frame cost, dependency retention/refit, all renderer implementation and both FPS/quality acceptance targets remain open. User-reported 6.2->4.3 FPS is not a new matched benchmark. Pinned wgpu-core currently rebuilds even with PreferUpdate.

RECOMMENDED NEXT WORK: Finish attribution consultation and implement typed complete-frame records plus shadow/AS coverage. Qualify a debug baseline before cost-driven scene, reflection, GI and first-person changes. Keep coherent atomic commits after exact-diff critic score >8/10.

SCOPE NOT IMPLEMENTED: All unchecked items of renderer-plan-3.md; no speedup, quality or complete-goal claim.
