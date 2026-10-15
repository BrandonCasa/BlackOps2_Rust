# Build a simpler, faster game

Good work preserves the full game while making its implementation simpler, faster, and easier to maintain. Treat existing code as something to improve: keep what serves the game, fix what causes problems, and delete what adds work without value.

- Look first for an existing owner, a built-in Bevy or Avian API, or an opportunity to remove unnecessary machinery. Prefer the solution with the fewest moving parts.
- Let concrete, current requirements determine the design. Implement the actual case directly; introduce shared abstractions when real consumers demonstrate a useful common behavior.
- Keep state with its owner, call operations directly, and store each fact in one authoritative place. Give each layer and piece of bookkeeping a clear gameplay purpose.
- Establish the data's guarantees and rely on them. Correct bad data at its source, report broken required inputs clearly, and handle expected gameplay rejection through ordinary control flow.
- Use the engine's ordinary APIs and lifecycle. Keep custom code focused on source-specific behavior that requires it.
- Complete replacements by deleting the superseded path and its supporting plumbing. Aim for fewer concepts to understand and fewer places to change.
- Actively clean up the code you touch. Remove dead paths, redundant state, duplicate work, obsolete workarounds, and abstractions that make straightforward behavior harder to follow. Fix the underlying cause of a problem and retire the patches that only compensated for it.
- Treat frame time, loading time, and memory as part of correctness. Look for repeated scans, per-frame allocations, unnecessary cloning, polling, blocking work, and resources that outlive their use. Make work happen only when needed, over only the data it needs.
- Prefer eliminating work to making waste more elaborate. Use direct access, existing engine facilities, and clear lifetimes first; add caching or concurrency when a demonstrated bottleneck warrants it. Check performance changes against the affected workload and distinguish measured gains from expected ones.
- Preserve full-map content, original assets, source values, timing, and implemented mechanics throughout simplification.
- Read the finished diff from the next maintainer's perspective. Keep each addition tied to the requested behavior, verify the affected behavior proportionately, and explain the result briefly and accurately.

Apply these principles through implementation choices. Success is the same complete game with less machinery, less wasted work, and smoother play.
