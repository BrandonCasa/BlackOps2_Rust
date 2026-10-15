import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time
import urllib.error

from test_support import Client, capture, wait_for_slot, write_report
from test_brp import require


def settled(client, timeout=120):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = client.call("origins/materials")
        require(not result["pipelines"]["error"], "Render pipeline failed")
        if result["pending"] == 0 and result["pipelines"]["pending"] == 0:
            consistency = result["pipelines"]["mesh_consistency"] or {}
            require(not consistency.get("mismatches"), "Mesh allocator offsets disagree")
            require(consistency.get("mismatch_frames", 0) == 0,
                    "Mesh allocator or skin offsets disagreed in an earlier frame")
            require(not consistency.get("first_mismatches"),
                    "Mesh allocator or skin offset mismatch history is nonempty")
            require(result["preprocessing"]["active_mode"] == "gpu_culling", "GPU culling disabled")
            if consistency.get("frames", 0) > 0 and consistency.get("checked", 0) > 0:
                return result
        time.sleep(0.1)
    raise TimeoutError("Render pipelines and skinned-mesh checks did not settle")


def inputs(client, key, ticks):
    batch = client.call("origins/input", {"events": [
        {"tick": 1, "key": key, "pressed": True},
        {"tick": ticks + 1, "key": key, "pressed": False},
    ]})
    return client.until(lambda s: s["fixed_tick"] >= batch["finish_tick"] + 3
                        and not s["held_inputs"] and not s["queued_inputs"])


def bound_key(state, action):
    for binding in state["controls"]["bindings"][action]:
        if isinstance(binding, dict) and "key" in binding:
            return binding["key"]
        if isinstance(binding, dict) and binding.get("mouse") in ("Left", "Right", "Middle"):
            return "Mouse" + binding["mouse"]
    raise AssertionError(f"No supported keyboard or mouse binding for {action}")


def weapon_views(client, report, output):
    client.reset()
    client.call("origins/setup", {"weapons": ["weapon.mp40_zm"], "clear_enemies": True,
                                  "pending": 0, "pause_spawning_seconds": 120})
    ready = client.until(lambda s: s.get("player") and s["player"]["action"] == "Ready"
                         and s["player"]["weapons"][0]["id"] == "weapon.mp40_zm")
    settled(client)
    case = {"weapon": "weapon.mp40_zm", "captures": {}, "ready": ready}
    report["weapon_views"] = case
    case["captures"]["hip"] = capture(client, output / "weapon-hip.png")["path"]
    aim = bound_key(ready, "aim")
    fire = bound_key(ready, "fire")
    reload_key = bound_key(ready, "reload")
    try:
        client.call("origins/input", {"events": [
            {"tick": 1, "key": aim, "pressed": True},
            {"tick": 3600, "key": aim, "pressed": False},
        ]})
        case["ads"] = client.until(lambda s: s["player"]["ads"] >= 0.99, timeout=10)
        case["captures"]["ads"] = capture(client, output / "weapon-ads.png")["path"]
        require(client.state()["player"]["ads"] >= 0.99, "ADS ended before capture completed")
    finally:
        client.release()
    if ready["controls"]["aim_mode"] == "toggle":
        inputs(client, aim, 1)
    before = client.until(lambda s: s["player"]["ads"] <= 0.01
                          and s["player"]["action"] == "Ready"
                          and not s["held_inputs"] and not s["queued_inputs"])
    try:
        client.call("origins/input", {"events": [
            {"tick": 1, "key": fire, "pressed": True},
            {"tick": 3600, "key": fire, "pressed": False},
        ]})
        case["firing"] = client.until(lambda s: s["session"]["shots"] > before["session"]["shots"],
                                      timeout=10)
        case["captures"]["firing"] = capture(client, output / "weapon-firing.png")["path"]
        case["after_firing_capture"] = client.state()
    finally:
        client.release()
    before_reload = client.until(lambda s: s["player"]["action"] == "Ready"
                                 and not s["held_inputs"] and not s["queued_inputs"])
    inputs(client, reload_key, 1)
    case["reload"] = client.until(lambda s: s["player"]["action"].startswith("Reloading"),
                                  timeout=5)
    case["captures"]["reload"] = capture(client, output / "weapon-reload.png")["path"]
    case["after_reload_capture"] = client.state()
    case["reloaded"] = client.until(lambda s: s["player"]["action"] == "Ready"
                                    and s["player"]["weapons"][0]["magazine"]
                                    > before_reload["player"]["weapons"][0]["magazine"])
    case["render_after"] = settled(client)


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


def material_probe(client, report, output):
    rows = client.call("world.query", {"data": {"components": [
        "bevy_mesh::components::Mesh3d", "bevy_camera::primitives::Aabb",
        "bevy_ecs::name::Name"], "option": ["bevy_gltf::assets::GltfMaterialName"]}})
    report["mesh_inventory"] = rows
    candidates = [row for row in rows if "maps/mp/zm_tomb.d3dbsp/*0" in row["components"]["bevy_ecs::name::Name"]]
    state = client.state()
    player = state["player"]
    position = player["position"][:]
    position[1] += player["eye_height"] - player["height"] * .5
    direction = [-math.sin(player["yaw"]) * math.cos(player["pitch"]),
                 math.sin(player["pitch"]), -math.cos(player["yaw"]) * math.cos(player["pitch"])]
    def distance(row):
        aabb = row["components"]["bevy_camera::primitives::Aabb"]
        near, far = 0, 100
        for axis in range(3):
            if abs(direction[axis]) < .00001:
                if abs(position[axis] - aabb["center"][axis]) > aabb["half_extents"][axis]:
                    return math.inf
                continue
            first = (aabb["center"][axis] - aabb["half_extents"][axis] - position[axis]) / direction[axis]
            last = (aabb["center"][axis] + aabb["half_extents"][axis] - position[axis]) / direction[axis]
            near, far = max(near, min(first, last)), min(far, max(first, last))
        return near if far >= near > 1 and min(aabb["half_extents"]) < .2 else math.inf
    candidates.sort(key=distance)
    selected = [row for row in candidates if math.isfinite(distance(row))][:2]
    require(selected, "No world surfaces found for the material probe")
    report["material_probe"] = {}
    for label, no_culling in (("source-culling", False), ("no-culling", True)):
        try:
            report["material_probe"][label] = client.call("origins/material-probe", {
                "entities": [row["entity"] for row in selected], "no_culling": no_culling})
            time.sleep(3)
            report["material_probe"][label]["render"] = client.call("origins/materials")
            report["screenshots"].append(capture(client, output / f"material-probe-{label}.png")["path"])
        finally:
            report["material_probe_restore"] = client.call("origins/material-probe", {})



def run(client, report, output, process, startup_only=False, require_bindless=False, water=False, probe=False, settle_seconds=0, nsight=None, nsight_session=None, nsight_graphics=None):
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
    report["screenshots"] = [capture(client, output / "initial.png")["path"]]
    if nsight:
        result = subprocess.run([str(nsight), "start", "--session=" + nsight_session],
                                capture_output=True, text=True, timeout=30,
                                creationflags=subprocess.CREATE_NO_WINDOW)
        report["nsight_start"] = {"exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
        require(result.returncode == 0, "Nsight failed to start capture: " + result.stderr)
        print(json.dumps({"status": "nsight_capturing", "seconds": 15}), flush=True)
        time.sleep(18)
        report["screenshots"].append(capture(client, output / "nsight-capture.png")["path"])
    if nsight_graphics:
        print(json.dumps({"status": "waiting_for_nsight_graphics_capture"}), flush=True)
        deadline = time.monotonic() + 300
        previous = None
        unchanged_since = time.monotonic()
        while time.monotonic() < deadline:
            files = [*output.glob("*.ngfx-bincap"), *output.glob("*.ngfx-capture")]
            sizes = [(str(path), path.stat().st_size) for path in files]
            if sizes != previous:
                previous = sizes
                unchanged_since = time.monotonic()
            if sizes and all(size > 0 for _, size in sizes) and time.monotonic() - unchanged_since > 10:
                report["nsight_graphics_captures"] = sizes
                break
            require(process.poll() is None, "Nsight Graphics target exited before capture completed")
            time.sleep(1)
        require("nsight_graphics_captures" in report, "Nsight Graphics capture did not finish")
        report["screenshots"].append(capture(client, output / "nsight-graphics.png")["path"])
    if settle_seconds:
        report["settle_test"] = {"seconds": settle_seconds, "before": client.state()}
        started = time.monotonic()
        print(json.dumps({"status": "waiting_in_game", "seconds": settle_seconds}), flush=True)
        time.sleep(settle_seconds)
        report["settle_test"]["after"] = client.state()
        elapsed = time.monotonic() - started
        frames = report["settle_test"]["after"]["render_frame"] - report["settle_test"]["before"]["render_frame"]
        report["settle_test"].update(elapsed_seconds=elapsed, frames=frames, average_fps=frames / elapsed)
        memory_command = "$process=Get-Process -Id " + str(process.pid) + "; $process | Select-Object WorkingSet64,PrivateMemorySize64 | ConvertTo-Json -Compress"
        report["settle_test"]["process_memory"] = json.loads(subprocess.check_output(
            ["powershell", "-NoProfile", "-NonInteractive", "-Command", memory_command],
            text=True, creationflags=subprocess.CREATE_NO_WINDOW))
        report["settle_test"]["whole_device_gpu"] = subprocess.check_output(
            ["nvidia-smi", "--query-gpu=memory.used,utilization.gpu", "--format=csv"],
            text=True, creationflags=subprocess.CREATE_NO_WINDOW).strip()
        report["settle_test"]["materials_after"] = client.call("origins/materials")
        report["screenshots"].append(capture(client, output / "after-wait.png")["path"])
    if probe:
        material_probe(client, report, output)
    if startup_only:
        return
    methods = {method["name"] for method in report["discovery"]["methods"]}
    if "origins/rendering/indirect" in methods:
        gpu_cases(client, report)
    if require_bindless:
        require(report.get("indirect_gpu", {}).get("capabilities", {}).get("opaque_bindless"),
                "Opaque materials fell back from bindless rendering")
    client.reset()
    client.call("origins/setup", {"clear_enemies": True, "pause_spawning_seconds": 120})
    before = client.wait_ticks(12)
    key = bound_key(before, "forward")
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
        report["screenshots"].append(capture(client, output / f"rotation-{index}.png")["path"])
    client.reset()
    before = client.wait_ticks(12)
    after = inputs(client, bound_key(before, "fire"), 12)
    require(after["session"]["shots"] > before["session"]["shots"], "Fire input produced no shot")
    report["firing"] = {"before": before, "after": after}
    report["screenshots"].append(capture(client, output / "firing.png")["path"])
    weapon_views(client, report, output)
    report["render_after"] = settled(client)
    shadows = report["render_after"]["shadows"]
    require(len(shadows["cameras"]) == 1 and shadows["cameras"][0]["active"], "Expected one active game camera")
    if water:
        report["water_view"] = client.call("origins/rendering", {
            "view": [-38.7, 5.5, -13.0, -38.7, 0.0, -21.0]})
        settled(client)
        report["screenshots"].append(capture(client, output / "water-a.png")["path"])
        client.wait_ticks(90)
        report["screenshots"].append(capture(client, output / "water-b.png")["path"])
        client.call("origins/rendering", {"restore_player_view": True})
    report["coverage_limits"] = [
        "Screenshots require visual review; pipeline success alone does not prove geometry.",
        "The offscreen GPU integration cases are separate from the native-scene gameplay checks.",
        "The MP40 inventory and enemy suppression are setup; ADS, firing and reload use normal inputs.",
    ]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--backend", choices=("vulkan",), required=True)
    parser.add_argument("--port", type=int, default=15702)
    parser.add_argument("--startup-only", action="store_true")
    parser.add_argument("--require-bindless", action="store_true")
    parser.add_argument("--water", action="store_true")
    parser.add_argument("--material-probe", action="store_true")
    parser.add_argument("--settle-seconds", type=float, default=0)
    parser.add_argument("--disable-implicit-layers", action="store_true")
    parser.add_argument("--nsight", type=Path)
    parser.add_argument("--nsight-cpu-sampling", action="store_true")
    parser.add_argument("--nsight-graphics", type=Path)
    args = parser.parse_args()
    require(not (args.nsight and args.nsight_graphics), "Capture with one Nsight tool at a time")
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parent.parent), "Output must be external")
    output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'backend': args.backend, 'startup_only': args.startup_only}
    report["implicit_layers_disabled"] = args.disable_implicit_layers
    client = Client(f"http://127.0.0.1:{args.port}", request_timeout=120, shutdown_timeout=60)
    wait_for_slot()
    nsight_session = "origins-renderer-" + str(os.getpid())
    with (output / "game.log").open("w", encoding="utf-8") as log:
        environment = {**os.environ, "WGPU_BACKEND": args.backend, "RUST_LOG": "info,wgpu=warn,naga=warn"}
        if args.disable_implicit_layers:
            environment["VK_LOADER_LAYERS_DISABLE"] = "*ReShade*,*OW_OVERLAY*,*OW_OBS_HOOK*" if args.nsight or args.nsight_graphics else "~implicit~"
        command = [str(bundle / "origins.exe"), "--validation-remote", str(args.port)]
        if args.nsight:
            sampling = "process-tree" if args.nsight_cpu_sampling else "none"
            command = [str(args.nsight), "profile", "--trace=vulkan,vulkan-annotations",
                       "--sample=" + sampling, "--cpuctxsw=" + sampling, "--debug-symbols=.",
                       "--start-later=true", "--duration=15",
                       "--kill=false", "--wait=all", "--session-new=" + nsight_session,
                       "--output=" + str(output / "renderer"), *command]
            report["nsight_command"] = command
        elif args.nsight_graphics:
            command = [str(args.nsight_graphics), "--exe", str(bundle / "origins.exe"),
                       "--working-dir", str(bundle), "--output-dir", str(output),
                       "--output-file", "renderer", "--frame-count", "1", "--no-bundle-replayer",
                       "--non-portable", "--capture-countdown-timer", "120000", "--no-hud",
                       "--args", "--validation-remote", str(args.port)]
            report["nsight_command"] = command
        process = subprocess.Popen(command,
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT,
            env=environment,
            creationflags=subprocess.CREATE_NO_WINDOW)
        report["pid"] = process.pid
        try:
            run(client, report, output, process, args.startup_only, args.require_bindless, args.water, args.material_probe, args.settle_seconds, args.nsight, nsight_session, args.nsight_graphics)
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
