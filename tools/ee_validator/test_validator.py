import json
import unittest
from unittest.mock import patch
from client import Client, ValidationFailure
from routes import tank_point, walk_to


class RunnerTests(unittest.TestCase):
    def walking_client(self, samples):
        from unittest.mock import Mock
        client = Mock()
        client.state.side_effect = [
            {"control": {"simulation_seconds": seconds, "failure": None, "aborted": False},
             "player": {"feet": [0, 0, z], "eye": [0, 1.6, z], "yaw": 0}}
            for seconds, z in samples
        ]
        client.send.return_value = {"request": {"id": 42}}
        return client

    def test_walking_holds_forward_until_arrival_including_across_pause(self):
        client = self.walking_client([(0, 0), (0.5, -2), (0.5, -2), (0.5, -2), (1, -4), (1.5, -5.5)])
        with patch("routes.time.sleep"):
            walk_to(client, [0, 0, -6])
        client.input.assert_not_called()
        actions = [call.args[0] for call in client.send.call_args_list]
        self.assertEqual(len(actions), 2)
        self.assertEqual(actions[0]["held"], ["forward"])
        self.assertFalse(client.send.call_args_list[0].kwargs["wait"])
        self.assertEqual(actions[1], {"op": "cancel", "command": 42})

    def test_blocked_walking_releases_forward(self):
        client = self.walking_client([(0, 0), (0.5, -1), (1.5, -1), (2.6, -1)])
        with patch("routes.time.sleep"), self.assertRaisesRegex(ValidationFailure, "blocked"):
            walk_to(client, [0, 0, -6])
        self.assertEqual(client.send.call_args.args[0], {"op": "cancel", "command": 42})

    def test_tank_waypoints_follow_the_current_rotated_platform(self):
        from math import sqrt
        tank = {"position": [10, 2, 30], "rotation": [0, sqrt(0.5), 0, sqrt(0.5)]}
        point = tank_point(tank, [2, 1, 0])
        for actual, expected in zip(point, [10, 3, 28]):
            self.assertAlmostEqual(actual, expected)

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
