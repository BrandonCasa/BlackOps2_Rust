TASK:
Expose the accepted native shaded preview in the full-map viewer with Bevy flight controls.

STATUS:
COMPLETE for the scoped interactive preview; original full-scene rendering remains PARTIAL.

FINDINGS:
- Captured family/state/vertex contracts admit 6,296 of 6,469 source surfaces,
  grouped into 718 material/lightmap/probe draws. The remaining 173 are explicitly
  counted as lacking a captured contract, without diagnostic material substitution.
- Source material constants/textures and per-surface secondary lightmaps/probes
  are assembled through existing validated pass resources. All 1,113 translated
  shader handles remain retained. Native samplers, mips and linear typed views persist.
- Full-map source positions use identity world placements qualified by captured
  templates. Distinct captured model placements cannot establish other placements.
- SH substitution requires a matching reference probe block; unresolved captured
  lighting overrides are preserved. See native-world-camera.md for the 11 overrides.

EVIDENCE:
- Existing captured-world frame/dispatch, world vertex routing and pass-resource
  contracts; external frame119013, full-scene manifest, shader bank and secondary stream.
- NativeCameraAdapter provides matrix-owned patches; source-to-Bevy axes remain
  (x,z,-y). Shader bytecode, image contents, captures and test outputs remain external.

CHANGES:
- --native-fly accepts the four explicit external inputs. Existing captured-world,
  single-surface and gray/unlit diagnostic modes remain available.
- Flight controls: WASD, Q/E vertical, arrows turn/look, Shift faster, Control slower,
  R exact initial pose, Escape exit. Fixed 1920x1080 viewport preserves frozen inputs.
- Interactive clear attachments replace frozen initial color/depth; retained original
  weapon/silhouette pixels are absent. Camera patches preserve material constants/SH.
- Uses Bevy Window, Camera, Mesh, Material, Assets and existing native dispatch plugin.
  No replacement renderer or gameplay controller is introduced.

VALIDATION:
- External GPU initial/moved+rotated/reset full-map renders PASS, with over 100,000
  shaded pixels per view; movement changes pixels and reset is pixel-exact.
- Native camera synthetic matrix/ownership/errors and exact pose reset tests pass.
- Workspace gates and main integration results are recorded in project-state.md.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- 173 omitted surfaces; runtime shadows, fog/light global generation, screen-space
  inputs, time/animation, additional passes and post-processing remain frozen or absent.
- Source-order draw grouping is inspection policy, not proven arbitrary-view native
  transparency/visibility ordering. The preview is not a complete game or matched frame.

RECOMMENDED NEXT WORK:
Qualify omitted contracts and runtime producers, then compare matched arbitrary views.

SCOPE NOT IMPLEMENTED:
Gameplay, original game control, remaining shaders/passes, runtime lighting producers.
