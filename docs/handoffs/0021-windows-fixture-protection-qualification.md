TASK:
Qualify Windows process/file protection controls on entirely user-created fixtures
before any separately admitted ordinary-name x86 capture-SDK startup qualifier.

STATUS:
PARTIAL
Four admitted cases are measured and terminal; directory-anchor rename protection
fails and job accounting/active-limit enforcement remains unresolved. No case repeat.

FINDINGS:
- Held read/READ-sharing config/cache handles allow reads and deny eight truncate/
  overwrite/delete/rename attempts with error32. Owned DACLs deny five staging/
  directory/unheld-file operations with error5.
- An attribute-only directory anchor permits new-file write and same-volume
  rename. Canonical identity checking detects the change; it does not prevent it.
- Suspended parent/child image, creation-time, exact-job, CWD, TEMP and user-DLL
  identity gates pass. Wrong hash rejects body effects before initial resume.
- Last job-handle close ends the exact live pair at the independent deadline;
  normal cases report active0. Deadline accounting reports active4 despite limit2,
  so effective limit enforcement and complete job inventory are not qualified.

EVIDENCE:
- Read full goal/rules, reverse-engineering/validation skills, architecture/state,
  handoffs0017-0020, findings0011-0014 and retained external guard/job/build/supervisor
  sources. No original data, SDK binaries, profiles or geometry input read.
- Ten Microsoft primary-page snapshots total677078bytes with URL/byte/SHA receipts.
  Windows11 build26340, CPython3.12.11 x64, MSVC19.51.36260/toolset14.51.36231,
  Windows SDK10.0.26100.0; user-created I386/PE32 EXE/DLL only.
- External0021-windows-fixture-protection-qualification retains source/build/ABI
  checks, pre-execution freeze, same-handle launch hashes/metadata, child launch
  identities, logs, file/ACL/cleanup receipts, failed case and terminal validation.

CHANGES:
- Added capture-protection-controls architecture record, findings0015 and this
  handoff; minimal docs/architecture/reverse-engineering indexes and state updates.
  Seven owned documentation candidates; no tracked code/config/skill/dependency change.
- External Windows FFI is qualification-only with checked ABI/ownership; all seven
  Rust crates retain unsafe prohibition and Bevy ownership. No commit or push.

VALIDATION:
- One valid/ten invalid synthetic admission cases and host ABI/build/export checks pass.
  Initial linker alias failure is preserved; no native execution occurred in preparation.
- Installed-header offset/width/constant/info-class static assertions compile for
  x64/x86 objects; empty-job query-back returns flags0x2008/limit2 and correct144/48
  byte buffers. No historical limit query-back or extra fixture run; discrepancy unexplained.
- Four native cases: normal PASS0.157s/2152B; mutation FAIL0.063s/2234B on allowed
  anchor rename; deadline PASS4.031s/2116B; suspended mismatch PASS0.016s/0B.
  Bounds30s/4s,32768B; exact fixture processes terminal, readers joined, pipes/handles closed.
- Cases1/3/4 retain same-handle/reopened preservation. Case2 reaches its directory
  failure after file checks but omits serialized end values; post-failure reopened
  identities and two owned ACL restorations pass. Renamed evidence is retained.
- All117 task0020 external artifacts and manifest unchanged; scoped tracked/config
  baseline, ownership/link/heading/privacy/LF/whitespace checks pass. No unchanged
  Cargo74-test/GPU/inspector gate repetition or original comparison.
- Two validator-only schema/baseline assumptions were corrected; one raw first
  trace was overwritten by filename reuse, with source/tool observation retained.
  Second trace and fresh final source/receipts are preserved; no native retry.
- Outer review accepts PARTIAL evidence after source/receipt review and independent
  preservation of 100 non-owned records, 117 prior artifacts and 147 task artifacts.
  All seven documentation candidates pass link/privacy/heading/LF/whitespace review;
  no extra native scenario, unchanged Cargo/GPU gate or SDK/original action occurred.

FIDELITY STATUS:
INVESTIGATING for data/access; overall/gameplay/visual UNKNOWN. Full stable Rust
with Bevy Origins objective remains active and unfinished; no MATCHED/VERIFIED claim.

UNRESOLVED:
- Attribute-only directory rename control fails; stronger name/ancestor controls,
  unexpected job accounting and effective process-limit enforcement are unqualified.
- Actual SDK resolver/target/standard-child identities, registration/fallback,
  early DbgHelp/system modules and Windows/library effects remain unobserved.
- Fixture cooperation is not a supported SDK pre-init callback; TEMP does not route
  KnownFolder config/cache. Post-last-close whole-job accounting is unavailable.

RECOMMENDED NEXT WORK:
- Separately admit one owned-job PID-list/creation-time discriminator for the
  accounting discrepancy and a required directory-name control if applicable.
  Outer-review that result before concrete ordinary-name SDK qualifier admission.
- Later measure actual target/child config/cache/CWD/module identities and
  registration mode under accepted protections; keep original/menu/calibration
  admission and gameplay/visual evidence separate.

SCOPE NOT IMPLEMENTED:
- SDK load/injection/capture/replay/registration/crash/dump, original game/Steam/
  process/header/memory/input/profile, accepted geometry, global/shared ACL/env/
  registry/service changes, broad process inventory/name-based kills, production
  sandbox/wrapper, Rust/Bevy/rendering/gameplay changes, installation, commit/push
  or nested delegation. No fixture destruction or extra native scenario.
