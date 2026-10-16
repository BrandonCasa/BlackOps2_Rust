import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from client import Client, ValidationFailure, changed
from contracts import Contract


class ReceiptTests(unittest.TestCase):
    def test_lost_acknowledgement_retries_the_exact_command(self):
        client = Client()
        client.session = 7
        calls = []
        def rpc(method, params):
            calls.append(json.loads(json.dumps(params)))
            if len(calls) == 1:
                raise OSError("Reply lost after server accepted command")
            return {"request": params, "phase": "accepted"}
        with patch.object(client, "rpc", side_effect=rpc):
            client.send({"op": "pause"}, wait=False)
        self.assertEqual(calls[0], calls[1])
        self.assertEqual(client.next_id, 2)

    def test_accepted_is_not_an_observed_outcome(self):
        client = Client()
        client.session = 3
        request = {"session": 3, "id": 1, "action": {"op": "input"}, "label": "", "expect": changed("/kills", 0, increase=1)}
        receipt = {"request": request, "phase": "executed"}
        with patch.object(client, "rpc", return_value=receipt), patch.object(client, "state", return_value={"control": {"failure": None, "aborted": False}, "receipts": [receipt]}), patch("client.time.sleep"):
            with patch.object(client, "wait", side_effect=lambda predicate, **_: self.assertFalse(predicate(client.state()))):
                client.send(request["action"], expect=request["expect"])


class ContractTests(unittest.TestCase):
    def test_missing_source_function_cannot_be_recorded_as_a_contract(self):
        contract = Contract("sample", "source.gsc", "required_function", (), "interact", (), "changed", "30s", ())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "source.gsc").write_text("another_function() {}", encoding="utf-8")
            with self.assertRaises(ValidationFailure):
                contract.record(root)
            (root / "source.gsc").write_text("required_function() {}", encoding="utf-8")
            self.assertEqual(contract.record(root)["source_path"], str(root / "source.gsc"))


if __name__ == "__main__":
    unittest.main()
