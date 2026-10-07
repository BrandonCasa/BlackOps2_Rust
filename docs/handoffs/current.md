TASK:
Fix barrier audio, slat placement and raised tearing zombies; implement TODO audio for existing gameplay and audit shared asset/event handling. Integrate latest main before continuing.

STATUS:
PARTIAL. Native audio and barrier fixes saved; integration and final merged validation remain.

FINDINGS:
Board animations omitted native root translations. Zombie grab/pull clips also require their authored downward root displacement. Source aliases can chain secondary bass/decay layers; B23R references required resolving the primary player shot. Dialogue throttling must exclude zombie vocals.

EVIDENCE:
External inspection/audio-recovery contains preparation, root-motion audit and import receipts. Window-audio-barriers-layers passed twelve windows, 72 boards, individual tearing/repair, grounding, restart and 36 native wood sinks. Wall-weapons-layers passed six MP40 shots and four B23R primary/bass layers. Speed-cola-final, Semtex-layers and Monkeys-layers completed actual-window cases. Door-layers exited 1 and needs diagnosis.

CHANGES:
Native SAB audio preparation, validated alias chains/action timelines/surfaces, shared Bevy mixer, audio controls/captions, current gameplay sound events, native board root animation and zombie interaction motion. Native sources remain untouched. 36 scoped checklist tasks newly marked complete.

VALIDATION:
Locked metadata, release build and formatting passed before the latest character-dialogue filter change. Runtime closure and staged LFS validation passed 2,547 files / 2,546 binary pointers. One concurrent-window Semtex run failed with GPU DeviceLost; sequential retry passed. Existing Vertex_Color errors were observed before main integration. Latest speech policy still requires compilation and actual-window checks. No unit tests, fixtures or Clippy used.

UNRESOLVED:
Main integration, door failure, final merged asset/compiler/window/package validation. Generic subtitles reflect events because recovered speech entries lack subtitle text. Debris fall and collision surfaces are documented reconstructions. Middle barrier attack slot only; full-map sustained play unqualified.

RECOMMENDED NEXT WORK:
Replay this saved milestone on latest local main, preserve its generator/rendering/presentation fixes, integrate audio for newly present gameplay, then validate and commit the coherent result.

SCOPE NOT IMPLEMENTED:
Audio for absent gameplay, full Samantha/quest speech, dynamic weather transitions, complete accessibility/settings persistence, left/right or upgraded barrier interactions and exact native acoustics.
