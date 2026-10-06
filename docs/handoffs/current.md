TASK:
Correct the loose rocks/rubble so their surface response and illumination fit the supplied original room reference; make a local commit.

STATUS:
COMPLETE - scoped rubble shading/lighting correction validated in the actual game. Exact original image equivalence remains unverified.

FINDINGS:
- Tile/macro normal binding 0 contains whole-object shape. The previous shader incorrectly tiled it fourfold, blended it as core detail and never sampled detail normal 2.
- Retained normal bindings 1/2 are coating/core, paired with color bindings 1/0. Specular bindings 0/1 also follow coating/core order. Macro red/green weights coating/core; diffuse already retains macro occlusion.
- Normal correction alone helped but did not resolve the user's appearance concern. At the spawn-room rubble, 708 nearby up-facing world corners have median irradiance RGB approximately (80,147,183); upward assigned SH at ordinary gain 22 gives (32,52,63). Separate tile gain 60 gives (86,142,172), narrowing the lighting discontinuity.

EVIDENCE:
Supplied original/current clipboard comparison; retained external game-prepare/instances/model-surfaces.json and committed material channels. Rubble macro/detail identities are p6_rocks_cluster_01_n, rubble_concrete_dust01_n and concrete_wall_broken_n; pillar/monolith bindings confirm the same layout. External inspection/rubble-normal-fix contains run-1 (normals only), run-2 (final), lighting-calibration.json, cargo-metadata.json and comparison.jpg. The comparison uses the previous visual-profile/run-11 and final run-2 room view 8; final camera receipts preserve pose/projection.

CHANGES:
- Sample macro normals at authored UVs; add masked coating/core detail slopes at fourfold UVs. Correct corresponding specular sampling and weights. Keep the shared diffuse bake sampled once.
- Add embedded tile_probe_radiance=60 for probe-only tile/macro materials. Ordinary prop gain 22, vertex/atlas gains and retained probe colors remain in use. Existing shared providers/placements and Bevy material pipeline remain intact.
- Update rendering guidance and this one handoff. Runtime asset bytes remain unchanged.

VALIDATION:
- Final locked Cargo metadata, release build and formatting passed with the installed toolchain; git diff whitespace check passed. Complete committed runtime closure validation passed with 1,252 hashed files.
- Both actual game-window driver receipts passed movement, Round 2, restart to Round 1, all 15 views and pending=0. Final info-level log has no shader/render warnings or errors. Inspected final fixed room, room side view and outdoor captures; loose rocks have clearer shape response and lighter cool illumination that joins the surrounding terrain.
- Initial texture inspection failed because default Python lacked Pillow; reran successfully through the existing uv dependency environment.
- Tracked Markdown remains 20. No tests/suites/fixtures or Clippy were added or run. Skipped manual/sustained play, packaging, pixel/native parity and push. Driver exercise is not manual play.

UNRESOLVED:
The later supplied close-up calls for a broader broken-concrete pattern on large rocks. That diffuse-pattern refinement was not implemented: the user requested committing the current normal/lighting changes and stopping further refinement. Fourfold repeats, slope addition and family radiance calibration are practical reconstructions. SH/native light transfer is approximate. The supplied original view differs in camera pose. Whole-room brightness, native smoke/sky and individual surface responses are outside this bounded correction.

RECOMMENDED NEXT WORK:
Continue reference-driven local surface refinements where useful. Preserve the complete gameplay roadmap; replace temporary weapon/arms, actors and HUD through licensed preparation in a separate milestone.

SCOPE NOT IMPLEMENTED:
Geometry redesign, new asset preparation/import, original shaders, a separate renderer/viewer, capture-fed scene content, unrelated gameplay/atmosphere changes, tests, Clippy, packaging or push.
