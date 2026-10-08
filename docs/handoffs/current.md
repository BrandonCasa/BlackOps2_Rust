# Current handoff

TASK: Configure debug/release profiles from the Bevy setup guide, commit the decisions, then compare real-game debug/release FPS.

STATUS: PARTIAL

FINDINGS: Development uses opt-level 1, dependency opt-level 3, thin LTO, one codegen unit, debug information/assertions and diagnostic logging. Release uses opt-level 3, full LTO, one codegen unit, no incremental compilation/debug information/assertions, all log/tracing calls disabled, no LogPlugin/renderer diagnostic plugin/pipeline inspection, and compile-time rejection of Tracy/Chrome features. Functional Bevy system names remain required for native renderer schedule wiring. Stable Rust and static linking are retained; bundled rust-lld is selected for Windows MSVC.

EVIDENCE: https://bevy.org/learn/quick-start/getting-started/setup/ reviewed in full. Cargo feature tree confirms log and tracing release_max_level_off. Local pinned Bevy PluginGroupBuilder source confirms setting a disabled LogPlugin does not re-enable it.

CHANGES: Cargo profiles/dependencies/lockfile, Windows linker configuration, build-profile selection, development-only profiling, benchmark profile/logging receipts and comparison guards, README and project rules. Debug is the build-script default; Release packages the validated runtime pack. Existing benchmark driver now accepts explicit debug measurements with matching qualification checks.

VALIDATION: Locked metadata, full formatting, PowerShell parsing and scoped diff checks pass. Two owned development builds were interrupted after user changes superseded their configuration; neither is a compiler pass. Compiler builds, frozen pack validation and actual-game FPS measurements are pending after the requested initial commit. No tests, fixtures, Clippy or push.

UNRESOLVED: No measured debug/release FPS or release-window qualification yet. Prior performance baselines lack the new logging/profile receipts and cannot qualify the changed build policy. Concurrent uncommitted combat-clip work is excluded from this commit and the planned source snapshot. Previous gameplay/static-review/TODO limitations remain recorded in Git.

RECOMMENDED NEXT WORK: Snapshot committed source and runtime assets, build both profiles from the same source, run three matched moving and combat samples per profile sequentially, preserve executable/asset hashes, compare FPS/p95, and record failures honestly. Resume the existing combat-clip TODO afterward.

SCOPE NOT IMPLEMENTED: Gameplay or asset changes, nightly/Cranelift/dynamic-linking adoption, performance gains, release package acceptance, or integration of concurrent combat edits.
