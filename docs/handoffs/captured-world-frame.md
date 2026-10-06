TASK:
Export one retained original frame's admitted world dispatch with exact per-draw resources and a validated engine-independent Rust loader.

STATUS:
COMPLETE for the bounded admitted main-color dispatch; the complete original frame remains PARTIAL.

FINDINGS:
- Retained D3D11 frame 119013 has 4,031 ordered draw receipts. Exact primary-corner joins establish 1,020 world draws and 1,321 distinct source surfaces across color and depth passes.
- The admitted main-color dispatch contains 694 draws, 1,248 distinct source surfaces, 131 materials and 37 technique families. Source formats present are 0/1/2/4/5/7; format 3 is absent from this interval.
- All 694 admitted draws pass exact primary and secondary input, native shader identity, physical constant-register, object-matrix and reflected resource admission. Batched draws retain multiple source slices; matching only the first source surface was insufficient.
- The full census explicitly excludes 326 source-joined depth-only draws and 1,489 draws with world-bank shaders but no exact primary join. Another 1,522 draws use shaders outside the admitted world bank. These counts total 4,031 with the admitted 694.
- Source b3 matrices are row-major, with translation at float indices 3/7/11. The common matrix, used by 681 draws, translates by (684,-2812,89); 11 and 2 draws have different retained translations. Camera origin (-684,2812,-89) is derived from the common static-world matrix, not an independently measured camera receipt. The source b0 view-projection rows at byte 576 are retained separately.
- A source UV exporter defect affected 1,630 of 449,721 corners: negative zero and negative subnormals disagreed with captured hardware R16G16_FLOAT interpretation. External UVs were regenerated from admitted packed records; the prior artifact was preserved externally. Full-scene admission now rejects this inconsistency using a private shared hardware half decoder.

EVIDENCE:
- Existing source geometry, full-scene, secondary-stream and native shader-bank APIs; retained capture and RenderDoc 1.46 public replay headers.
- External artifact key: full-scene-capture/captured-frame119013-v1/frame.json. Export tools remain external: frame-export.cpp, initial-export.cpp and build-frame-manifest.py. Original bytes remain external.
- Capture SHA-256: 03983025df820f56c23d7a82dafc874e3576147c9d25b6db60575feeecc26f87.
- Final manifest SHA-256: 85f48cf1e3cdad16c02d5d1b119b78399c26eb0cec3c1900a1198d8840309bd0.
- Dynamic image versions are tied to writer events 1716, 17070 and 17089. Static and dynamic payloads are content-hash deduplicated; loaded uniform/image payloads share Arc storage.
- Initial color and depth are retained immediately before event 6562, with four raw samples each. Color is RGBA8 at four bytes/pixel. D32S8 readback is eight bytes/pixel: float depth occupies the first four bytes; the trailing four bytes are zero in every retained pixel. Initial depth is finite, in [0,0.9846163]. All admitted stencil tests are disabled. Renderer depth initialization can extract the first float from each eight-byte record while retaining the original samples.
- Old/new external UV SHA-256: b9fa94d9bcacc75114796c634b8aaea654e41e7ed8c86d7ca01537012a206ca3 / 12d828ae8a5d51ac3e58717fc34693e293c759462df2e092cad00e2644c72522.

CHANGES:
- Added assets::captured_frame with versioned receipts, exact source slices, physical stage/register bindings, image views, samplers, pipeline/attachment state, camera data and initial samples.
- Loader validates paths, hashes, limits, source XYZ/attributes/UV/secondary inputs, draw ranges, native shader reflection, DDS dimensions/mips/cube/typed format, comparison samplers, object bytes and producer references. Missing dependencies are explicit errors.
- Added captured-frame synthetic safety checks and an opt-in external admission test. Added signed-zero/subnormal consistency regression to the full-scene tests. Moved the existing half decoder into a private shared appearance helper, avoiding a circular dependency between full-scene and secondary-stream modules.
- No game input, restart, capture deletion, commit or rendering/viewer edit was performed for this task.

VALIDATION:
- cargo test -p bo2-assets: PASS; external tests remain explicitly ignored in this synthetic run.
- cargo clippy -p bo2-assets --all-targets -- -D warnings: PASS.
- cargo fmt --all -- --check: PASS.
- Opt-in external_world_dispatch_admission: PASS, 4,031 receipts / 694 world draws, including initial attachments.
- Synthetic tests cover payload identity/missing aliases/path traversal/size limits/shared Arc storage, complete float DDS views, physical register mapping and hardware half UV consistency.
- All native texture views use the full retained mip range; zero view-versus-resource mip mismatches.
- This is CPU/data admission. Scene-wide Bevy execution, comparison and sustained traversal have not been performed by this task.

FIDELITY STATUS:
INVESTIGATING. Earlier selected opaque/shadow GPU comparisons remain scoped evidence; the admitted dispatch has not yet been rendered as a scene-wide Bevy comparison.

UNRESOLVED:
- Initial samples account for prior target contents, but excluded draws interleaved with admitted world draws still affect subsequent color/depth. This world-only dispatch cannot establish complete original-frame equivalence.
- Attachment and sampled-image producers are identified, not reconstructed as dynamic Bevy producer systems. Frozen captured image versions are valid only for this retained frame.
- World-bank draws without exact source joins, depth-only passes, other model/entity geometry, effects, final post-processing and out-of-view material families remain outside this admission.
- Native exporter implementation should adopt the hardware half correction before regenerating the external UV artifact.

RECOMMENDED NEXT WORK:
- Consume the admitted 694 ordered native draws through Bevy facilities, including retained primary/secondary attributes, exact physical uniforms, samplers and states. Initialize four color/depth samples explicitly or retain initialization as an explicit unsupported requirement.
- Validate a matched original/reconstructed main-world interval while declaring excluded interleaved passes. Expand source joins and producer systems in separate bounded tasks.

SCOPE NOT IMPLEMENTED:
- Whole-frame equivalence, all 6,469 visible source surfaces, all 56 families in one camera, gameplay, final commit/merge or renderer integration.

NATIVE STATE CENSUS AND ENUM INTERPRETATION:
- Numeric enums are RenderDoc 1.46 replay enums from replay_enums.h/common_pipestate.h, not D3D11 enum numbers. Uniform bN maps to binding N for VS and 16+N for PS; textures tN map to 32+N and samplers sN to 64+N. Reflection ordinal is never treated as physical register.
- CompareFunction: Never=0, AlwaysTrue=1, Less=2, LessEqual=3, Greater=4, GreaterEqual=5, Equal=6, NotEqual=7. All draws use GreaterEqual; the 72 comparison samplers use Greater.
- CullMode: NoCull=0, Front=1, Back=2. Back occurs in 691 draws; NoCull in 3. All fronts are clockwise. Depth clip is enabled; conservative raster and forced samples are zero.
- BlendMultiplier used here: Zero=0, One=1, SrcCol=2, SrcAlpha=6, InvSrcAlpha=7, InvDstAlpha=9; BlendOperation Add=0. Color blend is disabled in 669 draws. Enabled color equations: One/InvSrcAlpha 19, SrcAlpha/InvSrcAlpha 3, Zero/SrcCol 2, One/One 1. Alpha equations: InvDstAlpha/One 24, Zero/One 1. Write masks are RGBA=15 in 685 draws, RGB=7 in 9.
- Viewport is (0,0,1920,1080,0,0.984375); scissor is (0,0,8192,8192), enabled, in all draws. Render target is R8G8B8A8_UNORM with four samples. Alpha-to-coverage is disabled and sample mask is 0xffffffff. Depth bias/slope are 0/0 in 661 draws, 256/1 in 32 and 512/2 in 1.
- AddressMode: Wrap=0, Mirror=1, MirrorOnce=2, ClampEdge=3, ClampBorder=4. Recorded U/V/W tuples: Wrap/Wrap/Wrap 2,113, ClampEdge/ClampEdge/Wrap 1,330 and Wrap/ClampEdge/Wrap 8. W wrapping is retained even where a 2D view makes it unused.
- FilterMode: Point=1, Linear=2, Anisotropic=4; FilterFunction Normal=0, Comparison=1. Anisotropic normal samplers: 2,089; linear normal: 1,281; linear comparison: 72; point normal: 9. All mip biases and border colors are zero.
