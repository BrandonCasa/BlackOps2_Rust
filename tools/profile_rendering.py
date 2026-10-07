"""Validate Tracy CPU zone nesting and export a steady-view elapsed-time flamegraph.

Input: tracy-csvexport -u -s ';' capture.tracy (inclusive zone times).
GPU zones must be exported separately; they must never enter the CPU tree.
This measures instrumented thread elapsed time, including blocking, not CPU samples.
"""
import argparse
import csv
import html
import json
import re
import statistics
from collections import defaultdict
from pathlib import Path


def gpu_summary(path, begin, end, frames):
    """Keep GPU durations separate; nested pass durations are not additive."""
    zones = defaultdict(list)
    boundary_zones = 0
    with path.open(encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream, delimiter=";"):
            start = int(row["Time from start of program"])
            duration = int(row["GPU execution time"])
            if duration < 0:
                raise ValueError("Incomplete GPU zone; capture did not contain its completion")
            stop = start + duration
            if start < end and stop > begin:
                clipped = min(stop, end) - max(start, begin)
                boundary_zones += clipped != duration
                zones[row["name"]].append(clipped / 1e6)
    if not zones:
        raise ValueError("No GPU zones intersect the workload interval")
    result = {}
    for name, values in sorted(zones.items(), key=lambda item: -sum(item[1])):
        ordered = sorted(values)
        result[name] = {
            "zones": len(values), "inclusive_ms_per_frame": sum(values) / frames,
            "mean_ms_per_zone": statistics.mean(values), "median_ms_per_zone": statistics.median(values),
            "p95_ms_per_zone": ordered[min(len(ordered)-1, int(len(ordered)*0.95))],
            "max_ms_per_zone": ordered[-1],
        }
    return {"metric": "GPU timestamp durations within the CPU workload interval. Nested passes overlap; do not sum all pass totals.",
            "boundary_zones_clipped": boundary_zones, "passes": result}


def allocation_summary(path, begin, end, frames):
    pools = []
    with path.open(encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream, delimiter=";"):
            values = {key: int(value) for key, value in row.items()}
            if (values["begin_ns"], values["end_ns"]) != (begin, end):
                raise ValueError("Allocation summary does not match the workload interval")
            if values["end_live_bytes"] != values["begin_live_bytes"] + values["allocated_bytes"] - values["freed_bytes"]:
                raise ValueError("Allocation byte conservation failure")
            if values["peak_live_bytes"] < max(values["begin_live_bytes"], values["end_live_bytes"]):
                raise ValueError("Allocation peak is below an endpoint")
            values["allocated_bytes_per_frame"] = values["allocated_bytes"] / frames
            values["allocations_per_frame"] = values["allocations"] / frames
            pools.append(values)
    if not pools or not any(pool["allocations"] or pool["begin_live_bytes"] for pool in pools):
        raise ValueError("No native allocation evidence; use profile-tracy-memory")
    return {"metric": "Tracy native instrumented allocator events in [begin_ns,end_ns). Live bytes are tracked heap allocations, not process working set or GPU memory.",
            "pools": pools, "byte_conservation": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("zones", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--view", type=int, default=8)
    parser.add_argument("--markers", type=Path, help="Small Tracy export filtered to benchmark_sample")
    parser.add_argument("--gpu", type=Path, help="Separate Tracy GPU event CSV")
    parser.add_argument("--performance", type=Path, help="Completed game performance receipt, used to reject truncated captures")
    parser.add_argument("--allocations", type=Path, help="Bounded native MemEvent summary from the allocation exporter")
    parser.add_argument("--append-allocations", action="store_true", help="Validate and attach allocations to an existing analysis without rereading CPU zones")
    args = parser.parse_args()
    if args.append_allocations:
        if not args.allocations:
            parser.error("--append-allocations requires --allocations")
        report_path = args.output / "flamegraph.json"
        report = json.loads(report_path.read_text(encoding="utf-8"))
        report["allocations"] = allocation_summary(args.allocations, *report["window_ns"], report["frames"])
        report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report["allocations"], indent=2))
        return
    threads = defaultdict(list)
    markers = []
    with (args.markers or args.zones).open(encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream, delimiter=";"):
            name = row["name"]
            if "benchmark_sample" in name:
                view = re.search(r"view[= :]+(\d+)", name + " " + row["value"])
                if view and int(view[1]) == args.view:
                    markers.append(int(row["ns_since_start"]))
    if len(markers) < 10:
        raise ValueError("Missing steady workload markers or fewer than 10 frames")
    performance = None
    if args.performance:
        performance = json.loads(args.performance.read_text(encoding="utf-8-sig"))
        sample = performance["combat"] if args.view == 15 else performance["views"][str(args.view)]
        if sample["samples"] != len(markers):
            raise ValueError(f"Truncated capture: {len(markers)} workload markers, {sample['samples']} completed game samples")
    begin, end = min(markers), max(markers)
    with args.zones.open(encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream, delimiter=";"):
            start, duration = int(row["ns_since_start"]), int(row["exec_time_ns"])
            if duration < 0:
                raise ValueError("Incomplete zone: capture must end after application disconnect")
            if start < end and start + duration > begin:
                threads[row["thread"]].append((start, start + duration, row["name"]))
    folded = defaultdict(int)
    calls = defaultdict(int)
    zone_count = 0
    for thread, zones in threads.items():
        stack = []
        for start, stop, name in sorted(zones, key=lambda zone: (zone[0], -zone[1])):
            while stack and start >= stack[-1][1]:
                node = stack.pop()
                if node[3] < 0:
                    raise ValueError("Child durations exceed parent")
                folded[node[2]] += node[3]
            if stack and stop > stack[-1][1]:
                raise ValueError(f"Crossing zones on thread {thread}: {name}")
            clipped = max(0, min(stop, end) - max(start, begin))
            path = (f"thread {thread}", name) if not stack else (*stack[-1][2], name)
            if stack:
                stack[-1][3] -= clipped
            stack.append([start, stop, path, clipped])
            zone_count += clipped > 0
            calls[name] += clipped > 0
        while stack:
            node = stack.pop()
            if node[3] < 0:
                raise ValueError("Child durations exceed parent")
            folded[node[2]] += node[3]
    folded = {path: ns for path, ns in folded.items() if ns > 0}
    totals = defaultdict(int)
    children = defaultdict(set)
    for path, ns in folded.items():
        for depth in range(1, len(path) + 1):
            totals[path[:depth]] += ns
            children[path[:depth - 1]].add(path[:depth])
    total = sum(ns for path, ns in totals.items() if len(path) == 1)
    if total != sum(folded.values()):
        raise ValueError("Flamegraph conservation failure")
    hot = defaultdict(int)
    inclusive = defaultdict(int)
    for path, ns in folded.items():
        hot[path[-1]] += ns
    for path, ns in totals.items():
        if len(path) > 1:
            inclusive[path[-1]] += ns
    report = {
        "workload": (performance.get("workload", "combat") if performance else "combat") if args.view == 15 else f"{'moving' if performance and performance.get('continuous_camera_motion') else 'steady'} view {args.view}",
        "view": args.view, "window_ns": [begin, end], "wall_seconds": (end-begin)/1e9,
        "frames": len(markers)-1, "sample_markers": len(markers), "intersecting_zones": zone_count, "threads": len(threads),
        "instrumented_thread_seconds": total/1e9, "crossing_zones": 0,
        "self_time_conservation": True,
        "metric": "Instrumented thread elapsed time including waits; not CPU sampling. GPU excluded.",
        "top_exclusive_ms_per_frame": [(name, ns/1e6/(len(markers)-1)) for name, ns in sorted(hot.items(), key=lambda item: -item[1])[:40]],
        "top_inclusive_ms_per_frame": [(name, ns/1e6/(len(markers)-1)) for name, ns in sorted(inclusive.items(), key=lambda item: -item[1])[:40]],
        "focused_cpu_costs": {
            name: {"zones": calls[name],
                   "exclusive_ms_per_frame": hot.get(name, 0)/1e6/(len(markers)-1),
                   "inclusive_ms_per_frame": inclusive.get(name, 0)/1e6/(len(markers)-1)}
            for name in sorted(calls)
            if any(fragment in name for fragment in (
                "play_queued_audio_system", "audio_clip_decode",
                "specialize_shadows", "shared_shadow_pass", "per_view_shadow_pass",
                "gpu_preprocess", "shadow_indirect_stage_due", "shadow_late_indirect_due",
                "prepare_raytracing_scene_bindings", "origins::navigation::refresh",
                "navigation_geometry_admission", "origins::rendering::shadow_cache"))
        },
    }
    if performance:
        report["completed_game_samples"] = sample
        report["configuration"] = {key: performance.get(key) for key in
            ("adapter", "physical_resolution", "logical_resolution", "antialiasing", "lantern_shadow_map_size",
             "profile_tracy", "profile_chrome", "continuous_camera_motion", "vsync")}
        report["loaded_image_cpu_payload_bytes"] = performance.get("loaded_image_cpu_payload_bytes")
    if args.gpu:
        report["gpu"] = gpu_summary(args.gpu, begin, end, len(markers)-1)
    if args.allocations:
        report["allocations"] = allocation_summary(args.allocations, begin, end, len(markers)-1)
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "flamegraph.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    max_depth = max(map(len, totals))
    width, row_height = 1600, 24
    height = 100 + max_depth * row_height
    svg = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
           '<style>text{font:12px monospace}g:hover rect{stroke:black;stroke-width:2}</style>',
           '<rect width="100%" height="100%" fill="#fafafa"/>',
           f'<text x="10" y="24">Tracy {report["workload"]}: instrumented thread elapsed flamegraph (GPU excluded)</text>',
           f'<text x="10" y="44">{len(markers)-1} frame intervals, {(end-begin)/1e9:.2f}s wall; widths include blocking; hover for names and times</text>']
    def draw(parent, x):
        for path in sorted(children[parent]):
            w = totals[path] / total * (width - 20)
            y = height - 20 - len(path) * row_height
            color = f'hsl({sum(path[-1].encode()) % 55},80%,70%)'
            svg.append(f'<g><title>{html.escape(" / ".join(path))}: {totals[path]/1e6:.3f} ms inclusive, {folded.get(path,0)/1e6:.3f} ms exclusive</title><rect x="{x:.4f}" y="{y}" width="{w:.4f}" height="23" fill="{color}"/>')
            if w > 24:
                label = html.escape(path[-1][:max(1, int(w/7)-1)])
                svg.append(f'<text x="{x+3:.4f}" y="{y+16}">{label}</text>')
            svg.append('</g>')
            draw(path, x)
            x += w
    draw((), 10)
    svg.append('</svg>')
    (args.output / "flamegraph.svg").write_text("\n".join(svg), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
