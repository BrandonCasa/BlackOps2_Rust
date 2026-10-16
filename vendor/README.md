Bevy is a Git submodule based on the upstream 0.20.0 release. Origins records
its own commit exposing dlss_wgpu's current-color bias texture to the SR node.
Only `bevy_anti_alias` is overridden; other Bevy crates remain registry dependencies.

Initialize with `git submodule update --init vendor/bevy`, then build normally
with `cargo build --features dev-tools`.

Commit dependency changes in `vendor/bevy`, then commit its new revision in the
game repository. Publish the dependency commit to a fork and update `.gitmodules`
before distributing a game revision that depends on it; the upstream Bevy remote
does not contain local Origins commits.
