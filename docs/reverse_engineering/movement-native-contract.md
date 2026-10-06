# Native movement qualification

Task movement-native-01, 2026-10-05. PARTIAL. Source/configuration fidelity is
INVESTIGATING; the numerical movement controller remains UNKNOWN. No original
game input, capture, invocation or process reads were performed.

## Qualified source and reproduction

The installed Plutonium T6 Zombies disk executable has 13,090,072 bytes and
SHA-256 `2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
Its preferred image base is 0x400000. Use PE section mapping for RVA reads;
the code section starts at RVA 0x1000/raw offset 0x400. This is a distinct disk
identity from the observer's executable identity and Steam executable, not a
claim of unmodified Steam behavior. All 15 complete retained task0035 native
functions, 8,515 bytes, match this disk image exactly at their documented RVAs.
That independently qualifies this image as a static source for the existing
collision contract. No unqualified data relocation is inferred from that match.

External `movement-native-01/qualification-receipt.json` records five additional
complete functions, hashes, lengths, retained-code checks and script identity.
Resolve the inspection root through ignored local configuration. External
helpers use pefile and Capstone 5.0.7 to map/read disk bytes and decode x86;
raw bytes, disassembly and string discovery results remain external. The native
code locator is communicated privately; no installed path belongs in this file.

The retained licensed `maps/mp/zm_tomb.gsc` compiled buffer has 63,195 bytes,
SHA-256 `9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6`.
It comes from `zone/all/zm_tomb_patch.ff`, SHA-256
`770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca`,
Steam build 24784239; see [source receipt](damage-life-state-01.md). The buffer
hash was freshly checked and gsc-tool 1.4.10.333 freshly disassembled it into
the new external task directory. Existing decompilation is corroborated by
compiled operand/call sequences, without copying script text into this report.

## Admitted configuration and player anchors

The Origins personality-character setup callback and team-character setup
callback both assign movement scale **1**, sprint duration **4** and cooldown
**0**. These are character-setup assignments, not afterlife assumptions.
The registered Origins custom weapon callback selects **c96_zm** as the starting
weapon, overriding the generic Zombies starter. These callback assignments
qualify baseline configuration inputs, not the complete active movement profile.

The static script-method table resolves these native functions:

| Method / table RVA | Code RVA | Complete bytes | SHA-256 |
| --- | --- | ---: | --- |
| setsprintduration / 0x8696b8 | 0x0e88d0 | 198 | `304bc3024a2506f6d7f4c05855c7ed55b7ac61d324f1ec2b00fe24d296f4fbd0` |
| setsprintcooldown / 0x8696d0 | 0x0be2f0 | 198 | `3fe9a2a93f474189d76a1a5a63bd0c27ff0e34bd3da5e72bbcd50a5d55a290a8` |
| setmovespeedscale / 0x945c70 | 0x448990 | 44 | `c97111198bbc3bbb139b4db59d0d651eb7bc23ee98513f3ff26cf127082bc62d` |
| sprintbuttonpressed / 0x868818 | 0x090700 | 141 | `0fbaadd9b6f9444d39fdbea74042d2692971f0cf8456ca6577a463090a03f870` |
| getplayerviewheight / 0x869b80 | 0x424d30 | 112 | `db3d41108b2a44ded51ee6f56dec9057dff8973124036e3d6bce552ec3d988e8` |

Duration/cooldown setters resolve the entity through the same player association
used by retained native player methods: indexed entity record, stride 0x31c,
client pointer at entity+0x154. They accept a scalar that is nonnegative and
strictly below **16383**, multiply by the scalar-f32 constant **1000**, truncate
to signed integer, then write client+0x4cc / client+0x4d0 respectively. Thus
the qualified Origins assignments encode **4000** and **0** in native millisecond
storage. This establishes neither the sprint debit/recharge algorithm nor its
relation to command substeps. Invalid arguments take script-error paths; no
Rust NaN behavior is inferred. Scale is written to client+0x54e0 without an
additional clamp in this consumer.

The sprint-button method reports true if bit 30 is set in either client+0x568c
or client+0x56a4. Their current/previous command ownership is still unresolved;
this is not permission to assign keyboard controls or command-layout names.
The view-height method returns client+0x208. Separate static netfield tables
associate `commandTime` with offset 0, `pm_type` with 4 and `viewHeightCurrent`
with 0x208, supporting a player-state prefix, not an entire C/Rust structure.

## Next exact investigation and remaining blockers

Scale-field references locate native entity-associated setup/update code:
0x01bed8 writes client+0x54e0 and copies it to +0x50c; function beginning
0x16cc30 obtains entity+0x154 and copies +0x54e0 to +0x50c at 0x16cc98.
This latter function has multiple entity-state branches and updates cached
entity/render data. It is **not yet qualified as Pmove**; its helper 0x41ed60
copies velocity/state into entity presentation and is not the motor.
Start the next bounded task from the command storage consumers at 0x034bc4,
0x034c5c and 0x18f205, and follow the caller that prepares movement state and
collision callbacks. Related complete setter/reader roots above establish real
player ownership without guessing player offsets from nearby objects.

No ordinary alive updater, movement command layout, time clamp/substep policy,
posture flags, standing/crouch/prone hulls, player contents mask, acceleration,
friction, gravity integration, step/slide ordering, ground classification,
jump/landing rules, stance clearance or view-height interpolation is admitted.
The c96 weapon movement/ADS/sprint modifiers are missing. Movement-related
string names and a jump-height registration lead at 0x241ec1 alone do not prove
these contracts; incidental adjacent literal references are excluded.
The earlier [forwarding investigation](0025-original-standing-player-trace-request.md)
remains the downstream trace anchor, without player defaults.

## Proposed reusable boundary (implementation choices)

The primary owner may stabilize API names now, keeping unresolved numbers
explicit rather than supplying defaults:

- `MovementState`: authoritative source-space origin/velocity, command clock,
  separate posture, grounded/contact state and native-qualified sprint/jump/view
  transition state once recovered. `LifeState::Standing` is not posture.
- `MovementCommand`: ordered identifier, explicit native command time and raw
  qualified movement/button/view inputs. Field widths/encoding remain open.
- `MovementProfile`: admitted baseline/loadout identity, explicit qualified
  motor/view/stance parameters and collision filter/extent policy. The three
  callback scalars above are admissible; an executable baseline is not complete.
- `MovementStep`: returned authoritative state and explicit consumed timing /
  qualified collision outcomes. `MovementError`: unsupported profile/timing,
  malformed input, nonfinite arithmetic and collision errors, as Rust safety
  policies distinct from original behavior.
- `step_movement`: pure numerical transition against explicit qualified collision
  access; stage the result and publish only after all operations succeed.

No implementation was authorized for this investigation. Do not implement a
generic motor against these missing rules or silently treat the existing
diagnostic free camera as authentic movement.

## Validation

Five complete functions decode through their final return; all internal direct
jumps land on decoded instruction starts. Fifteen retained collision functions
match disk exactly. The compiled Origins script identity and freshly decoded
configuration assignments agree. Four independent f32 timing-conversion
examples (0, 0.001, 4, 16382) produce 0, 1, 4000, 16382000; they check the
transcription only, not original native execution. No gameplay/fidelity
comparison or Cargo gate was run for this documentation/external-only task.
