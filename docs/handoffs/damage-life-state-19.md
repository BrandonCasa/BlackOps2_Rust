TASK:
Replace unsupported actor terminal numeric health with explicit uncertainty and a separate signed pre-notification subtraction stage.

STATUS:
COMPLETE

FINDINGS:
- ActorDead current() is None; no invented zero is stored or exposed. Private last positive modeled standing health is retained. Human current() remains Some of its prior modeled value.
- ActorSubtraction records before/amount/signed after separately. Positive admitted signed-domain inputs safely cover negative results. No later -999 floor or final callback health is fabricated.
- Ordinary terminal routing, ignored repeats, ordered replay, human revival/expiry, bounds and token invariants remain intact.

EVIDENCE:
- Native14/15 evidence qualifies direct subtraction before notifications, then leaves notification/death-handler final health unresolved. Ordinary-policy admission remains caller responsibility.
- Relevant damage/revive/terminal tests, Origins configuration and demo inspected. No original access or measurements in this implementation task.

CHANGES:
- crates/gameplay/src/lib.rs: optional qualified current, typed subtraction stage and terminal outcome.
- crates/gameplay/tests/{damage,revive,terminal}.rs and game/zombies/origins/tests/configuration.rs: migrated callers; added signed-stage/unknown-final boundary cases.
- tools/comparison_tools/damage_life_state_demo/src/main.rs: exact stage/unknown-final assertions and scoped player comparison wording.
- docs/architecture/damage-life-state.md and this handoff: contract and limitations.

VALIDATION:
- Focused gameplay/Origins locked tests:24 pass, exit0.
- Locked workspace metadata:exit0, unchanged10 members.
- Locked workspace tests:104 pass, exit0.
- All-workspace fmt check and locked all-target Clippy with -D warnings:exit0.
- Locked headless demo:21 exact asserted outcomes, exit0.
- Diff/content review and git diff --check:pass; no dependencies, unsafe, original content or local asset paths added.
- Final diagnostic wording correction followed by fmt check/demo:exit0; no behavior changes.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Final native actor callback health, selected actor classification and active dispatch/invocation. Scoped live player100->40 agreement does not qualify full/coop/actor behavior.

RECOMMENDED NEXT WORK:
- Controlled selected-actor before/after comparison with explicit identity limits; qualify final health before adding any terminal numeric rule.

SCOPE NOT IMPLEMENTED:
- Native/UI investigation, observer changes, timers, raw impacts, AI, solo/session policy, dependencies, commits or pushes.
