# Origins

Rebuild the complete Black Ops II Zombies Origins experience in stable Rust and Bevy. Prioritize playable systems and maintainable code; practical reconstruction is welcome. Be clear about approximations and unfinished features.

Use your judgment to choose tools, proportional validation, and implementation strategy. Do not use subagents unless the current user request explicitly asks for them. Preserve concurrent changes.

Keep one Cargo package and one game executable, `origins.exe`. Build on Bevy and Avian rather than duplicating engine infrastructure. Retain safe Rust and the locked dependency versions unless the task calls for changing them.

Keep `todo.md` as the sole project checklist, with concrete implementation tasks. Performance documentation is allowed: record benchmark procedures, settings, hardware, system load, executable and asset identities, measurements, limitations, and the relevant commit SHA. Other new documentation and code comments require user authorization. Leave rendering and graphics work out of the checklist.

Preserve original source assets. Keep prepared runtime assets under `assets/origins/`, validate their dependency closure when changing the pack, and use Git LFS for substantial binaries. Keep secrets, machine-specific paths, and generated builds out of commits.

Development builds use `dev-dynamic`; release builds omit it. Use matched builds of the requested type for performance targets, release builds for release comparisons, and development builds for diagnostics. Account for system load outside the game. Avoid overlapping actual-game runs or GPU captures. Close abandoned validation processes belonging to this work normally without asking again; never stop a process owned by another active task. Cancel administrator prompts and use tools that work without elevation.

Launching the game for work authorized by the user does not require a separate approval request. Before every launch, check running game processes across all checkouts and worktrees. If a game is already running, wait and periodically recheck until it closes; do not launch another instance, ask the user for launch approval, or stop someone else's game to make room. Continue independent work while waiting when possible, and recheck immediately before launching. Apply the same wait-and-recheck rule when another active GPU capture would overlap the run.

Prefer external, interpreted gameplay validation scenarios against a reusable development executable. Keep scenario sequencing, inputs, waits, assertions, and report generation in Python or PowerShell under `tools/`, so changing a scenario does not require recompiling the game. Exercise the real game, assets, Bevy schedules, and Avian physics; keep Rust validation support small and reusable rather than adding a compiled driver and CLI flag for every case.

Use the existing Bevy Remote Protocol (BRP) interface in `src/verification/remote.rs` and the external client in `tools/gameplay_remote.py`. Consult the pinned Bevy crate source rather than assuming docs.rs `/latest` matches our version. Keep the interface development-only, explicitly enabled for validation, and bound to loopback; release builds must not expose it. Discover supported methods and type schemas instead of assuming every game type is reflected.

Drive actions through the normal input/message path and observe resulting gameplay state; direct ECS edits are scenario setup, not proof that gameplay produced an outcome. Wait for readiness and observable conditions with bounded timeouts, release held inputs, and reset the session between cases. Fixed-tick timing cases need a reusable scheduled-input hook rather than HTTP timing or arbitrary sleeps. Preserve existing compiled checks until external replacements cover their behavior; do not claim the external workflow is available before its initial integration is built and exercised.

Reuse a known executable directly for repeated cases and external-script changes; do not invoke a build merely to launch validation. Record its source revision and dirty-source snapshot, build features, executable/DLL identity, and runtime asset identity with results. Preserve matching development DLLs when copying an executable. Runtime-loaded asset/data edits can reuse it with a restart, or with asset watching only where enabled and consumed correctly. Embedded `include_str!`/`include_bytes!` content is compiled input. Changed Rust, embedded content, dependencies, features, or build configuration requires a new matching build before claiming validation of those changes. Batch coherent code changes into one build and reuse it across the relevant cases.

Keep compiler checks, actual-game behavior checks, and performance evidence distinct. A review cannot establish that no bugs remain, and a build or reduced workload alone does not prove an FPS improvement.

Review your changes, preserve unrelated work, and make coherent local commits with a clear summary and useful body. Push only when requested. Report what was actually checked and any remaining limitations.
