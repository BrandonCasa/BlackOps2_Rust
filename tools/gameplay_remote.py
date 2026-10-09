import argparse
import itertools
import json
from pathlib import Path
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
    else:
        result = client.call("origins/" + args.command)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
