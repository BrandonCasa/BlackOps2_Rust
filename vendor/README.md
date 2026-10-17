Bevy is a Git submodule of BrandonCasa/bevy. Use the commit recorded by Origins,
which includes native rendering support, DLSS current-color bias, and optional
Neural Rendering. Cargo patches the Bevy crates to this checkout so the game,
Avian, and renderer share the same engine and ECS types.

Initialize with `git submodule update --init --recursive`, then follow the
[root setup and build instructions](../README.md). Development builds use
`cargo build --features dev-tools --locked`; release builds omit `dev-tools`.

Commit and push dependency changes in `vendor/bevy`, then commit its new revision
in the game repository.
