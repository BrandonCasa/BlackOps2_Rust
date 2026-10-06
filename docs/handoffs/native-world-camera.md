TASK:
Make the admitted shaded world respond to a canonical Bevy flying camera through evidenced native uniforms.

STATUS:
COMPLETE CPU camera adapter. Parent owns interactive viewer and GPU comparison integration.

FINDINGS:
- Current 694 draws use 22 vertex identities. Native pass mappings identify
  Code213: VS b3 byte0 object matrix; Code229: VS b0 byte576 camera-relative
  view-projection; optional Code237: VS b0 byte768 shadow lookup. All are four
  row-major float4 rows. No Code233 precombined WVP is used by these identities.
- Captured VP rows0/1/3 give right/up/forward axes and lens scales. Canonical
  initial translation is (-684,-89,-2812), with rotation derived from those rows.
  The admitted lens is infinite reversed-Z with near4; projection is preserved
  as an exact matrix, without inventing a far plane.
- Object translation subtracts the new source camera displacement. Original
  model linear coefficients and each distinct object placement remain unchanged.
  Shadow lookup adds that displacement before its captured transform, preserving
  world anchoring of the frozen atlas.
- SH audit: 595 captured b3 blocks at bytes128..176 bit-match the first source
  slice's probe SH. Another 88 mismatches have reflected b3 size64, so SH bytes
  are outside the referenced shader block. Eleven active size176 blocks use
  source probe4 surfaces but contain exact probe26 SH. Their lighting selection
  is unresolved; preserve captured values instead of universally substituting probe4.
- Bank CodeTexture IDs are only6/7/9/15/16/18. No primary lightmap CodeTexture
  appears. Previously qualified custom flags select secondary lightmap t13/s13
  and reflection probe t15/s15; no primary binding is invented.

EVIDENCE:
- Captured frame119013 v1, loaded physical native uniforms, hash-bound shader bank
  argument mappings and translated VS matrix arithmetic; prior captured-world
  and world-vertex-routing handoffs. Original artifacts remain external.
- Source probe SH compared against every retained eligible b3 payload.

CHANGES:
- Rendering native_camera exposes NativeCameraAdapter::from_frame,
  initial_pose, projection, source_view_projection, uniform_updates and
  uniform_patches. Patch updates carry event/binding/offset/bytes for caller's
  material-handle association.
- Camera patches touch only object translation scalars at12/28/44 and the
  qualified VP/shadow matrix ranges; per-material constants and probe SH survive.
- Initial pose returns original bytes exactly. Callers reset with initial_pose
  verbatim; recomputing Euler angles can introduce small rounding differences.
- Invalid/nonfinite poses, nonunit rotations, scaled cameras, unsupported matrix
  layouts, conflicting pass mappings and malformed buffers are explicit errors.

VALIDATION:
- Three synthetic tests PASS: bit-exact reset, canonical XYZ translation,
  yaw/pitch response, distinct object placement, shadow world anchoring,
  patch ownership and malformed input errors.
- Actual external camera admission PASS: all694 templates; moved/rotated updates
  and exact original reset buffers. SH audit counts above were reproduced.
- Rendering all-targets strict Clippy and fmt PASS.
- Opt-in reproduction: set external BO2_SHADER_BANK, BO2_SHADER_BANK_SHA256,
  BO2_FULL_SCENE and BO2_CAPTURED_FRAME, then cargo test -p bo2-rendering --lib
  external_all_694_camera -- --ignored --nocapture.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Fog coefficients, light positions, probe contents and screen-space resources
  remain frozen captures. For example, native fog uses camera-relative height
  with b0 rows27/28; their generation is not proven and is not guessed.
- Runtime producers, arbitrary projection changes, omitted scene contracts and
  full original-game appearance are outside this preview adapter.

RECOMMENDED NEXT WORK:
- Parent viewer applies patches to saved material base buffers, validates initial,
  translated/rotated and reset pixels, then exposes flying controls with explicit
  coverage and frozen-input limits.

SCOPE NOT IMPLEMENTED:
- Viewer/assets edits, full-map assembly, game control, agents or commits.
