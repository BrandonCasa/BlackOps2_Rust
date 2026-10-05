# Signed PC fastfile envelope

Recorded: 2026-10-04. Question: what metadata can one bounded read-only inspector
establish for the cataloged `TAff0100` / candidate-147 family without decoding
its contents? **Result:** a 312-byte envelope contract is supported on six local
samples. Format understanding is **INVESTIGATING**; overall/gameplay/visual
fidelity remains **UNKNOWN**. This is sufficient evidence for one envelope-only
inspector, subject to outer-agent review, not a complete fastfile validator.

## Sources and method

The licensed originals are the specification. Source selection and Steam Zombies
manifest build `24784239` are recorded in [catalog 0001](0001-local-asset-catalog.md).
No live build identification or all-file integrity verification was performed.
Three `tomb` samples and three contrasts were inspected; no game process, scripts,
entity definitions, payload decoder, extraction tool or game write was used.

Primary corroboration is maintained OpenAssetTools (OAT), inspected at immutable
revision `59904a3e1dd698ce1c8b8e750d12b47e19cf83ba`. Its
[license](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/LICENSE)
is GPL-3.0. Only independently described format facts and links are recorded;
no source implementation, keys or algorithms are copied or adopted.

1. Resolve installation/output roots from ignored `config/local_assets.toml`.
2. For each relative sample below, snapshot file length/write time and stream a
   full SHA-256 digest. Open read-only and read exactly bytes `[0,312)` twice.
   Interpret only the stated markers, version, zero flags and bounded name field;
   do not retain or print signature bytes. No structural read beyond 312 occurred.
3. Repeat the full-stream hash and metadata snapshot; compare both passes and
   identities. Hashing reads the entire file but emits only its digest.
4. Save only metadata, relative identifiers, digests and repeat/mutation results
   externally as `0002-fastfile-envelope.json`. Reproduction helper
   `0002-fastfile-envelope-observe.ps1` is also in the configured external output;
   it accepts the local configuration path and emits no original byte arrays.

Twelve bounded prefix reads total 3,744 bytes; twelve full-file hash passes were
performed. All repeat observations, hashes, lengths and write times agreed. The
three `tomb` digests also agree with catalog 0001. No source mutation was observed.

## Observed schema and support boundary

Offsets and widths are on-disk bytes, zero-based; ranges are half-open. **O** means
direct observation in all six samples; **C** means public-source corroboration;
**P** is our proposed conservative inspector policy, not an original-game rule.

| Offset | Width | Endian/encoding | Field and evidence | Inspector policy |
| ---: | ---: | --- | --- | --- |
| 0 | 8 | Exact ASCII marker | `TAff0100` (**O**); signed Treyarch variant (**C**, OAT constants) | Support this exact marker only |
| 8 | 4 | Little-endian u32 | `147` (**O**); PC zone version (**C**, OAT header/factory) | Support 147 only; no byte swapping fallback |
| 12 | 8 | Exact ASCII marker | `PHEEBs71` (**O**, **C**, OAT constants/auth steps) | Require exact marker |
| 20 | 4 | Opaque bytes; endian unknown | All zero (**O**); OAT calls these loading flags but skips them (**C**) | Zero-only variant (**P**); nonzero means unsupported, not proven corrupt |
| 24 | 32 | Bounded name buffer | NUL-terminated ASCII file stem, remaining bytes zero (**O**); OAT reads a bounded filename (**C**) | Nonempty printable ASCII, NUL within buffer and zero padding (**P**) |
| 56 | 256 | Opaque | Signature-sized region is present; signature meaning/width corroborated by OAT (**C**) | Establish bounds only; do not emit, verify or interpret bytes |
| 312 | File length minus 312 | Unknown to this inspector | Remainder present in all samples (**O**) | Report offset/length only; no reading/decoding |

OAT's [header definition](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Common/Zone/ZoneTypes.h)
establishes the 8-byte marker and u32 field. Its
[T6 constants](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneCommon/Game/T6/ZoneConstantsT6.h#L15-L42)
name PC version 147 and other markers/platform versions. The
[T6 factory](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Game/T6/ZoneLoaderFactoryT6.cpp#L50-L69)
classifies this combination as PC, signed/encrypted, with a compressed payload.
These are source-corroborated classifications, not successful local authentication
or decoding. Its
[auth steps](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Game/T6/ZoneLoaderFactoryT6.cpp#L255-L269)
give the field order; [magic handling](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Loading/Steps/StepVerifyMagic.cpp)
reads exactly the marker length, and
[signature handling](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Loading/Steps/StepLoadSignature.cpp)
requires all requested bytes. Thus `8+4+8+4+32+256=312` is an evidence-backed
envelope boundary, not an expanded-content offset.

OAT's [filename check](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Loading/Steps/StepVerifyFileName.cpp)
compares against a caller's expected name, truncated to 31 characters; it does
not enforce ASCII or zero padding. Our stricter name policy describes the observed
subset. A name variant outside it is unsupported, without asserting the game
would reject it. Comparing with a disk stem belongs in the diagnostic wrapper,
not intrinsic parsing; renaming a file does not change its envelope metadata.

## Sample measurements and relationships

All samples have both stated markers, version 147, zero flag bytes, a nonempty
ASCII NUL-terminated name matching their file stem, zero name padding and a complete
signature-sized region. All lengths are divisible by 64; this observation is
**not** an acceptance rule. No suffix was read, so zero-padding validity is unknown.

| Relative identifier | File length | Embedded name | Opaque remainder length |
| --- | ---: | --- | ---: |
| `zone/all/zm_tomb.ff` | 78,753,344 | `zm_tomb` | 78,753,032 |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 | `zm_tomb_patch` | 1,288,072 |
| `zone/english/en_zm_tomb.ff` | 74,176 | `en_zm_tomb` | 73,864 |
| `zone/all/common_zm.ff` | 10,488,640 | `common_zm` | 10,488,328 |
| `zone/all/zm_transit.ff` | 58,935,232 | `zm_transit` | 58,934,920 |
| `zone/english/en_zm_transit.ff` | 704 | `en_zm_transit` | 392 |

Every remainder equals file length minus 312 and is positive. The three new
contrast SHA-256 identities are:

| Relative identifier | SHA-256 |
| --- | --- |
| `zone/all/common_zm.ff` | `b0663e765b50a17e3e7a53e2675ca18a1eeca186e0356df74e9b2c615f09ce9e` |
| `zone/all/zm_transit.ff` | `a77b22580d701c7206134003fc89468028e8f7f3ae520d5aa53b437aa2a7c38d` |
| `zone/english/en_zm_transit.ff` | `f6931d07b2886249a42be6e2b378824293466339085b3154096a66922dfdc518` |

**Observation:** base/patch/localized candidates use the same envelope shape;
their names and lengths differ. There is no confirmed patch/language discriminator
in the inspected fields besides the names. **Inference:** an envelope abstraction
can serve these source families without Origins coupling. Loading order, override
semantics, required dependencies, map identity and actual contents remain unknown;
the `_patch` and `en_` naming patterns do not establish them.

## Proposed one-task implementation contract

One reusable tooling-level `PcSignedFastfileEnvelope` value may hold version 147,
bounded embedded name, total length (`u64`), signature byte range (`56..312`),
and opaque remainder offset/length (`u64`). Accepted variant identifies the two
markers and zero-flags/name policy; it contains no original byte buffers. Do not
define decoded assets, entities, map relationships or Bevy loader APIs here.

Implement safe Rust parsing over a bounded reader plus authoritative length from
the same open file. Stage the 12-byte discriminator first, reject unsupported
marker/version, then read through byte 312. Cap transient storage at 312 bytes;
use explicit little-endian conversion and checked subtraction. Never allocate
from declared file length or follow an embedded name as a path. The wrapper uses
an explicitly selected external input; path-containing I/O errors stay local or
are redacted in shareable output. TOML discovery and recursive inventory are
separate concerns, not prerequisites to this parser.

Return distinct I/O, truncated envelope (needed/available bytes), unsupported
marker/version/flags/name-variant, malformed authentication marker, and inconsistent
reader/length errors. Unsupported includes alternate signed/unsigned/server/LZX
markers and any PC/console version outside the exact observed combination;
similar prefixes must not trigger another layout. Name mismatch with the disk
stem is diagnostic metadata, not corruption. Do not echo arbitrary marker/name
bytes from unsupported input in shareable errors.

A length of 312 with a complete envelope may succeed with zero remainder:
**success means envelope metadata parsed**, never a loadable/authentic fastfile.
Larger claimed lengths prove only range arithmetic, not payload completeness.
No payload length, chunk count, expanded size, asset count, checksum or signature
validity can be established here. Existing Bevy ownership remains unchanged;
tooling must not introduce a replacement asset server or engine subsystem.

Meaningful synthetic tests for that one implementation task:

- Construct markers/version, zero flags, a user-created ASCII name and arbitrary
  256 signature bytes; assert exact ranges/remainder for lengths 312 and 704.
- Truncate at every byte before 312, including discriminator/name/signature
  boundaries; assert typed failure and no panic or read beyond the fixed limit.
- Corrupt auth marker; distinguish malformed supported-envelope structure from
  unknown marker, version 146/148, byte-swapped 147 and alternate variant rejection.
- Exercise nonzero flags, empty/nonprintable/non-ASCII/unterminated names, nonzero padding,
  longest supported 31-byte name and filename mismatch; document subset policy.
- Inject short reads and I/O failures, inconsistent supplied length and `u64::MAX`
  length; test bounded allocations/reads and checked arithmetic.
- Vary arbitrary signature/remainder bytes without changing metadata success;
  prove no authentication or payload-validity claim is made and output contains
  no source path, raw field dump, signature bytes or payload bytes.

## Validation and limits

Local observations reproduced twice on six samples; identities and bounds passed.
Documentation links, prohibited-content/local-path review and `git diff --check`
passed. Rust/dependencies are unchanged; workspace compiler/tests were intentionally
not rerun and the previously accepted executable baseline is unchanged.

Unknowns remain: flag meaning/endian, other name/marker/platform variants, payload
layout and completeness, authentication, expanded assets and references, patch
relationships, rendering and original behavior. The next task can implement this
conservative envelope inspector; decoding requires a separate evidence task.
