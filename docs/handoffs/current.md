# Current handoff

TASK: Enable development-only Bevy dynamic linking and 16 codegen units on main; catalog static frames; migrate the entire applicable renderer to deferred in a new worktree, preserving or improving fidelity and achieving at least 10x matched average FPS.

STATUS: PARTIAL.

FINDINGS: Cargo dependency features are selected per command, not per profile. dev-dynamic forwards bevy/dynamic_linking; Debug build/profiling commands select it, and direct development Cargo commands require --features dev-dynamic. Release omits the feature and its profile is unchanged. A compile-time guard rejects dev-dynamic without debug assertions. Bevy 0.19.1 supplies deferred phases and lighting, but its packed G-buffer quantizes response and replaces geometric normals with shading normals; complete existing semantics need deliberate preservation. Avian supplies physics, not a rendering backend.

EVIDENCE: Bevy setup guide https://bevy.org/learn/quick-start/getting-started/setup/ and pinned Bevy 0.19.1 source. External task root: E:/Coding/Captures/origins-deferred-20261008. Previous main normal-window references and per-profile timings remain at E:/Coding/Captures/origins-debug-release-20261008-1791447061644. The later ray-compaction paired debug results remain at E:/Coding/Captures/origins-ray-compaction-20261008/paired/comparison.json and failed their performance gate; they are not a speedup baseline claim.

CHANGES: Cargo.toml adds dev-dynamic and changes only development codegen-units to 16. Cargo.lock adds only bevy_dylib 0.19.1. Debug build/profiling commands select dynamic linking; README and project rules describe the commands. tools/build.ps1 resolves workspace rustc/rustdoc before entering dependency directories and restores the caller environment afterward.

VALIDATION: Locked metadata and cargo fmt pass. Initial build failed because direct Cargo left rustc behind the rustup shim: ktx2's own rust-toolchain.toml selected Rust 1.85 and reported E0514 incompatible bitflags metadata. Only the owned compiler tree was stopped. Repaired full optimized development/dynamic build is running with verified Rust 1.99 compiler processes. No unit tests, fixtures, Clippy or push. Runtime pack is unchanged; asset validation and actual-window qualification remain pending.

UNRESOLVED: Dynamic build and actual-window/DLL-launch qualification pending. Renderer is still Forward+. No fidelity comparison or matched 10x average-FPS acceptance yet. Prior full-debug-survey timeout and wider gameplay limitations remain unresolved.

RECOMMENDED NEXT WORK: Finish and preserve the dynamic build with all required DLLs. Catalog a few existing actual-game static frame references with hashes and camera/profile provenance, then create a new main-based renderer worktree/branch. Implement complete deferred material/lighting paths, including first-person, transparency, shared transport, shadows, atmosphere and diagnostics; validate images and matched workloads before sign-off.

SCOPE NOT IMPLEMENTED: Deferred renderer, renderer speedup, visual acceptance, release rebuild/package, broader gameplay fixes or original-game equivalence.
