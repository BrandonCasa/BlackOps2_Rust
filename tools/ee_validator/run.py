"""Launch a visible Origins validator run. Failures retain the paused game."""
import argparse
import datetime
import json
import os
from pathlib import Path
import subprocess
import time
import socket
import csv

from client import Client, ValidationFailure
from routes import ROUTE

REPO = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--until", choices=tuple(ROUTE), default="foundation")
    parser.add_argument("--seed", type=lambda s: int(s, 0), default=0xB02)
    parser.add_argument("--port", type=int, default=15702)
    parser.add_argument("--exe", type=Path, default=REPO / "target/debug/origins.exe")
    parser.add_argument("--assets", type=Path, default=REPO / "target/assets")
    parser.add_argument("--report", type=Path)
    parser.add_argument("--build", action="store_true")
    parser.add_argument("--watchdog-probe", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.watchdog_probe:
        client = Client(args.port)
        client.session = client.state()["session"]
        client.beat_start()
        client.input(1800, ("forward", "fire"), label="Foundation: terminate this controlling runner")
        return 0
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    report = (args.report or REPO / "reports/ee-validator" / stamp).resolve()
    report.mkdir(parents=True, exist_ok=False)
    exe, assets = args.exe.resolve(), args.assets.resolve()
    client = Client(args.port, report)
    result = {"status": "running", "requested_milestone": args.until, "seed": args.seed,
              "proves": "Only the assisted gameplay steps completed in this run",
              "does_not_prove": ["normal spawning", "survival balance", "economy", "unscripted timing", "performance"]}
    process = None
    output = None
    try:
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", args.port))
        if os.name == "nt":
            rows = csv.reader(subprocess.check_output(["tasklist", "/FI", "IMAGENAME eq origins.exe", "/FO", "CSV", "/NH"], text=True).splitlines())
            if any(row and row[0].lower() == "origins.exe" for row in rows):
                raise ValidationFailure("An Origins process already owns the game/capture slot")
        if args.build:
            with (report / "build.log").open("wb") as log:
                subprocess.run(["cargo", "build", "--features", "dev-tools", "--locked"], cwd=REPO, stdout=log, stderr=subprocess.STDOUT, check=True)
        env = os.environ.copy()
        env.update(ORIGINS_VALIDATOR_REPORT=str(report), ORIGINS_VALIDATOR_SEED=str(args.seed), ORIGINS_VALIDATOR_PORT=str(args.port),
                   DISABLE_VULKAN_OW_OBS_CAPTURE="1", DISABLE_VK_LAYER_reshade_1="1")
        output = (report / "game.log").open("wb")
        process = subprocess.Popen([str(exe), "--asset-root", str(assets)], cwd=REPO, env=env, stdout=output, stderr=subprocess.STDOUT)
        (report / "process.json").write_text(json.dumps({"pid": process.pid, "exe": str(exe), "port": args.port}), encoding="utf-8")
        client.beat_start()
        deadline = time.monotonic() + 600
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise ValidationFailure(f"Game exited during startup: {process.returncode}; inspect {report / 'game.log'}")
            try:
                state = client.state()
                if state.get("process_id") != process.pid:
                    raise ValidationFailure("BRP endpoint belongs to a different game process")
                break
            except OSError:
                pass
            time.sleep(0.25)
        else:
            raise ValidationFailure("Game connection deadline exceeded")
        client.session = state["session"]
        state = client.wait(lambda s: s["ready"] and s["game_state"] == "Playing", seconds=600, realtime=True)
        client.session = state["session"]
        result["increments"] = {}
        for name, route in ROUTE.items():
            print(f"Running {name}; report: {report}", flush=True)
            result["increments"][name] = route(client)
            if name == args.until:
                break
        client.send({"op": "finish"}, label=f"Milestone {args.until} complete: inspect scene")
        result["status"] = "passed"
    except BaseException as error:
        result["status"] = "failed"
        result["error"] = f"{type(error).__name__}: {error}"
        try:
            if process is None or process.poll() is not None:
                raise ValidationFailure("No owned responsive game process to pause")
            client.session = client.state()["session"]
            client.send({"op": "fail", "reason": result["error"]}, wait=False, allow_failure=True)
            result["failure_state"] = client.wait(lambda s: s["control"]["paused"], seconds=5, allow_failure=True, realtime=True)
        except Exception as recovery:
            result["recovery_error"] = str(recovery)
    finally:
        client.flush()
        client.beat_stop()
        if output:
            output.close()
        (report / "summary.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(json.dumps({"status": result["status"], "report": str(report), "error": result.get("error")}), flush=True)
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
