import argparse
import hashlib
import json
import math
from pathlib import Path

from gameplay_remote import Client, write_report


def check(report, name, passed, **evidence):
    report["cases"].append({"case": name, "passed": bool(passed), **evidence})
    if not passed:
        raise RuntimeError(name)


def interact(client, state):
    bindings = state["controls"]["bindings"]["interact"]
    key = next((binding["key"] for binding in bindings if "key" in binding), None)
    if key is None:
        key = next(("Mouse" + binding["mouse"] for binding in bindings
                    if binding.get("mouse") in ("Left", "Right", "Middle")), None)
    if key is None:
        raise RuntimeError("No supported interaction binding")
    client.call("origins/input", {"events": [
        {"tick": 1, "key": key, "pressed": True},
        {"tick": 6, "key": key, "pressed": False},
    ]})


def approach(client, content, perk):
    interaction = next(item for item in content["interactions"]
                       if item["action"].get("perk") == perk["id"])
    placement = next(item for item in content["placements"] if item["id"] == interaction["placement"])
    origin = placement["position"]
    yaw = placement["yaw"]
    front = [math.sin(yaw), 0, math.cos(yaw)]
    target = [origin[0] + front[0] * 0.8, origin[1] + 1, origin[2] + front[2] * 0.8]
    for distance in (1.4, 1.7, 1.2, 2.0):
        state = client.state()
        player = state["player"]
        position = [origin[0] + front[0] * distance, origin[1] + player["height"] / 2,
                    origin[2] + front[2] * distance]
        eye = [position[0], position[1] + player["eye_height"] - player["height"] / 2, position[2]]
        delta = [target[i] - eye[i] for i in range(3)]
        client.call("origins/setup", {"position": position,
                    "yaw": math.atan2(-delta[0], -delta[2]),
                    "pitch": math.atan2(delta[1], math.hypot(delta[0], delta[2]))})
        state = client.wait_ticks(2)
        if f"buy {perk['name']} [" in state["focus"]["prompt"]:
            return state
    raise RuntimeError("Could not focus native " + perk["name"] + " machine")


def setup(client, content, perk_id, owned):
    client.reset()
    fixture = {"clear_enemies": True, "points": 10000, "pending": 6,
               "pause_spawning_seconds": 300, "owned_perks": owned}
    client.call("origins/setup", fixture)
    perk = next(item for item in content["perks"] if item["id"] == perk_id)
    return approach(client, content, perk), perk, fixture


def rejected(client, report, content, name, perk_id, owned, rejection):
    before, perk, fixture = setup(client, content, perk_id, owned)
    interact(client, before)
    after = client.until(lambda state: state["focus"]["rejection"] == rejection, timeout=2)
    check(report, name, set(after["player"]["perks"]) == set(owned)
          and not after["player"]["drinking"]
          and after["session"]["points"] == before["session"]["points"]
          and after["focus"]["accepted"] == before["focus"]["accepted"],
          fixture=fixture, before=before, after=after)
    client.release()


def run(client, report, content):
    three = ["perk.juggernog", "perk.speed_cola", "perk.mule_kick"]
    before, perk, fixture = setup(client, content, "perk.quick_revive", three)
    interact(client, before)
    after = client.until(lambda state: "perk.quick_revive" in state["player"]["perks"]
                         and not state["player"]["drinking"], timeout=15)
    check(report, "fourth_perk_purchase_accepted", set(after["player"]["perks"]) == set(three + [perk["id"]])
          and after["session"]["points"] == before["session"]["points"] - perk["price"]
          and after["player"]["solo_charges"] == 1
          and after["focus"]["accepted"] == before["focus"]["accepted"] + 1,
          fixture=fixture, before=before, after=after)
    client.release()
    rejected(client, report, content, "fifth_perk_rejected_without_payment_or_drink", "perk.quick_revive",
             three + ["perk.stamin_up"], "PerkLimit")
    rejected(client, report, content, "duplicate_keeps_priority_at_four_perks", "perk.juggernog",
             three + ["perk.stamin_up"], "Duplicate")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default="http://127.0.0.1:15702")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--identity", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    if args.output.resolve().is_relative_to(root):
        parser.error("Output must be outside the repository")
    client = Client(args.url)
    report = {"passed": False, "scenario": "default_perk_purchase_limit", "cases": [],
              "setup_is_gameplay_evidence": False,
              "limitations": ["Ownership is direct scenario setup; purchase outcomes use native input.",
                              "Earned slots and local generator power are outside this scenario."]}
    try:
        if args.identity is not None:
            identity = args.identity.resolve()
            if identity.is_relative_to(root):
                raise ValueError("Pin identity must be outside the repository")
            report["identity"] = {"path": str(identity), "sha256": hashlib.sha256(identity.read_bytes()).hexdigest()}
        game_path = root / "assets/origins/game.json"
        data = game_path.read_bytes()
        report["content"] = {"path": str(game_path), "sha256": hashlib.sha256(data).hexdigest()}
        content = json.loads(data)["content"]
        client.until(lambda state: state.get("state") == "Playing" and state.get("player"), timeout=120)
        run(client, report, content)
        report["passed"] = True
    except Exception as error:
        report["error"] = str(error)
    finally:
        try:
            client.release()
        except Exception as error:
            report["passed"] = False
            report["release_error"] = str(error)
        write_report(args.output, "perk-limit.json", report)
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
