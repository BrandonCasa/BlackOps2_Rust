TASK:
Implement a source-informed Origins-inspired opaque Bevy material for ordinary world and static-model surfaces.

STATUS:
COMPLETE - opaque material milestone implemented; original arithmetic and complete scene appearance remain approximate.

FINDINGS:
- The retained pack already supplies material identity/family/channels/preparation, tangents/signs, colors/UVs, raw specular/gloss, packed directional lightmaps, probe faces/SH and static lighting assignments. No extraction or runtime pack regeneration was required.
- The selected opaque research establishes normal slopes, RGB/alpha light radiance and normal-dependent directional irradiance. Native intermediate color transfer and reflection modulation do not map directly to Bevy PBR; explicit calibration is appropriate.
- Startup material/lightmap layout changes on visible glTF instances exposed a Bevy render validation failure. Assembly now occurs while the render root is hidden; Bevy reveals it once opaque assignments are ready. Collision copies remain hidden and excluded.

EVIDENCE:
- Current runtime metadata/preparation/import source and retained opaque contract from Git commit dbf869c informed the implementation. Original shader binaries were not loaded or restored.
- External opaque-material-import.json records unchanged runtime pack reconciliation. External opaque-material-window-reveal holds the final actual-window logs, screenshots and verification.json. Earlier failed/visual calibration runs remain external.

CHANGES:
- Added project-owned embedded opaque.wgsl and an ExtendedMaterial implementation within the existing application. Raw source slopes, spatial gloss/F0, directional/HDR lightmaps and exact assigned Bevy-filtered reflection cubes replace blanket normal softening, fixed reflectance and concrete-name roughness overrides.
- Tile/macro normal/specular layers use retained control weights and preparation tiling; the existing diffuse bake is sampled once. Missing channels have explicit physical defaults. Raw channels remain intact.
- Static lighting uses retained instance-color means or assigned source-basis SH. Baked opaque surfaces exclude duplicate ambient/sunlight diffuse, with explicit radiance-unit calibration. Lantern lighting, cover-based exposure, fog, AgX and bloom remain Bevy-owned.
- Scene reveal waits for material assembly. The existing real-window driver sweeps position/yaw at eight viewpoints and records opaque coverage/pending counts. Updated existing state/asset/rendering references; the tracked Markdown working set remains 20 files.

VALIDATION:
- Locked metadata, release build and formatting passed using the existing Cargo executable. Asset import --check/source reconciliation passed: 1,252 files, 1,249 PNGs, 690,620,957 bytes; runtime pack bytes are unchanged.
- Initial Rust compile issues (mutable mesh borrow/private system parameter) and a WGSL import dependency were corrected. The first source-radiance pass was too dark; explicit Bevy-unit calibration restored visibility without duplicate baked light. A later startup bind-group mismatch was corrected by hidden scene assembly.
- Final actual-window run passed movement, six kills/Round 2, eight position/yaw surveys and restart to Round 1, with no render/shader errors. Coverage: 2,132 rendered world groups and 15,536 static primitives; 15,945 normal bindings, 3,689 specular/gloss bindings, 17,668 assigned reflections, 2,132 directional atlases, 1,464 instance-color means, 14,072 SH fallbacks and 232 tile/macro primitives across 3,808 shared material variants; pending=0. Counts are runtime primitives/groups, not original source surfaces or placements. Inspected final spawn/gameplay and indoor/outdoor survey images show source normal detail, differentiated surface response, readable geometry and warm lanterns; outdoor vegetation/helper defects remain.
- Skipped: unit tests/suites/fixtures and Clippy (banned), packaging, sustained/manual play, exact native equivalence, original mip recovery and push.

UNRESOLVED:
Gloss exponent, F0/radiance calibration, tile-family normal strength and fourfold tiling are reconstruction parameters. Static color means lose within-model gradients. Probe SH interpretation, source direction/UV decoding, sRGB versus native intermediate transfer, reflection normalization and native lobe arithmetic remain approximate. Source mip chains are absent. Existing vegetation alpha, layered/control-map and helper/debug-color defects remain outside this milestone.

RECOMMENDED NEXT WORK:
Address general layered terrain/control disposition and cutout/transparency conversion independently, then preserve per-vertex static lighting without duplicating providers. Refine calibration against licensed moving-view references within the same Bevy application.

SCOPE NOT IMPLEMENTED:
General terrain layering, cutout/transparency, water or atmosphere redesign; exact native arithmetic/pixel matching; another pipeline/viewer, original shader import, retired renderers, gameplay expansion, tests, delegated work or remote push.
