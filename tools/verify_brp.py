import argparse
import hashlib
import itertools
import json
import math
from pathlib import Path
import queue
import shutil
import subprocess
import threading
import time
import urllib.parse

from gameplay_remote import Client, write_report


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def assets_match(identity):
    root = Path(identity["asset_root"]).resolve()
    actual = {path.relative_to(root).as_posix(): path for path in root.rglob("*") if path.is_file()}
    require(actual.keys() == identity["assets"].keys(), "Runtime asset paths changed")
    rehashed = 0
    for name, expected in identity["assets"].items():
        path = actual[name]
        before = path.stat()
        require(before.st_size == expected["size"], f"Runtime asset size changed: {name}")
        if before.st_mtime_ns != expected["mtime_ns"]:
            require(digest(path) == expected["sha256"], f"Runtime asset content changed: {name}")
            after = path.stat()
            require((before.st_size, before.st_mtime_ns) == (after.st_size, after.st_mtime_ns),
                    f"Runtime asset changed during verification: {name}")
            rehashed += 1
    return {"mode": "paths and size; SHA256 when mtime differs", "files": len(actual), "rehashed": rehashed}


class Mcp:
    def __init__(self, command, output):
        self.log = (output / "mcp-stderr.log").open("w", encoding="utf-8")
        try:
            self.process = subprocess.Popen(
                [command], cwd=output, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=self.log, text=True, encoding="utf-8", bufsize=1,
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
        except Exception:
            self.log.close()
            raise
        self.messages = queue.Queue()
        self.ids = itertools.count(1)
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        try:
            for line in self.process.stdout:
                self.messages.put(json.loads(line))
        except Exception as error:
            self.messages.put(error)
        finally:
            self.messages.put(EOFError("MCP stdout closed"))

    def send(self, message):
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", **message}) + "\n")
        self.process.stdin.flush()

    def call(self, method, params=None, timeout=30):
        identity = next(self.ids)
        self.send({"id": identity, "method": method, "params": params or {}})
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                message = self.messages.get(timeout=max(0.001, deadline - time.monotonic()))
            except queue.Empty as error:
                raise TimeoutError(f"MCP {method} timed out") from error
            if isinstance(message, Exception):
                raise message
            if "method" in message:
                if "id" in message:
                    response = {"result": {}} if message["method"] == "ping" else {
                        "error": {"code": -32601, "message": "Unsupported client request"}}
                    self.send({"id": message["id"], **response})
                continue
            require(message.get("id") == identity, "Unexpected MCP response identity")
            require("error" not in message, f"MCP {method}: {message.get('error')}")
            return message["result"]
        raise TimeoutError(f"MCP {method} timed out")

    def close(self):
        try:
            self.process.stdin.close()
        except BrokenPipeError:
            pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        self.process.stdout.close()
        self.log.close()
        return self.process.returncode


def mcp_case(command, output, port):
    executable = shutil.which(command)
    require(executable is not None, f"MCP command is unavailable: {command}")
    executable = Path(executable).resolve()
    result = {"binary": {"path": str(executable), "sha256": digest(executable)}}
    mcp = Mcp(str(executable), output)
    try:
        result["initialize"] = mcp.call("initialize", {
            "protocolVersion": "2025-03-26", "capabilities": {},
            "clientInfo": {"name": "origins-brp-smoke", "version": "1"}})
        mcp.send({"method": "notifications/initialized"})
        tools, cursor = [], None
        for _ in range(16):
            page = mcp.call("tools/list", {"cursor": cursor} if cursor else {})
            tools.extend(page["tools"])
            cursor = page.get("nextCursor")
            if not cursor:
                break
        require(not cursor, "MCP tools pagination exceeded its bound")
        execute = next((tool for tool in tools if tool["name"] == "brp_execute"), None)
        require(execute is not None, "MCP brp_execute is unavailable")
        properties = execute["inputSchema"]["properties"]
        require({"method", "port"} <= properties.keys(), "Unsupported brp_execute schema")
        result["tools"] = [tool["name"] for tool in tools]
        result["execute_schema"] = execute["inputSchema"]
        for method in ("origins/schema", "origins/state"):
            response = mcp.call("tools/call", {
                "name": execute["name"], "arguments": {"method": method, "port": port}})
            require(not response.get("isError"), f"MCP tool failed: {response}")
            data = response.get("structuredContent")
            if data is None:
                data = json.loads(next(block["text"] for block in response["content"]
                                       if block.get("type") == "text"))
            require(data.get("status") == "success", f"MCP BRP failed: {data}")
            value = data["result"]
            require("methods" in value if method.endswith("schema") else "fixed_tick" in value,
                    f"Unexpected MCP BRP result: {value}")
            result[method] = data
        return result
    finally:
        result["exit_code"] = mcp.close()


def run(client, report, args, output):
    report["ready"] = client.until(lambda s: s["state"] == "Playing" and s.get("player"), 120)
    discovery = client.call("rpc.discover")
    methods = {method["name"] for method in discovery["methods"]}
    require("registry.schema" in methods, "Discovered methods lack registry.schema")
    schema = client.call("origins/schema")
    registry = client.call("registry.schema")
    require(isinstance(registry, dict) and registry, "Empty reflected type registry")
    report["discovery"] = {"protocol": discovery, "gameplay": schema,
                           "reflected_types": sorted(registry), "input_schemas": {
                               name: value for name, value in registry.items()
                               if name.rsplit("::", 1)[-1] in (
                                   "KeyCode", "KeyboardInput", "MouseButtonInput", "WindowEvent")}}
    report["mcp"] = mcp_case(args.mcp_command, output, urllib.parse.urlparse(args.url).port or 80)
    client.reset()
    client.wait_ticks(12)
    before = client.state()
    key = next((binding["key"] for binding in before["controls"]["bindings"]["forward"]
                if isinstance(binding, dict) and "key" in binding), None)
    require(key is not None, "Forward has no native keyboard binding")
    batch = client.call("origins/input", {"events": [{"tick": 1, "key": key, "pressed": True},
                                                     {"tick": 24, "key": key, "pressed": True}]})
    after = client.until(lambda s: s["fixed_tick"] >= batch["finish_tick"]
                         and not s["held_inputs"] and not s["queued_inputs"])
    start, end = before["player"]["position"], after["player"]["position"]
    distance = math.hypot(end[0] - start[0], end[2] - start[2])
    yaw = before["player"]["yaw"]
    forward = -(end[0] - start[0]) * math.sin(yaw) - (end[2] - start[2]) * math.cos(yaw)
    report["movement"] = {"before": before, "after": after, "key": key,
                           "horizontal_distance": distance, "forward_distance": forward}
    require(forward > 0.15, f"Native {key} input did not move player forward: {forward}")
    stopped = client.wait_ticks(12)
    residual = math.hypot(stopped["player"]["position"][0] - end[0],
                          stopped["player"]["position"][2] - end[2])
    report["movement"].update(stopped=stopped, residual_distance=residual)
    require(residual < 0.15, f"Movement continued after automatic release: {residual}")
    client.reset()
    try:
        client.call("origins/input", {"events": [{"tick": 3600, "key": "KeyW", "pressed": True},
                                                 {"tick": 0, "key": "KeyW", "pressed": False}]})
    except RuntimeError as error:
        require("-32602" in str(error), f"Unexpected invalid-input error: {error}")
        report["invalid_input"] = {"rejection": str(error)}
    else:
        raise RuntimeError("Invalid input batch was accepted")
    state = client.wait_ticks(3)
    report["invalid_input"]["after"] = state
    require(not state["held_inputs"] and not state["queued_inputs"], "Invalid batch partially queued")
    client.reset()
    require("origins/input" in schema["methods"], "Native input hook is unavailable")
    report["pause_resume"] = []
    for expected in ("Paused", "Playing"):
        batch = client.call("origins/input", {"events": [
            {"tick": 1, "key": "Escape", "pressed": True},
            {"tick": 2, "key": "Escape", "pressed": False}]})
        state = client.until(lambda s: s["state"] == expected
                             and s["fixed_tick"] >= batch["finish_tick"]
                             and not s["held_inputs"] and not s["queued_inputs"])
        report["pause_resume"].append(state)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default="http://127.0.0.1:15702")
    parser.add_argument("--output", required=True)
    parser.add_argument("--identity", required=True)
    parser.add_argument("--mcp-command", default="bevy_brp_mcp")
    args = parser.parse_args()
    client = Client(args.url)
    output = Path(args.output).resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parent.parent), "Output must be external")
    output.mkdir(parents=True, exist_ok=True)
    identity_path = Path(args.identity).resolve()
    identity = json.loads(identity_path.read_text(encoding="utf-8"))
    report = {"passed": False, "url": args.url, "identity_path": str(identity_path),
              "identity": identity, "started_unix": time.time()}
    try:
        for name, expected in identity["binaries"].items():
            require(Path(name).name == name, "Invalid pinned binary name")
            require(digest(identity_path.parent / name) == expected, f"Pinned binary changed: {name}")
        report["runtime_asset_check"] = assets_match(identity)
        run(client, report, args, output)
        report["passed"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if "ready" in report:
            try:
                client.release()
                report["final_reset"] = client.reset()
                client.release()
                report["final_state"] = client.wait_ticks(2)
            except Exception as error:
                report["cleanup_error"] = f"{type(error).__name__}: {error}"
                report["passed"] = False
        report["finished_unix"] = time.time()
        path = write_report(output, "brp-smoke.json", report)
        print(json.dumps({"passed": report["passed"], "report": str(path), "error": report.get("error")}))
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
