# Current handoff

TASK: Remove rendering-improvement TODOs and commit first, then implement one remaining TODO at a time using recovered Black Ops II source.

STATUS: PARTIAL

FINDINGS: Cleanup committed at 19a5199ac removes 32 rendering entries. First remaining TODO, stable IDs for map placements, is complete. The prepared catalog already had validated typed placement IDs; the missing runtime association now binds those IDs to all 59 authored gameplay interaction roots. BO2 capture-zone source indexes named sites independently of temporary runtime entities. The checklist has 1,213 tasks: 475 complete and 738 open.

EVIDENCE: zm_tomb_capture_zones.gsc::init_capture_zone indexes level.zone_capture.zones by script_noteworthy and rejects duplicates; authored sites are found through getent/getstructarray targetname lookups. Existing preparation uses source entity identities, generator/barrier names and perk-site aliases. E:/Coding/Captures/origins-placement-identities-20261007-232445 contains compiler-gates.json, asset-integrity.json, launch.json, exit.json and window/verification.json. Preserved executable SHA256: 962A9D1DE2491150DAFE50103832189CF5D12EA1F02F5148BDC9B2239A12DB93.

CHANGES: PlacementId is a Bevy component and is attached when interaction roots assemble. The existing real-game driver checks missing, duplicate, wrong-site and incomplete bindings, then compares all stable IDs and replaced ECS entities after restart. Only the placement-ID TODO is newly checked; content IDs, assets and gameplay rules are retained.

VALIDATION: Locked metadata, workspace release build (3m 14s), format and complete 4,137-file asset closure passed. Owned actual-game PID 47132 exited 0: six ordinary kills, Round 2, movement, fifteen rendered views, clean Round 1 restart, 59 initial/59 restarted placement bindings, all IDs preserved and all entities replaced, zero streaming errors. Inspected the rendered playing capture. The initial asset invocation failed because default Python lacked tomllib; the installed Python >=3.11 invocation passed. No unit tests, fixtures, Clippy or push.

UNRESOLVED: 738 gameplay checklist entries remain. Existing data-specific placement names/hash disambiguation are retained; native engine entity-number parity and sustained manual play are unqualified. Current static render geometry is separate from the gameplay placement catalog.

RECOMMENDED NEXT WORK: Inspect the next open TODO: Extend asset preparation with trigger volume conversion. Reuse retained collision submodel extraction and original map/script definitions.

SCOPE NOT IMPLEMENTED: No other TODO implementation, new asset extraction/import, renderer work, standalone application, original-game equivalence, performance acceptance or release-package refresh.
