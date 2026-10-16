import json
import unittest
from unittest.mock import patch
from client import Client, ValidationFailure


class RunnerTests(unittest.TestCase):
    def test_lost_reply_retries_same_command(self):
        client = Client()
        client.session = 7
        first_id = client.next_id
        calls = []
        def rpc(method, params):
            calls.append(json.loads(json.dumps(params)))
            if len(calls) == 1:
                raise OSError("Reply lost after acceptance")
            return {"request": params, "phase": "accepted"}
        with patch.object(client, "rpc", side_effect=rpc):
            client.send({"op": "pause"}, wait=False)
        self.assertEqual(calls[0], calls[1])
        self.assertEqual(client.next_id, first_id + 1)

    def test_accepted_command_is_not_finished(self):
        client = Client()
        state = {"control": {"failure": None, "aborted": False, "paused": False}}
        with patch.object(client, "rpc", return_value={"phase": "accepted"}), patch.object(client, "wait", side_effect=lambda predicate, **_: self.assertFalse(predicate(state))):
            client.send({"op": "pause"})

    def test_failed_command_reports_failure(self):
        client = Client()
        state = {"control": {"failure": None, "aborted": False, "paused": False}}
        def rpc(method, params):
            return {"phase": "failed", "error": "blocked"}
        with patch.object(client, "rpc", side_effect=rpc), patch.object(client, "wait", side_effect=lambda predicate, **_: predicate(state)):
            with self.assertRaisesRegex(ValidationFailure, "blocked"):
                client.send({"op": "input"})

    def test_heartbeat_thread_stops_with_its_client(self):
        client = Client()
        with patch.object(client, "rpc", return_value={}):
            client.beat_start()
            thread = client._heartbeat_thread
            self.assertTrue(thread.daemon)
            client.beat_stop()
            self.assertFalse(thread.is_alive())


if __name__ == "__main__":
    unittest.main()
