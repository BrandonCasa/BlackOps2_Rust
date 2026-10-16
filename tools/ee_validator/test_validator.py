import json
import unittest
from unittest.mock import patch
from client import Client, ValidationFailure
from routes import tank_point, walk_to, shoot_at


class RunnerTests(unittest.TestCase):
    def test_expected_staging_rejection_is_returned_only_when_explicitly_allowed(self):
        client = Client()
        state = {"control": {"failure": None, "aborted": False, "paused": False}}
        receipt = {"phase": "rejected", "error": "Destination occupied"}
        with patch.object(client, "rpc", return_value=receipt), patch.object(client, "wait", side_effect=lambda predicate, **_: predicate(state)):
            result = client.send({"op": "teleport", "feet": [1, 2, 3]}, allow_rejection=True)
            self.assertEqual(result, receipt)
            with self.assertRaisesRegex(ValidationFailure, "Destination occupied"):
                client.send({"op": "teleport", "feet": [1, 2, 3]})

    def waiting_client(self, samples):
        from unittest.mock import Mock
        client = Client()
        client.state = Mock(side_effect=[{"session":1,"game_state":"Paused" if paused else "Playing",
            "control":{"simulation_seconds":seconds,"paused":paused,"failure":None,"aborted":False,
                       "tick":int(seconds*60),"label":"Portal approach"},
            "player":{"feet":[1,2,3]},"enemies":["unused world data" * 1000]}
            for seconds,paused in samples])
        return client

    def test_wait_deadline_uses_simulation_time_at_accelerated_speed(self):
        client = self.waiting_client([(0,False),(2,False)])
        with patch("client.time.sleep"), patch("client.time.monotonic", return_value=0):
            with self.assertRaisesRegex(ValidationFailure, "tick 120, feet") as error:
                client.wait(lambda s:False,seconds=1)
        self.assertEqual(client.state.call_count,2)
        self.assertLess(len(str(error.exception)),300)

    def test_paused_wait_does_not_consume_gameplay_deadline(self):
        client = self.waiting_client([(0,False),(0,True),(0,True),(.5,False),(1.1,False)])
        with patch("client.time.sleep"), patch("client.time.monotonic", return_value=1000):
            with self.assertRaises(ValidationFailure):
                client.wait(lambda s:False,seconds=1)
        self.assertEqual(client.state.call_count,5)

    def test_real_time_wait_expires_even_when_gameplay_is_paused(self):
        client = self.waiting_client([(0,True),(0,True)])
        with patch("client.time.sleep"), patch("client.time.monotonic", side_effect=[0,.6,1.2]):
            with self.assertRaises(ValidationFailure):
                client.wait(lambda s:False,seconds=1,realtime=True)
        self.assertEqual(client.state.call_count,2)

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

    def test_portal_transit_releases_walking_before_testing_the_new_world_position(self):
        from unittest.mock import Mock
        client = Mock()
        def sample(feet, trips):
            return {"control":{"simulation_seconds":trips,"failure":None,"aborted":False,
                               "active":{"id":42},"queued":0},
                    "player":{"feet":feet,"eye":[0,1.62,0],"yaw":0},"portals":{"trips":trips}}
        client.state.side_effect = [sample([0,0,0],0),sample([200,-10,200],1)]
        client.send.return_value = {"request":{"id":42}}
        client.receipt.return_value = {"phase":"accepted"}
        walk_to(client,[0,0,-6],through_portal=True)
        self.assertEqual(client.send.call_args.args[0],{"op":"cancel","command":42})

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
