TASK:
Establish the licensed Origins damage-to-lethal-state contract for an unperked
player and ordinary zombie sufficient to guide the first shared foundation.

STATUS:
COMPLETE
Bounded script investigation completed; native application and live fidelity
remain explicitly unverified, not claimed complete by this investigation.

FINDINGS:
- Original compiled patch scripts are accessible using existing OAT and an
  official hash-verified external gsc-tool; no gameplay inspector is required.
- Shared player/actor routes differ. Player equality enters lethal handling,
  last-stand assigns health1, and sole/last-player game end is a separate branch.
- No-perk/no-persistent-bonus player max health100; ordinary zombie melee60 and
  fallback health150 are source observations with branch/configuration limits.
- Origins calls shared initialization and wraps actor/player damage. Ordinary
  zombie route delegates; special zombies/weapons introduce explicit exclusions.

EVIDENCE:
- Current project state, architecture, handoff0027, core source and investigation
  skill; existing OAT source and asset-directory evidence inspected.
- Steam build24784239; three licensed fastfiles with before/after hashes.
- Nine selected server-script decompilations and four corroborating disassemblies;
  see ../reverse_engineering/damage-life-state-01.md for identities/procedure.
- No game launched or live runtime measurements taken.

CHANGES:
- docs/reverse_engineering/damage-life-state-01.md and this handoff only.
- Recommend reusable resolved-damage/health boundary with separate Zombies life
  policy and Origins assembly; no gameplay API implemented or Cargo change.
- Raw scripts, compiled buffers, listings, tool and identity receipt remain in
  distinct external damage-life-state-01 task directory below configured output.

VALIDATION:
- Three original LIST runs and two script-only dumps exit0; dumps report zero
  warnings/errors. Nine decompilations/four disassemblies succeed.
- Synthetic truncated4-byte GSC rejects with exit1; no new inspector.
- Fastfile/executable hashes agree before/after; original data not written.
- Scope/content review and git diff --check pass. Cargo gates unchanged and not
  rerun by worker; outer baseline validation is separate.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Native finishplayerdamage/finishactordamage arithmetic, clamping/conversion and
  engine callback ordering; actual patch load order and live original comparisons.
- Current-health spawn initialization and actual round1 actor health need separate
  evidence beyond observed max-health/fallback configuration.
- Precise regen/invulnerability, revival/bleedout and coop session-end integration.

RECOMMENDED NEXT WORK:
- Implement scoped resolved positive integer damage and distinct player downing/
  actor death; preserve native arithmetic as explicit inference pending comparison.
- Execute noninvasive coop downing/sole-player control, downed damage control and
  ordinary zombie terminal-death comparison cases described in findings.

SCOPE NOT IMPLEMENTED:
- Gameplay, engine disassembly, original runtime instrumentation, revival/AI,
  weapons/rounds, broader script decoding, commits, pushes, nested delegation.
