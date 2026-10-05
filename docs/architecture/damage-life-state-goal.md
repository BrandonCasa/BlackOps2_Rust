# Damage and life-state worktree

Status: the user paused this goal and requested main-worktree integration on
2026-10-05. Reviewed work through `7e241cc` is integrated; the session-loss
implementation task was stopped before edits. This brief records the original
task ownership, not an active instruction to resume it.

## Goal and ownership

Implement evidence-backed shared damage and life-state systems for Origins
players and ordinary zombies, with Origins configuration, a runnable headless
demonstration, deterministic tests and scoped original-game comparisons.
Synthetic tests alone do not complete this goal.

This worktree owns damage application, health and lethal-state behavior, followed
by supported player downing/revival and zombie death. Collision/movement and
static-world appearance are owned by other worktrees. Weapon definitions, AI
navigation, rounds, models and animations remain outside this goal.

User clarification on 2026-10-05 permits one active subagent **per worktree**.
Review each local worker's handoff, evidence and diff before the next assignment.
Keep external tooling task directories separate between worktrees.

## Evidence-first sequence

1. Resolve damage application and player/zombie lethal routing from licensed
   sources; separate source observations from live measurements and unknowns.
2. Review the supported shared boundary, then implement it with safe Rust and
   Bevy components/systems. Do not introduce guessed game constants or a second
   entity registry, scheduler or ECS.
3. Investigate and implement remaining down/revive/death transitions in bounded
   increments, preserving determinism and explicit Origins configuration.
4. Run the headless demonstration and applicable workspace checks, then compare
   implemented cases with the original using recorded conditions and identities.

No API, health value, timing, damage modifier or transition is selected by this
ownership brief. Gameplay fidelity remains UNKNOWN until evidence supports a
more specific status. Original data, raw scripts and captures remain external.
