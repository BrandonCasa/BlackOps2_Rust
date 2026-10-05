"""User-created abstract purchase model checks; no original code or runtime.

Only the documented serial, modifier-supplied scenario subset is modeled.
This does not resolve native scheduling, callback/score races, or refund policy.
"""

import json
from pathlib import Path


def admit(state):
    """Apply the abstract admission order, returning the first failure."""
    if state["busy"]:
        return "busy"
    if state["active_owned"] or state["paused_owned"]:
        return "owned"
    if not state["zone_controlled"]:
        return "unpowered"
    if state["points"] < state["price"]:
        return "insufficient"
    if state["count"] >= state["limit"]:
        return "capacity"
    state["points"] -= state["price"]
    state["score_mirror"] = state["points"]
    state["busy"] = True
    return "paid_pending"


def require(condition, case_id, observable):
    if not condition:
        raise AssertionError(f"{case_id}: {observable}")


def main():
    fixture = Path(__file__).with_name("synthetic-purchase-cases.json")
    cases = json.loads(fixture.read_text(encoding="utf-8"))
    passed = []
    for case in cases["cases"]:
        state = cases["defaults"] | case["input"]
        initial_points, initial_count = state["points"], state["count"]
        result = admit(state)
        require(result == case["expect"], case["id"], "admission reason")
        require(state["points"] == case["balance"], case["id"], "balance")
        require(state["count"] == initial_count, case["id"], "grant is delayed")
        if result == "paid_pending":
            require(not state["active_owned"], case["id"], "ownership is delayed")
            require(
                initial_points - state["points"] == state["price"],
                case["id"],
                "one effective-price debit",
            )
            require(
                state["score_mirror"] == state["points"], case["id"], "score mirror"
            )
        else:
            require(state["points"] == initial_points, case["id"], "no failed debit")
        passed.append(case["id"])

    for case in cases["staged_cases"]:
        state = dict(cases["defaults"])
        grants, debits, pending = 0, 0, False
        for event in case["events"]:
            if event == "use":
                if admit(state) == "paid_pending":
                    debits += 1
                    pending = True
            elif event == "complete" and pending:
                state["active_owned"] = True
                state["count"] += 1
                state["busy"] = False
                pending = False
                grants += 1
            elif event == "cancel":
                state["busy"] = False
                pending = False
            else:
                raise AssertionError(f"{case['id']}: unsupported staged event {event}")
        actual = (state["points"], grants, debits, state["active_owned"], state["count"])
        expected = (
            case["balance"], case["grants"], case["debits"], case["owned"], case["count"]
        )
        require(actual == expected, case["id"], "staged observables")
        passed.append(case["id"])
    print(json.dumps({"status": "PASS", "case_count": len(passed), "cases": passed}))


if __name__ == "__main__":
    main()
