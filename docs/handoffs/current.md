TASK:
Create and locally commit todo.md containing every outstanding solo Origins implementation task as individual unchecked bullets, each at most 15 words.

STATUS:
COMPLETE - checklist creation only. Gameplay implementation remains outstanding.

FINDINGS:
The repository already tracked 20 Markdown files. The broad roadmap is replaced by the atomic checklist, preserving that limit and all ten required skills. The accepted scope uses intended BO2 behavior, finished reconstructed presentation where needed, mouse and keyboard with efficient 8,000 Hz handling, original HUD information and measured playability. The user explicitly permits necessary file types and raw inputs in this local-only repository.

EVIDENCE:
todo.md contains 1,219 unique unchecked implementation tasks; the longest task is 10 words. Items cover content preparation, reusable gameplay, input, movement, presentation, weapons, damage, enemies, rounds, purchases, generators, perks, powerups, robots, tank, digging, crafting, staffs, puzzles, challenges, quests, audio, HUD and menus.

CHANGES:
Add root todo.md as a flat checkbox list without headings, prose or external instructions. Retire docs/roadmap.md. Redirect README and project-state links. Record the approved asset permission in AGENTS.md and asset documentation. Replace this current handoff.

VALIDATION:
Checklist formatting, word counts, duplicate items, forbidden verification-only wording, stale roadmap links, Markdown count and diff whitespace checked before commit. Initial documentation patch rejected conflicting replacement operations; corrected with updates. Application compiler gates, asset assembly checks and actual-window gameplay are skipped because this change contains documentation only. No unit tests, fixtures or Clippy. No push.

UNRESOLVED:
All 1,219 gameplay checklist items remain unchecked. No application, importer, asset or gameplay changes are implemented by this task. Prior rendering review and implementation evidence remain in Git history.

RECOMMENDED NEXT WORK:
Implement checklist items sequentially within origins.exe and check each item only after its individual code or content change is complete. Preserve the project rules for coherent milestone integration.

SCOPE NOT IMPLEMENTED:
Gameplay/content implementation, renderer changes, asset conversions, new verification code, builds, packaged delivery and gameplay acceptance.
