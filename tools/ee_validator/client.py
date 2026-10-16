"""Small JSON-RPC client. Transport retries always reuse the original command ID."""
from __future__ import annotations

import json
import threading
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
        self.next_id = time.time_ns()
        self._heartbeat_stop = threading.Event()
        self._heartbeat_thread = None
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

    def receipt(self, command, *, session=None):
        return self.rpc("origins/validator/receipt", {"session": self.session if session is None else session, "id": command})

    def wait(self, predicate, *, seconds=30, allow_failure=False, realtime=False):
        remaining = seconds
        previous = time.monotonic()
        last = None
        while remaining > 0:
            last = self.state()
            now = time.monotonic()
            if realtime or not last["control"]["paused"]:
                remaining -= now - previous
            previous = now
            if not allow_failure and (last["control"]["failure"] or last["control"]["aborted"]):
                raise ValidationFailure(last["control"]["failure"] or "Run aborted")
            if predicate(last):
                return last
            time.sleep(0.1)
        raise ValidationFailure(f"Wait timed out after {seconds}s; last state: {last}")

    def send(self, action, *, label="", wait=True, allow_failure=False):
        request = {"session": self.session, "id": self.next_id, "action": action, "label": label}
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
        goal = "executed"
        def complete(state):
            nonlocal receipt
            receipt = self.receipt(request["id"], session=request["session"])
            if receipt["phase"] in ("failed", "rejected", "cancelled"):
                raise ValidationFailure(f"Command {request['id']}: {receipt['phase']}: {receipt.get('error')}")
            return receipt["phase"] == goal
        self.wait(complete, seconds=600 if action["op"] == "start" else 90, allow_failure=allow_failure, realtime=action["op"] in ("start", "pause", "resume", "step", "abort", "finish", "capture", "quit"))
        self.timeline.append(receipt)
        self.flush()
        return receipt

    def input(self, ticks=1, held=(), *, aim=None, aim_entity=None, look=(0, 0), label=""):
        return self.send({"op": "input", "ticks": ticks, "held": list(held), "aim": aim,
                          "aim_entity": aim_entity, "look": list(look)}, label=label)

    def beat_start(self):
        self.beat_stop()
        self._heartbeat_stop.clear()
        def heartbeat():
            while not self._heartbeat_stop.is_set():
                try:
                    self.rpc("origins/validator/heartbeat", {"session": self.session}, timeout=1)
                except ValidationFailure:
                    try:
                        self.session = self.state()["session"]
                    except (OSError, ValidationFailure):
                        pass
                except OSError:
                    pass
                self._heartbeat_stop.wait(0.5)
        self._heartbeat_thread = threading.Thread(target=heartbeat, daemon=True)
        self._heartbeat_thread.start()

    def beat_stop(self):
        self._heartbeat_stop.set()
        if self._heartbeat_thread:
            self._heartbeat_thread.join(timeout=6)
            self._heartbeat_thread = None

    def flush(self):
        if self.report:
            (self.report / "commands.json").write_text(json.dumps(self.timeline, indent=2), encoding="utf-8")
