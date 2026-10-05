---
name: rust-performance
description: Profile one established Rust/Bevy workload or optimize one measured bottleneck while preserving correctness, determinism and fidelity. Use after relevant behavior is established, not for speculative micro-optimization or broad rewrites.
---

# Rust performance

## Bounded task

Choose profiling or one justified optimization, with a workload, subsystem,
metric, owned files and completion condition. Read
[project rules](../../../AGENTS.md), correctness/fidelity evidence, previous
benchmarks, [architecture](../../../docs/architecture/README.md) and handoff.
Do not spawn agents, commit or combine unrelated hot paths into one task.

## Investigation

1. Establish a reproducible workload with simulation seed, replay inputs,
   population/scene size, fixed-step settings and warm-up. Use synthetic fixtures
   or external licensed data without copying proprietary content into benchmarks.
2. Record stable Rust/Bevy versions, hardware class, build mode, profiler,
   sampling method and relevant system settings without local paths.
3. Measure a baseline distribution; separate loading, simulation and rendering.
   Attribute cost to the assigned path using profiling rather than intuition.
4. Inspect relevant Bevy queries, scheduling and engine facilities before adding
   custom caches/adapters. Check allocations, contention, data access and ordering
   only where the profile identifies them.
5. Form one optimization hypothesis and predict its correctness/determinism risks.
   Preserve unresolved fidelity limits; faster behavior is not more faithful.

## Required evidence

Record method, workload, hardware class, repetitions, variance and bottleneck
attribution in permitted performance documentation under `docs/fidelity/`.
Link baseline correctness/fidelity tests. Raw proprietary profiler reports and
machine-specific paths stay external. Label unsupported hypotheses explicitly.

## Implementation expectations

Profiling tasks do not authorize optimization. An assigned optimization must
preserve observable semantics, replay determinism and architecture boundaries.
Prefer safe, data-oriented Rust and Bevy facilities; do not weaken
`forbid(unsafe_code)`, replace the ECS or change simulation rates for throughput.
An unsafe need is an architectural issue for outer-agent review, not a shortcut.

## Validation

Run meaningful correctness and deterministic replay checks before and after,
plus applicable workspace gates. Repeat the same release workload and report
before/after distributions, regressions and resource costs. Retain or reject the
change according to evidence; never report an unmeasured speedup. Distinguish
synthetic throughput from live-game fidelity and sustained runtime measurements.

## Handoff and escalation

Save and return the [required handoff](../../../AGENTS.md#required-handoff) with
baseline, profiler attribution and actual results. Preserve existing fidelity
status; performance evidence cannot confer `MATCHED`/`VERIFIED`. Report missing
correctness baselines, unavailable profiling tools, unstable workloads, unsafe
requirements or bottlenecks outside scope to the outer agent before broadening work.
