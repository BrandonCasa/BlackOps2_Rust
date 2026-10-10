# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import time

from verify_brp import assets_match, digest, require


def powershell(command):
    return subprocess.check_output(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", command],
        text=True, creationflags=subprocess.CREATE_NO_WINDOW,
    ).strip()


def processes():
    value = powershell(
        "@(Get-CimInstance Win32_Process | Where-Object { $_.Name -match "
        "'^(origins|cargo|rustc|nsys|ncu|qrenderdoc|renderdoccmd|tracy-capture|WinPixGpuCapturer|obs64|ffmpeg).*' } "
        "| Select-Object Name,ProcessId,ExecutablePath,CommandLine,KernelModeTime,UserModeTime) | ConvertTo-Json -Compress"
    )
    result = json.loads(value) if value else []
    return [result] if isinstance(result, dict) else result


def wait_for_slot():
    while active := processes():
        print(f"Waiting for shared game/capture/compiler slot: {active}", flush=True)
        time.sleep(10)


def gpu():
    return subprocess.check_output(
        ["nvidia-smi", "--query-gpu=name,driver_version,utilization.gpu,utilization.memory,"
         "memory.used,power.draw,temperature.gpu,clocks.gr,clocks.mem", "--format=csv,noheader,nounits"],
        text=True, creationflags=subprocess.CREATE_NO_WINDOW,
    ).strip()


def cpu_snapshot():
    return json.loads(powershell(
        "Get-Process | Select-Object Id,ProcessName,CPU | ConvertTo-Json -Compress"
    ))


def run(executable, output, workload):
    output.mkdir()
    wait_for_slot()
    cpu_before = {row["Id"]: row for row in cpu_snapshot()}
    started = time.monotonic()
    environment = {**os.environ, "RUST_LOG": "info,wgpu=error,naga=warn", "NO_COLOR": "1"}
    samples = []
    interference = []
    with (output / "game.log").open("w", encoding="utf-8") as log:
        process = subprocess.Popen(
            [str(executable), f"--release-benchmark-{workload}", str(output)],
            cwd=executable.parent, stdout=log, stderr=subprocess.STDOUT, env=environment,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        try:
            while process.poll() is None:
                current = processes()
                active = [item for item in current if item["ProcessId"] != process.pid]
                interference.extend(active)
                samples.append({"seconds": time.monotonic() - started, "gpu": gpu(),
                                "game": next((item for item in current if item["ProcessId"] == process.pid), None)})
                require(time.monotonic() - started < 960, "Benchmark timed out")
                time.sleep(2)
            require(process.returncode == 0, f"Game exited {process.returncode}")
        finally:
            if process.poll() is None:
                powershell(f"(Get-Process -Id {process.pid} -ErrorAction SilentlyContinue).CloseMainWindow()")
                process.wait(timeout=30)
            background = []
            for row in cpu_snapshot():
                previous = cpu_before.get(row["Id"])
                if previous and row["CPU"] is not None and previous["CPU"] is not None:
                    background.append({"name": row["ProcessName"], "pid": row["Id"],
                                       "cpu_seconds": row["CPU"] - previous["CPU"]})
            background.sort(key=lambda row: -row["cpu_seconds"])
            (output / "load.json").write_text(json.dumps({
                "pid": process.pid, "exit_code": process.returncode,
                "elapsed_seconds": time.monotonic() - started,
                "interference": interference, "samples": samples, "background_cpu": background[:20],
            }, indent=2) + "\n", encoding="utf-8")
    require(not interference, f"Concurrent game, capture, or compiler invalidated {output}")
    result = json.loads((output / "performance.json").read_text(encoding="utf-8"))
    require(result["passed"] and result["build_profile"] == "debug", "Expected passing development workload")
    require(result["physical_resolution"] == [1920, 1080], "Resolution changed")
    require(not result["profile_tracy"] and not result["profile_chrome"], "Tracing invalidates acceptance timing")
    require(result["sample_wall_seconds"] >= 30, "Incomplete workload")
    require(all(result["run_environment"].values()), "Unattended window/sleep protection failed")
    if workload == "combat":
        require(result["living_min_max"] == [6, 6], "Combat cohort changed")
    print(f"{output.name}: {result['summary']['fps']:.3f} FPS, p95 {result['summary']['p95_ms']:.3f} ms", flush=True)
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=1, choices=range(1, 10))
    parser.add_argument("--workload", choices=("combat", "moving"), default="combat")
    parser.add_argument("--target-fps", type=float)
    args = parser.parse_args()
    bundle = args.bundle.resolve()
    output = args.output.resolve()
    root = Path(__file__).resolve().parent.parent
    require(not output.is_relative_to(root), "Benchmark evidence must be external")
    identity = json.loads((bundle / "identity.json").read_text(encoding="utf-8"))
    require(identity["source"]["profile"] == "dev" and identity["source"]["features"] == ["dev-dynamic"],
            "Expected pinned dev-dynamic build")
    for name, expected in identity["binaries"].items():
        require(digest(bundle / name) == expected, f"Pinned binary changed: {name}")
    output.mkdir(parents=True, exist_ok=False)
    receipt = {"bundle": str(bundle), "identity_sha256": digest(bundle / "identity.json"),
               "assets_before": assets_match(identity), "runs": []}
    try:
        for index in range(1, args.repeats + 1):
            receipt["runs"].append(run(bundle / "origins.exe", output / f"{args.workload}-{index}", args.workload))
        fps = [row["summary"]["fps"] for row in receipt["runs"]]
        receipt["summary"] = {
            "median_fps": statistics.median(fps), "min_fps": min(fps), "max_fps": max(fps),
            "aggregate_fps": sum(row["summary"]["samples"] for row in receipt["runs"]) /
                             sum(row["summary"]["sample_elapsed_seconds"] for row in receipt["runs"]),
            "median_p95_ms": statistics.median(row["summary"]["p95_ms"] for row in receipt["runs"]),
        }
        receipt["assets_after"] = assets_match(identity)
        if args.target_fps is not None:
            receipt["target_fps"] = args.target_fps
            receipt["target_passed"] = min(fps) >= args.target_fps
            require(receipt["target_passed"], "At least one run missed the requested average FPS")
        receipt["passed"] = True
        print(json.dumps(receipt["summary"]), flush=True)
    finally:
        (output / "summary.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
