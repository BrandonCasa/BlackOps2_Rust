# Stationary vertical-segment terrain contract: checkpoint

Status: PARTIAL. Fidelity: INVESTIGATING. This investigation stopped at the requested stable checkpoint. It adds no Rust and does not qualify an implementation-ready capsule kernel. Complete bodies are available; remaining work is analysis/fixtures and floating-point policy qualification, rather than a missing body.

## Admitted static evidence

Licensed disk identity SHA256 `2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`. Complete bounded bodies, internal direct branches and returns rechecked against the disk and retained terrain01 instruction records:

| RVA | Bytes | SHA256 |
| --- | ---: | --- |
| `482680` | 1,984 | `d93dee9bb02e7bd7b44f4c0cbd2e68b174e3519e5b49f38903862b34639c1bbb` |
| `4822e0` | 917 | `9ddff5730cfe17d60d95a9b5f089194ddc7a8e712dde3ce5b9e6164f15bef33e` |
| `482e40` | 205 | `79879d178ac89797133616c7decd2263167332e33e426a79a02ed229ae213530` |

Total3,106 admitted bytes. The triangle body directly calls only the segment helper, three times in stored edge order; the helper has no calls. External task07 contains `static_decode.py`, `qualify.py`, `qualification-receipt.json`, `read_constants.py` and `constants.json`. Reproduction follows task06's read-only PE section mapping and bounded direct control-flow decoding, then concatenates and compares retained instruction bytes. No original process reads, invocation, instruction execution/emulation or capture occurred.

Required f64 constants, read through PE sections: VA `c31750` positive zero; `bfcc38` two; `be6290` sign mask `8000000000000000`; `c29878` one. These are separate from the sphere helper's f32 constants.

## Qualified caller and output boundary

`482e40` selects the sphere branch when the half segment at request+c4 equals zero. For the nonzero branch, X/Y come from request+80/+84. It constructs first endpoint P=(X,Y,centerZ+halfSegment), second Q=(X,Y,centerZ-halfSegment), using separate scalar f32 additions/subtractions. It separately squares request+c0 radius in f32. Triangle identity is passed in EAX, lower endpoint pointer in EDX, upper endpoint pointer and radius squared on the stack. No endpoint, radius or half-height default is established here.

A true return writes bytes2a/2b=1 and positive-zero fraction, preserving normal,2c and material/contents. A false return leaves the result unchanged. Mask/material/stamp admission remains outside this raw triangle boundary. These writes agree with the separately accepted sphere branch.

## Qualified triangle prefix and branch order

All vector subtractions, cross products, initial dot products and interpolation operations below are scalar f32, separately rounded per instruction. Let stored vertices A/B/C, E=B-A, G=C-A. N=E cross G; the source X component is Gz*Ey-Gy*Ez, Y is Ez*Gx-Gz*Ex, Z is Ex*Gy-Ey*Gx. No normal normalization or square root occurs in this body.

Define XYZ(X,Y)=((Xx*Yx)+(Xy*Yy))+(Xz*Yz), and YXZ(X,Y)=((Xy*Yy)+(Xx*Yx))+(Xz*Yz), preserving separate products/additions. Base plane scalar is YXZ(N,A). Endpoint plane scalars use XYZ(N,P) and XYZ(N,Q), each narrowed scalar promoted to f64 before subtracting the promoted base scalar. Thus dP/dQ are f64 differences of already-rounded f32 dots, not an all-f32 plane distance or f64 geometric dot.

The first branch tests f64 dP*dQ strictly less than zero. On straddling endpoints, it computes f64 t=dP/(dP-dQ), narrows t to f32, and forms each interpolation coordinate as `(Q_i-P_i)*t+P_i` in f32. The triangle coefficients q=XYZ(E,E), r=YXZ(G,E), s=YXZ(G,G) are computed in f32 then promoted to f64. The determinant q*s-r*r is f64. With offset=A-interpolatedPoint, u=XYZ(offset,E) and v=YXZ(offset,G) are computed in f32 then promoted. The f64 numerators v*r-u*s and u*r-v*q must both be >=0 and their ordered sum <=determinant. Equality is accepted. Success returns true without a radius test, including radius zero. These signs differ from the sphere helper's center-minus-A offset; reuse of its region logic is not established.

If that crossing test misses, endpoint slab eligibility compares promoted f32 `(YXZ(N,N)*radiusSquared)` strictly greater than f64 dP*dP/dQ*dQ. An eligible endpoint undergoes the same promoted barycentric tests with offset=A-endpoint. P is tested before Q. For nonstraddling endpoints, both slab flags false returns false before edge helpers. For straddling endpoints with a failed interior test, both flags false still proceeds to edge helpers. This distinction is observed in the separate branch join at482bbd versus the same-side early exit at482b1e.

Edge helper calls are ordered A->B, A->C, then B->C. Segment direction D=Q-P is scalar f32. The first two triangle directions reuse E/G; the third direction C-B is freshly subtracted in f32. Each returned helper distance is f32 and compared strictly below the stored f32 radius squared; the first accepted edge returns true.

## Segment helper: admitted prefix, unfinished region analysis

`4822e0` receives segment base P, direction D and edge base/direction. Its initial coefficients are computed from f32 vector products/sums then promoted to f64: segment squared length XYZ(D,D), cross dot XYZ(edgeDirection,D), edge squared length XYZ(edgeDirection,edgeDirection), offset=P-edgeBase, segment/edge offset dots XYZ, and offset squared length YXZ. Products, determinant, region comparisons and divisions then use f64. Every return narrows a f64 expression to f32. Several paths use f64 sign-bit negation after division; a generic clamped closest-segment library would not establish operand/branch equivalence.

The complete eight-return piecewise region derivation, exact expression evaluation order per return, degenerate/protection domain and precision witnesses have **not** been independently reconstructed at this checkpoint. No claim of nonnegative output, parallel fallback, geometric equivalence, safe degeneracy handling or numerical implementation readiness is made. Complete source bytes are retained for the next bounded investigation.

## Remaining qualification and limits

- Finish every segment-distance region/equality/return expression and degenerate branch from the admitted helper; do not substitute a generic closest-distance algorithm.
- Reconstruct independent interior/edge/vertex/tangent/degenerate/nonaxis/precision fixtures before Rust implementation. This task has not run such fixtures; the earlier sphere/point fixtures do not validate this family.
- Identify effective SSE floating-point policy (rounding mode, denormal/flush behavior and exception masking) or explicitly bound a Rust reconstruction domain. These bodies neither read nor initialize MXCSR; no x87 arithmetic occurs in these three bodies. Absence of local policy writes does not establish the thread policy.
- Source does not validate finite inputs/references or provide Rust atomic errors. A safe finite arithmetic admission/error contract still requires reviewed numerical analysis; do not invent native protection behavior.
- Effective runtime code equivalence and original comparison remain unverified. No MATCHED/VERIFIED claim, capsule implementation, aggregate/world coverage or player movement readiness follows from this checkpoint.
