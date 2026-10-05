TASK:
Locate the external licensed BO2 source and catalog Origins-relevant container families sufficiently to select one bounded format investigation.

STATUS:
COMPLETE

FINDINGS:
- Four registered Steam libraries checked; one BO2 installation directory shared by base app 202970 and Zombies app 212910. Zombies manifest build/target build 24784239, English labels; no independent ownership, runtime-build or Steam integrity verification.
- 309 files totaling 9,268,005,069 bytes. Zone .ff/.ipak, sound .sabl/.sabs and video .webm/.bik families exist; zm_tomb/zmb_tomb identifiers form the next Origins candidate set, without proven map identity or references.
- Three zm_tomb .ff samples share TAff0100 and little-endian u32 147 at offset 8. Version/field semantics remain candidates, not established format support.

EVIDENCE:
- Read AGENTS.md, asset-inspection skill, architecture/state, prior Bevy handoff, documentation indices, ignore rules and config example.
- Steam registry/library metadata and BO2 manifests; scoped directory metadata; executable version-resource metadata; seven bounded header samples (16 bytes each, three reread once) and eight full-stream SHA-256 identities.
- Metadata aggregation only for players/ancillary content. No script/entity definitions, data bodies, decoded assets, live game comparisons or gameplay measurements inspected.
- Detailed allowed metadata summary remains in configured external output; tracked findings contain only relative identifiers, aggregate measurements, prefix field observations and hashes.

CHANGES:
- Ignored config/local_assets.toml: actual absolute external installation/output roots, existing two-key assets schema; no application consumer added.
- docs/reverse_engineering/0001-local-asset-catalog.md: procedure, source identifiers, coverage, counts/lengths, candidate identifiers, prefix/hash evidence and one next investigation.
- Updated reverse-engineering/documentation indices and docs/project-state.md; added this handoff. No Rust code or architectural API changes.

VALIDATION:
- Python tomllib parses actual/example configuration; section/key sets match. Both absolute existing roots resolve outside the repository. git check-ignore confirms actual configuration excluded; example remains trackable.
- All eight sample hashes, lengths and last-write timestamps unchanged on repeated verification. Read-only streams/hashing used; no source writes, original copies or extraction.
- Aggregate counts/lengths sum to 309 files/9,268,005,069 bytes; scoped scan found no reparse points.
- Documentation links, candidate content/path review and git diff --check passed. Compiler/workspace tests intentionally not rerun because Rust/dependencies were unchanged; prior accepted executable baseline is unchanged.

FIDELITY STATUS:
INVESTIGATING
Data availability only; overall/gameplay/visual fidelity remains UNKNOWN.

UNRESOLVED:
- Outer-agent review/integration; actual Origins map identity, runtime build, asset completeness, dependency/patch order, all body schemas and original behavior.
- No search of unregistered installations or other launchers after one Steam-backed source was confirmed. No all-file Steam integrity or independent ownership check.

RECOMMENDED NEXT WORK:
- One bounded investigation of TAff0100/candidate-147 .ff envelope metadata on the three base/patch/localized samples; establish header semantics/version bounds with authoritative evidence before parser/loader implementation.

SCOPE NOT IMPLEMENTED:
- Launching/modifying the game; memory/process inspection; downloading/reinstalling; decoding/decryption/decompression/conversion/extraction; original script/content copying; engine APIs, asset loading, rendering/gameplay; commits or pushes.
