import json
import unittest
from unittest.mock import patch
from client import Client, ValidationFailure
from routes import tank_point, walk_to, shoot_at


class RunnerTests(unittest.TestCase):
    def test_shooting_queues_native_cadence_without_round_trips_between_shots(self):
        from unittest.mock import Mock
        for speed in [1, 10]:
            client = Mock()
            client.state.return_value = {"speed":speed, "firing":{"interval":.096,"automatic":False,"magazine":8,"ready":True,"aim_seconds":.2},
                "player":{"eye":[0,1.62,0],"yaw":0,"pitch":0},
                "enemies":[{"entity":42,"position":[0,0,-5],"aim":[0,1.62,-5]}]}
            shoot_at(client, 42)
            client.send.assert_called_once()
            steps = client.send.call_args.args[0]
            self.assertEqual([(s["ticks"],s["held"]) for s in steps[1:-1]], [(1,["aim","fire"]),(5,["aim"])]*4)
            self.assertEqual(steps[-1]["held"],[])
            self.assertTrue(all(s["aim_entity"]==42 for s in steps))
            client.input.assert_not_called()

    def test_shooting_waits_for_native_reload_completion_before_queuing_fire(self):
        from unittest.mock import Mock
        client = Mock()
        busy = {"firing": {"ready":False,"magazine":0}}
        loaded = {"firing":{"ready":True,"magazine":8,"interval":.096,"automatic":False,"aim_seconds":.2},
                  "player":{"eye":[0,1.62,0],"yaw":0,"pitch":0},
                  "enemies":[{"entity":42,"position":[0,1.62,-5],"aim":[0,1.62,-5]}]}
        client.state.return_value = busy
        client.wait.return_value = loaded
        shoot_at(client, 42)
        predicate = client.wait.call_args.args[0]
        self.assertFalse(predicate(busy))
        self.assertTrue(predicate(loaded))
        client.input.assert_not_called()
        client.send.assert_called_once()
        self.assertEqual(sum("fire" in step["held"] for step in client.send.call_args.args[0]), 4)

    def walking_client(self, samples):
        from unittest.mock import Mock
        client = Mock()
        client.state.side_effect = [
            {"control": {"simulation_seconds": seconds, "failure": None, "aborted": False, "active": {"id": 42}, "queued": 0},
             "player": {"feet": [0, 0, z], "eye": [0, 1.6, z], "yaw": 0}}
            for seconds, z in samples
        ]
        client.send.return_value = {"request": {"id": 42}}
        client.receipt.return_value = {"phase": "accepted"}
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

    def test_facing_a_tank_waypoint_tracks_the_platform_before_walking(self):
        from unittest.mock import Mock
        client = Mock()
        control = {"simulation_seconds":0,"failure":None,"aborted":False,"active":None,"queued":0}
        client.state.side_effect = [
            {"control":control,"player":{"feet":[10,2,0],"eye":[10,3.62,0],"yaw":0},
             "tank":{"entity":77,"position":[10,0,0],"rotation":[0,0,0,1]}},
            {"control":control,"player":{"feet":[13,2,0],"eye":[13,3.62,0],"yaw":-1.57},
             "tank":{"entity":77,"position":[11,0,0],"rotation":[0,0,0,1]}},
        ]
        client.send.return_value = {"request":{"id":42}}
        client.receipt.return_value = {"phase":"accepted"}
        walk_to(client,[2,2,0],on_tank=True)
        preface = client.send.call_args_list[0].args[0]
        self.assertEqual(preface["aim_entity"],77)
        self.assertEqual(preface["aim_offset"],[2,3.62,0])
        self.assertNotIn("aim",preface)

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

    def test_lost_batch_reply_reuses_all_ids(self):
        client = Client()
        first_id = client.next_id
        calls = []
        def rpc(method, params):
            calls.append((method, json.loads(json.dumps(params))))
            if len(calls) == 1:
                raise OSError("Batch accepted but reply lost")
            return [{"request": request, "phase": "accepted"} for request in params]
        with patch.object(client, "rpc", side_effect=rpc):
            result = client.send([{"op":"input","ticks":8,"held":["forward","sprint"]},
                                  {"op":"input","ticks":18,"held":["forward","sprint","jump"]}], wait=False)
        self.assertEqual(calls[0], calls[1])
        self.assertEqual([r["id"] for r in calls[0][1]], [first_id, first_id + 1])
        self.assertEqual(result["request"]["id"], first_id + 1)

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
