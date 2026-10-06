# Bevy runtime coordinates

Decision: 2026-10-05, following the user's explicit runtime convention request.

Runtime positions, vectors and cameras use Bevy +Y up, camera-local -Z forward,
and its right-handed basis. Original data is adapted at the engine boundary.
`bo2_core::coordinates` defines the proper rotation `(x,y,z) -> (x,z,-y)` for
source Z-up data and its inverse. It preserves lengths, handedness, winding and
raw unit scale; positions and directions use the same rotation. Normal vectors
use that rotation, while UVs and indices are unchanged. Source AABB adaptation
exchanges reflected endpoints.

Rendering depends on core and applies the shared adapter to snapshot and
textured-corner positions. Viewer cameras frame those canonical positions; no
camera-axis workaround or per-launch orientation flag is required. The raw
`diagnostic_mesh` remains available for source inspection, while the viewer uses
`canonical_mesh`. Raw reader records, surface metadata, physics diagnostics and
the native-evidenced trace contract stay in source space. Future engine-facing
physics/gameplay consumers must adapt through this boundary rather than changing
raw inspection/native arithmetic. No gameplay adapter is implemented here.

This is application policy, prompted by the vertical map display and the user's
request. It does not prove original angle semantics, physical units, camera basis
or complete visual fidelity. [Coordinate investigation](../reverse_engineering/0009-origins-coordinate-evidence.md)
retains its evidence limits. Fidelity is UNKNOWN pending original comparison.
