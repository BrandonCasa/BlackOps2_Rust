TASK:
Create the ten required repository-local skills for bounded, evidence-first Origins work.

STATUS:
COMPLETE

FINDINGS:
- Required responsibilities are distinct: general reverse engineering, architecture extraction, asset inspection, Origins mechanics, Rust performance, fidelity comparisons, zombie AI, weapons, rendering and validation.
- Skills preserve Bevy as the engine/ECS/scheduling/rendering foundation, reusable lower-layer ownership, safe Rust and sequential outer-agent review.
- No original-game findings, speculative constants, approved gameplay APIs or new engine facilities were introduced.

EVIDENCE:
- Inspected root AGENTS.md, architecture/Bevy ADR, project state, documentation index, reverse-engineering/fidelity indexes and the foundation handoff.
- Read the supplied objective and system skill-creator instructions; inspected its bundled quick_validate.py.
- No original game data, scripts/entity definitions, live behavior or performance measurements were inspected.

CHANGES:
- Replaced .agents/skills/.gitkeep with ten named skill folders, each containing one concise SKILL.md with discoverable name/description frontmatter.
- Each skill defines one bounded task, focused investigation, permitted evidence, implementation expectations, appropriate validation, the root handoff reference and escalation conditions.
- Updated docs/README.md and docs/project-state.md; added this handoff. No Rust/configuration edits or additional skill resources.
- Escalation reports evidence/tooling/scope issues to the outer agent without adding permission gates; delegated agents cannot spawn agents or commit.

VALIDATION:
- Ran bundled quick_validate.py for each of ten skills through uv run --no-project --with pyyaml python; all returned "Skill is valid!" with exit code 0. PyYAML was isolated, not installed globally.
- Checked the exact ten required names, folder/frontmatter agreement and all 30 relative links plus handoff anchors; passed.
- Reviewed triggers and instructions against all eight required skill contents and project boundaries. Skills contain 57-66 lines each; no placeholders or speculative original schemas/constants.
- git diff --check passed. Final path/whitespace checks covered all 13 task text candidates and resolved all 38 skill/documentation links. Candidate text review found no proprietary content or real installation paths.
- No Rust code changed; Cargo metadata/test/fmt/Clippy were not rerun. Compiler/runtime validation remains pending from foundation. No original-game comparisons occurred.

FIDELITY STATUS:
UNKNOWN

UNRESOLVED:
- Outer-agent acceptance review of the skill diff and this handoff.
- Existing toolchain availability, Bevy compatibility/integration and missing original-game evidence remain unchanged.

RECOMMENDED NEXT WORK:
- Outer-agent review/integration of the skills, then a dedicated toolchain availability and workspace-validation task.
- Continue with bounded external asset cataloging and evidence-backed architecture investigation.

SCOPE NOT IMPLEMENTED:
- Rust/gameplay changes, toolchain installation, Bevy integration, asset discovery/extraction, live game interactions, original measurements, additional agents, commits and pushes.
