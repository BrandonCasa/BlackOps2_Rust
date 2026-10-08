# Current handoff

TASK: Configure debug/release profiles from the Bevy setup guide, commit the decisions, then compare real-game debug/release FPS.

STATUS: PARTIAL

FINDINGS: Development uses opt-level 1, dependency opt-level 3, thin LTO, one codegen unit, debug information/assertions and diagnostic logging. Release uses opt-level 3, full LTO, one codegen unit, panic abort, no incremental compilation/debug information/assertions, all log/tracing calls disabled, no LogPlugin/renderer diagnostic plugin/pipeline inspection, and compile-time rejection of Tracy/Chrome features. Functional Bevy system names remain required for native renderer schedule wiring. Stable Rust and static linking are retained; bundled rust-lld is selected for Windows MSVC.

EVIDENCE: https://bevy.org/learn/quick-start/getting-started/setup/ reviewed in full. Cargo feature tree confirms log and tracing release_max_level_off. Local pinned Bevy PluginGroupBuilder source confirms setting a disabled LogPlugin does not re-enable it. Debug startup exposed naga_oil identifier validation rejecting exported uv3; pinned Naga appends an underscore to names ending in digits, while release Bevy uses a non-validating composer. Evidence is preserved under E:/Coding/Captures/origins-debug-release-20261008-1791447061644/debug-window-unrecorded/pipeline-failure.json.

CHANGES: Cargo profiles/dependencies/lockfile, Windows linker configuration, build-profile selection, development-only profiling, benchmark profile/logging receipts and comparison guards, README and project rules. Debug is the build-script default; Release packages the validated runtime pack. Existing benchmark driver accepts explicit debug measurements with matching qualification checks. Ray/material shader fields, texture globals and exported sampling functions now use names without numeric suffixes; references and vertex interfaces match, retaining layouts, bindings and arithmetic.

VALIDATION: Locked metadata, full formatting, PowerShell parsing and scoped diff checks pass. The pre-shader-fix optimized debug compiler build passed and its executable was preserved. Frozen runtime import/validation passed 4,137 files and 3,358,926,284 bytes; source and imported hashes match. Its first actual-window launch failed shader composition before gameplay and produced no qualified FPS or gameplay capture. OBS was closed and no recording started. Shader-fix compiler rebuild and actual-window retry are pending. No tests, fixtures, Clippy or push.

UNRESOLVED: No measured debug/release FPS or release-window qualification yet. Debug startup shader repair awaits rebuilt-window confirmation. The first launch also logged a failed ReShade Vulkan layer load and the existing partial-density pack warning. Prior baselines lack the new logging/profile receipts and cannot qualify the changed build policy. Concurrent combat-clip work is excluded from the source snapshot. Previous gameplay/static-review/TODO limitations remain recorded in Git.

RECOMMENDED NEXT WORK: Rebuild the repaired committed shaders, retry the unrecorded debug gameplay run with occasional game-window screenshots, then build both profiles from that same source. Reuse the frozen validated pack, run three matched moving and combat samples per profile sequentially, preserve hashes, compare FPS/p95, and record failures honestly. The requested OBS debug recording follows the unrecorded test. Resume the existing combat-clip TODO afterward.

SCOPE NOT IMPLEMENTED: Gameplay or asset changes, nightly/Cranelift/dynamic-linking adoption, performance gains, release package acceptance, or integration of concurrent combat edits.
