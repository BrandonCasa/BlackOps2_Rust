TASK:
Establish the smallest coherent local Rust repository foundation for the Origins reimplementation.

STATUS:
COMPLETE

FINDINGS:
- Repository started empty; no ancestor repository instructions were found.
- Bevy is the required engine/ECS/rendering/application foundation. Integration is pending.
- Git and Python are available; Rust tooling is unavailable on PATH.
- The outer agent reports that the Visual Studio x64 linker and Windows SDK are available; no Rust build has occurred.

EVIDENCE:
- Inspected the supplied objective, workspace/ancestor instruction locations, and available commands.
- No original game data, scripts, entity definitions, behavior, or performance measurements were inspected.

CHANGES:
- Initialized local Git on main without a commit or remote.
- Added AGENTS.md, Cargo workspace, safe minimal core/diagnostic boot, asset config placeholders, ignores, and responsibility directories.
- Added architecture, Bevy decision, evidence/fidelity indexes, project state, and this handoff.
- Reserved engine integration, reusable Zombies simulation, and Origins assembly boundaries without speculative gameplay APIs.

VALIDATION:
- Parsed all three Cargo manifests and config example with Python TOML; checked members, source paths, and the local dependency edge.
- Checked ignored local assets/extractions/dumps/reports/config/builds/secrets and trackable example/code/docs.
- Reviewed candidate content and diff formatting; no proprietary content or machine-specific installation paths were introduced.
- Cargo metadata/test/fmt/Clippy and executable validation are pending because Rust tooling is unavailable. No Bevy validation occurred.

FIDELITY STATUS:
UNKNOWN

UNRESOLVED:
- Rust toolchain availability and pending Cargo checks/lockfile generation.
- Bevy version compatibility and application integration.
- Original asset locations/formats and original-game architecture/behavior evidence.

RECOMMENDED NEXT WORK:
- Outer-agent review of this foundation, followed by the required bounded agent-skill task.
- Resolve toolchain availability separately, then perform pending validation and evidence-backed Bevy integration.

SCOPE NOT IMPLEMENTED:
- Skills, original asset location/cataloging, reverse engineering, toolchain installation, rendering, entity architecture, asset loading, gameplay, comparisons, commits, and pushes.
