import argparse
import http.client
import itertools
import json
from pathlib import Path
import socket
import time
import urllib.error
import urllib.parse
import urllib.request


class Client:
    def __init__(self, url="http://127.0.0.1:15702"):
        parsed = urllib.parse.urlparse(url)
        if parsed.scheme != "http" or parsed.hostname != "127.0.0.1":
            raise ValueError("Validation requires an HTTP loopback endpoint")
        self.url = url
        self.ids = itertools.count(1)

    def call(self, method, params=None):
        identity = next(self.ids)
        body = {"jsonrpc": "2.0", "id": identity, "method": method}
        if params is not None:
            body["params"] = params
        request = urllib.request.Request(
            self.url, json.dumps(body).encode(), {"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(request, timeout=5) as response:
            message = json.load(response)
        if message.get("id") != identity:
            raise RuntimeError("Mismatched BRP response")
        if "error" in message:
            raise RuntimeError(f"{method}: {message['error']}")
        return message["result"]

    def state(self):
        return self.call("origins/state")

    def until(self, predicate, timeout=30):
        deadline = time.monotonic() + timeout
        last = None
        while time.monotonic() < deadline:
            try:
                last = self.state()
                if predicate(last):
                    return last
            except (urllib.error.URLError, TimeoutError, ConnectionError) as error:
                last = str(error)
            time.sleep(0.025)
        raise TimeoutError(f"Gameplay condition timed out after {timeout}s: {last}")

    def wait_ticks(self, count, timeout=30):
        target = self.state()["fixed_tick"] + count
        return self.until(lambda state: state["fixed_tick"] >= target, timeout)

    def release(self):
        return self.call("origins/release")

    def quit(self, timeout=15):
        endpoint = urllib.parse.urlparse(self.url)
        address = (endpoint.hostname, endpoint.port or 80)
        with socket.create_connection(address, timeout=3):
            pass
        try:
            result = self.call("origins/quit")
        except (TimeoutError, ConnectionError, urllib.error.URLError, http.client.RemoteDisconnected) as error:
            result = {"exit_requested": None, "response_error": f"{type(error).__name__}: {error}"}
        else:
            if not result.get("exit_requested"):
                raise RuntimeError(f"Game did not accept shutdown: {result}")
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                with socket.create_connection(address, timeout=min(3, max(0.01, deadline - time.monotonic()))):
                    pass
            except ConnectionRefusedError:
                return {**result, "endpoint_closed": True}
            except (TimeoutError, ConnectionResetError):
                pass
            time.sleep(0.05)
        raise TimeoutError(f"BRP endpoint remained open after shutdown: {self.url}")

    def shutdown(self, report):
        try:
            report["shutdown"] = self.quit()
        except Exception as error:
            report["passed"] = False
            report["shutdown_error"] = f"{type(error).__name__}: {error}"
            report.setdefault("error", report["shutdown_error"])

    def reset(self, timeout=30):
        before = (self.state().get("player") or {}).get("entity")
        self.release()
        self.call("origins/reset")
        return self.until(
            lambda state: state["state"] == "Playing"
            and state.get("player")
            and state["player"]["entity"] != before,
            timeout,
        )


def write_report(output, name, report):
    root = Path(__file__).resolve().parent.parent
    output = Path(output).resolve()
    if output.is_relative_to(root) or Path(name).name != name:
        raise ValueError("Reports must use an external directory and a plain filename")
    output.mkdir(parents=True, exist_ok=True)
    target = output / name
    target.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return target


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default="http://127.0.0.1:15702")
    parser.add_argument("command", choices=("discover", "state", "reset", "release", "quit"))
    args = parser.parse_args()
    client = Client(args.url)
    if args.command == "discover":
        result = {
            "protocol": client.call("rpc.discover"),
            "gameplay": client.call("origins/schema"),
        }
    elif args.command == "reset":
        result = client.reset()
    elif args.command == "quit":
        result = client.quit()
    else:
        result = client.call("origins/" + args.command)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
