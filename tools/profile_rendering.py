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
from collections import defaultdict
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("zones", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--view", type=int, default=8)
    parser.add_argument("--markers", type=Path, help="Small Tracy export filtered to benchmark_sample")
    args = parser.parse_args()
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
    begin, end = min(markers), max(markers)
    with args.zones.open(encoding="utf-8-sig", newline="") as stream:
        for row in csv.DictReader(stream, delimiter=";"):
            start, duration = int(row["ns_since_start"]), int(row["exec_time_ns"])
            if duration < 0:
                raise ValueError("Incomplete zone: capture must end after application disconnect")
            if start < end and start + duration > begin:
                threads[row["thread"]].append((start, start + duration, row["name"]))
    folded = defaultdict(int)
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
    for path, ns in folded.items():
        hot[path[-1]] += ns
    report = {
        "workload": "combat" if args.view == 15 else f"steady view {args.view}",
        "view": args.view, "window_ns": [begin, end], "wall_seconds": (end-begin)/1e9,
        "frames": len(markers)-1, "sample_markers": len(markers), "intersecting_zones": zone_count, "threads": len(threads),
        "instrumented_thread_seconds": total/1e9, "crossing_zones": 0,
        "self_time_conservation": True,
        "metric": "Instrumented thread elapsed time including waits; not CPU sampling. GPU excluded.",
        "top_exclusive_ms_per_frame": [(name, ns/1e6/(len(markers)-1)) for name, ns in sorted(hot.items(), key=lambda item: -item[1])[:40]],
    }
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
