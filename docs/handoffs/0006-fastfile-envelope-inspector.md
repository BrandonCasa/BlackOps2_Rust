TASK:
Implement one reusable safe Rust signed-PC fastfile envelope metadata inspector from accepted findings 0002, with a thin explicit-input CLI.

STATUS:
COMPLETE

FINDINGS:
- One std-only format adapter serves base, patch, localized and contrast samples without Origins or Bevy coupling; the engine remains Bevy-owned.
- Success means envelope metadata only. Exact TAff0100 / little-endian 147 / PHEEBs71, zero flags and the observed printable nonempty ASCII NUL-terminated zero-padded name subset are supported. Other variants are not silently decoded.
- Reads stage 12 then 312 bytes; one fixed 312-byte buffer and at most 31 allocated metadata-name bytes. No allocation depends on file length; range arithmetic is checked. Signature bytes are opaque/discarded; remainder bytes are never read.

EVIDENCE:
- AGENTS.md; asset-inspection/validation skills; architecture/state; handoff 0005; findings 0001/0002; workspace manifests, Bevy source/tests and local/example configuration boundaries.
- Accepted findings 0002 supply all format/support decisions; no new format, script, entity or runtime investigation occurred.
- CLI opened the same six findings-0002 external files read-only and obtained length from each same opened handle. The separate external Python validation driver resolved paths from ignored local configuration and used full-stream SHA-256 only for identity checks.

CHANGES:
- crates/assets: bo2-assets library with fastfile metadata API, typed redacted errors and synthetic integration tests. Replaced placeholder.
- tools/asset_inspector: bo2-asset-inspector CLI, help/exit handling, redacted source/output failures and synthetic CLI tests. Replaced placeholder.
- Workspace members/Cargo.lock updated; no new registry dependencies. All local crate roots forbid unsafe code.
- README, architecture, project state and documentation indices updated; no loader, asset server or entity API introduced.
- CLI emits only supported version/name/length, opaque signature/remainder ranges, diagnostic stem match and explicit envelope-only/authentication-unverified/payload-unread status. It never emits source paths, raw malformed fields, signature bytes or payload bytes and never follows embedded names.

VALIDATION:
- cargo metadata --locked --no-deps --format-version 1: exit 0, four members with the intended acyclic dependency edges.
- cargo test --workspace: exit 0, 25 tests (14 parser, nine CLI/output and two unchanged Bevy lifecycle tests).
- Synthetic cases cover every truncation before 312; alternate known/invented markers; versions 146/148 and byte-swapped 147; authentication mismatch; every nonzero flag position; unsupported names/padding and longest 31-byte name; short/interrupted reads; injected I/O and invalid reader count; observable reader/length inconsistency; u64::MAX arithmetic; signature invariance and a reader that rejects payload access.
- CLI tests establish success/help/failure/usage exits, source/argument/invalid-field redaction, missing/truncated/non-file inputs, diagnostic-only filename mismatch, returned output errors and source preservation. Entire output is identical when synthetic signature and remainder bytes change while metadata stays fixed.
- cargo fmt --all -- --check and cargo clippy --workspace --all-targets -- -D warnings: exit 0. cargo run -p bo2-boot -- --smoke: exit 0, startup=1 / updates=3 / probe_updates=3.
- All six external samples: inspector exit 0; names/lengths/ranges/remainders match findings 0002, stems match. 1,872 structural inspector bytes total. Twelve separate full-stream hash passes match accepted digests; lengths/write times/hashes unchanged before/after. Driver 0006-fastfile-envelope-validate.py and permitted metadata record 0006-fastfile-envelope-inspector.json remain in configured external output.
- Candidate dependency/unsafe/proprietary-content/local-path review and git diff --check passed. Existing Cargo is invoked from its installed user bin because inherited shell PATH has not refreshed; no toolchain provisioning occurred.

FIDELITY STATUS:
INVESTIGATING
Envelope metadata understanding only; overall/gameplay/visual fidelity remains UNKNOWN. No live original-game, graphical, other-platform or performance comparison occurred.

UNRESOLVED:
- Outer-agent review/integration; authentication, payload layout/completeness, flag semantics, unobserved variants, decoded assets/references and base/patch/localized loading relationships.
- The API detects length inconsistencies observable within the envelope. A larger declared length, concurrent mutation or payload EOF beyond byte 312 cannot be verified without payload access; metadata success makes no such guarantee.

RECOMMENDED NEXT WORK:
- Review/integrate this bounded task, then investigate one payload boundary or original asset structure before proposing a decoder or Bevy loader.

SCOPE NOT IMPLEMENTED:
- Payload reading/decoding, compression/decryption/keys/signature verification, extraction/conversion, new format investigation, recursive cataloging/TOML discovery, entity/gameplay/asset-loading/rendering APIs, original-game launch/writes, commits or pushes.
