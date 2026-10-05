TASK:
Investigate the transport/authentication framing immediately after the supported 312-byte signed-PC envelope and establish the next bounded reader/data-access decision.

STATUS:
COMPLETE

FINDINGS:
- All six originals have little-endian four-byte plaintext stored lengths, opaque bodies and an all-zero suffix. Exact range/file-length accounting passes without reading chunk bodies.
- No serialized authentication/hash table is consumed at byte 312. OAT's 16,000-byte hash state is derived internally and updated from decrypted chunks; signature verification depends on decoded processing.
- Transport streams rotate 0/1/2/3; the tiny localized contrast has only two chunks. Absolute 524,288-byte prefix-boundary filler is source-corroborated, but none occurred in these samples.
- Recommend external OAT --list on one zm_tomb.ff as the next task, ahead of another metadata implementation. It can observe actual asset type/name pairs; dependency edges remain a later structure investigation.

EVIDENCE:
- Root AGENTS/objective; asset-inspection/bo2-reverse-engineering skills; architecture/state; findings 0001/0002; handoff 0006; existing fastfile source/tests and ignored configuration.
- Maintained primary OAT source at 59904a3e1dd698ce1c8b8e750d12b47e19cf83ba; immutable links/field arithmetic in findings 0003. Current head checked equal; signed-PC/listing files unchanged in official release v0.33.0 commit 7d027e8f89118196713e955b0e11f8404149c54d.
- Six licensed sample identities: same hashes/lengths/write times as accepted evidence. Read-only unbuffered noncontiguous reads touched envelope, size prefixes and bounded zero suffix only; every opaque body seek-skipped.
- External helper 0007-fastfile-block-framing-observe.py and metadata 0007-fastfile-block-framing.json remain in configured output. No raw bytes/signatures/keys/scripts or original content in tracked work.

CHANGES:
- Findings 0003: observed/corroborated/policy schema, sample variation, framing/authentication dependencies, exact optional scanner contract, proposed synthetic tests and concrete external listing recommendation.
- Reverse-engineering/documentation indices and minimal project-state update; no Rust, manifest, loader/entity/engine API or architecture change.

VALIDATION:
- Two structural passes per sample agreed; 134,422 structural bytes total, maximum combined per sample 88,842 under 262,144 cap. Twelve separate before/after full-stream hashes agree with accepted identities; no mutation observed.
- Positive chunks: zm_tomb 10,998; patch 113; en_tomb 31; common_zm 519; zm_transit 4,521; en_transit 2. All lengths within source-supported 32,768 ceiling; all suffixes 64-127 zero bytes with N=align_up(last_end+64,64).
- All six satisfy complete envelope/prefix/body/suffix byte accounting. No observed boundary filler; that branch remains source-only evidence needing synthetic tests if implemented.
- Read-only local tooling probe: Unlinker absent on PATH/configured output; VS2026 C++ x86/x64 tools registered, Git available. Official Windows release metadata/digest reviewed; no download/build/decoder run.
- Documentation link/content/path review and git diff --check passed. Rust/compiler gates intentionally not rerun: no tracked code changes. Proposed scanner tests are not claimed executed.
- Outer-agent review accepted the findings and scope. An independent same-handle scan reproduced all six chunk/range/stream/suffix/accounting summaries with 67,211 structural bytes and twelve additional matching identity hash passes. Official release size/digest and primary source were checked independently; documentation links/content boundaries and whitespace checks passed. The parent metadata record remains external.

FIDELITY STATUS:
INVESTIGATING
Transport-format understanding only. Authentication, decoded assets and overall/gameplay/visual fidelity UNKNOWN.

UNRESOLVED:
- Source-only boundary filler; unobserved envelope/suffix variants; authenticity, expanded sizes, asset directory/counts/references, patch/base loading relationships and actual map identity.
- External listing success, binary/runtime validation and original signature acceptance are unverified. Stock tool reads/allocates whole-zone data; --list does not report dependency edges.

RECOMMENDED NEXT WORK:
- Provision the pinned official OAT release externally with digest verification, then run one explicitly bounded zm_tomb asset-directory listing; preserve sample identity and record type counts/selected identifiers only in reviewed findings.
- Use observed geometry families to choose one later decoded structure/reference investigation. Implement framing metadata only if it directly helps that access path.

SCOPE NOT IMPLEMENTED:
- Rust scanner/decoder, keys/cryptography/decompression, original payload reads except identity hashing, decoded assets/extraction/conversion, external decoder/game installation/launch, original writes, other formats/platforms, Bevy/entity/gameplay/rendering APIs, commits or pushes.
