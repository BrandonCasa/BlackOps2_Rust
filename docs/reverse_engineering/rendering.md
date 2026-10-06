# Standard rendering

The previous viewer depended on captured draw membership, translated native shaders, frozen lighting/resources and camera patches. It admitted 6,296 of 6,469 world surfaces and omitted static model rendering. That is unsuitable for an arbitrary player camera.

The game loads the complete world and all placed static models through standard glTF. Bevy owns PBR, transparency, shadows, bounds, culling, cameras and post-processing. Normals, source tangents/handedness, RGBA vertex colors, UV seams and world secondary UVs survive preparation. An opaque shadow-helper surface receives collision disposition without being drawn as a visible wall.

Current practical material conversion selects the first color/normal layer, reconstructs BC5 normal Z, flips DirectX normal Y and maps gloss alpha to roughness without clamping its range. Most surfaces are dielectric. All available layers and raw specular RGB/gloss alpha survive as PNGs referenced by compact surface metadata. Alpha-tested surfaces use masks; blended/special surfaces and water use standard approximations. Source lightmap images are attached through Bevy Lightmap. Both lightmap channels, all probe faces/coefficients and world/static assignments are retained for later material use. HDR, AgX, bloom and distance fog are camera components.

This is APPROXIMATE. Layer mixing, source reflection probes, exact native lighting, animated water, particles and special visual families need later work in the same game. Captured-frame equality is not an acceptance requirement. Evaluate coherent appearance and complete geometry from moving/distant views.

Release surveys account for the complete source geometry but expose rough vegetation alpha, debug-colored surfaces and helper-like shapes in some locations. These are unresolved conversion/disposition defects; successful loading and round progression do not establish complete appearance.
