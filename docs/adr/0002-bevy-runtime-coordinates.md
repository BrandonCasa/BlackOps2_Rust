# Runtime coordinates

Accepted: source `(x,y,z)` maps to Bevy `(x,z,-y) * 0.0254`. Runtime distances are meters, +Y is up, and camera forward is -Z. This basis preserves handedness.

Apply the basis to positions, normals, tangents and model axes during offline preparation. Origins spawn, entry points and navigation nodes use the same conversion. For model placements, conjugate the source rotation by the basis and scale source translation. OAT glTF already uses this Y-up basis; scale its positions without rotating them twice.

The old viewers kept source-sized geometry. That policy is retired. Neither captured camera matrices nor frozen clip-space patches participate in the game camera.
