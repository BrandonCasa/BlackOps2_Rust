import json
import math
import numpy as np

REGIONS = {
    "t6_wpn_zmb_mc96_view": (.25, .278, .060, .075),
    "t6_wpn_ar_m14_view": (.65, .70, .045, .060),
    "t6_wpn_smg_mp40_view": (.68, .70, .11, .125),
    "t6_wpn_shotty_870mcs_view": (.32, .35, .105, .115),
    "t6_wpn_pistol_b2023r_view": (.175, .19, .055, .065),
}


def quaternion(q):
    x, y, z, w = q
    return np.array([[1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w), 0],
                     [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w), 0],
                     [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y), 0],
                     [0, 0, 0, 1.]])


def local(node):
    result = quaternion(node.get("rotation", [0., 0., 0., 1.]))
    result[:3, :3] *= node.get("scale", [1., 1., 1.])
    result[:3, 3] = node.get("translation", [0., 0., 0.])
    return result


def values(document, buffers, index):
    accessor = document["accessors"][index]
    view = document["bufferViews"][accessor["bufferView"]]
    width = {"SCALAR": 1, "VEC3": 3, "VEC4": 4, "MAT4": 16}[accessor["type"]]
    dtype = {5126: "<f4", 5123: "<u2", 5121: "u1"}.get(accessor["componentType"])
    if dtype is None or "byteStride" in view:
        raise ValueError("unsupported ADS geometry/animation accessor")
    return np.frombuffer(buffers[view["buffer"]], dtype, count=accessor["count"]*width,
                         offset=view.get("byteOffset", 0)+accessor.get("byteOffset", 0)).reshape(-1, width)


def load(pack, model, clip):
    path = pack / "actors" / (model + "_lod0.gltf")
    document = json.loads(path.read_text())
    buffers = [(path.parent / b["uri"]).read_bytes() for b in document["buffers"]]
    nodes = [dict(n) for n in document["nodes"]]
    animation = next((a for a in document.get("animations", []) if a["name"] == clip), None)
    if animation:
        for channel in animation["channels"]:
            sampler = animation["samplers"][channel["sampler"]]
            nodes[channel["target"]["node"]][channel["target"]["path"]] = values(document, buffers, sampler["output"])[0].tolist()
    return document, buffers, nodes


def globals_of(nodes, parent, torso=None):
    result = {}
    def visit(index, frame):
        node = nodes[index]
        pose = local(node)
        if node.get("name") == "tag_torso" and torso:
            pose[:3, 3] += torso["translation"]
            rotation = quaternion(torso["rotation"])
            pose[:3, :3] = pose[:3, :3] @ rotation[:3, :3]
        result[index] = frame @ pose
        for child in node.get("children", []):
            visit(child, result[index])
    children = {c for n in nodes for c in n.get("children", [])}
    for root in set(range(len(nodes))) - children:
        visit(root, parent)
    return {nodes[i].get("name"): p for i, p in result.items()}


def optics(fields, pack, curve, identity):
    model = fields.get("gunModel")
    if identity not in {"c96_zm", "m14_zm", "mp40_zm", "870mcs_zm", "beretta93r_zm"} or model not in REGIONS or not curve:
        return None
    if not (pack / "actors" / (model + "_lod0.gltf")).is_file():
        return None
    document, buffers, gun = load(pack, model, fields["idleAnim"])
    primitives = [p for m in document["meshes"] for p in m["primitives"]]
    points = np.concatenate([values(document, buffers, p["attributes"]["POSITION"]) for p in primitives])
    weights = np.concatenate([values(document, buffers, p["attributes"]["WEIGHTS_0"]) for p in primitives])
    joints = np.concatenate([values(document, buffers, p["attributes"]["JOINTS_0"]) for p in primitives])
    lo, hi, bottom, top = REGIONS[model]
    mask = ((points[:, 0] >= lo) & (points[:, 0] <= hi)
                        & (points[:, 1] >= bottom) & (points[:, 1] <= top)
                        & (abs(points[:, 2]) < .004))
    candidates = points[mask]
    if not len(candidates):
        raise ValueError("authored ADS sight region is empty: " + model)
    height = candidates[:, 1].max()
    edge_mask = mask & (abs(points[:, 1] - height) < .000001)
    edge = points[edge_mask]
    if not np.allclose(weights[edge_mask, 0], 1.):
        raise ValueError("ADS post does not have a rigid skin binding: " + model)
    binding = joints[edge_mask, 0]
    if not np.all(binding == binding[0]) or len(document["skins"]) != 1:
        raise ValueError("ambiguous ADS post joint: " + model)
    skin = document["skins"][0]
    joint = document["nodes"][skin["joints"][int(binding[0])]]["name"]
    inverse = values(document, buffers, skin["inverseBindMatrices"])[int(binding[0])].reshape(4, 4).T
    point = inverse @ np.array([float(edge[:, 0].mean()), float(height), 0., 1.])
    _, _, arms = load(pack, "c_zom_dempsey_viewhands", fields["idleAnim"])
    half = math.sqrt(.5)
    frame = globals_of(arms, quaternion([0., half, 0., half]), curve[-1])
    posed = globals_of(gun, frame["tag_weapon"] @ quaternion([half, 0., 0., half]))
    eye = posed[joint] @ point
    direction = eye[:3] / np.linalg.norm(eye[:3])
    target = np.array([0., 0., -1.])
    cross = np.cross(direction, target)
    q = np.array([*cross, 1. + np.dot(direction, target)])
    q /= np.linalg.norm(q)
    if eye[2] >= -.01 or 2*math.acos(min(1., abs(q[3]))) > math.radians(2.):
        raise ValueError("ADS optical correction exceeds reconstruction bound: " + model)
    return {"joint": joint, "point": point[:3].tolist(), "rotation": q.tolist()}
