import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time
import urllib.error

from benchmark_development import wait_for_slot
from gameplay_remote import Client, write_report
from verify_brp import assets_match, digest, require


def settled(client, timeout=120):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = client.call("origins/materials")
        require(not result["pipelines"]["error"], "Render pipeline failed")
        if result["pending"] == 0 and result["pipelines"]["pending"] == 0:
            consistency = result["pipelines"]["mesh_consistency"]
            require(not consistency.get("mismatches"), "Mesh allocator offsets disagree")
            require(result["preprocessing"]["active_mode"] == "gpu_culling", "GPU culling disabled")
            return result
        time.sleep(0.1)
    raise TimeoutError("Render pipelines did not settle")


def inputs(client, key, ticks):
    batch = client.call("origins/input", {"events": [
        {"tick": 1, "key": key, "pressed": True},
        {"tick": ticks + 1, "key": key, "pressed": False},
    ]})
    return client.until(lambda s: s["fixed_tick"] >= batch["finish_tick"] + 3
                        and not s["held_inputs"] and not s["queued_inputs"])


def capture(client, path):
    client.call("origins/capture", {"path": str(path)})
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if path.exists() and path.stat().st_size > 0:
            return str(path)
        time.sleep(0.1)
    raise TimeoutError(f"Screenshot was not written: {path}")


def gpu_cases(client, report):
    capabilities = client.call("origins/rendering/indirect")
    require(capabilities["counted"], "Counted indirect draws are disabled")
    report["indirect_gpu"] = {"capabilities": capabilities, "cases": []}
    for indexed in (False, True):
        cases = [
            ("visible", 1, 1, 0, False, False, 0, True),
            ("zero_after_visible", 1, 0, 0, False, False, 0, False),
            ("count_clamped", 2, 0xffffffff, 1, False, False, 0, True),
            ("invalid_arguments", 1, 1, 0, True, False, 0, False),
            ("direct_after_indirect", 1, 0, 0, False, True, 0, True),
            ("zero_capacity", 0, 0xffffffff, None, False, False, 0, False),
            ("split_excluded", 70000, 69000, 69000, False, False, 12, False),
            ("split_included", 70000, 69001, 69000, False, False, 12, True),
            ("split_zero_after_visible", 70000, 0, 69000, False, False, 12, False),
            ("split_clamped", 70000, 0xffffffff, 69000, False, False, 12, True),
        ]
        window = capabilities["source_window_bytes"]
        stride = 20 if indexed else 16
        offset = window // 256 * 256 - 512
        if offset + 128 * stride <= capabilities["max_buffer_size"] and window < capabilities["max_buffer_size"]:
            cases += [
                ("source_split_excluded", 128, 100, 100, False, False, offset, False),
                ("source_split_included", 128, 101, 100, False, False, offset, True),
                ("source_split_zero", 128, 0, 100, False, False, offset, False),
            ]
        else:
            report["indirect_gpu"]["source_window_limit"] = "The requested source-window crossing exceeds this device maximum buffer size."
        for name, capacity, count, live, invalid, direct, offset, visible in cases:
            request = {"indexed": indexed, "capacity": capacity, "gpu_count": count,
                       "live_command": live, "invalid": invalid, "direct_after": direct,
                       "argument_offset": offset}
            result = client.call("origins/rendering/indirect", request)
            expected = [0, 255, 0, 255] if visible else [0, 0, 0, 255]
            report["indirect_gpu"]["cases"].append({"name": name, "request": request,
                                                   "result": result, "expected_pixel": expected})
            require(result["pixel"] == expected, f"GPU indirect case failed: {indexed=} {name}: {result}")

    for padding in (4, 1024 * 1024 - 4, 1024 * 1024, 1024 * 1024 + 4):
        request = {"indexed": True, "capacity": 4, "gpu_count": 1,
                   "live_command": 0, "upload_padding": padding}
        result = client.call("origins/rendering/indirect", request)
        report["indirect_gpu"]["cases"].append({"name": "upload_boundary",
            "request": request, "result": result, "expected_pixel": [0, 255, 0, 255]})
        require(result["pixel"] == [0, 255, 0, 255], f"GPU upload boundary failed: {padding}")


def run(client, report, output, process, startup_only=False, require_bindless=False, water=False):
    deadline = time.monotonic() + 360
    while time.monotonic() < deadline:
        require(process.poll() is None, f"Game exited during startup: {process.returncode}")
        try:
            state = client.state()
            if state["state"] == "Playing" and state.get("player"):
                report["ready"] = state
                break
        except (urllib.error.URLError, TimeoutError, ConnectionError):
            pass
        time.sleep(0.1)
    require("ready" in report, "Game did not become ready")
    report["discovery"] = client.call("rpc.discover")
    report["schema"] = client.call("origins/schema")
    registry = client.call("registry.schema")
    report["render_before"] = settled(client)
    report["screenshots"] = [capture(client, output / "initial.png")]
    methods = {method["name"] for method in report["discovery"]["methods"]}
    if "origins/rendering/indirect" in methods:
        gpu_cases(client, report)
    if require_bindless:
        require(report.get("indirect_gpu", {}).get("capabilities", {}).get("opaque_bindless"),
                "Opaque materials fell back from bindless rendering")
    if startup_only:
        return
    client.reset()
    client.call("origins/setup", {"clear_enemies": True, "pause_spawning_seconds": 120})
    before = client.wait_ticks(12)
    key = next(binding["key"] for binding in before["controls"]["bindings"]["forward"]
               if isinstance(binding, dict) and "key" in binding)
    after = inputs(client, key, 24)
    distance = math.dist(before["player"]["position"], after["player"]["position"])
    require(distance > 0.15, "Normal movement input did not move player")
    report["movement"] = {"before": before, "after": after, "distance": distance}
    client.reset()
    before = client.wait_ticks(12)
    mouse_type = next((name for name in registry if name.endswith("::MouseMotion")), None)
    require(mouse_type is not None, "MouseMotion is not reflected")
    report["mouse_schema"] = registry[mouse_type]
    client.call("world.write_message", {"message": mouse_type, "value": {"delta": [240.0, 20.0]}})
    after = client.until(lambda s: abs(s["player"]["yaw"] - before["player"]["yaw"]) > 0.01)
    report["camera"] = {"before": before["player"], "after": after["player"]}
    for index in range(4):
        client.call("world.write_message", {"message": mouse_type, "value": {"delta": [400.0, 0.0]}})
        client.wait_ticks(12)
        settled(client)
        report["screenshots"].append(capture(client, output / f"rotation-{index}.png"))
    client.reset()
    before = client.wait_ticks(12)
    after = inputs(client, "MouseLeft", 12)
    require(after["session"]["shots"] > before["session"]["shots"], "Fire input produced no shot")
    report["firing"] = {"before": before, "after": after}
    report["render_after"] = settled(client)
    shadows = report["render_after"]["shadows"]
    require(any(light["shadow_maps"] for light in shadows["lights"]), "No active shadow maps")
    if water:
        report["water_view"] = client.call("origins/rendering", {
            "view": [-38.7, 5.5, -13.0, -38.7, 0.0, -21.0]})
        settled(client)
        report["screenshots"].append(capture(client, output / "water-a.png"))
        client.wait_ticks(90)
        report["screenshots"].append(capture(client, output / "water-b.png"))
        client.call("origins/rendering", {"restore_player_view": True})
    report["coverage_limits"] = [
        "Screenshots require visual review; pipeline success alone does not prove geometry.",
        "The offscreen GPU integration cases are separate from the native-scene gameplay checks.",
    ]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--backend", choices=("dx12", "vulkan"), required=True)
    parser.add_argument("--port", type=int, default=15702)
    parser.add_argument("--startup-only", action="store_true")
    parser.add_argument("--require-bindless", action="store_true")
    parser.add_argument("--water", action="store_true")
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parent.parent), "Output must be external")
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads((bundle / "identity.json").read_text())
    report = {"passed": False, "identity_sha256": digest(bundle / "identity.json"),
              "source": identity["source"], "backend": args.backend, "startup_only": args.startup_only}
    for name, expected in identity["binaries"].items():
        require(digest(bundle / name) == expected, f"Binary changed: {name}")
    report["assets_before"] = assets_match(identity)
    client = Client(f"http://127.0.0.1:{args.port}", request_timeout=120, shutdown_timeout=60)
    wait_for_slot()
    with (output / "game.log").open("w", encoding="utf-8") as log:
        process = subprocess.Popen([str(bundle / "origins.exe"), "--validation-remote", str(args.port)],
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT,
            env={**os.environ, "WGPU_BACKEND": args.backend, "RUST_LOG": "info,wgpu=warn,naga=warn"},
            creationflags=subprocess.CREATE_NO_WINDOW)
        report["pid"] = process.pid
        try:
            run(client, report, output, process, args.startup_only, args.require_bindless, args.water)
            report["assets_after"] = assets_match(identity)
            report["passed"] = True
        except Exception as error:
            report["error"] = f"{type(error).__name__}: {error}"
        finally:
            client.shutdown(report)
            if process.poll() is None and not report.get("shutdown", {}).get("endpoint_closed"):
                subprocess.run(["powershell", "-NoProfile", "-NonInteractive", "-Command",
                    f"(Get-Process -Id {process.pid} -ErrorAction SilentlyContinue).CloseMainWindow()"],
                    creationflags=subprocess.CREATE_NO_WINDOW, check=False)
            try:
                report["exit_code"] = process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                report["passed"] = False
                report["close_error"] = "Owned game did not exit after normal close request"
            path = write_report(output, "rendering.json", report)
            print(json.dumps({"passed": report["passed"], "report": str(path), "error": report.get("error")}))
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
