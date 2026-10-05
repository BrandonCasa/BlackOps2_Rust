TASK:
Establish ordinary coop Downed->Standing revive completion from licensed Origins
scripts, including health restoration/cancellation and minimal deterministic input.

STATUS:
COMPLETE
Bounded source contract established; native restored current-health gap precisely
identified and not replaced with an invented value.

FINDINGS:
- Ordinary completion calls native reviveplayer, then max-health reboot100,
  deletes downed trigger and clears ignore; restored current health is unknown.
- Base attempt accumulator3seconds with0.05second waits; cancellation restarts
  at0. Final interval eligibility and same-frame events require qualification.
- Downed predicate uses trigger presence, so script laststand-flag clearing after
  failed attempt does not establish standing.
- Bleedout defers while beingrevived; don't impose unconditional death priority.
- Origins rejects robot-stomped revive target; ordinary path uses shared checks.

EVIDENCE:
- Current state, architecture/Vitality/tests, handoffs01/02 and investigation skill.
- Five licensed buffers re-decompiled; _zm_laststand disassembly corroborates
  timing literals and completion order. Fresh external task03 identity receipt.
- Original fastfile hashes match task01/build24784239. Plutonium r5354 is an outer
  UI observation, not verified runtime equivalence with Steam compiled evidence.
- Findings: ../reverse_engineering/damage-life-state-03.md.

CHANGES:
- Findings and this handoff only. No gameplay/config/Cargo/project-state edits.
- Raw source/disassembly and receipt remain external below configured task03 root.

VALIDATION:
- Five decompilations/one disassembly succeed; selected input/output hashes recorded.
- Permitted content/path review and git diff --check pass.
- Unchanged Cargo gates not rerun; no live original or health observer used.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Native reviveplayer/setmaxhealth current effects and same-frame race ordering.
- Runtime Plutonium override/version compatibility and actual coop comparison.
- Raw interaction geometry, native start/stop behavior and pending expiry policy.

RECOMMENDED NEXT WORK:
- Qualify native restored current/max values before fixing revive health100.
- Implement resolved completion with downed-episode/attempt identity and validated
  restoration values; qualify comparison cases in findings.

SCOPE NOT IMPLEMENTED:
- Gameplay, bleedout/timer framework, interaction/HUD, perks/devices/solo revive,
  original UI launch/input, commits/pushes or nested delegation.
