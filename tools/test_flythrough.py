import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.error

from test_support import Client, write_report
from test_brp import assets_match, digest, require


def tour_status(client):
    return client.call("world.get_resources", {"resource": "origins::flythrough::TourStatus"})["value"]


def wait_status(client, predicate, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        status = tour_status(client)
        if predicate(status):
            return status
        time.sleep(0.05)
    raise TimeoutError(f"Flythrough status condition timed out: {status}")


def press(client, key):
    client.release()
    client.call("origins/input", {"events": [{"tick": 1, "key": key, "pressed": True},
                                             {"tick": 20, "key": key, "pressed": False}]})


def wait_for_slot():
    command = "@(Get-Process -Name origins,origins_flythrough,cargo,rustc,capture,nsys,ncu,qrenderdoc,renderdoccmd,tracy-capture,WinPixGpuCapturer,obs64,ffmpeg -ErrorAction SilentlyContinue | Select-Object ProcessName,Id) | ConvertTo-Json -Compress"
    while True:
        active = subprocess.check_output(["powershell", "-NoProfile", "-NonInteractive", "-Command", command],
                                         text=True, creationflags=subprocess.CREATE_NO_WINDOW).strip()
        if not active or active == "[]":
            return
        print(f"Waiting for shared game/capture/compiler slot: {active}", flush=True)
        time.sleep(10)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--speed", type=float, default=4)
    parser.add_argument("--port", type=int, default=15708)
    args = parser.parse_args()
    bundle = args.bundle.resolve()
    output = args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parent.parent), "Evidence must be external")
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads((bundle / "identity.json").read_text(encoding="utf-8"))
    require("origins.exe" in identity["binaries"], "Bundle does not contain the pinned game executable")
    for name, expected in identity["binaries"].items():
        require(digest(bundle / name) == expected, f"Pinned binary changed: {name}")
    report = {"passed": False, "identity_sha256": digest(bundle / "identity.json"),
              "assets_before": assets_match(identity), "statuses": [], "captures": []}
    client = Client(f"http://127.0.0.1:{args.port}")
    wait_for_slot()
    with (output / "game.log").open("w", encoding="utf-8") as log:
        process = subprocess.Popen([str(bundle / "origins.exe"), "--flythrough", "--output", str(output),
                                    "--speed", str(args.speed), "--validation-remote", str(args.port)],
                                   cwd=bundle, stdout=log, stderr=subprocess.STDOUT,
                                   env={**os.environ, "RUST_LOG": "info,wgpu=error,naga=warn", "NO_COLOR": "1"},
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        report["pid"] = process.pid
        try:
            client.until(lambda state: state["state"] == "Playing", timeout=180)
            discovery = client.call("rpc.discover")
            report["discovery"] = discovery
            methods = {method["name"] for method in discovery["methods"]}
            require({"world.get_resources", "origins/capture", "origins/input"} <= methods, "Required BRP methods missing")
            schemas = client.call("registry.schema", {"with_crates": ["origins"]})
            require("origins::flythrough::TourStatus" in schemas, "Tour status is not reflected")
            deadline = time.monotonic() + 600
            captured = set()
            previous_label = None
            controls_checked = False
            while time.monotonic() < deadline:
                require(process.poll() is None, f"Game exited early: {process.returncode}")
                status = tour_status(client)
                report["statuses"].append(status)
                if status['label'] != previous_label:
                    print(f"{status['phase']}: {status['label']} ({status['elapsed_seconds']:.1f}s)", flush=True)
                    previous_label = status['label']
                require(status["phase"] != "failed", status["label"])
                if status["phase"] == "running" and status["elapsed_seconds"] >= 2 and not controls_checked:
                    press(client, "Space")
                    paused = wait_status(client, lambda row: row["paused"])
                    client.wait_ticks(30)
                    frozen = tour_status(client)
                    require(frozen["elapsed_seconds"] == paused["elapsed_seconds"], "Camera clock advanced while paused")
                    press(client, "Space")
                    wait_status(client, lambda row: not row["paused"])
                    client.wait_ticks(30)
                    press(client, "KeyR")
                    restarted = wait_status(client, lambda row: row["shot"] == 0 and row["elapsed_seconds"] < 1)
                    client.wait_ticks(30)
                    report["controls"] = {"paused": paused, "frozen": frozen, "restarted": restarted}
                    controls_checked = True
                    continue
                label = status["label"]
                if status["phase"] == "running" and status["shot_elapsed_seconds"] >= 0.5 and not label.startswith(("Flying", "Transition")) and label not in captured:
                    path = output / f"shot-{status['shot']:02}.png"
                    client.call("origins/capture", {"path": str(path)})
                    captured.add(label)
                    report["captures"].append(str(path))
                if status["phase"] == "finished":
                    break
                time.sleep(0.4)
            else:
                raise TimeoutError("Flythrough did not finish")
            benchmark = json.loads((output / "flythrough.json").read_text(encoding="utf-8"))
            report["benchmark"] = {key: value for key, value in benchmark.items()
                                   if key not in ("frame_times_ms", "route", "camera_trace", "resource_samples")}
            require(benchmark["completed"] and benchmark["frames"] > 100, "Incomplete frame samples")
            require(benchmark["average_cpu_percent"] is not None, "CPU telemetry unavailable")
            require(benchmark["average_working_set_mib"] > 0, "RAM telemetry unavailable")
            require(benchmark["average_gpu_busiest_engine_percent"] is not None, "GPU telemetry unavailable")
            labels = [shot["label"] for shot in benchmark["route"]]
            require(all(any(name in label for label in labels) for name in
                        ("Spawn", "Quick Revive", "Juggernog", "Speed Cola", "Stamin-Up", "Mule Kick",
                         "Lantern", "Church", "Excavation", "Crazy Place")), "Route coverage missing")
            require(benchmark["camera_trace"][-1]["shot"] >= len(labels) - 2, "Camera did not traverse the route")
            require(not status["report_error"], status["report_error"])
            require(controls_checked, "Pause/resume/restart controls were not exercised")
            require(all(Path(path).is_file() for path in report["captures"]), "Screenshot was not written")
            report["assets_after"] = assets_match(identity)
            report["passed"] = True
        except Exception as error:
            report["error"] = f"{type(error).__name__}: {error}"
        finally:
            client.shutdown(report)
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                report["passed"] = False
                report["process_exit_error"] = "Owned flythrough did not exit after BRP shutdown"
            report["exit_code"] = process.poll()
            write_report(output, "verification.json", report)
    require(report["passed"], report.get("error", "Flythrough verification failed"))
    print(json.dumps({key: value for key, value in report["benchmark"].items()
                      if not key.startswith("bundle_")}, indent=2))


if __name__ == "__main__":
    main()
