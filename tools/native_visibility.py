"""Preserve T6's authored DPVS cells, portals, trees and occluders."""
import json
import struct
import sys
from pathlib import Path


def export_visibility(world, assets):
    cells = world["cells"]
    cell_indices = {cell.address: index for index, cell in enumerate(cells)}
    surface_count = world["surfaceCount"]
    static_count = world["dpvs.staticSurfaceCount"]
    model_count = world["dpvs.smodelCount"]
    sorted_surfaces = [index for (index,) in world["dpvs.sortedSurfIndex"].unpack("<H")]
    assert len(sorted_surfaces) == static_count
    assert all(index < surface_count for index in sorted_surfaces)

    def bounds(record):
        return [list(record.unpack("<3f", field)) for field in ("mins", "maxs")]

    result = {"surfaceCount": surface_count, "modelCount": model_count,
              "sortedSurfaces": sorted_surfaces, "cells": [], "occluders": [],
              "surfaceBounds": [[list(surface.unpack("<3f", f"bounds[{i}]")) for i in (0, 1)]
                                for surface in world["dpvs.surfaces"]],
              "modelBounds": [bounds(model) for model in world["dpvs.smodelInsts"]]}
    for cell in cells:
        trees = []
        for index, tree in enumerate(cell["aabbTree"] or ()):
            offset = tree["childrenOffset"]
            child_count = tree["childCount"]
            assert not child_count or offset > 0 and offset % 40 == 0
            first = index + offset // 40 if child_count else 0
            assert not child_count or first + child_count <= cell["aabbTreeCount"]
            source = tree["smodelIndexes"]
            models = [i for (i,) in source.view(0, tree["smodelIndexCount"]).unpack("<H")] if source is not None else []
            assert len(models) == tree["smodelIndexCount"]
            assert all(i < model_count for i in models)
            start, count = tree["startSurfIndex"], tree["surfaceCount"]
            assert start + count <= static_count
            trees.append({"bounds": bounds(tree), "children": [first, child_count],
                          "surfaces": [start, count], "models": models})
        assert len(trees) == cell["aabbTreeCount"]
        portals = []
        for portal in cell["portals"] or ():
            vertices = [list(v) for v in portal["vertices"].unpack("<3f")]
            assert len(vertices) == (portal["vertexCount"] & 255) and len(vertices) >= 3
            portals.append({"cell": cell_indices[portal["cell"].address],
                            "plane": list(portal.unpack("<4f", "plane.coeffs")),
                            "vertices": vertices,
                            "hullAxes": [list(v) for v in struct.iter_unpack("<3f", portal.bytes("hullAxis"))],
                            "bounds": [list(v) for v in struct.iter_unpack("<3f", portal.bytes("bounds"))]})
        assert len(portals) == cell["portalCount"]
        result["cells"].append({"bounds": bounds(cell), "trees": trees, "portals": portals})
    for occluder in world["occluders"] or ():
        result["occluders"].append({"flags": occluder["flags"],
                                    "points": [list(v) for v in struct.iter_unpack("<3f", occluder.bytes("points"))]})
    assert len(result["cells"]) == world["dpvsPlanes.cellCount"]
    assert len(result["modelBounds"]) == model_count
    assert len(result["occluders"]) == world["numOccluders"]
    path = Path(assets) / "world_visibility.json"
    path.write_text(json.dumps(result, separators=(",", ":")), encoding="utf-8")
    print(f"Original visibility: {len(cells)} cells, {sum(len(c['portals']) for c in result['cells'])} portals, "
          f"{sum(len(c['trees']) for c in result['cells'])} tree nodes, {static_count} static surfaces, "
          f"{model_count} models, {len(result['occluders'])} occluders", flush=True)


if __name__ == "__main__":
    root, external, zone = map(Path, sys.argv[1:])
    sys.path.insert(0, str(external))
    from scripts.cod import backends
    walk = backends.for_fastfile(str(zone)).walk(str(zone))
    walk.check()
    asset, = walk.by_type("gfxworld")
    export_visibility(asset.record, root / "target/assets")
