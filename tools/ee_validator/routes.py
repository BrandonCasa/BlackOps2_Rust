"""Watchable route functions. Every step inspects the actual game state."""
import math
import struct
import time
from client import ValidationFailure, changed


def distance(a, b):
    return math.hypot(a[0] - b[0], a[2] - b[2])


def capture(client, name):
    receipt = client.send({"op": "capture", "name": name}, label=f"Capture: {name}")
    path = client.report / f"{name}.png"
    deadline = time.monotonic() + 10
    while not path.exists() and time.monotonic() < deadline:
        time.sleep(0.1)
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n" or len(data) < 64:
        raise ValidationFailure(f"Capture is not a PNG: {path}")
    width, height = struct.unpack(">II", data[16:24])
    if (width, height) != (receipt["after"]["width"], receipt["after"]["height"]):
        raise ValidationFailure("Capture dimensions disagree with game receipt")
    return {"path": str(path), "width": width, "height": height}


def walk_to(client, point, tolerance=0.8, seconds=30):
    deadline = time.monotonic() + seconds
    previous = None
    while time.monotonic() < deadline:
        state = client.state()
        feet = state["player"]["feet"]
        remaining = distance(feet, point)
        if remaining <= tolerance:
            client.input(1)
            return
        target = [point[0], state["player"]["eye"][1], point[2]]
        client.input(60, aim=target, label="Turn toward walking waypoint")
        client.input(min(30, max(1, int(remaining / 4 * 60))), ("forward",), aim=target, label="Walk to waypoint")
        if previous is not None and abs(previous - remaining) < 0.01:
            raise ValidationFailure(f"Walking route is blocked near {feet}; target {point}")
        previous = remaining
    raise ValidationFailure(f"Walking route deadline: {point}")


def foundation(client):
    results = {}
    client.send({"op": "assist", "god": True, "points": 100000, "ammo": True, "doors": False}, label="Foundation: assistance enabled")
    client.input(120, label="Watch match settle")
    state = client.state()
    shovels = [t for t in state["targets"] if t["target"]["kind"] == "shovel"]
    if not shovels or state["player"]["shovel"]:
        raise ValidationFailure("Foundation requires an unowned, ordinary shovel")
    shovel = min(shovels, key=lambda t: distance(t["position"], state["player"]["feet"]))
    target = shovel["position"]
    feet = state["player"]["feet"]
    delta = [feet[0] - target[0], 0, feet[2] - target[2]]
    length = math.hypot(delta[0], delta[2]) or 1
    approach = [target[0] + delta[0] / length * 0.65, feet[1], target[2] + delta[2] / length * 0.65]
    walk_to(client, approach)
    aim = [target[0], target[1] + 0.8128, target[2]]
    client.input(120, aim=aim, label="Foundation: inspect shovel")
    state = client.state()
    if state["interaction"]["target"] != shovel["entity"]:
        raise ValidationFailure("The shovel is not the focused interaction")
    accepted = state["interaction"]["accepted"]
    client.input(30, ("interact",), aim=aim, expect=changed("/player/shovel", False, True), label="Foundation: collect shovel through interaction")
    state = client.state()
    if state["interaction"]["accepted"] <= accepted:
        raise ValidationFailure("Shovel ownership changed without an accepted interaction")
    client.input(1)
    results["interaction"] = capture(client, "foundation-shovel")
    state = client.wait(lambda s: any(e["emerged"] and e["life"] == "ActorAlive" for e in s["enemies"]), seconds=60)
    target = min((e for e in state["enemies"] if e["emerged"] and e["life"] == "ActorAlive"), key=lambda e: distance(e["position"], state["player"]["feet"]))
    enemy = target["entity"]
    before_kills = state["evidence"]["kills"]
    before_sequence = state["evidence"]["sequence"]
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        client.input(30, aim_entity=enemy, label="Foundation: aim at a native zombie")
        client.input(10, ("fire",), aim_entity=enemy, label="Foundation: fire equipped weapon")
        client.input(10, aim_entity=enemy)
        state = client.state()
        events = [e for e in state["evidence"]["events"] if e["sequence"] > before_sequence]
        if any(e["kind"] == "kill" and e["data"]["target"] == enemy for e in events):
            if not any(e["kind"] == "hit" and e["data"]["target"] == enemy for e in events):
                raise ValidationFailure("A kill without a correlated gameplay hit is not proof")
            break
        client.send({"op": "assist", "god": True, "points": state["round"]["points"], "ammo": True, "doors": False})
    else:
        raise ValidationFailure(f"Could not kill native actor {enemy} through weapon fire")
    if state["evidence"]["kills"] <= before_kills:
        raise ValidationFailure("Combat kill counter did not advance")
    results["combat"] = capture(client, "foundation-combat")

    client.send({"op": "pause"}, label="Foundation: pause")
    paused = client.wait(lambda s: s["game_state"] == "Paused")
    tick = paused["control"]["tick"]
    time.sleep(0.3)
    if client.state()["control"]["tick"] != tick:
        raise ValidationFailure("Simulation advanced while paused")
    client.send({"op": "step"}, label="Foundation: one ordinary simulation tick")
    stepped = client.wait(lambda s: s["game_state"] == "Paused")
    if stepped["control"]["tick"] != tick + 1:
        raise ValidationFailure("Step did not advance exactly one fixed tick")
    if not math.isclose(stepped["physics_seconds"] - paused["physics_seconds"], 1 / 60, abs_tol=1e-7):
        raise ValidationFailure("Step did not include exactly one timestep of physics")
    client.send({"op": "resume"}, label="Foundation: resume")
    client.wait(lambda s: s["game_state"] == "Playing")
    pending = client.send({"op": "input", "ticks": 1200, "held": ["fire"], "look": [0, 0], "aim": None, "aim_entity": None}, wait=False)
    client.send({"op": "cancel", "command": pending["request"]["id"]}, label="Foundation: cancel held fire")
    state = client.state()
    cancelled = next(r for r in state["receipts"] if r["request"]["id"] == pending["request"]["id"] and r["request"]["session"] == client.session)
    if cancelled["phase"] != "cancelled":
        raise ValidationFailure("Cancellation did not produce a cancelled receipt")

    client.send({"op": "input", "ticks": 1200, "held": ["fire"], "look": [0, 0], "aim": None, "aim_entity": None}, wait=False, label="Foundation: deliberately terminate Python heartbeat")
    client.beat_stop()
    failed = client.wait(lambda s: s["control"]["failure"] == "Runner heartbeat expired" and s["game_state"] == "Paused", seconds=15, allow_failure=True)
    if failed["control"]["active"] is not None or failed["control"]["queued"]:
        raise ValidationFailure("Watchdog left queued controls active")
    if not (client.report / "game-failure.json").is_file():
        raise ValidationFailure("Watchdog did not save its own failure state")
    tick = failed["control"]["tick"]
    time.sleep(0.3)
    if client.state()["control"]["tick"] != tick:
        raise ValidationFailure("Watchdog did not freeze gameplay")
    client.beat_start()
    client.send({"op": "resume"}, allow_failure=True, label="Foundation: explicit watchdog recovery")
    client.wait(lambda s: s["game_state"] == "Playing")
    client.input(180, label="Foundation passed: inspect scene")
    results["watchdog"] = "Python heartbeat process terminated; game independently cancelled, recorded, and paused"
    results["final_state"] = client.state()
    return results


ROUTE = {"foundation": foundation}
