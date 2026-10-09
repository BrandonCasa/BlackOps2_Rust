import argparse
import hashlib
import math
from pathlib import Path
import time

from gameplay_remote import Client, write_report


def require(report, name, passed, **evidence):
    report["cases"].append({"case": name, "passed": bool(passed), **evidence})
    if not passed:
        raise RuntimeError(name)


def interaction_key(state):
    for binding in state["controls"]["bindings"]["interact"]:
        if "key" in binding:
            return binding["key"]
        if binding.get("mouse") in ("Left", "Right", "Middle"):
            return "Mouse" + binding["mouse"]
    raise RuntimeError("No supported native interaction binding")


def focus_generator(client, site):
    state = client.state()
    height = state["player"]["height"]
    reach = site["interaction_reach"]
    center = site["position"]
    for index in range(8):
        angle = index * math.tau / 8
        radius = min(2.3, reach * 0.7)
        position = [center[0] + math.sin(angle) * radius,
                    center[1] + height / 2, center[2] + math.cos(angle) * radius]
        client.call("origins/setup", {"position": position})
        state = client.wait_ticks(2)
        player = state["player"]
        eye = list(player["position"])
        eye[1] += player["eye_height"] - player["height"] / 2
        delta = [center[0] - eye[0], center[1] + reach / 2 - eye[1], center[2] - eye[2]]
        yaw = math.atan2(-delta[0], -delta[2])
        pitch = math.atan2(delta[1], math.hypot(delta[0], delta[2]))
        client.call("origins/setup", {"yaw": yaw, "pitch": pitch})
        state = client.wait_ticks(2)
        if f"activate Generator {site['number']} " in state["focus"]["prompt"]:
            return state
    raise RuntimeError("Native generator interaction could not be focused")


def scenario(client, report, count):
    state = client.reset()
    client.call("origins/setup", {"clear_enemies": True, "points": 5000,
                                  "pending": 6, "pause_spawning_seconds": 300})
    state = client.state()
    site = next(site for site in state["generators"]["sites"] if site["number"] == 1)
    state = focus_generator(client, site)
    origin = state["player"]["position"]
    actors = []
    for index in range(count):
        angle = index * math.tau / count
        actors.append({"position": [origin[0] + math.cos(angle) * 12,
                                     origin[1], origin[2] + math.sin(angle) * 12], "health": 100})
    setup = {"clear_enemies": True, "pending": 6, "pause_spawning_seconds": 300, "enemies": actors}
    client.call("origins/setup", setup)
    baseline = client.wait_ticks(2)
    original = {enemy["entity"] for enemy in baseline["enemies"] if enemy.get("defender") is None}
    prefix = f"population_{count}"
    require(report, prefix + "_native_actor_setup", len(original) == count,
            setup=setup, before=baseline)
    key = interaction_key(baseline)
    client.call("origins/input", {"events": [
        {"tick": 1, "key": key, "pressed": True},
        {"tick": 3, "key": key, "pressed": False},
    ]})
    expected = max(0, count - 20)
    snapshots = []
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        state = client.state()
        snapshots.append(state)
        if (state["generators"]["activations"] == baseline["generators"]["activations"] + 1
                and state["generators"]["excess_retired"] == baseline["generators"]["excess_retired"] + expected):
            break
        time.sleep(0.01)
    state = client.wait_ticks(3)
    snapshots.append(state)
    current = {enemy["entity"] for enemy in state["enemies"]}
    remaining = original & current
    selected_site = next(value for value in state["generators"]["sites"] if value["id"] == site["id"])
    require(report, prefix + "_normal_input_capture", selected_site["state"] == "Capturing"
            and state["generators"]["activations"] == baseline["generators"]["activations"] + 1,
            key=key, site=selected_site)
    require(report, prefix + "_exact_excess_retirement", len(remaining) == min(count, 20)
            and state["generators"]["excess_retired"] - baseline["generators"]["excess_retired"] == expected,
            removed=sorted(original - current), remaining=sorted(remaining), after=state)
    ghosts = [enemy for sample in snapshots for enemy in sample["enemies"]
              if enemy["entity"] in original and enemy["health"] is None]
    require(report, prefix + "_retired_membership_and_collision", all(
        ghost["collider_disabled"] for ghost in ghosts)
        and not any(enemy["health"] is None for enemy in state["enemies"] if enemy["entity"] in original),
        observed_ghosts=ghosts, removed_entities=sorted(original - current))
    before_session, after_session = baseline["session"], state["session"]
    require(report, prefix + "_no_rewards_or_requeue", all(
        after_session[field] == before_session[field] for field in ("pending", "score", "kills"))
        and after_session["points"] == before_session["points"] - site["price"]
        and state["round_evidence"]["requeued"] == baseline["round_evidence"]["requeued"]
        and state["drops"]["spawned"] == baseline["drops"]["spawned"],
        before=before_session, after=after_session)
    require(report, prefix + "_reserved_capture_slots", state["generators"]["reserve"] == 4)
    client.release()
    client.wait_ticks(4)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default="http://127.0.0.1:15702")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--identity", type=Path)
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(Path(__file__).resolve().parent.parent):
        parser.error("Output must be outside the repository")
    client = Client(args.url)
    report = {"passed": False, "scenario": "native_generator_population_retirement", "cases": [],
              "setup_is_gameplay_evidence": False,
              "limitations": ["Native tesla_elec_kill presentation is not prepared.",
                              "Solo single-generator capture only; recapture and multiple contested zones are unfinished.",
                              "One-frame ghosts are checked when observed; final entity absence is always required."]}
    try:
        if args.identity is not None:
            identity = args.identity.resolve()
            if identity.is_relative_to(Path(__file__).resolve().parent.parent):
                raise ValueError("Pin identity must be outside the repository")
            report["identity"] = {"path": str(identity),
                                  "sha256": hashlib.sha256(identity.read_bytes()).hexdigest()}
        client.until(lambda state: state.get("state") == "Playing" and state.get("player")
                     and state.get("generators", {}).get("sites"), timeout=120)
        for count in (20, 24):
            scenario(client, report, count)
        report["passed"] = True
    except Exception as error:
        report["error"] = str(error)
    finally:
        try:
            client.release()
        except Exception as error:
            report["passed"] = False
            report["release_error"] = str(error)
        write_report(args.output, "generator-population.json", report)
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
