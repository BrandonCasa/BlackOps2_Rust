# Stationary sphere versus one terrain triangle

Fidelity: INVESTIGATING. This is an admitted static scalar reconstruction, with no original invocation, capture or comparison. It is neither a vertical capsule nor a terrain/world query.

## Static qualification

The licensed admitted disk image SHA256 is `2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`. Complete control-flow-followed bodies, including all direct branch targets and returns, also equal the retained terrain01 instruction bytes:

| RVA | Bytes | SHA256 |
| --- | ---: | --- |
| `4821a0` | 314 | `b6319b3d5454c052e8b27d6865ddafc7ac262c62793fa8ee247cebd295e430cc` |
| `481f10` | 649 | `f3b3a2299828404f908b1c060fa2061711c67574171657a17df95d158294277e` |
| `482e40` | 205 | `79879d178ac89797133616c7decd2263167332e33e426a79a02ed229ae213530` |

Reproduction: use the ignored licensed disk identity, map executable RVAs through its section table, recursively decode only the three entry bodies and their internal direct branches, concatenate retained instruction-byte records and compare. Read constants through PE section mappings: VA `bed8d0` is the sign-bit mask `80000000`, `bfdd24` is f32 one, `bfc42c` is f32 two, and `d1ffa8` is positive zero. Owned external task06 contains `static_decode.py`, `qualify.py`, `qualification-receipt.json`, `independent_cases.py` and the independent scalar receipt. None are native instruction execution or instruction emulation.

## Admitted numerical contract

`4821a0` reads the ordered triangle's three u16 vertex identities from the global vertex pool. The immutable admitted terrain snapshot supplies those identities and f32 bit patterns. Let A/B/C be stored vertices, E=B-A, G=C-A and O=center-A, each component subtraction separately rounded f32. Define `YXZ(X,Y)=((Xy*Yy)+(Xx*Yx))+(Xz*Yz)` and `XYZ(X,Y)=((Xx*Yx)+(Xy*Yy))+(Xz*Yz)`, with separately rounded products and additions.

The helper receives q=YXZ(E,E), r=YXZ(G,E), s=YXZ(G,G), computes u=YXZ(E,O), v=XYZ(G,O), determinant=q*s-r*r, x=v*r-u*s and y=u*r-v*q. Its comparisons use determinant versus **y+x**. Native region cases choose two weights, retain exact zero/equality branches, clamp selected edge weights, and reconstruct each returned coordinate as `(E_i*weightB+A_i)+G_i*weightC`. The scalar divisions happen before sign-bit negation for the single-edge regions. The opposing-edge numerator is `((s-r)+v)-u`, denominator `(q-r*2)+s`; the interior branch multiplies x/y by separately rounded `1/determinant`. Source code comments identify the bounded numerical regions; no vector dot helper, f64 promotion, fused operation or generic triangle-distance algorithm replaces them.

The **center-minus-A signs are observed**, even though conventional closest-point geometry uses different signs with these numerators. For A=(0,0,0), B=(2,0,0), C=(0,2,0), center=(.5,.5,1), the admitted scalar flow returns A and squared distance1.5; center=(-.5,-.5,1) returns (.5,.5,0) and distance3. These are exact source-flow observations rather than proof of effective original runtime semantics. Runtime code rewriting/effective implementation equivalence remains unverified. The earlier terrain01 flat-face radius examples do not establish the complete helper's conventional geometric equivalence.

`4821a0` subtracts center from the returned point and computes squared distance in Y-X-Z order. The sphere branch of `482e40` compares separately rounded radius*radius strictly greater than that distance. Tangency is clear. On overlap it writes bytes2a/2b=1 and fraction positive zero, preserving normal, 2c, contents and surface flags. The raw API has no masks, material gates, stamps or aggregation. Its diagnostic distance and triangle identity are returned separately from the authoritative trace result.

## Safe Rust boundary and limits

`TerrainStationarySphereRequest` supplies finite center and finite nonnegative radius (negative zero is accepted); radius is not interpreted as a hull/default. `trace` accepts an admitted snapshot, explicit triangle ordinal and validated finite incoming normal/fraction in [0,1]. It checks all executed scalar arithmetic. Missing references, invalid request/result or nonfinite arithmetic return typed errors without changing authoritative fields. Native NaN/Inf fall-through is outside this safe finite domain, explicitly rejected rather than reinterpreted as clear or overlap. Degenerate triangles can divide by zero and return an arithmetic error; collinear geometry can also follow a finite native branch after f32 cancellation and is preserved. There is no chosen fallback closest point or geometric repair.

The implementation owns no allocation, workspace, world membership, mask, timing, pose or lifetime policy. Public snapshots cannot be mutated into malformed references after reader admission; the kernel nevertheless checks both triangle and vertex references.

## Validation

Independent scalar f32 fixtures were reconstructed before implementation, from the admitted equations/regions and synthetic coordinates. Seven axis fixtures cover vertex, single-edge, opposing-edge, interior and equality cases. An oblique cancellation-sensitive fixture has squared-distance bits `43006e7a`. Synthetic Rust checks also cover radius tangency/overlap, supplied winding, seeded flags/fraction, signed zero, degenerate finite/error branches, invalid references/input/fraction and late radius/center arithmetic failure atomicity. Actual focused and crate gates are recorded in [the handoff](../handoffs/movement-terrain-06.md).

No claim of MATCHED/VERIFIED, full stationary terrain, player capsule, map collision or alternate receiver access is made.

