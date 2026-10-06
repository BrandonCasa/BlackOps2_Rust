TASK:
Make origins.exe use committed repository assets by default and package a portable adjacent asset pack. The user's follow-up removes runtime overrides and additional packaging flags.

STATUS:
COMPLETE — repository/Cargo, other-working-directory and relocated-package launches verified in the actual game window; local integration complete.

FINDINGS:
- Asset discovery starts at the executable: adjacent assets/origins takes priority; Cargo outputs find the repository through executable ancestors. Working directory, original BO2 installation and machine-specific configuration are unnecessary.
- Runtime --config and its TOML dependency are removed. The existing --verify driver remains. Local configuration is exclusively for offline preparation/import.
- Release pack: 557 runtime files, 322,002,166 bytes, including game.json, map.gltf, map.bin and 554 PNGs. Copy the whole release directory when relocating.

EVIDENCE:
- External logs, screenshots, verification receipts and relocated package: %TEMP%/origins-portable-20261006 (cargo, other-cwd, relocated).
- All three real-window runs exited 0 and reported movement=true, round_two=true, restart_round=1, survey_views=8, source_surfaces=6469 and static_instances=14480.
- Logs select repository assets for the other-CWD run and relocated-package/assets/origins for the portable run. Portable launch used an unrelated CWD plus nonexistent BEVY_ASSET_ROOT/CARGO_MANIFEST_DIR values.
- missing-pack.png shows the actual error window with the missing directory and restoration/Git LFS instructions; missing-pack.log records the same error.

CHANGES:
- Replaced mandatory runtime local configuration with executable-relative discovery. Bevy AssetPlugin receives the resolved absolute pack root and retains scene/dependency loading ownership.
- Loading errors include failing paths, Bevy dependency errors and recovery steps; HUD text wraps within the window.
- tools/build.ps1 runs metadata/release/fmt gates, invokes the existing offline importer directly to validate/copy the complete runtime closure and compare every hash, and ships assets beside origins.exe. It removes the obsolete packaged local config.
- Existing README, architecture, project state, asset reference and config template updated. No new Markdown files, executables, dependencies, tests or CLI flags.

VALIDATION:
- Compiled: locked Cargo metadata, release workspace build and formatting gates passed through tools/build.ps1.
- Asset integrity: glTF references/scene counts, buffer/index/accessor bounds, finite values, decoded PNGs, lightmap dependencies, exact file closure, unresolved-LFS rejection and copied SHA-256 comparison passed without original inputs or preparation receipts.
- Rendered/played: the existing same-application driver passed Cargo, other-CWD and relocated-package runs; repository spawn screenshot inspected. Missing-pack error window inspected and closed.
- Actual failures/fixes: removing TOML required an offline lockfile refresh before the locked build passed. Packaging initially selected Python below 3.11; selecting Python >=3.11 fixed tomllib import and packaging passed.
- Skipped: unit tests, test suites and Clippy (banned); fresh preparation/source reconciliation (runtime bytes unchanged); manual or sustained play; original-game equivalence; push.

UNRESOLVED:
- Git checkouts require materialized LFS payloads; package assets must remain beside the executable. Build-time validation needs uv, Python >=3.11, NumPy 2 and Pillow 12; the game package does not.
- Cargo output discovery assumes the executable is within its repository; custom target directories outside the checkout need the adjacent pack layout.
- Existing rough materials, temporary weapon/zombie visuals and remaining original systems are unchanged.

RECOMMENDED NEXT WORK:
Refine material presentation and continue the existing complete-systems roadmap in the same game application.

SCOPE NOT IMPLEMENTED:
Runtime asset overrides (removed at user direction), asset regeneration, material/gameplay changes, custom external Cargo target discovery, exact original equivalence, remote upload/push.
