"""Preparation through the existing generator, crafting and portal owners."""
import math
import json
from client import ValidationFailure
from routes import activate_generator, capture, distance, shoot_at, target_of, use_target, walk_to


def opportunity_rejections(client):
    client.send({"op": "pause"}, label="Inspect opportunity rejection accounting while simulation is paused")
    client.wait(lambda s: s["game_state"] == "Paused")
    results = []
    for action in ({"op": "spawn_choice", "candidate": 4294967295},
                   {"op": "robot_choice", "robot": 99, "foot": 0},
                   {"op": "powerup_choice", "id": "powerup.invalid"}):
        before = client.state()
        accepted = client.send(action, wait=False)
        command = accepted["request"]["id"]
        client.wait(lambda s: client.receipt(command)["phase"] != "accepted", realtime=True)
        receipt = client.receipt(command)
        after = client.state()
        if receipt["phase"] != "rejected" or after["control"]["failure"]:
            raise ValidationFailure(f"Ineligible opportunity did not reject normally: {receipt}")
        if any(before[key] != after[key] for key in ("round", "robots", "weather", "parts")):
            raise ValidationFailure(f"Rejected opportunity changed paused gameplay accounting: {action}")
        results.append(receipt)
    # Snow remains subject to the existing round's schedule and overdue rules.
    accepted = client.send({"op": "weather_choice", "kind": "snow"}, wait=False)
    command = accepted["request"]["id"]
    client.wait(lambda s: client.receipt(command)["phase"] != "accepted", realtime=True)
    results.append(client.receipt(command))
    if results[-1]["phase"] == "executed":
        before = client.state()
        accepted = client.send({"op": "weather_choice", "kind": "snow"}, wait=False)
        command = accepted["request"]["id"]
        client.wait(lambda s: client.receipt(command)["phase"] != "accepted", realtime=True)
        receipt = client.receipt(command)
        if receipt["phase"] != "rejected" or client.state()["weather"] != before["weather"]:
            raise ValidationFailure("A duplicate weather selection changed the pending opportunity")
        results.append(receipt)
    (client.report / "opportunity-rejections.json").write_text(json.dumps(results, indent=2))
    client.send({"op": "resume"})
    client.wait(lambda s: s["game_state"] == "Playing")
    return [{"op": r["request"]["action"]["op"], "phase": r["phase"]} for r in results]


def stage_near(client, target, label, *, focus=False, radii=(1.4,), view=None,
               heights=(-1.3, -0.2, 0.8), elevation=(-0.4, 2.1), bullet_view=False):
    """Inspect nearby floors before requesting the collision-checked staging cheat."""
    x, y, z = target["position"]
    if view is None and target.get("target"):
        kind = target["target"]["kind"]
        offset = {"craft_part": 0.35, "bench": 0.35, "capture_reward": 0.8128,
                  "mound": 0.508, "hidden": 0.508, "generator": 1.6256}.get(kind, 0)
        view = [x, y + offset, z]
    points = [[x + radius * math.cos(i * math.pi / 4), y + height,
               z + radius * math.sin(i * math.pi / 4)] for radius in radii
              for height in heights for i in range(8)]
    probes = []
    for start in range(0, len(points), 64):
        probes.extend(client.rpc("origins/validator/jump_probes", [
            {"feet": p, "velocity": [0, 0, 0], "aim": p, "view": view, "include_tank": True,
             "bullet_view": bullet_view}
            for p in points[start:start+64]])["results"])
    floors = [p["staging_feet"] for p in probes if p["landed"] is not None and p["clear"] and p["playable"]
              and elevation[0] <= y - p["feet"][1] <= elevation[1] and (view is None or p["view_clear"])]
    if not floors:
        raise ValidationFailure(f"No clear supporting floor beside {label}: {target['position']}")
    current = client.state()["player"]["feet"]
    for feet in sorted(floors, key=lambda p: distance(p, current)):
        receipt = client.send({"op": "teleport", "feet": feet}, label=f"Stage beside {label}", allow_rejection=True)
        if receipt["phase"] == "rejected":
            continue
        client.input(12)
        if not focus:
            return
        client.input(120, aim=view or target["position"], label=f"Inspect the ordinary interaction for {label}")
        if client.state()["interaction"]["target"] == target["entity"]:
            return
    raise ValidationFailure(f"No inspected floor gives ordinary interaction access to {label}")


def part_target(state, name):
    return next(t for t in state["targets"] if t["target"]["kind"] in ("part", "craft_part", "capture_reward")
                and t["target"]["value"] == name)


def collect_part(client, name, *, stage=True):
    state = client.state()
    if state["parts"]["pieces"].get(name, {}).get("state") in ("owned", "consumed"):
        return
    target = part_target(state, name)
    if stage:
        stage_near(client, target, name, focus=True)
    state = use_target(client, part_target(client.state(), name), f"Collect {name} through interaction")
    part = state["parts"]["pieces"].get(name, {})
    if part.get("state") != "owned" or part.get("player") != state["player"]["entity"]:
        raise ValidationFailure(f"Part was not collected by this player: {name}")


def bench_target(state, recipe):
    choices = [t for t in state["targets"] if t["target"]["kind"] == "bench"
               and t["target"]["recipe"] == recipe]
    return min(choices, key=lambda t: distance(t["position"], state["player"]["feet"]))


def craft(client, recipe):
    target = bench_target(client.state(), "open_table" if recipe == "equip_dieseldrone_zm" else recipe)
    stage_near(client, target, f"{recipe} workbench")
    client.input(120, aim=target["position"], label=f"Inspect the {recipe} workbench")
    if recipe == "equip_dieseldrone_zm":
        client.input(1, ("buildable",), aim=target["position"], label="Select Maxis Drone on the ordinary workbench")
        client.input(1, aim=target["position"])
    state = client.state()
    if state["interaction"]["target"] != target["entity"] or recipe.replace("_", " ") not in state["interaction"]["prompt"]:
        raise ValidationFailure(f"The requested recipe is not focused: {recipe}; {state['interaction']}")
    client.input(240, ("interact",), aim=target["position"], label=f"Hold the normal crafting interaction for {recipe}")
    client.input(1)
    if recipe not in client.state()["crafting"]["crafted"]:
        raise ValidationFailure(f"Crafting did not complete: {recipe}")
    state = use_target(client, target, f"Take the crafted {recipe}")
    if recipe == "equip_dieseldrone_zm" and recipe not in state["buildables"]["owned"]:
        raise ValidationFailure("The crafted Maxis Drone was not taken")


def collect_crystal(client, element, index):
    state = client.state()
    gramophone = target_of(state, "gramophone", index + 1)
    entrance = min((t for t in state["targets"] if t["target"]["kind"] == "portal_entrance"),
                   key=lambda t: distance(t["position"], gramophone["position"]))
    stage_near(client, gramophone, f"{element} gramophone site")
    use_target(client, gramophone, f"Place the gramophone for the {element} portal")
    client.wait(lambda s: s["portals"]["entrances"][index] == "Open", seconds=20)
    stage_near(client, entrance, f"open {element} portal approach")
    walk_to(client, entrance["position"], tolerance=0.35, seconds=10, through_portal=True)
    client.input(150, label="Watch the ordinary portal transit finish")
    take_crystal_and_return(client, element, index)


def take_crystal_and_return(client, element, index):
    state = client.state()
    if not state["portals"]["visited"][index] or state["player"]["feet"][0] < 200:
        raise ValidationFailure(f"The {element} portal did not deliver this player to the Crazy Place")
    name = f"part.elemental_staff_{element}.gem"
    crystal = part_target(state, name)
    stage_near(client, crystal, f"{element} crystal plinth")
    client.input(900, aim=crystal["position"], label=f"Watch the {element} plinth reveal its crystal")
    if not client.state()["portals"]["crystals_ready"][index]:
        raise ValidationFailure(f"The {element} plinth did not reveal its crystal")
    collect_part(client, name, stage=False)
    returning = target_of(client.state(), "portal_return", index)
    stage_near(client, returning, f"{element} return portal")
    trips = client.state()["portals"]["trips"]
    use_target(client, returning, f"Activate the {element} return portal")
    client.wait(lambda s: s["portals"]["trips"] > trips, seconds=40)
    client.input(150, label="Watch the return transit finish")
    if client.state()["player"]["feet"][0] > 200:
        raise ValidationFailure("Return portal did not leave the Crazy Place")
    gramophone = target_of(client.state(), "gramophone", index + 1)
    stage_near(client, gramophone, "gramophone retrieval")
    state = use_target(client, gramophone, "Retrieve the gramophone through interaction")
    if not state["crafting"]["gramophone"]:
        raise ValidationFailure("Gramophone was not retrieved after portal travel")


def select_snow(client):
    if client.state()["weather"]["selection"] is not None:
        return
    accepted = client.send({"op": "weather_choice", "kind": "snow"}, wait=False,
                           label="Choose snow only at the owner's eligible next round opportunity")
    command = accepted["request"]["id"]
    client.wait(lambda s: client.receipt(command)["phase"] != "accepted", realtime=True)
    if client.receipt(command)["phase"] not in ("executed", "rejected"):
        raise ValidationFailure("Weather opportunity did not resolve normally")


def dig_snow_parts(client):
    regions = {"lower_staff": "bunker", "middle_staff": "nml", "upper_staff": "village"}
    client.input(180, label="Allow native snow presentation and mound replenishment")
    while True:
        state = client.state()
        missing = {f"part.elemental_staff_water.{piece}": region for piece, region in regions.items()
                   if state["parts"]["pieces"].get(f"part.elemental_staff_water.{piece}", {}).get("state") != "owned"}
        for name in list(missing):
            if state["parts"]["pieces"].get(name, {}).get("state") == "available":
                collect_part(client, name)
                del missing[name]
        if not missing or state["weather"]["kind"] != "snow":
            return
        mounds = [t for t in state["targets"] if t["target"]["kind"] == "mound"
                  and any(region in t["zone"] for region in missing.values())]
        if not mounds:
            return
        mound = min(mounds, key=lambda t: distance(t["position"], state["player"]["feet"]))
        stage_near(client, mound, f"snow dig in {mound['zone']}", focus=True)
        x, y, z = mound["position"]
        use_target(client, mound, "Dig the ordinary snow mound", aim=[x, y + 0.508, z])
        client.input(60, label="Let the normal dig reward rise")


def clear_round(client):
    state = client.state()
    round_number = state["round"]["number"]
    deadline = state["control"]["simulation_seconds"] + 600
    sequence = state["evidence"]["sequence"]
    last_progress = state["control"]["simulation_seconds"]
    previous_kills = state["evidence"]["kills"]
    while state["round"]["number"] == round_number:
        if state["control"]["simulation_seconds"] > deadline:
            raise ValidationFailure(f"Normal combat could not finish round {round_number}")
        if state["evidence"]["kills"] != previous_kills:
            previous_kills = state["evidence"]["kills"]
            last_progress = state["control"]["simulation_seconds"]
        planes = [p for p in state["planes"] if not p["quest"]]
        if planes:
            shoot_at(client, planes[0]["entity"], shots=2, label="Shoot the glowing fire-part plane normally")
        else:
            live = [e for e in state["enemies"] if e["life"] == "ActorAlive" and e["emerged"]
                    and e["generator"] is None]
            if live:
                enemy = min(live, key=lambda e: distance(e["position"], state["player"]["feet"]))
                # Staging stays on native floors; every kill still uses the equipped weapon.
                if (distance(enemy["position"], state["player"]["feet"]) > 38.608
                        and enemy["zone"] is not None and enemy["definition"] != "enemy.panzer"):
                    eye = state["player"]["eye"]
                    away = [2 * eye[0] - enemy["position"][0], eye[1],
                            2 * eye[2] - enemy["position"][2]]
                    client.input(120, aim=away, label="Look away while the native distant-actor owner checks its ordinary opportunity")
                    state = client.state()
                    continue
                if (distance(enemy["position"], state["player"]["feet"]) > 25
                        or state["control"]["simulation_seconds"] - last_progress > 20):
                    stage_near(client, enemy, "ordinary combat approach", radii=(1.4, 3, 5, 8, 12, 20),
                               heights=(-1.3, -0.2, 0.8, 2, 4), elevation=(-4, 6),
                               view=enemy["aim"], bullet_view=True)
                    last_progress = client.state()["control"]["simulation_seconds"]
                client.send({"op": "assist", "god": True, "points": state["round"]["points"], "ammo": True, "doors": False})
                shoot_at(client, enemy["entity"], shots=8, head=enemy["definition"] == "enemy.panzer",
                         label=f"Clear round {round_number} with player-attributed weapon fire")
            else:
                client.input(60, label=f"Wait for the next native round {round_number} spawn")
        state = client.state()
    kills = [e for e in state["evidence"]["events"] if e["sequence"] > sequence and e["kind"] == "kill"]
    if not kills:
        raise ValidationFailure("Round advanced without any observed player-attributed kill")
    return {"round": round_number, "kills": len(kills)}


def fire_and_ice_parts(client):
    result = []
    names = [f"part.elemental_staff_fire.{p}" for p in ("middle_staff", "lower_staff")]
    names += [f"part.elemental_staff_water.{p}" for p in ("upper_staff", "middle_staff", "lower_staff")]
    while True:
        state = client.state()
        for name in names:
            if state["parts"]["pieces"].get(name, {}).get("state") == "available":
                collect_part(client, name)
        state = client.state()
        if all(state["parts"]["pieces"].get(name, {}).get("state") == "owned" for name in names):
            return result
        if state["round"]["number"] > 15:
            raise ValidationFailure("Fire and snow opportunities did not yield their normal parts by round 15")
        dig_snow_parts(client)
        select_snow(client)
        stage_near(client, {"position": [20.024852, 3.519932, -11.958066]}, "open No Man's Land combat area")
        result.append(clear_round(client))


def preparation(client):
    result = {"opportunities": opportunity_rejections(client)}
    speed = int(client.state()["speed"])
    client.input(120 * speed, label="Preparation: power the map, gather parts, and craft the four staffs")
    for suffix in ("tank_trench", "mid_trench", "nml_right", "nml_left", "church"):
        generator = f"generator.generator_{suffix}"
        if client.state()["generators"]["sites"][generator]["state"] != "Active":
            target = target_of(client.state(), "generator", generator)
            stage_near(client, target, generator)
            result[generator] = activate_generator(client, generator, f"generator-{suffix}-powered")
    if not all(g["state"] == "Active" for g in client.state()["generators"]["sites"].values()):
        raise ValidationFailure("Preparation did not leave all six generators powered")
    result["generators"] = capture(client, "all-generators-powered")
    collect_part(client, "part.elemental_staff_fire.upper_staff")
    for record in ("player", "master", "fire", "air", "elec", "ice"):
        collect_part(client, f"part.gramophone_vinyl.{record}")
    gramophone = target_of(client.state(), "gramophone", 0)
    stage_near(client, gramophone, "excavation gramophone")
    use_target(client, gramophone, "Open excavation through the gramophone")
    client.wait(lambda s: s["portals"]["excavation_open"], seconds=25)
    use_target(client, gramophone, "Retrieve the gramophone after opening excavation")
    for piece in ("brain", "body", "engine"):
        collect_part(client, f"part.equip_dieseldrone_zm.{piece}")
    craft(client, "equip_dieseldrone_zm")
    result["drone"] = capture(client, "maxis-drone-crafted")
    for index, element in enumerate(("fire", "air", "lightning", "water")):
        collect_crystal(client, element, index)
    result["fire_and_ice"] = fire_and_ice_parts(client)
    pieces = client.state()["parts"]["pieces"]
    missing = [f"part.elemental_staff_{element}.{piece}"
               for element in ("fire", "air", "lightning", "water")
               for piece in ("upper_staff", "middle_staff", "lower_staff")
               if pieces.get(f"part.elemental_staff_{element}.{piece}", {}).get("state") != "owned"]
    if missing:
        raise ValidationFailure(f"Preparation still needs normal elemental-part acquisition: {missing}")
    for element in ("fire", "air", "lightning", "water"):
        craft(client, f"elemental_staff_{element}")
    result["staffs"] = capture(client, "four-staffs-crafted")
    client.input(180 * speed, label="Preparation complete: inspect the four crafted staffs")
    return result
