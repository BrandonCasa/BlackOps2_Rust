# Origins

Rebuild the complete Black Ops II Zombies Origins experience in stable Rust and Bevy. Prioritize playable systems and maintainable code; practical reconstruction is welcome. Be clear about approximations and unfinished features.

Use your judgment to choose tools, validation, and implementation strategy. Subagents and parallel work are welcome when useful; give them distinct ownership and preserve concurrent changes.

Keep one Cargo package and one game executable, `origins.exe`. Build on Bevy and Avian rather than duplicating engine infrastructure. Retain safe Rust and the locked dependency versions unless the task calls for changing them.

Keep `todo.md` as the sole project checklist, with concrete implementation tasks. Do not add code comments or codebase documentation unless explicitly authorized by the user. Do not create new Markdown documents anywhere in the repository folder, including subfolders, unless explicitly authorized by the user. Leave rendering and graphics work out of the checklist.

Preserve original source assets. Keep prepared runtime assets under `assets/origins/`, validate their dependency closure when changing the pack, and use Git LFS for substantial binaries. Keep secrets, machine-specific paths, and generated builds out of commits.

Use validation proportional to the change. Development builds use `dev-dynamic`; release builds omit it. Use release builds for performance comparisons and development builds for diagnostics. Coordinate actual-game runs so only one owns the validation slot; never stop another task's process.

Prefer external, interpreted gameplay validation scenarios against a reusable development executable. Keep scenario sequencing, inputs, waits, assertions, and report generation in Python or PowerShell under `tools/`, so changing a scenario does not require recompiling the game. Exercise the real game, assets, Bevy schedules, and Avian physics; keep Rust validation support small and reusable rather than adding a compiled driver and CLI flag for every case.

Use Bevy Remote Protocol (BRP) for this interface. The pinned Bevy `0.20.0-rc.2` includes `examples/remote/app_under_test.rs` and `examples/remote/integration_test.rs`, demonstrating `RemotePlugin`, `RemoteHttpPlugin`, ECS queries, input messages, and screenshots. Consult the pinned crate source rather than assuming docs.rs `/latest` matches our version. Origins does not yet enable BRP: enabling `bevy/bevy_remote`, installing the plugins, and exposing the required reflected types or small custom methods requires an initial development build. Keep the interface development-only, explicitly enabled for validation, and bound to loopback; release builds must not expose it. Discover supported methods and type schemas instead of assuming every game type is reflected.

Drive actions through the normal input/message path and observe resulting gameplay state; direct ECS edits are scenario setup, not proof that gameplay produced an outcome. Wait for readiness and observable conditions with bounded timeouts, release held inputs, and reset the session between cases. Fixed-tick timing cases need a reusable scheduled-input hook rather than HTTP timing or arbitrary sleeps. Preserve existing compiled checks until external replacements cover their behavior; do not claim the external workflow is available before its initial integration is built and exercised.

Reuse a known executable directly for repeated cases and external-script changes; do not invoke a build merely to launch validation. Record its source revision and dirty-source snapshot, build features, executable/DLL identity, and runtime asset identity with results. Preserve matching development DLLs when copying an executable. Runtime-loaded asset/data edits can reuse it with a restart, or with asset watching only where enabled and consumed correctly. Embedded `include_str!`/`include_bytes!` content is compiled input. Changed Rust, embedded content, dependencies, features, or build configuration requires a new matching build before claiming validation of those changes. Batch coherent code changes into one build and reuse it across the relevant cases.

Dynamic linking reduces linking work; it does not load edited Rust into an existing executable. Bevy asset hot reload covers supported assets, not Rust systems. System hotpatching still compiles code and has compatibility limits; it is not the default solution for avoiding validation-driver rebuilds. Keep compiler checks, actual-game behavior checks, and release performance evidence distinct.

Review your changes, preserve unrelated work, and make coherent local commits with a clear summary and useful body. Push only when requested. Report what was actually checked and any remaining limitations.
