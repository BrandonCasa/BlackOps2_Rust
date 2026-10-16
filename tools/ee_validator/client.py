"""Small JSON-RPC client. Transport retries always reuse the original command ID."""
from __future__ import annotations

import json
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path


class ValidationFailure(RuntimeError):
    pass


class Client:
    def __init__(self, port: int = 15702, report: Path | None = None):
        self.url = f"http://127.0.0.1:{port}"
        self.port = port
        self.report = report
        self.session = 0
        self.next_id = 1
        self.heartbeat_process: subprocess.Popen | None = None
        self.timeline = []

    def rpc(self, method: str, params=None, timeout=5):
        body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
        request = urllib.request.Request(self.url, body, {"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=timeout) as response:
            result = json.load(response)
        if "error" in result:
            raise ValidationFailure(str(result["error"]))
        return result["result"]

    def state(self):
        return self.rpc("origins/validator/state")

    def wait(self, predicate, *, seconds=30, allow_failure=False):
        deadline = time.monotonic() + seconds
        last = None
        while time.monotonic() < deadline:
            last = self.state()
            if not allow_failure and (last["control"]["failure"] or last["control"]["aborted"]):
                raise ValidationFailure(last["control"]["failure"] or "Run aborted")
            if predicate(last):
                return last
            time.sleep(0.1)
        raise ValidationFailure(f"Wait timed out after {seconds}s; last state: {last}")

    def send(self, action, *, label="", expect=None, wait=True, allow_failure=False):
        request = {"session": self.session, "id": self.next_id, "action": action, "label": label, "expect": expect}
        self.next_id += 1
        for attempt in range(3):
            try:
                receipt = self.rpc("origins/validator/command", request)
                break
            except (OSError, urllib.error.URLError):
                if attempt == 2:
                    raise
        if not wait:
            return receipt
        goal = "outcome_observed" if expect else "executed"
        def complete(state):
            nonlocal receipt
            receipt = next(r for r in state["receipts"] if r["request"]["session"] == request["session"] and r["request"]["id"] == request["id"])
            if receipt["phase"] in ("failed", "rejected", "cancelled"):
                raise ValidationFailure(f"Command {request['id']}: {receipt['phase']}: {receipt.get('error')}")
            return receipt["phase"] == goal
        self.wait(complete, seconds=600 if action["op"] == "start" else 90, allow_failure=allow_failure)
        self.timeline.append(receipt)
        self.flush()
        return receipt

    def input(self, ticks=1, held=(), *, aim=None, aim_entity=None, look=(0, 0), expect=None, label=""):
        return self.send({"op": "input", "ticks": ticks, "held": list(held), "aim": aim,
                          "aim_entity": aim_entity, "look": list(look)}, label=label, expect=expect)

    def beat_start(self):
        self.beat_stop()
        self.heartbeat_process = subprocess.Popen(
            [sys.executable, str(Path(__file__).with_name("heartbeat.py")), str(self.port), str(self.session)],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))

    def beat_stop(self):
        if self.heartbeat_process is not None:
            self.heartbeat_process.terminate()
            self.heartbeat_process.wait(timeout=5)
            self.heartbeat_process = None

    def flush(self):
        if self.report:
            (self.report / "commands.json").write_text(json.dumps(self.timeline, indent=2), encoding="utf-8")


def changed(path, before, equals=None, increase=None, seconds=30):
    return {"path": path, "before": before, "equals": equals, "increase": increase, "seconds": seconds}
