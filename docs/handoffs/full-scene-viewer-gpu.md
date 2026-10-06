TASK:
Integrate bounded full-scene texture diagnostics and captured material GPU comparisons.

STATUS:
PARTIAL

FINDINGS:
- The full primary-color diagnostic renders all 6,469 surfaces as 280 grouped meshes
  with 114 shared image/sampler pairs. It explicitly marks 77 no-color surfaces and
  reports 2,327 layered surfaces; these are not original material admissions.
- The exact captured source506/event4462 shader pair executes through Bevy Material.
- Original target has four samples. Sample-zero data cannot be compared directly
  with center sampling or a multisample resolve at silhouette/occlusion boundaries.

EVIDENCE:
- Asset, shader-bank and rendering handoffs; externally retained RenderDoc frame6325.
- Fixture source corner byte equality, image payload equality, SH equality and native
  constant buffers, typed shader resources and pipeline/viewport records.
- External GPU receipts and readbacks; no generated original data is tracked.

CHANGES:
- Viewer CLI retains raw, gray and single-texture modes and adds explicit texture
  diagnostics, synthetic material probe and captured handwritten/translated probes.
- Generic bounded GPU readiness, pipeline error, deadline and screenshot checks.
- Captured translated programs selected by native bytecode hash, rather than indices.
- Matching MSAA resolve comparator admits only pixels changed in all four original
  samples and reports excluded coverage; errors are not hidden by masking actual output.

VALIDATION:
- Synthetic TECH4 GPU: center161/117/84/255 agrees with analytic response within2 bytes.
- Full texture diagnostic GPU:24 updates;19,820 foreground pixels.
- Translated original pair GPU:24 updates;14,826 fully covered native pixels;
  all59,304 channels within2/255, maximum1/255, mean0.003794/255.
- Excludes674 partial/occluded sample pixels. Raw sample-zero comparison remains
  visibly different (maximum12/255) and is retained in the receipt.
- Meaningful comparator unit tests cover missing samples, no evidence, partial
  coverage, deterministic resolution and retained mismatches. Workspace gates pending.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full original pass dispatch, engine globals, secondary inputs, shadows and effects.
- Boundary coverage, original scene depth, complete frame comparison and traversal.
- Workspace acceptance and main integration have not completed.

RECOMMENDED NEXT WORK:
- Integrate the explicit pass-resource assembler and evidenced engine resource adapter.

SCOPE NOT IMPLEMENTED:
- Original full-map appearance claim, gameplay, asset redistribution or merge.
