TASK:
Render one actual Origins world surface through an external appearance bundle,
retaining the gray viewer and unchanged geometry snapshot reader.

STATUS:
COMPLETE
For the assigned single-surface diagnostic increment only; full appearance incomplete.

FINDINGS:
- Surface508/category0 resolves one original color DDS:128x512,BC1,10mips.
- Original six triangle corners retain recovered UV order, repeatUV,4x anisotropy,
  linear mips, source positions and source-index correspondence without welding/flips.
- Explicit unlit/linear diagnostic mode does not claim original material/lighting.

EVIDENCE:
- Current project state, task0027, architecture, rendering/asset skills, source/tests.
- Pinned OAT7d027e8 T6 declarations, custom-half UV helper, image converter/DDS writer.
- External selected native decoder, bounded pool/stream admission and dependency joins.
- Findings: ../reverse_engineering/materials-0001-textured-surface.md.
- Original zone and geometry hashes remain their expected identities; shared OAT dirty
  status unchanged. External final native run exits0 with0warnings/0errors.

CHANGES:
- bo2-assets bounded versioned manifest/dependency/identity/UV/DDS admission and typed errors.
- bo2-rendering MaterialScene asset, corner-preserving Mesh/nativeDDS image/sampler adapter.
- Viewer --scene offscreen and interactive modes; gray diagnostic paths retained.
  Legacy gray interactive-smoke is rejected for appearance scenes.
- Synthetic loader/path/version/identity/malformed/UV/seam/sampler/linearDDS tests.
- Cargo.lock includes justified JSON,SHA256 and BevyDDS dependencies.

VALIDATION:
- Focused assets/rendering/viewer tests pass; focused Clippy all-targets -Dwarnings passes.
- Outer reviewed source, handoff and rendered pipe capture. Locked workspace metadata,
  workspace tests, formatting, Clippy all-targets -Dwarnings and diff checks pass.
- Actual selected-surface GPU run exits0 at20updates,3x512x512 captures. RawX has
 47616foreground/47578chromatic pixels; oblique42883/42869; rawZ0(edge-on).
- GPU image admission verifies128x512,10mips,Bc1RgbaUnorm and43720source-chain
  bytes match the prepared GPU descriptor before requesting captures.
- Visually inspected rawX: original pipe color texture detail is visible, no gray fallback.
- User-created four-quadrant DDS/plane GPU run exits0 at20updates; rawX quadrant
  ordering red/green above blue/white inspected. No original comparison claimed.

- Main integration preserves the eight-member physics graph and workspace serde pins.
- Combined main gates pass: locked metadata,100tests (one external opt-in ignored),
  formatting, Clippy workspace/all-targets -Dwarnings and diff check. Original GPU
  receipts above were retained; no new original comparison was performed in integration.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Original gamma/UV comparison, normal/tangent formats, full material equations/lighting.
- Remaining world surfaces, streamed dependencies, lightmaps/probes and unsupported families.
- Window appearance mode compiles; physical interactive use not performed by this worker.

RECOMMENDED NEXT WORK:
- Review this first-surface increment, then admit supported attribute/material families
  and selected streamed image dependencies before widening coverage.

SCOPE NOT IMPLEMENTED:
- Full appearance acceptance, lighting, collision, gameplay, imported models, proprietary
  tracked content, original comparisons, commits, pushes or nested agents.
