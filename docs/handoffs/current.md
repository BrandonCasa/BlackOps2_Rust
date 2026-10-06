TASK:
Correct reference-room appearance and the shared material conversion responsible for stretched, fuzzy pillar textures.

STATUS:
PARTIAL - shared tile/macro preparation and Bevy lighting implemented; original appearance and other world layer families remain approximate.

FINDINGS:
- Pillar source materials use two repeating diffuse surfaces plus a macro control map. Selecting only the first diffuse bound the coarse 256-square core texture across the complete model. This is a shared family conversion defect, also affecting rubble, rocks and statues.
- Source smooth concrete is 512-square; the macro map is 512-square. A larger output alone cannot create source detail. Proper layer choice, tiled detail and macro weights address the stretched appearance.
- The source lightmap PNG is a three-band packed radiance/direction atlas, not ordinary displayable color. Runtime now decodes its radiance bands before Bevy samples it.

EVIDENCE:
- Retained external model-surfaces.json, standard OAT model exports and decoded raw PNGs establish texture bindings/UVs. Original input files are untouched. User screenshots guide appearance, never select geometry/camera.
- Git history supplied packed-stream/channel research; retired renderers were not restored.
- External tile-material-import.json and tile-material-window contain final import and actual-window evidence.

CHANGES:
- Generic tile_blend family handling bakes both diffuse layers with macro red/green weights in linear space at 2048 square, retaining all raw layers. Thirty-one material records share fourteen new outputs; no pillar-name or placement overrides. Compact metadata records the four-repeat tiling approximation; importer requires a derived diffuse binding distinct from all three raw layers.
- Regenerated runtime assets stay under assets/origins, preserving standard relative references, shared providers and placements.
- Shared decoded linear lightmaps, restrained cool fill/sunlight/fog, warm emissive lanterns and point lights at glass bounds, nearby lantern shadows, indoor/outdoor exposure and smaller HUD. Concrete remains matte; runtime normal XY strength is reduced. Added albedo smoothing was removed.
- Updated existing project, asset and rendering references; no added Markdown files, shaders, dependencies, viewers or tests.

VALIDATION:
- Locked metadata, release build and formatting passed. Cargo was absent from shell PATH; gates used the existing user Cargo binary. Earlier compile failures for mutable asset access and NotShadowCaster import were corrected.
- An import overlapped an attempted regeneration and rejected differing copied hashes. Regeneration was stopped; the pack was regenerated completely before the final import. Final import/source reconciliation passed: 1,252 runtime files, 1,249 PNGs, 690,620,957 bytes. Staged closure passed: 1,251 LFS pointers and ordinary Git game.json. A direct index check initially selected an older Python without tomllib; rerunning with Python >=3.11 passed.
- Earlier actual-window runs passed movement, Round 2, eight map views and restart. Final release-window driver exited 0: movement=true, round_two=true, restart_round=1, survey_views=8. Spawn screenshot inspected: pillars now show smooth coated surfaces and localized aggregate damage; warm lantern light is visible. Purple surfaces and ceiling/helper geometry remain visibly incorrect.
- Skipped: unit tests, fixtures, suites and Clippy (banned), sustained/manual play, native shader equivalence and push.

UNRESOLVED:
Fourfold tile scale and macro channel interpretation are practical reconstructions, not recovered native arithmetic. Normal/specular detail layers are retained but not fully combined. Other world layered materials still select the first layer; secondary UV/control evaluation, remaining purple/helper surfaces, probe/SH shading, exact directional lightmap response and original visual effects remain unfinished. Runtime lightmap radiance clips above four. Original compressed mip chains are not retained. The source textures impose a real resolution limit.

RECOMMENDED NEXT WORK:
Implement qualified world layer/secondary-UV handling and remaining control-map dispositions, then refine tiled normal/specular response and probe lighting in the same Bevy application.

SCOPE NOT IMPLEMENTED:
Complete pixel equivalence, general native layer shader reconstruction, native registers/buffers/pipelines, original shader import, higher model LODs, gameplay expansion, test suites, delegated work or remote push.
