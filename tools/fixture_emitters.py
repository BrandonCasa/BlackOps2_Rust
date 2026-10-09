import json
import os
from pathlib import Path
import tempfile

import numpy as np

FIXTURE_SURFACES = {
    "p6_zm_tm_gas_lamp_straight_on": "mc/mtl_p6_zm_tm_gas_lamp_glass_on",
    "p6_zm_tm_hanging_light_fixture_on": "mc/mtl_p6_zm_tm_hanging_light_fixture_on",
    "p6_zm_tm_lantern_01_on": "mc/mtl_p6_zm_tm_lantern_glass_on_01",
    "p6_zm_tm_portable_lantern_on": "mc/mtl_p6_zm_tm_gas_lamp_glass_on",
    "p6_zm_tm_robot_lamp": "mc/mtl_p6_zm_tm_lantern_glass_on_01",
}


def emitter_surface(model, surfaces):
    material = FIXTURE_SURFACES.get(model)
    if material is None:
        return None
    matches = [i for i, surface in enumerate(surfaces) if surface["material"] == material]
    if len(matches) != 1:
        raise ValueError(f"Fixture {model} requires one emitting glass surface, got {len(matches)}")
    return matches[0]


def prepare(config):
    from prepare_origins import external
    root = external(Path(config["assets"]["inspection_output"]))
    settings = config.get("preparation", {})
    instances_path = external(Path(settings.get("instances", root / "game-prepare/instances/static-instances.json")))
    instances = json.loads(instances_path.read_text())
    surfaces = json.loads((instances_path.parent / "model-surfaces.json").read_text())
    path = external(Path(config["game"]["asset_pack"]) / "map.gltf")
    document = json.loads(path.read_text(encoding="utf-8-sig"))
    nodes = [node for node in document["nodes"] if node.get("name") == "static_instance"]
    cursor, emitted = 0, 0
    basis = np.array([[1, 0, 0], [0, 0, 1], [0, -1, 0]], dtype=float)
    for item in instances:
        model = item["model"]
        definitions = surfaces[model]
        selected = emitter_surface(model, definitions)
        placed = nodes[cursor:cursor + len(definitions)]
        if len(placed) != len(definitions):
            raise ValueError("Missing source fixture placement")
        matrix = np.eye(4)
        matrix[:3, :3] = basis @ np.asarray(item["axis"]).reshape(3, 3).T @ basis.T * item["scale"]
        matrix[:3, 3] = np.asarray(item["origin"])[[0, 2, 1]] * [0.0254, 0.0254, -0.0254]
        for index, (node, surface) in enumerate(zip(placed, definitions)):
            mesh = document["meshes"][node["mesh"]]
            if not np.allclose(node["matrix"], matrix.T.flatten(), rtol=1e-7, atol=1e-7):
                raise ValueError("Fixture placement differs from retained source")
            if len(mesh["primitives"]) != 1 or document["materials"][mesh["primitives"][0]["material"]]["name"] != surface["material"]:
                raise ValueError("Fixture primitive differs from retained source")
            node["extras"].update(source_model=model, fixture_emitter=index == selected)
        emitted += selected is not None
        cursor += len(placed)
    if cursor != len(nodes):
        raise ValueError("Extra source fixture placements")
    descriptor, temporary = tempfile.mkstemp(prefix=".fixture-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            stream.write(json.dumps(document, separators=(",", ":")) + "\n")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    print(f"Retained {len(instances)} fixtures / {cursor} surfaces; {emitted} analytic emitters. Geometry, materials and textures unchanged.")
