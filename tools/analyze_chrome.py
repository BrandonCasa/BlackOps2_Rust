import argparse
from collections import defaultdict
import json
from pathlib import Path

from verify_brp import digest, require


def events(path, contains=None):
    closed = False
    with path.open(encoding="utf-8") as stream:
        require(next(stream).strip() == "[", "Expected tracing-chrome event array")
        for line in stream:
            value = line.strip().removesuffix(",")
            if value == "]":
                closed = True
                break
            if value and (contains is None or contains in value):
                yield json.loads(value)
    require(closed, "Chrome trace was not flushed completely")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("trace", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--performance", type=Path)
    args = parser.parse_args()
    require(not args.output.resolve().is_relative_to(Path(__file__).resolve().parent.parent),
            "Trace evidence must be external")
    if args.performance:
        performance = json.loads(args.performance.read_text(encoding="utf-8"))
        require(performance["passed"] and performance["workload_revision"] == 3, "Incomplete compiled workload")
        markers = [event for event in events(args.trace, "origins::verification::release_performance")
                   if event.get("cat") == "origins::verification::release_performance" and event["ph"] == "i"]
        require(len(markers) == 3, "Expected cohort-ready, sampling-start, and completion markers")
        begin, end = markers[1]["ts"], markers[2]["ts"]
        frames = performance["summary"]["samples"]
        require(abs((end - begin) / 1e6 - performance["sample_wall_seconds"]) < 0.1,
                "Trace interval does not match the completed workload")
    else:
        markers = [event for event in events(args.trace, "external_performance_sample")
                   if event.get("name", "").startswith("external_performance_sample") and event["ph"] in ("B", "E")]
        begins = [event["ts"] for event in markers if event["ph"] == "B"]
        ends = [event["ts"] for event in markers if event["ph"] == "E"]
        require(len(begins) == len(ends) and len(begins) >= 2, "Missing or incomplete sample markers")
        begin, end = min(begins), max(ends)
        frames = len(begins)
    stacks = defaultdict(list)
    totals = defaultdict(lambda: {"spans": 0, "inclusive_ms": 0., "self_ms": 0., "max_ms": 0.})
    threads = {}
    renamed = 0
    for event in events(args.trace):
        phase = event["ph"]
        if phase == "M" and event.get("name") == "thread_name":
            threads[event["tid"]] = event["args"]["name"]
            continue
        if phase not in ("B", "E"):
            continue
        stack = stacks[event["tid"]]
        if phase == "B":
            stack.append((event, 0.))
            continue
        require(bool(stack), "Unmatched Chrome span exit")
        entered, children = stack.pop()
        require(entered.get("cat") == event.get("cat") and entered.get(".file") == event.get(".file")
                and entered.get(".line") == event.get(".line"), "Chrome span nesting mismatch")
        renamed += entered["name"] != event["name"]
        require(event["ts"] >= entered["ts"], "Negative Chrome span duration")
        duration = max(0., min(end, event["ts"]) - max(begin, entered["ts"])) / 1000.
        require(children <= duration + 0.001, "Child spans exceed their parent")
        if stack:
            parent, previous = stack[-1]
            stack[-1] = (parent, previous + duration)
        if duration:
            total = totals[(event["tid"], entered["name"])]
            total["spans"] += 1
            total["inclusive_ms"] += duration
            total["self_ms"] += max(0., duration - children)
            total["max_ms"] = max(total["max_ms"], duration)
    unclosed = [entered for stack in stacks.values() for entered, _ in stack if entered["ts"] < end]
    rows = [{"thread": threads.get(tid, str(tid)), "tid": tid, "name": name, **total,
             "inclusive_ms_per_frame": total["inclusive_ms"] / frames,
             "self_ms_per_frame": total["self_ms"] / frames}
            for (tid, name), total in totals.items()]
    rows.sort(key=lambda row: -row["self_ms"])
    result = {"trace_sha256": digest(args.trace), "window_microseconds": [begin, end],
              "frames": frames, "seconds": (end - begin) / 1e6,
              "metric": "Instrumented thread elapsed time, including blocking; inclusive spans overlap.",
              "renamed_spans": renamed, "unclosed_spans_intersecting_sample": unclosed, "rows": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"frames": frames, "seconds": result["seconds"], "top_self": rows[:25]}, indent=2))


if __name__ == "__main__":
    main()
