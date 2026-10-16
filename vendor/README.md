Bevy is a Git submodule of BrandonCasa/bevy's `release-0.20.0` branch. Origins records
its own commit exposing dlss_wgpu's current-color bias texture to the SR node.
Only `bevy_anti_alias` is overridden; other Bevy crates remain registry dependencies.

Initialize with `git submodule update --init vendor/bevy`, then build normally
with `cargo build --features dev-tools`.

Commit and push dependency changes in `vendor/bevy`, then commit its new revision
in the game repository.
