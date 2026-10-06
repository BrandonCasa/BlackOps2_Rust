TASK:
Make the captured original shadow shader execute with its native float atlas.

STATUS:
COMPLETE

FINDINGS:
- WebGPU forbids CPU texture writes into Depth32Float. The previous probe hid
  that validation failure until its screenshot deadline.
- Captured R32 atlas contains 33,554,432 finite values in [0, 0.29022455].
- A GPU textureLoad-to-fragment-depth pass preserves those values for native
  comparison sampling; source values outside [0,1] are explicitly rejected.

EVIDENCE:
- Original source2546/event4633 fixture and captured R32_FLOAT SRV with GREATER
  comparison sampler. Original payloads, receipts and images remain external.
- Bevy PipelineCache, RenderDevice, RenderQueue and ordered Render systems.

CHANGES:
- Viewer depth_atlas module uploads float data to a color texture and copies it
  into a depth attachment through Bevy's GPU facilities before scene rendering.
- Manual probe advances pipeline tasks and device callbacks and reports GPU
  validation failures immediately instead of treating them as readiness delays.

VALIDATION:
- Native translated shadow draw PASS: 785 fully covered pixels, 3,140 channels
  within 2/255, maximum error 1/255, mean error 0.001592/255; 184 partial pixels excluded.
- Original opaque regression PASS: 14,826 pixels, maximum error 1/255.
- Synthetic GPU 2x2 depth copy PASS: values 0, 0.25, 0.75 and 1 retain texel
  orientation and expected readback. Viewer strict all-target Clippy PASS.
- These are frozen captured resource comparisons, not complete scene fidelity.

FIDELITY STATUS:
MATCHED
Only the admitted fully covered response of the captured shadow draw is matched.

UNRESOLVED:
- Runtime shadow generation, boundary coverage and full frame execution.

RECOMMENDED NEXT WORK:
- Reuse depth uploads for captured world-frame resource bindings.

SCOPE NOT IMPLEMENTED:
- Whole-scene completion, gameplay or a replacement renderer.
