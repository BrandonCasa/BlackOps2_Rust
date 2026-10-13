# Origins

Rebuild Black Ops II Zombies Origins in stable Rust and Bevy as a graphical remaster and faithful gameplay clone. Preserve original mechanics, behavior, timing, progression, and balance. Be explicit about missing features and approximations.

Use the smallest maintainable implementation that delivers the requested behavior. Use Bevy and Avian's existing capabilities before adding project infrastructure. Keep one Cargo package and one game executable, `origins.exe`, safe Rust, and the locked dependency versions unless the user requests a dependency change. Do not use subagents unless the current user request explicitly asks for them. Preserve concurrent changes and unrelated staged work.

## Asset extraction and runtime data

Mandatory source locations: `C:\Users\brand\Documents\T6ZMResearch\t6zm-master\` is the only authorized reverse-engineering reference and workspace. Learn the original formats and extraction behavior there, and perform all reverse-engineering work there. Never search elsewhere on this PC for reverse-engineering material, use another copy, or create a duplicate reference workspace.

`C:\Program Files (x86)\Steam\steamapps\common\Call of Duty Black Ops II` is the only authorized source of game assets. Extract required assets directly from that installation. Never search elsewhere on this PC for assets or use assets from another installation, dump, cache, or reference directory. If either authorized location lacks required material, report the gap instead of looking elsewhere.

Never extract and save individual source assets for reference, inspection, debugging, staging, or later reuse. Never create loose asset dumps, reference collections, temporary asset files, or individual converted asset outputs anywhere. Read and transform assets in memory and write generated assets only inside the final runtime pack files. The reverse-engineering workspace is for understanding and implementing extraction, not an exception permitting asset dumps. Preserve the game installation and existing reference material; do not duplicate their assets.

`tools/build_assets.rs` and its Rust utility scripts are the sole asset extraction and conversion pipeline. Their job is to read the original game's contents and produce Bevy-ready pack files, including KTX2 textures and binary data. Extend this pipeline directly; do not create competing builders, wrapper pipelines, or separate preparation frameworks.

Derive game data and asset properties from the original game's source and contents. Store generated runtime data inside pack files, not loose JSON, TOML, configuration directories, sidecar manifests, or external caches. Do not invent replacement values when source recovery is missing; report the specific gap. Preserve the original source inputs. Do not recreate loose asset or configuration trees after the user removes them. Keep necessary Bevy shader assets as shaders; this does not authorize loose generated data.

No project-owned C or C++ code, headers, native extraction helpers, or FFI shims. Do not restore `tools/asset_builder/native_pools.h`. Runtime gameplay must be Rust ECS systems; Bevy must not invoke GSC, Python, PowerShell, C, or C++ scripts. Existing Python or PowerShell gameplay validation tools are permitted, but they must not become asset builders.

Do not add vendored crates or customize, fork, patch, or modify any public crate, including Bevy and Avian. Do not edit dependency sources in place, add local dependency overrides, or recreate removed custom crates under `vendor/`. Use upstream Bevy and Avian APIs and suitable publicly available crates within the dependency-change policy above. If you believe Bevy cannot deliver the requested behavior and cannot find an online crate that does it properly, treat that as a failure of your approach or research: the agent is in the wrong. Revisit the implementation and investigate supported APIs and existing crates; that conclusion never authorizes vendoring, modifying public crates, or reducing the requested behavior.

Do not create or restore build pinning tools, SHA gates, source snapshots, identity manifests, external comparison receipts, or hash-tracking systems. `Cargo.lock` and normal Git history remain the standard dependency and source records. Validate packs by checking that their contents load and their dependencies resolve; do not build a provenance framework around that check. Keep secrets, machine-specific paths, and generated builds out of commits. Use Git LFS for substantial binaries that belong in version control.

## Scope and verification

Do the requested work without expanding it into a framework, migration program, audit system, or documentation project. Delete obsolete code and callers when replacing an implementation; do not keep parallel paths just in case. Do not add speculative abstractions, unnecessary wrappers, or tests that merely repeat the implementation. Never describe scaffolding or a successful build as a finished feature.

Keep `todo.md` as the sole project checklist, with concrete implementation tasks. Leave rendering and graphics work out of it. New documentation and code comments require user authorization. Performance documentation may record procedures, settings, hardware, system load, measurements, limitations, and the relevant Git commit; it must not introduce pinning or identity infrastructure.

Use proportional checks and respect explicit instructions such as "do not test" or "do not commit." Keep compiler checks, actual-game behavior, visual inspection, and measured performance distinct. A review cannot prove that no bugs remain; compilation cannot prove gameplay correctness or an FPS improvement.

Development builds use `cargo build --features dev-dynamic --locked`; release builds omit `dev-dynamic`. Build only when changed compiled inputs require it. Reuse `target/debug/origins.exe` with its matching development DLLs for repeated scenarios. Do not copy the executable alone. Use release builds for release performance comparisons and account for other system load.

Before every game launch, check running game processes across all checkouts and worktrees and check for overlapping GPU captures. If either is active, wait and periodically recheck; continue independent work where possible and recheck immediately before launch. Do not launch another instance, stop another task's process, or ask for redundant launch approval. Close abandoned processes owned by this work normally. Cancel administrator prompts and use tools that do not require elevation.

Use the existing development-only, explicitly enabled, loopback BRP interface in `src/verification/remote.rs` and `tools/test_support.py` for external gameplay scenarios. Consult the locked Bevy crate source and discover supported methods and reflected schemas. Release builds must not expose BRP. Keep scenario sequencing, inputs, waits, assertions, and reports in existing Python or PowerShell tools; keep Rust validation support small and reusable.

Drive gameplay through normal inputs and messages. Direct ECS edits are setup, not evidence that gameplay produced an outcome. Use bounded readiness and state waits, release held inputs, and reset between cases. Fixed-tick timing checks require a reusable scheduled-input hook. Preserve existing compiled checks until external replacements cover their behavior.

Every BRP scenario script must call `client.shutdown(report)` in `finally` before writing its final report, whether it passes or fails. Only close the game owned by that run. Do not claim runtime validation of changed Rust, embedded data, dependencies, features, or build configuration using an older executable.

Review your own changes and make coherent local commits when authorized by the task. Commit only the requested scope; a request to commit staged changes does not authorize staging unrelated files. Push only when requested. Report what changed, what was actually checked, and what remains incomplete.
