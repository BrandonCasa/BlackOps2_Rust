"""Separate Python heartbeat process, deliberately killable in the foundation contract."""
import sys
import time
from client import Client, ValidationFailure

client = Client(int(sys.argv[1]))
session = int(sys.argv[2])
while True:
    try:
        client.rpc("origins/validator/heartbeat", {"session": session})
    except ValidationFailure:
        try:
            session = client.state()["session"]
        except Exception:
            pass
    except OSError:
        pass
    time.sleep(0.5)
