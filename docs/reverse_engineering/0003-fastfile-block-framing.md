# Signed PC fastfile transport framing

Recorded: 2026-10-04. Question: what follows the supported 312-byte envelope,
and can it be mapped without decrypting or decompressing assets? **Result:** all
six original samples have plaintext length-prefixed opaque chunks followed by a
zero suffix. Exact range accounting is possible with bounded noncontiguous reads.
There is no additional serialized authentication hash table at byte 312 on this
path. Format understanding remains **INVESTIGATING**; authentication, decoded
contents and overall/gameplay/visual fidelity remain **UNKNOWN**.

The next useful task is a bounded external-tool asset-directory observation,
described below. A framing scanner contract is recorded for later use; implementing
another metadata layer is not a prerequisite to accessing actual asset types.

## Sources and read boundary

Licensed originals remain the specification: the same six samples and Steam
Zombies manifest build `24784239` from [findings 0001](0001-local-asset-catalog.md)
and [0002](0002-fastfile-envelope.md). Existing parser/tests, architecture and
handoff 0006 were inspected. No scripts, entity definitions or game behavior were
observed, and no decoder/game was installed or launched.

Public corroboration uses OpenAssetTools (OAT), GPL-3.0, at immutable revision
`59904a3e1dd698ce1c8b8e750d12b47e19cf83ba`, also the repository's current head
when checked. GitHub dates that commit 2026-10-03. Only independently described
format facts are recorded; no implementation, cryptographic constants or original
bytes are copied into this repository.

External helper `0007-fastfile-block-framing-observe.py` resolves both roots from
ignored `config/local_assets.toml`. It opens each source read-only and unbuffered,
obtains length from the same handle, reads the 312-byte envelope, then seeks to
each four-byte length field. It seek-skips every opaque chunk body. Only an
expected 1-3-byte boundary filler and the bounded zero suffix may be read besides
the envelope and length fields. No body bytes, signatures or raw hex are emitted.
The metadata record `0007-fastfile-block-framing.json` stays in configured external
output and includes counts/ranges, relative identifiers, identity digests and
repeat/mutation results. Original paths occur only in ignored configuration.

Two structural passes per sample agreed. Their **combined** reads were capped at
262,144 bytes per sample; the maximum actually read was 88,842. All noncontiguous
reads are the metadata ranges defined below, regardless of absolute offset.
Separate before/after full-stream SHA-256 passes emitted digests only. All six
hashes matched the accepted findings-0001/0002 identities, and lengths/write times
were unchanged. No original writes or copies were performed.

## Framing schema and arithmetic

Offsets/widths are zero-based file bytes, ranges half-open. **O** means directly
observed; **C** public-source corroboration; **P** proposed conservative policy.
Let `N` be same-handle file length, `B=524288`, and `p` initially 312.

| Position | Width | Encoding | Meaning and bounds |
| --- | ---: | --- | --- |
| `[0,312)` | 312 | Existing envelope | Exact supported marker/version/flags/name subset from findings 0002; unchanged (**O/C**). |
| Before a length field, if `B-(p mod B)<4` | 1-3 | Zero filler (**C**) | Advance to the next absolute `B` boundary before reading a complete length. No sample encountered this case; zero-only acceptance is **P**, corroborated by OAT's writer. |
| Current `p` | 4 | Little-endian u32 | Stored opaque chunk length `s`, excluding this prefix (**O/C**). Positive observed lengths range from 14 to 32,701; source ceiling is 32,768 (**C**). |
| `[p+4,p+4+s)` for positive `s` | `s` | Opaque | Encrypted compressed bytes (**C**); not read/interpreted here. Checked `p+4+s<=N`, then next candidate position is `p+4+s` (**O**). Bodies may cross `B` boundaries. |
| Current `p`, when `s=0` | 4 | Little-endian zero | OAT stops transport consumption here (**C**); first four bytes of the observed suffix, not a separately stored chunk (**O**). |
| Last positive chunk end `q` through `N` | 64-127 observed | All zero | Every sample satisfies `N=align_up(q+64,64)` (**O**), corroborated by OAT suffix writing (**C**). Zero suffix/alignment acceptance is the proposed subset (**P**), not a proved original loader requirement. |

The schema applies only to the existing signed PC `TAff0100`, little-endian 147,
`PHEEBs71` envelope subset. It does not authorize unsigned, server, OAT-authored,
console, LZX, nonzero-flags or other unobserved variants.

The [chunk size type](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Common/Zone/ZoneTypes.h#L13)
is four bytes. The [T6 constants](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneCommon/Game/T6/ZoneConstantsT6.h#L32-L40)
give four streams, 32,768-byte chunk capacity, the 524,288-byte boundary and suffix
constants. [ProcessorXChunks](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Loading/Processor/ProcessorXChunks.cpp#L220-L300)
reads a size outside the per-chunk decoder, rejects sizes above capacity and uses
stream ordinals `0,1,2,3,0,...`. These are inferred stream labels, not stored IDs.
Initialization may reach terminal input before every stream has a chunk. Its
short-prefix EOF also stops reading; our proposed scanner instead reports a
missing complete terminal field as a distinct structural failure.

[OutputProcessorXChunks](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneWriting/Writing/Processor/OutputProcessorXChunks.cpp#L18-L55)
corroborates length-before-body ordering and zero filler only when a length would
straddle the buffer boundary. [T6 suffix steps](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneWriting/Game/T6/ZoneWriterFactoryT6.cpp#L129-L135)
append at least 64 zero bytes and align to 64. This writer is corroboration; it
does not establish which suffix forms the original game accepts.

## Sample variation

`K` counts positive length fields; `S` sums their stored sizes; `q` is the last
positive body end. No boundary filler occurred in any of the six chains, so every
sample exactly satisfies **`N = 312 + 4K + S + (N-q)`**.

| Relative identifier | `K` | First stored sizes | `S` | `q` / zero field | Zero suffix | Combined structural reads |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| `zone/all/zm_tomb.ff` | 10,998 | 25, 5,831, 4,684, 10,087 | 78,708,923 | 78,753,227 | 117 | 88,842 |
| `zone/all/zm_tomb_patch.ff` | 113 | 20, 10,921, 12,493, 14,870 | 1,287,529 | 1,288,293 | 91 | 1,710 |
| `zone/english/en_zm_tomb.ff` | 31 | 17, 5,836, 5,399, 3,635 | 73,638 | 74,074 | 102 | 1,076 |
| `zone/all/common_zm.ff` | 519 | 20, 16,242, 28,000, 25,632 | 10,486,135 | 10,488,523 | 117 | 5,010 |
| `zone/all/zm_transit.ff` | 4,521 | 26, 5,442, 8,357, 14,184 | 58,916,741 | 58,935,137 | 95 | 36,982 |
| `zone/english/en_zm_transit.ff` | 2 | 14, 289 | 303 | 623 | 81 | 802 |

Total structural reads across both passes: 134,422 bytes; twelve separate identity
hash passes. For the tiny contrast, lengths are at 312 and 330, opaque ranges
`[316,330)` and `[334,623)`, terminal zero at 623, and zero suffix `[623,704)`.
For `zm_tomb`, the last length is at 78,741,422 with size 11,801; its body ends at
78,753,227. Payload ranges cross absolute `B` boundaries 150 times in this file,
two in its patch, 20 in `common_zm`, and 112 in `zm_transit`. No payload-boundary
padding should be invented. Four-byte prefixes are not generally aligned.

**Inference:** one reusable transport-framing adapter can serve base, patch,
localized and contrast files without an Origins dependency. Different sizes/names
do not establish map identity, patch order, overrides or asset references.

## Authentication and decoding dependencies

The [T6 factory](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Game/T6/ZoneLoaderFactoryT6.cpp#L272-L304)
starts XChunks immediately after the envelope; its PC path decrypts before raw
DEFLATE decompression. Chunk lengths are plaintext; expanded zone sizes, asset
directory/counts, block sizes and references are inside decoded output. The small
first chunk has no established semantic meaning in this investigation.

OAT's [hash-state declaration](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneCommon/Zone/XChunk/AbstractSalsa20Processor.h#L29-L44)
describes `200 * 4 * 20 = 16000` internal bytes.
[Initialization/capture](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneCommon/Zone/XChunk/AbstractSalsa20Processor.cpp#L15-L55)
derives that state from the bounded zone name, rather than loading it from the
file. [Decryption processing](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneCommon/Zone/XChunk/XChunkProcessorSalsa20Decryption.cpp#L18-L49)
updates it using decrypted chunks. The final state feeds
[signature verification](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/ZoneLoading/Loading/Steps/StepVerifySignature.cpp#L21-L38),
scheduled after content loading by the factory. Those are source-corroborated
dependencies, not locally reproduced cryptographic results. No hash-state value,
key, signature, crypto implementation or decoded body was emitted or retained in
tracked work. Framing success cannot establish authentic or complete decoded
content, and a transport chunk is not an asset boundary.

## Exact optional scanner contract

If a reusable framing reader becomes necessary, extend `bo2-assets` beside the
existing envelope parser and keep the explicit-input CLI thin. Compose
`inspect_pc_signed_envelope` once, over `Read + Seek` at file offset zero with
same-handle `u64` length. Output the existing envelope plus aggregate chunk count,
per-stream counts, minimum/maximum/sum stored length, first four/last chunk ranges,
boundary-filler count, zero-field offset and suffix range. No growing vector,
payload bytes, authenticity assertion, asset/entity model or Bevy loader API.

Use checked `u64` offset arithmetic and checked seek-position results. Request
only envelope bytes, four-byte prefixes, at most three boundary filler bytes and
at most 127 suffix bytes. Seek over opaque bodies; validate declared ranges
against `N` without pretending this proves skipped bytes accessible or immutable.
Keep fixed transient buffers at most 443 bytes (312 envelope, four length, 127
suffix; filler can reuse suffix storage), besides the existing at-most-31-byte
name. Policy caps: 16,384 positive chunks and 262,144 structural bytes per scan.
An exceeded resource cap means incomplete inspection, not corrupt game data.

Accept positive sizes up to 32,768, then require a complete zero-length terminal
field. For the conservative suffix subset, let `q` be last body end (312 for an
empty synthetic chain), require `N=align_up(q+64,64)` with checked arithmetic and
all `[q,N)` zero. If an expected boundary filler precedes the terminal field, it
belongs to that suffix and must not be double-counted. Distinguish envelope errors,
I/O/seek failures, prefix truncation, range/length inconsistency, size beyond the
supported ceiling, arithmetic overflow, unsupported nonzero boundary filler,
unsupported suffix variant and resource-limit exhaustion. Redact source paths,
underlying error text and invalid raw bytes. Never follow the embedded name.

Meaningful synthetic tests for such a later implementation:

- Zero, one, two and five user-created chunks; exact ranges, per-stream rotation,
  unaligned prefixes, suffix lengths 64/127 and arithmetic totals.
- Sizes 1/32,768, reject 32,769 and byte-swapped sizes; signature/body changes must
  leave metadata identical. A reader must fail if any opaque body is requested.
- Place a next prefix at `B-3`, `B-2`, `B-1`, `B-4` and exactly `B`; check filler,
  distinguish body crossing from prefix crossing, and test terminal-after-filler.
- Every partial four-byte prefix, truncated declared body range, missing zero
  field, premature zero followed by nonzero data, nonzero/short/oversized suffix,
  nonzero filler and invalid 64-byte end alignment.
- Short/interrupted reads, redacted injected I/O, wrong seek return and observable
  same-handle length disagreement; checked `u64::MAX` arithmetic; exact resource
  caps and one-over limits without file-length allocation or unbounded output.

These tests are a proposed contract, not executed Rust functionality. The probe
is external evidence tooling only. No Rust/compiler gates were rerun because
tracked Rust and manifests are unchanged.

## Recommended next data-access task

Use standalone external **OAT Unlinker `--list --no-color` on one `zm_tomb.ff`**
to observe its decoded asset directory, without any dumping/conversion option.
[Arguments](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Unlinking/UnlinkerArgs.cpp#L59-L63)
select listing; [object-load policy](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Unlinking/Unlinker.cpp#L120-L123)
skips raw object loading, and the
[listing branch](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Unlinking/Unlinker.cpp#L218-L224)
does not enter the dump branch.
[ContentPrinter](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Unlinking/ContentLister/ContentPrinter.cpp#L13-L23)
emits asset type/name pairs. This can establish actual geometry-family presence;
it does not print dependency edges or prove map rendering. T6's
[type-name table](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/src/Common/Game/T6/GameT6.cpp#L11-L30)
provides leads such as `gfxworld`, `clipmap`, `comworld`, `mapents`, `xmodel`,
`material` and `image`, without proving any is present in this source yet.

The official [v0.33.0 release](https://github.com/Laupetin/OpenAssetTools/releases/tag/v0.33.0)
is pinned to `7d027e8f89118196713e955b0e11f8404149c54d`. Its
[GitHub release API](https://api.github.com/repos/Laupetin/OpenAssetTools/releases/tags/v0.33.0)
reports `oat-windows.zip`, 3,636,251 bytes, SHA-256
`4bf366cec03558a4b0f20bc8f197a51f4b9dad0aa3edc52b0e40fef2dd95a4f5`.
The T6 loader, input framing processor, Unlinker, arguments and content printer
files are byte-identical between that release and the reviewed head. Thus there
is a concrete signed-PC listing route without writing Rust cryptography.
Download/binary digest, runtime dependencies, decoding success and signature
acceptance remain unverified because no tool was installed/run here.

Read-only availability checks found no Unlinker on PATH or in the configured
inspection output (not a machine-wide search). Visual Studio Community/Build
Tools 2026 with x86/x64 C++ tools are registered; Git is usable. They satisfy the
major [documented build prerequisites](https://github.com/Laupetin/OpenAssetTools/blob/59904a3e1dd698ce1c8b8e750d12b47e19cf83ba/README.md#L55-L94),
but no OAT checkout/submodules/Premake build was validated. The verified-release
binary is the smaller access route.

The next task must constrain inputs to that one sample, keep tool/binary/raw
listing/logs outside the repository, preserve original identity, and enforce an
initial 120-second process deadline and 8 MiB combined output cap (**P**). Report
cap exhaustion as incomplete observation. Stock OAT loads the whole zone and has no researched
hard memory/output cap; this is a bounded task/input route, not the metadata
reader's structural-byte guarantee. Retain only reviewed type counts and selected
permitted identifiers in findings. If loading fails, report the precise decoder
failure without relaxing authentication or blindly adding dependencies. Once
geometry asset families are observed, assign a separate task to inspect one
decoded structure/reference boundary; do not treat listing as dependency proof.
