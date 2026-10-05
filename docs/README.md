# Documentation

- [Project state](project-state.md): completed work, evidence limits, and next tasks.
- [Architecture](architecture/README.md): current workspace and planned boundaries.
- [Reverse engineering](reverse_engineering/README.md): evidence requirements and findings index.
- [Fidelity](fidelity/README.md): comparison statuses and evidence requirements.
- [Decisions](adr/README.md): architecture decision records.
- [Foundation handoff](handoffs/0001-workspace-foundation.md): first task's result and validation.
- [Skill handoff](handoffs/0002-project-skills.md): the ten bounded project skills and validation.
- [Bevy build handoff](handoffs/0003-bevy-build-foundation.md): stable toolchain, headless lifecycle and executable checks.
- [Local asset catalog](reverse_engineering/0001-local-asset-catalog.md): external source availability, candidate containers and bounded prefix/hash evidence.
- [Asset catalog handoff](handoffs/0004-licensed-asset-catalog.md): read-only discovery, output boundaries and validation.
- [Fastfile envelope evidence](reverse_engineering/0002-fastfile-envelope.md): 312-byte signed PC metadata contract, source corroboration and unknown payload boundaries.
- [Envelope evidence handoff](handoffs/0005-fastfile-envelope-evidence.md): bounded sample observations, inspector scope and validation.
- [Envelope inspector handoff](handoffs/0006-fastfile-envelope-inspector.md): reusable bounded parser, explicit-input CLI, synthetic tests and six external sample checks.
- [Fastfile transport framing](reverse_engineering/0003-fastfile-block-framing.md): plaintext chunk metadata, suffix accounting, authentication dependencies and the next decoded-directory access route.
- [Framing evidence handoff](handoffs/0007-fastfile-block-framing-evidence.md): bounded noncontiguous probes, unchanged originals and external-tool recommendation.
- [Decoded Origins candidate directory](reverse_engineering/0004-origins-asset-directory.md): verified external OAT listing, type/reference-marker counts and a bounded geometry-access question.
- [Directory evidence handoff](handoffs/0008-origins-asset-directory.md): repeated listing, tool/source identity and read/emit validation.
- [GfxWorld surface boundary](reverse_engineering/0005-gfxworld-surface-boundary.md): repeated aggregate index-range/material-pointer observations and unestablished vertex-stream interpretation.
- [Surface boundary handoff](handoffs/0009-gfxworld-surface-boundary.md): pinned external metadata tooling, synthetic checks, observed relationships and next vertex-stream investigation.
- [GfxWorld vertex-stream boundary](reverse_engineering/0006-gfxworld-vertex-stream-boundary.md): complete candidate stream-0 group accounting, zero-count distinctions, logical-base intervals and stream-1 limits.
- [Vertex-stream handoff](handoffs/0010-gfxworld-vertex-stream-boundary.md): aggregate observations, source/tool preservation, remaining encoding questions and next position-prefix validation.
- [GfxWorld candidate position prefix](reverse_engineering/0007-gfxworld-position-prefix.md): finite float32 prefixes and exact per-surface bounds consistency across every observed format, with contradicted embedded bounds interpretation.
- [Position-prefix handoff](handoffs/0011-gfxworld-position-prefix.md): bounded complete observations, numerical policy, synthetic checks, restoration and remaining layout questions.
- [Diagnostic geometry snapshot contract](architecture/diagnostic-geometry-snapshot.md): project-owned v1 byte layout, exact preservation, ordered surface ranges, bounds checks, fixed policies and engine boundaries.
- [Diagnostic snapshot reader handoff](handoffs/0012-diagnostic-geometry-snapshot-reader.md): std-only consumer, explicit-input inspector, synthetic validation and unimplemented producer/Bevy integration.

Keep original assets, raw extractions, copyrighted scripts, dumps, and local
installation paths outside tracked documentation. Record observations separately
from inferences and unknowns.
