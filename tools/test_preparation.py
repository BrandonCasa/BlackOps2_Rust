# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///
"""Synthetic preparation checks; no licensed fixtures or original-game invocation."""
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("prepare", Path(__file__).with_name("prepare_origins.py"))
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class PreparationTests(unittest.TestCase):
    def test_geometry_accounting_and_coordinate_basis(self):
        vertices = struct.pack("<9f", 0, 0, 0, 1, 0, 0, 0, 1, 0)
        indexes = struct.pack("<3I", 0, 1, 2)
        surfaces = struct.pack("<QQII6f", 0, 3, 0, 3, 0, 0, 0, 1, 1, 0)
        length = 64 + len(vertices) + len(indexes) + len(surfaces)
        header = b"BO2DGS\0\0" + struct.pack("<II4Q", 1, 0, 3, 3, 1, length) + bytes(16)
        points, records = prepare.geometry(header + vertices + indexes + surfaces)
        self.assertEqual(len(points), 3)
        self.assertEqual(records[0][:2], (0, 3))
        self.assertAlmostEqual(float(points[1, 0]), 0.0254)
        self.assertAlmostEqual(float(points[2, 2]), -0.0254)
        with self.assertRaises(ValueError):
            prepare.geometry(header + vertices + struct.pack("<3I", 0, 1, 99) + surfaces)
        with self.assertRaises(ValueError):
            prepare.geometry((header + vertices + indexes + surfaces)[:-1])

    def test_asset_dependencies_cannot_escape(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            with self.assertRaises(ValueError):
                prepare.dependency(root, {"file": "../escaped.bin"})
            path = root / "data.bin"
            path.write_bytes(b"synthetic")
            with self.assertRaises(ValueError):
                prepare.dependency(root, {"file": path.name, "sha256": "incorrect"})

    def test_shadow_helper_and_collision_dispositions(self):
        import numpy as np
        gltf = prepare.Gltf()
        gltf.document["materials"].append({})
        points = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0]])
        gltf.mesh([("POSITION", points, "VEC3")], 0, "surface")
        gltf.mesh([("POSITION", points, "VEC3")], 0, "helper", visible=False)
        self.assertEqual(len(gltf.document["scenes"][0]["nodes"]), 1)
        self.assertEqual(len(gltf.document["scenes"][1]["nodes"]), 2)
        primitive = gltf.document["meshes"][0]["primitives"][0]
        self.assertEqual(gltf.document["accessors"][primitive["indices"]]["componentType"], 5125)
        with tempfile.TemporaryDirectory() as folder:
            gltf.save(Path(folder))
            document = json.loads((Path(folder) / "map.gltf").read_text())
            self.assertEqual(document["buffers"][0]["byteLength"], len(gltf.binary))


if __name__ == "__main__":
    unittest.main()
