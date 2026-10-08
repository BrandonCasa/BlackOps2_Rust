# Current handoff

TASK: Put the rendered version shown in the user's two screenshots on local main while keeping the completed texture-streaming fixes.

STATUS: COMPLETE

FINDINGS: Main already contained c548f966f's streamer and viewmodel fixes through 34b71ee5c, but its runtime assets still had the old version-one index: 1,358 entries and no surface regions. The previous subagent's executable-relative package has the version-two index, 1,356 entries, 3,986 surface regions, and the qualified higher-resolution chains/material bindings. The local release executable was also older than the current main build.

EVIDENCE: Photo 1 matches E:/Coding/Captures/origins-viewmodel-streaming-20261007/v2-stable-ordinary/zombie-attack.png; Photo 2 matches startup-fixed-shadow-qualified/sun-lit-on.png. Their retained launch receipts identify executables 636C385672D78219FE23146C18CBCE10DBF21FE067FCA7B5F3EE5281B86F6849 and 0E789B4A9C5E7E0C81B0ED492647C192EFAE4535E191BB61C7429C8FE7EAD978. The package/assets/origins tree is the prepared pack used by this task's retained rendered runs. main-import-20261007.json records the imported candidates.

CHANGES: Imported 51 prepared files: 24 material-bearing glTF documents, 26 DDS bootstrap/full-chain files and texture-streaming.json. Retired two full-chain files replaced by constant bootstrap textures. All geometry/binary buffers, game.json and current Rust rendering/streaming implementation are retained. The runtime pack has 4,137 files. Refreshed ignored release/assets/origins and release/origins.exe from the imported pack and this chat's current-main build, SHA256 B317673612B00FDFB4E42D55B367D60B9752443CE1E854F4573CF94CD849E52C.

VALIDATION: No new compiler, format, asset-validation or gameplay-test gates after the user's explicit stop. This chat's earlier locked release build, metadata and format checks passed before that stop; its first combat driver and direct asset check were stopped without acceptance. The external config-based check failed on invalid audio alias fields, and one direct-check invocation used an older Python without tomllib. Historical subagent receipts qualify the source pack and exercised streamer/shadow/gameplay paths; they are not new post-import acceptance. The normal release window is opened without a verification driver. No unit tests, fixtures, Clippy or push.

UNRESOLVED: The separate complete-four-material 4x cook subsequently finished externally (16 maps, 47 bindings, 4,169 validated files in its retained receipt), but is not part of the user's accepted screenshot pack and is not imported by this task. It remains preserved under the task's complete-materials evidence and external preparation directories. No new pixel-equivalence or performance claim.

RECOMMENDED NEXT WORK: Use this main runtime pack and refreshed release package. Keep streamer fixes enabled. Further texture migration is the existing broad todo.md item; adopt the separate complete-four-material candidate only as a subsequent requested milestone.

SCOPE NOT IMPLEMENTED: No renderer rollback, new texture cook, new PBR reauthoring, gameplay changes, additional validation or tests.
