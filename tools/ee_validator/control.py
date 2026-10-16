"""Control the existing visible run without taking ownership or sending heartbeats."""
import argparse
import json
import time
from client import Client

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("action", choices=("pause", "resume", "step", "abort", "state", "quit"))
parser.add_argument("--port", type=int, default=15702)
args = parser.parse_args()
client = Client(args.port)
state = client.state()
if args.action == "state":
    print(json.dumps(state, indent=2))
else:
    client.session = state["session"]
    client.next_id = time.time_ns()
    print(json.dumps(client.send({"op": args.action}, allow_failure=True), indent=2))
