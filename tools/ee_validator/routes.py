"""Watchable route functions. Every step inspects the actual game state."""
import math
import json
import struct
import time
from client import ValidationFailure
import subprocess
import sys
from pathlib import Path


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


def walk_to(client, point, tolerance=0.8, seconds=30, *, on_tank=False):
    state = client.state()
    local = point if on_tank else None
    tank = state.get("tank") if on_tank else None
    if on_tank:
        point = tank_point(tank, local)
    remaining = distance(state["player"]["feet"], point)
    if remaining <= tolerance:
        return
    deadline = state["control"]["simulation_seconds"] + seconds
    target = [point[0], state["player"]["eye"][1], point[2]]
    feet = state["player"]["feet"]
    yaw = math.atan2(feet[0] - point[0], feet[2] - point[2])
    turn = abs((yaw - state["player"]["yaw"] + math.pi) % math.tau - math.pi)
    if turn > math.radians(10):
        aiming = {"aim_entity": tank["entity"],
                  "aim_offset": [local[0], local[1] + state["player"]["eye"][1] - feet[1], local[2]]} if on_tank else {"aim": target}
        client.send({"op":"input","ticks":math.ceil(turn / (math.pi / 2) * 60),
                     "held":[],"look":[0,0],**aiming}, label="Face walking destination")
    pending = client.send({"op": "input", "ticks": 1800, "held": ["forward"],
                           "aim": None if on_tank else target, "aim_entity": tank["entity"] if on_tank else None,
                           "aim_offset": [local[0], local[1] + state["player"]["eye"][1] - feet[1], local[2]] if on_tank else None,
                           "look": [0, 0], "arrival_radius": tolerance},
                          wait=False, label=f"Walk continuously to {point}")
    command = pending["request"]["id"]
    progress_at = None
    best = remaining
    try:
        while True:
            state = client.state()
            control = state["control"]
            if control["failure"] or control["aborted"]:
                raise ValidationFailure(control["failure"] or "Run aborted")
            feet = state["player"]["feet"]
            if on_tank:
                point = tank_point(state["tank"], local)
            remaining = distance(feet, point)
            if remaining <= tolerance:
                return
            if control["active"] is None and not control["queued"]:
                if client.receipt(command)["phase"] == "executed":
                    if remaining > tolerance + 0.2:
                        raise ValidationFailure(f"Walking stopped before arrival near {feet}; target {point}")
                    return
            now = control["simulation_seconds"]
            if progress_at is None:
                progress_at = now
            if now >= deadline:
                raise ValidationFailure(f"Walking route deadline: {point}")
            if remaining < best - 0.15:
                best, progress_at = remaining, now
            elif now - progress_at > 2:
                raise ValidationFailure(f"Walking route is blocked near {feet}; target {point}")
            time.sleep(0.05)
    finally:
        # Cancel releases the ordinary input owner even when the path is blocked.
        if not state["control"]["failure"] and not state["control"]["aborted"]:
            if client.receipt(command)["phase"] == "accepted":
                client.send({"op": "cancel", "command": command}, label="Release walking input")


def shoot_at(client, entity, *, shots=4, label="Fire equipped weapon at its native cadence"):
    state = client.state()
    firing = state["firing"]
    if not firing["magazine"] and firing["ready"]:
        client.input(1, ("reload",), label="Reload the empty equipped weapon")
    if not firing["ready"] or not firing["magazine"]:
        state = client.wait(lambda s: s["firing"]["ready"] and s["firing"]["magazine"] > 0,
                            seconds=10)
        firing = state["firing"]
    enemy = next(e for e in state["enemies"] if e["entity"] == entity)
    target = enemy["aim"]
    eye = state["player"]["eye"]
    yaw = math.atan2(eye[0] - target[0], eye[2] - target[2])
    pitch = math.atan2(target[1] - eye[1], distance(eye, target))
    turn = max(abs((yaw - state["player"]["yaw"] + math.pi) % math.tau - math.pi),
               abs(pitch - state["player"]["pitch"]))
    aiming = {"op": "input", "aim_entity": entity, "look": [0, 0]}
    steps = []
    steps.append({**aiming, "ticks": max(1, math.ceil(turn / (math.pi / 2) * 60),
                                        math.ceil(firing["aim_seconds"] * 60) + 1), "held": ["aim"]})
    count = min(shots, firing["magazine"])
    interval = max(2, math.ceil(firing["interval"] * 60))
    if firing["automatic"]:
        steps.append({**aiming, "ticks": interval * count, "held": ["aim", "fire"]})
    else:
        for _ in range(count):
            steps.extend([{**aiming, "ticks": 1, "held": ["aim", "fire"]},
                          {**aiming, "ticks": interval - 1, "held": ["aim"]}])
    steps.append({**aiming, "ticks": 1, "held": []})
    client.send(steps, label=label)


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
    client.input(30, ("interact",), aim=aim, label="Foundation: collect shovel through interaction")
    state = client.state()
    if not state["player"]["shovel"] or state["interaction"]["accepted"] <= accepted:
        raise ValidationFailure("Shovel ownership changed without an accepted interaction")
    client.input(1)
    results["interaction"] = capture(client, "foundation-shovel")
    state = client.wait(lambda s: any(e["emerged"] and e["life"] == "ActorAlive" for e in s["enemies"]), seconds=60)
    target = min((e for e in state["enemies"] if e["emerged"] and e["life"] == "ActorAlive"), key=lambda e: distance(e["position"], state["player"]["feet"]))
    enemy = target["entity"]
    before_kills = state["evidence"]["kills"]
    before_sequence = state["evidence"]["sequence"]
    deadline = state["control"]["simulation_seconds"] + 60
    killed = False
    while client.state()["control"]["simulation_seconds"] < deadline:
        shoot_at(client, enemy, label="Foundation: aim and fire at the equipped weapon's native cadence")
        state = client.state()
        events = [e for e in state["evidence"]["events"] if e["sequence"] > before_sequence]
        if any(e["kind"] == "kill" and e["data"]["target"] == enemy for e in events):
            if not any(e["kind"] == "hit" and e["data"]["target"] == enemy for e in events):
                raise ValidationFailure("A kill without a correlated gameplay hit is not proof")
            killed = True
            break
        client.send({"op": "assist", "god": True, "points": state["round"]["points"], "ammo": True, "doors": False})
    if not killed:
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
        raise ValidationFailure(f"Step did not include exactly one timestep of physics: delta={stepped['physics_seconds'] - paused['physics_seconds']}")
    results["simulation_step"] = {"ticks": stepped["control"]["tick"] - tick,
                                  "physics_seconds": stepped["physics_seconds"] - paused["physics_seconds"]}
    client.send({"op": "resume"}, label="Foundation: resume")
    client.wait(lambda s: s["game_state"] == "Playing")
    pending = client.send({"op": "input", "ticks": 1200, "held": ["fire"], "look": [0, 0], "aim": None, "aim_entity": None}, wait=False)
    client.send({"op": "cancel", "command": pending["request"]["id"]}, label="Foundation: cancel held fire")
    state = client.state()
    cancelled = client.receipt(pending["request"]["id"])
    if cancelled["phase"] != "cancelled":
        raise ValidationFailure("Cancellation did not produce a cancelled receipt")

    client.beat_stop()
    probe = subprocess.Popen([sys.executable, str(Path(__file__).with_name("run.py")),
                              "--watchdog-probe", "--port", str(client.port)],
                             creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
    try:
        client.wait(lambda s: s["control"]["active"] is not None and s["control"]["label"] == "Foundation: terminate this controlling runner", seconds=4, realtime=True)
        probe.terminate()
        probe.wait(timeout=5)
        failed = client.wait(lambda s: s["control"]["failure"] == "Runner heartbeat expired" and s["game_state"] == "Paused", seconds=15, allow_failure=True, realtime=True)
    finally:
        if probe.poll() is None:
            probe.terminate()
            probe.wait(timeout=5)
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
    results["watchdog"] = "Controlling Python runner terminated; game independently cancelled, recorded, and paused"
    results["final_state"] = client.state()
    return results


def target_of(state, kind, value=None):
    matches = [t for t in state["targets"] if t["target"]["kind"] == kind
               and (value is None or t["target"].get("value") == value)]
    if len(matches) != 1:
        raise ValidationFailure(f"Expected one {kind} target {value}; found {len(matches)}")
    return matches[0]


def use_target(client, target, label, *, aim=None):
    aim = target["position"] if aim is None else aim
    client.input(120, aim=aim, label=label)
    before = client.state()
    if before["interaction"]["target"] != target["entity"]:
        raise ValidationFailure(f"{label}: target is not focused; {before['interaction']}")
    client.input(12, ("interact",), aim=aim, label=label)
    client.input(1)
    after = client.state()
    if after["interaction"]["accepted"] <= before["interaction"]["accepted"]:
        raise ValidationFailure(f"{label}: interaction was not accepted; {after['interaction']}")
    return after


def tank_point(tank, local):
    """Authored route points follow the current tank pose, including turns."""
    x, y, z, w = tank["rotation"]
    a, b, c = local
    tx, ty, tz = 2 * (y*c-z*b), 2 * (z*a-x*c), 2 * (x*b-y*a)
    rotated = [a+w*tx+y*tz-z*ty, b+w*ty+z*tx-x*tz, c+w*tz+x*ty-y*tx]
    return [p+d for p, d in zip(tank["position"], rotated)]


def generators_tank(client):
    results = {}
    client.send({"op": "assist", "god": True, "points": 100000, "ammo": True, "doors": True},
                label="Generator and tank: open routes and replenish equipment")
    client.input(120 * int(client.state()["speed"]), label="Activate and defend generator one, then ride the tank")
    client.send({"op": "teleport", "feet": [63.0, -9.2, -135.3]},
                label="Stage beside the starting-room M14")
    client.input(12)
    rifle = target_of(client.state(), "wall_weapon", "weapon.m14_zm")
    state = use_target(client, rifle, "Buy the starting-room M14 through its ordinary wall interaction")
    if not any(w["id"] == "weapon.m14_zm" for w in state["player"]["inventory"]):
        raise ValidationFailure("The M14 wall purchase did not grant the weapon")
    client.input(120, label="Equip the purchased rifle before defending the generator")
    generator = "generator.generator_start_bunker"
    client.send({"op": "teleport", "feet": [57.25, -7.3914, -122.9]},
                label="Stage on the floor beside generator one")
    client.input(12)
    target = target_of(client.state(), "generator", generator)
    # generators::use_point selects the pump at half of the native use-box height.
    x, y, z = target["position"]
    state = use_target(client, target, "Generator one: purchase activation", aim=[x, y + 1.6256, z])
    if state["generators"]["sites"][generator]["state"] != "Capturing":
        raise ValidationFailure("Generator interaction did not begin capture")
    sequence = state["evidence"]["sequence"]
    defenders = set()
    deadline = state["control"]["simulation_seconds"] + 30
    while True:
        state = client.state()
        guards = [e for e in state["enemies"] if e["generator"] == generator]
        defenders.update(e["entity"] for e in guards)
        if state["generators"]["sites"][generator]["state"] == "Active":
            break
        if state["control"]["simulation_seconds"] > deadline:
            raise ValidationFailure("Generator capture deadline exceeded")
        live = [e for e in guards if e["emerged"] and e["life"] == "ActorAlive"]
        if live:
            enemy = min(live, key=lambda e: distance(e["position"], state["player"]["feet"]))
            shoot_at(client, enemy["entity"], label="Defend the generator at the equipped rifle's native cadence")
            client.send({"op": "assist", "god": True, "points": state["round"]["points"], "ammo": True, "doors": False})
        else:
            client.input(12, label="Hold the generator capture area")
    kills = [e for e in state["evidence"]["events"] if e["sequence"] > sequence
             and e["kind"] == "kill" and e["data"]["target"] in defenders]
    if not kills:
        raise ValidationFailure("Generator powered without a player-attributed defender kill")
    results["generator"] = {"id": generator, "defender_kills": len(kills),
                             "capture": capture(client, "generator-one-powered")}
    client.input(180 * int(state["speed"]), label="Generator one powered: inspect the scene")

    results["boarding"] = board_tank(client)
    target = target_of(client.state(), "trig_use_tank")
    state = use_target(client, target, "Tank: purchase the ordinary trip")
    if state["tank"]["phase"] != "Moving":
        raise ValidationFailure("Tank interaction did not start its route")
    results.update(first_lightning_part(client))
    return results


def board_tank(client):

    tank = client.state()["tank"]
    if tank["phase"] != "Ready" or tank["station"] != 0:
        raise ValidationFailure("Tank must begin at its ordinary church station")
    client.send({"op": "teleport", "feet": [3.0, 1.05, 67.0]}, label="Inspect the native church tank placement")
    client.input(120, aim=[11.19886, 2.4, 67.925188])
    capture(client, "tank-placement")
    # Stage beside the rear hull. Boarding and the staff jump use movement only.
    client.send({"op": "teleport", "feet": tank_point(tank, [1.5, 0.35, 4.8])},
                label="Stage beside the tank at the church")
    client.input(12)
    walk_to(client, tank_point(tank, [1.5, 0.35, 4.2]), tolerance=0.2)
    boarding = tank_point(tank, [1.5, 1.0, 3.5])
    client.input(120, aim=[boarding[0], client.state()["player"]["eye"][1], boarding[2]],
                 label="Inspect the tank boarding route")
    client.send([
        {"op": "input", "ticks": 14, "held": ["jump"], "look": [0, 0], "aim": None, "aim_entity": None},
        {"op": "input", "ticks": 24, "held": ["forward", "jump"], "look": [0, 0], "aim": None, "aim_entity": None},
    ], label="Jump upward, then move over the rear tank lip")
    client.input(24)
    walk_to(client, tank_point(tank, [0.7, 1.272, 2.1]), tolerance=0.12, seconds=8)
    roof = tank_point(tank, [0.7, 1.81, 1.0])
    aim = [roof[0], client.state()["player"]["eye"][1], roof[2]]
    client.input(120, aim=aim, label="Face the raised tank roof")
    client.send([
        {"op": "input", "ticks": 14, "held": ["jump"], "look": [0, 0]},
        {"op": "input", "ticks": 18, "held": ["forward", "jump"], "aim": aim, "look": [0, 0]},
        {"op": "input", "ticks": 30, "held": [], "look": [0, 0]},
    ], label="Jump normally onto the raised native tank roof")
    walk_to(client, tank_point(tank, [0.0, 2.398, -3.5]), tolerance=0.5, seconds=12)
    state = client.state()
    if state["player"]["platform"] != tank["entity"]:
        raise ValidationFailure("Tank boarding did not establish platform support")
    return capture(client, "tank-boarded")


def first_lightning_part(client):
    results = {}
    state = client.state()
    tank = state["tank"]
    if state["tank"]["phase"] != "Moving":
        raise ValidationFailure("The paid tank trip must be moving")
    start = state["tank"]["position"]
    deadline = state["control"]["simulation_seconds"] + 150
    supported = 0
    while state["tank"]["position"][2] > -20:
        if state["control"]["simulation_seconds"] > deadline:
            raise ValidationFailure("Tank did not reach the first lightning jump")
        if state["player"]["platform"] != tank["entity"]:
            raise ValidationFailure("Platform support was lost during the tank ride")
        supported += 1
        client.input(12, aim=[100.0, state["player"]["eye"][1], state["player"]["feet"][2]],
                     label="Ride the tank to the first lightning-part platform")
        state = client.state()
    if distance(start, state["tank"]["position"]) < 50 or supported < 5:
        raise ValidationFailure("The tank ride did not demonstrate sustained platform carry")
    results["ride"] = {"supported_observations": supported}
    results["ride_capture"] = capture(client, "tank-ride")
    walk_to(client, [1.5, 2.36, -6.5], tolerance=0.2, seconds=10, on_tank=True)
    client.input(30, label="Settle on the front deck before timing the jump")
    state = client.state()
    sequence = state["evidence"]["sequence"]
    aim = [44.0, -3.76, -84.8]
    tank = state["tank"]
    # The front deck clears the fence's southern end before the southwest turn.
    takeoff = [32.335, 0, -83.08]
    (client.report / "tank-jump-plan.json").write_text(json.dumps({
        "feet":state["player"]["feet"],"velocity":state["player"]["velocity"],
        "tank":tank["position"],"wait_point":takeoff,"runup_ticks":4,"aim":aim,
    },indent=2))
    client.send([
        {"op": "input", "ticks": 1800, "held": [], "aim": aim, "look": [0, 0],
         "arrival_point": takeoff, "arrival_entity":tank["entity"], "arrival_radius": 0.1},
        {"op": "input", "ticks": 4, "held": ["forward", "sprint"], "aim": aim, "look": [0, 0]},
        {"op": "input", "ticks": 18, "held": ["forward", "sprint", "jump"], "aim": aim, "look": [0, 0]},
        {"op": "input", "ticks": 24, "held": ["forward", "sprint"], "aim": aim, "look": [0, 0]},
        {"op": "input", "ticks": 60, "held": [], "aim": aim, "look": [0, 0]},
    ], label="Jump from the tank to the near staff bank and settle after landing")
    state = client.wait(lambda s: s["player"]["grounded"], seconds=5)
    landings = [e for e in state["evidence"]["events"] if e["sequence"] > sequence
                and e["kind"] == "landing" and e["data"]["owner"] == state["player"]["entity"]]
    if not landings or state["player"]["platform"] == tank["entity"] or distance(state["player"]["feet"], [36.5, -2.0, -88.5]) > 2.0:
        raise ValidationFailure("Lightning jump has no landing away from the tank")
    jumps = [e for e in state["evidence"]["events"] if e["sequence"] > sequence
             and e["kind"] == "jump" and e["data"]["platform"] == tank["entity"]]
    if not jumps:
        raise ValidationFailure("Lightning jump has no native takeoff from the tank")
    # Standing idle releases the native sprint toggle before this narrow path.
    for point in ([37,-2.1,-88.5], [37,-2.62,-87.5], [37.5,-2.82,-87.5],
                  [37.5,-3.12,-87], [38,-3.29,-87], [38,-3.84,-86],
                  [46.5,-3.75,-86], [46.5,-3.75,-85], [49,-3.79,-85],
                  [49,-3.71,-84], [52,-3.53,-84], [52,-3.49,-83.5],
                  [52.5,-3.4,-83.5], [52.5,-3.4,-83], [53,-3.38,-83],
                  [53,-3.54,-82.5], [57,-3.56,-82.5], [57.2,-3.4,-76.949554]):
        walk_to(client, point, tolerance=0.25, seconds=15)
    part = "part.elemental_staff_lightning.upper_staff"
    target = target_of(client.state(), "part", part)
    state = use_target(client, target, "Take the lightning staff part through interaction")
    owned = state["parts"]["pieces"][part]
    if owned["state"] != "owned" or owned["player"] != state["player"]["entity"]:
        raise ValidationFailure("Lightning part was not collected by this player")
    results["lightning_part"] = {"id": part, "jump": jumps[-1]["data"],
                                  "landing": landings[-1]["data"], "capture": capture(client, "lightning-first-part")}
    client.input(180 * int(state["speed"]), label="Generator and tank checkpoint passed: inspect the part")
    return results


ROUTE = {"foundation": foundation, "generators-tank": generators_tank}
