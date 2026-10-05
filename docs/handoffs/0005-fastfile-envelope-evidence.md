TASK:
Establish the evidence-backed metadata/support contract for the observed TAff0100 / candidate-147 fastfile envelope before a payload decoder.

STATUS:
COMPLETE

FINDINGS:
- Six read-only local samples corroborate a 312-byte signed PC envelope; public OAT source establishes 147 as the little-endian PC zone version.
- Markers, zero flag bytes and bounded NUL-terminated ASCII name metadata are supported for one conservative inspector. Signature bytes and the remainder stay opaque; parsing is not authentication or full-file validation.
- Base/patch/localized candidates share the envelope shape; no dependency order or patch semantics was inferred.

EVIDENCE:
- AGENTS.md, asset-inspection and bo2-reverse-engineering skills, architecture/state, catalog 0001, handoff 0004, documentation indices, ignore rules and local/example configuration.
- Three cataloged tomb samples plus common_zm, zm_transit and en_zm_transit contrasts: exactly 312 prefix bytes per sample twice, full SHA-256 streaming twice and length/write-time snapshots. No bytes beyond the prefix interpreted; no game process or source write.
- OpenAssetTools GPL-3.0 revision 59904a3e1dd698ce1c8b8e750d12b47e19cf83ba: header/constants, T6 inspection/auth steps, filename/magic/signature handling. Exact primary-source URLs and evidence distinctions are in findings 0002.
- Metadata-only observation script/JSON retained in configured external output; no source bytes, original scripts, keys or proprietary content in tracked findings.

CHANGES:
- docs/reverse_engineering/0002-fastfile-envelope.md: field table, support subset, measurements, bounds, proposed reusable tooling metadata/parser contract and synthetic scenarios.
- Updated reverse-engineering/documentation indices and minimal project state; added this handoff. No tracked implementation or finalized loader/entity APIs.

VALIDATION:
- All six observations reproduced twice; hashes, lengths and write times unchanged. Tomb identities match catalog 0001.
- 12+8+4+32+256=312; each remainder equals actual file length minus 312 and is positive. Prefix reads stayed below the 4096-byte limit.
- Documentation/link/local-path and content review, git diff --check passed. Rust gates intentionally not rerun for documentation-only changes; prior executable validation is unchanged.

FIDELITY STATUS:
INVESTIGATING
Format understanding only; overall/gameplay/visual fidelity remains UNKNOWN.

UNRESOLVED:
- Outer-agent review/integration; flag semantics, unobserved envelope variants, payload layout/completeness, signature validity, expanded assets/references and base/patch/localized loading relationships.

RECOMMENDED NEXT WORK:
- Implement exactly one bounded safe Rust envelope metadata inspector under tooling, using findings 0002 and synthetic cases; re-observe these external samples. Decoder/loader investigation remains separate.

SCOPE NOT IMPLEMENTED:
- Rust parser; crypto/keys, decompression/decryption, extraction/conversion, payload reading, asset/entity schemas, Bevy loaders/rendering/gameplay, original-game launch/process access/writes, commits or pushes.
