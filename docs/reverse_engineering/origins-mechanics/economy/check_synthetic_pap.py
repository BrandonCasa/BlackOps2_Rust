"""User-created serial abstract checks, not original code or native emulation.

Removal, readiness, terminal cleanup and grant are supplied distinct events.
The take/cleanup/grant schedule is an illustrative interleaving, not a claim
about native thread order. Timer values are model script units, not wall time.
"""

import json
from pathlib import Path


def apply(state, event):
    phase = state["phase"]
    if isinstance(event, dict):
        if phase != "ready":
            raise AssertionError("timer event requires ready phase")
        state["ready_elapsed"] += event["advance"]
        if state["ready_elapsed"] >= 15:
            state["phase"] = "terminal_dispose"
        return
    if event == "use":
        if phase == "idle" and state["trigger"] and state["points"] >= 5000:
            state["points"] -= 5000
            state["debits"] += 1
            state["in_use"] = True
            state["phase"] = "processing"
    elif event == "remove":
        if phase != "processing":
            raise AssertionError("removal requires processing phase")
        state["original"] = False
    elif event == "ready":
        if phase != "processing" or state["original"]:
            raise AssertionError("representative readiness requires removal")
        state["phase"] = "ready"
        state["ready_elapsed"] = 0
    elif event == "take":
        if phase == "ready":
            state["phase"] = "terminal_taken"
    elif event in ("other_take", "invalid_take"):
        if phase != "ready":
            raise AssertionError("refused retrieval requires ready phase")
    elif event == "lose_power":
        state["pending_disable"] = True
        if not state["in_use"]:
            state["trigger"] = False
    elif event == "disconnect":
        if phase != "ready":
            raise AssertionError("pre-ready disconnect excluded")
        state["phase"] = "terminal_dispose"
    elif event == "cleanup":
        if phase not in ("terminal_taken", "terminal_dispose"):
            raise AssertionError("cleanup requires terminal notification")
        state["in_use"] = False
        if state["pending_disable"]:
            state["trigger"] = False
        state["phase"] = "grant_pending" if phase == "terminal_taken" else "disposed"
    elif event == "grant":
        if phase != "grant_pending":
            raise AssertionError("grant requires supplied accepted-return schedule")
        state["upgraded"] = True
        state["grants"] += 1
        state["phase"] = "acquired"
    else:
        raise AssertionError(f"unsupported event: {event}")


def main():
    fixture = Path(__file__).with_name("synthetic-pap-cases.json")
    cases = json.loads(fixture.read_text(encoding="utf-8"))["cases"]
    passed = []
    for case in cases:
        state = {
            "points": case["points"], "phase": "idle", "original": True,
            "upgraded": False, "debits": 0, "grants": 0, "in_use": False,
            "trigger": True, "pending_disable": False,
        }
        for event in case["events"]:
            was_in_use = state["in_use"]
            apply(state, event)
            if event == "lose_power" and was_in_use and not state["trigger"]:
                raise AssertionError(f"{case['id']}: premature shutdown")
            if case["points"] - state["points"] != state["debits"] * 5000:
                raise AssertionError(f"{case['id']}: debit conservation")
            if state["debits"] > 1 or state["grants"] > 1:
                raise AssertionError(f"{case['id']}: serial replay duplicates")
        for key, expected in case["expect"].items():
            if state[key] != expected:
                raise AssertionError(f"{case['id']}: {key}: {state[key]} != {expected}")
        passed.append(case["id"])
    print(json.dumps({"status": "PASS", "case_count": len(passed), "cases": passed}))


if __name__ == "__main__":
    main()
