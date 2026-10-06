# Original material comparison fixture

2026-10-05. Fidelity status: INVESTIGATING.

Original Plutonium r5354 LAN, solo Origins, round 1 Nikolai, 1920x1080 window,
D3D11. RenderDoc frame 6325 replays successfully. Event 4462 is source surface
506, using the byte-identical inspected TECH4 shader pair and source probe 2.
See the [runtime contract](../reverse_engineering/full-scene-render-contracts.md).

The external comparison fixture retains six exact packed corners, source index
bytes, vertex-stage output, both constant buffers, five bound DDS resources with
all mip levels, full sampler metadata, and RGBA8 target readbacks immediately
before and after the draw. Each target is 8,294,400 bytes.

The original target has four MSAA samples. Default raw readback selects sample 0;
the fixture now retains all four before/after sample buffers and an explicitly
computed UNORM arithmetic average with nearest-even rounding. The SDK's saved
DDS resolve differs from that arithmetic and is excluded from admitted comparison
references. A single-sample Bevy render cannot qualify the original boundary
pixels merely by matching interior shading.

The draw changes 15,154 pixels in bounding rectangle x=1164..1311, y=310..539.
The external difference mask identifies visible original surface coverage in the
existing scene. Compare those pixels with an equivalent Bevy camera/projection,
runtime uniforms, images, samplers and linear target. Include original depth
occlusion or restrict comparison to the retained visible mask. The final game
frame contains further draws and postprocessing; this intermediate comparison
does not alone prove final appearance.

Retained captures also include user-triggered PrintScreen observations. Later
camera experiments use explicitly enabled offline cheats. God Mode ON was
observed in the live original HUD; it was not inferred from a console command.
A later teleport returned to the lobby, so god mode must not be assumed to
prevent scripted map deaths or to establish controlled gameplay behavior.

No reconstructed pixel comparison has been completed by this capture lane.
No MATCHED or VERIFIED claim is made. Source/runtime data identity and successful
original capture/replay are distinct from Bevy rendering agreement.
