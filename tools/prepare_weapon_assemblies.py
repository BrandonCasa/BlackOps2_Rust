import base64
import copy
import hashlib
import json
import shutil
from pathlib import Path

import numpy as np

from prepare_actors import convert
from prepare_arsenal import ACTIONS
from prepare_gameplay_content import external

DTYPES = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
BASIS = np.array([[1., 0., 0., 0.], [0., 0., 1., 0.], [0., -1., 0., 0.], [0., 0., 0., 1.]])


def transform(node):
    if "matrix" in node:
        return np.asarray(node["matrix"]).reshape(4, 4).T
    x, y, z, w = node.get("rotation", [0., 0., 0., 1.])
    result = np.eye(4)
    result[:3, :3] = [[1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w)],
                      [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w)],
                      [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y)]]
    result[:3, :3] *= np.asarray(node.get("scale", [1., 1., 1.]))
    result[:3, 3] = node.get("translation", [0., 0., 0.])
    return result


def angles(pitch, yaw, roll):
    p, y, r = np.radians([pitch, yaw, roll]) * .5
    cp, cy, cr = np.cos([p, y, r]); sp, sy, sr = np.sin([p, y, r])
    return [float(sr*cp*cy-cr*sp*sy), float(cr*sp*cy+sr*cp*sy),
            float(cr*cp*sy-sr*sp*cy), float(cr*cp*cy+sr*sp*sy)]


class Assembly:
    def __init__(self, native):
        self.doc = {"asset": {"version": "2.0"}, "scene": 0,
                    "scenes": [{"nodes": [0]}], "nodes": [{"name": "weapon_assembly", "children": []}],
                    "meshes": [], "skins": [], "materials": [], "images": [], "textures": [],
                    "samplers": [], "accessors": [], "bufferViews": []}
        self.binary = bytearray()
        self.bones = {}
        self.offsets = {}
        self.hidden_triangles = 0
        self.native = native
        self.native_translations = {}

    def array(self, index):
        accessor = self.doc["accessors"][index]
        view = self.doc["bufferViews"][accessor["bufferView"]]
        return np.frombuffer(self.binary, dtype=DTYPES[accessor["componentType"]],
                             count=accessor["count"]*WIDTHS[accessor["type"]],
                             offset=view["byteOffset"]).reshape(accessor["count"], -1).copy()

    def accessor(self, values, kind, component=5126, normalized=False):
        values = np.asarray(values, dtype=DTYPES[component]).reshape(-1, WIDTHS[kind])
        if not len(values) or not np.isfinite(values).all():
            raise ValueError("empty/non-finite assembly accessor")
        while len(self.binary) % 4:
            self.binary.append(0)
        self.doc["bufferViews"].append({"buffer": 0, "byteOffset": len(self.binary), "byteLength": values.nbytes})
        self.binary.extend(values.tobytes())
        result = {"bufferView": len(self.doc["bufferViews"])-1, "componentType": component,
                  "count": len(values), "type": kind}
        if normalized:
            result["normalized"] = True
        if kind != "MAT4":
            result.update(min=values.min(axis=0).tolist(), max=values.max(axis=0).tolist())
        self.doc["accessors"].append(result)
        return len(self.doc["accessors"])-1

    def globals(self):
        result = {}
        def visit(index, parent):
            if index in result:
                raise ValueError("assembly contains multiply parented node")
            result[index] = parent @ transform(self.doc["nodes"][index])
            for child in self.doc["nodes"][index].get("children", []):
                visit(child, result[index])
        visit(0, np.eye(4))
        return result

    def add(self, source_path, images_out, fallback, socket=None, offset=None, rotation=None, hidden=()):
        source = json.loads(source_path.read_text(encoding="utf-8-sig"))
        native = self.native(source_path.stem.removesuffix("_lod0"))
        raw_buffers = [base64.b64decode(b["uri"].split(",", 1)[1]) if b["uri"].startswith("data:")
                       else (source_path.parent / b["uri"]).read_bytes() for b in source["buffers"]]
        def array(index):
            a = source["accessors"][index]; v = source["bufferViews"][a["bufferView"]]
            dt = np.dtype(DTYPES[a["componentType"]]); width = WIDTHS[a["type"]]
            return np.ndarray((a["count"], width), dt, raw_buffers[v["buffer"]],
                              v.get("byteOffset", 0)+a.get("byteOffset", 0),
                              (v.get("byteStride", width*dt.itemsize), dt.itemsize)).copy()
        joints = {j for skin in source.get("skins", []) for j in skin["joints"]}
        parents = {c: i for i, n in enumerate(source["nodes"]) for c in n.get("children", [])}
        roots = sorted(j for j in joints if parents.get(j) not in joints)
        offset = offset or [0., 0., 0.]; rotation = rotation or [0., 0., 0., 1.]
        if socket is not None:
            if socket not in self.bones:
                raise ValueError(f"missing native socket {socket}: {source_path.name}")
            socket_node = self.bones[socket]
            placement = self.globals()[socket_node] @ transform({"translation": offset, "rotation": rotation}) @ np.linalg.inv(BASIS)
        else:
            socket_node, placement = 0, np.eye(4)
        mapped = {}
        for j in sorted(joints):
            old = source["nodes"][j]; name = old["name"]
            if name in self.bones:
                if j not in roots:
                    raise ValueError(f"ambiguous child joint {name}: {source_path.name}")
                mapped[j] = self.bones[name]
                continue
            n = copy.deepcopy(old); n.pop("children", None)
            if socket is not None and j in roots:
                n["translation"], n["rotation"] = offset, rotation
                if np.linalg.norm(offset) > 0:
                    self.offsets[name] = offset
            mapped[j] = len(self.doc["nodes"])
            self.bones[name] = mapped[j]; self.doc["nodes"].append(n)
            self.native_translations[name] = native[name]
        for j in sorted(joints):
            parent = mapped.get(parents.get(j), socket_node)
            if parent == mapped[j]:
                continue
            children = self.doc["nodes"][parent].setdefault("children", [])
            if mapped[j] not in children:
                children.append(mapped[j])
        material_base, texture_base, image_base, sampler_base = [len(self.doc[k]) for k in ("materials", "textures", "images", "samplers")]
        self.doc["samplers"].extend(copy.deepcopy(source.get("samplers", [])))
        for image in source.get("images", []):
            path = (source_path.parent / image["uri"]).resolve()
            if not path.is_file(): path = fallback / path.name
            key = hashlib.sha256(path.read_bytes()).hexdigest()[:24] if path.is_file() else path.name
            target = images_out / (key + ".dds") if path.is_file() else images_out / path.name
            if path.is_file() and not target.exists(): shutil.copyfile(path, target)
            self.doc["images"].append({"uri": "../images/"+target.name})
        for texture in source.get("textures", []):
            texture = copy.deepcopy(texture); texture["source"] += image_base
            if "sampler" in texture: texture["sampler"] += sampler_base
            self.doc["textures"].append(texture)
        for material in source.get("materials", []):
            material = copy.deepcopy(material)
            def textures(item):
                if isinstance(item, dict):
                    for key, value in item.items():
                        if key.endswith("Texture"): value["index"] += texture_base
                        else: textures(value)
            textures(material); self.doc["materials"].append(material)
        hidden_nodes = {j for j in joints if source["nodes"][j]["name"] in hidden}
        for _ in joints:
            hidden_nodes.update(j for j in joints if parents.get(j) in hidden_nodes)
        skin_map = {}
        for i, skin in enumerate(source.get("skins", [])):
            skin_map[i] = len(self.doc["skins"])
            self.doc["skins"].append({"joints": [mapped[j] for j in skin["joints"]], "skeleton": 0})
        for old_index, old in enumerate(source["nodes"]):
            if "mesh" not in old: continue
            mesh = source["meshes"][old["mesh"]]; primitives = []
            for primitive in mesh["primitives"]:
                indices = array(primitive["indices"]).reshape(-1, 3)
                attrs = primitive["attributes"]
                if hidden_nodes and "skin" in old:
                    sj = source["skins"][old["skin"]]["joints"]
                    joint_indices, weights = array(attrs["JOINTS_0"]), array(attrs["WEIGHTS_0"])
                    h = np.isin(joint_indices, [i for i, j in enumerate(sj) if j in hidden_nodes])
                    exclusively_hidden = np.all(h | (weights == 0), axis=1)
                    discard = exclusively_hidden[indices].all(axis=1)
                    self.hidden_triangles += int(discard.sum()); indices = indices[~discard]
                if not len(indices): continue
                attributes = {}
                mesh_placement = placement @ transform(old)
                for semantic, index in attrs.items():
                    a = source["accessors"][index]; values = array(index)
                    if semantic == "POSITION":
                        values = (np.column_stack((values, np.ones(len(values)))) @ mesh_placement.T)[:, :3]
                    elif semantic in ("NORMAL", "TANGENT"):
                        vectors = values[:, :3] @ np.linalg.inv(mesh_placement[:3, :3])
                        values[:, :3] = vectors / np.linalg.norm(vectors, axis=1, keepdims=True)
                    attributes[semantic] = self.accessor(values, a["type"], a["componentType"], a.get("normalized", False))
                primitives.append({"attributes": attributes, "indices": self.accessor(indices.reshape(-1, 1), "SCALAR", 5125),
                                   "material": primitive["material"]+material_base})
            if not primitives: continue
            node = {"name": source_path.stem+"_"+old.get("name", str(old_index)), "mesh": len(self.doc["meshes"])}
            if "skin" in old: node["skin"] = skin_map[old["skin"]]
            self.doc["meshes"].append({"name": node["name"], "primitives": primitives})
            self.doc["nodes"][0]["children"].append(len(self.doc["nodes"])); self.doc["nodes"].append(node)

    def write(self, path):
        globals_ = self.globals()
        for skin in self.doc["skins"]:
            matrices = np.array([np.linalg.inv(globals_[j]).T.flatten() for j in skin["joints"]])
            skin["inverseBindMatrices"] = self.accessor(matrices, "MAT4")
        self.doc["buffers"] = [{"byteLength": len(self.binary), "uri": "data:application/octet-stream;base64,"+base64.b64encode(self.binary).decode()}]
        path.write_text(json.dumps(self.doc, separators=(",", ":")))


def composition(fields, world=False, ads=False):
    prefix = "attachWorldModel" if world else "attachViewModel"
    result = []
    for slot in range(1, 9):
        if slot == 8 and not ads: continue
        if slot == 1 and ads and fields.get(prefix+"8"): continue
        model = fields.get(prefix+str(slot))
        if not model: continue
        offset = [float(fields.get(prefix+"Offset"+axis+str(slot)) or 0) for axis in "XYZ"]
        rotation = angles(*[float(fields.get(prefix+"Offset"+axis+str(slot)) or 0) for axis in ("Pitch", "Yaw", "Roll")])
        result.append({"model": model, "socket": fields.get(prefix+"Tag"+str(slot)) or ("tag_weapon" if world else "j_gun"), "offset": offset, "rotation": rotation})
    return result


def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    settings = config.get("preparation", {})
    output = external(settings.get("weapon_assembly_output", inspection / "weapon-assembly"))
    source_out = output / "source/model_export"; source_out.mkdir(parents=True, exist_ok=True)
    images_out = source_out.parent / "images"; images_out.mkdir(exist_ok=True)
    models = Path(settings.get("models", inspection / "game-prepare/models"))
    model_roots = [models / "model_export", inspection / "solo-content/shared/model_export"]
    fallback = inspection / "solo-content/actor-images"
    native_roots = [external(p) / "model_export" for p in settings.get("native_skeleton_exports", [output / "native-models" / mode for mode in ("patch", "shared", "base")])]
    native_cache = {}
    def native(name):
        if name not in native_cache:
            path = next((root / (name+"_lod0.gltf") for root in native_roots if (root / (name+"_lod0.gltf")).is_file()), None)
            if path is None:
                raise ValueError("missing native skeleton baseline export: "+name)
            document = json.loads(path.read_text())
            joints = {j for skin in document.get("skins", []) for j in skin["joints"]}
            native_cache[name] = {document["nodes"][j]["name"]: document["nodes"][j]["extras"]["origins_native_translation"] for j in joints}
        return native_cache[name]
    clips_root = inspection / "solo-content/source/xanim"
    recovered = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig")); content = game["content"]
    missing_audio = set()
    rows = {"weapon."+w["source"]: w["fields"] for w in recovered["weapons"]}
    def model_path(name):
        candidate = next((root / (name+"_lod0.gltf") for root in model_roots if (root / (name+"_lod0.gltf")).is_file()), None)
        if candidate is None: raise ValueError("missing native weapon model: "+name)
        return candidate
    def register(identity, relative, notes):
        generated = [{"id": "animation."+identity.removeprefix("actor.")+"."+name,
                      "path": "actors/"+relative+f"#Animation{i}", "skeleton": identity, "notifications": notifications}
                     for i, (name, notifications) in enumerate(notes)]
        content["animations"][:] = [a for a in content["animations"] if a["skeleton"] != identity] + generated
        presentation = {"id": identity, "scene": "actors/"+relative+"#Scene0", "animations": [a["id"] for a in generated], "sounds": []}
        content["presentations"][:] = [p for p in content["presentations"] if p["id"] != identity] + [presentation]
        for animation in generated:
            sounds = [{"seconds": n["seconds"], "alias": n["event"].split("#", 1)[1]}
                      for n in animation["notifications"] if n["event"].startswith("sndnt#")]
            missing_audio.update(n["alias"] for n in sounds if n["alias"] not in game.get("audio", {}).get("aliases", {}))
            if sounds:
                game.setdefault("audio", {}).setdefault("timelines", {})[animation["id"]] = [n for n in sounds if n["alias"] in game["audio"].get("aliases", {})]
        return {a["id"].rsplit(".", 1)[1]: a["id"] for a in generated}
    rig_clips = {}
    for a in content["animations"]:
        if a["skeleton"].startswith("actor."):
            rig_clips.setdefault(a["skeleton"][6:], set()).add(a["id"].rsplit(".", 1)[1])
    extra = {f.get(k) for f in rows.values() for k in ACTIONS.values()} - {"", None}
    for character in ("dempsey", "nikolai", "richtofen", "takeo"):
        name = "c_zom_"+character+"_viewhands"
        names = sorted(rig_clips.get(name, set()) | extra)
        relative, notes = convert(model_path(name), pack / "actors", fallback, [clips_root / n for n in names], native_translations=native(name))
        register("actor."+name, relative, notes)
    cache, report = {}, []
    for weapon in content["weapons"]:
        fields = rows[weapon["id"]]
        if not fields.get("gunModel") or fields.get("gunModel") in {"defaultweapon", "viewmodel_default"}:
            report.append({"weapon": weapon["id"], "status": "native_definition_has_no_viewmodel", "source_model": fields.get("gunModel")})
            continue
        names = sorted({fields.get(k) for k in ACTIONS.values()} - {"", None})
        names = sorted(set(names) | {a.rsplit(".", 1)[1] for a in weapon.get("animation_actions", {}).values()})
        hidden = fields.get("hideTags", "").split()
        evidence = {"weapon": weapon["id"], "status": "assembled", "compositions": []}
        for mode in ("view", "world", "ads"):
            world, ads = mode == "world", mode == "ads"
            if ads and not fields.get("attachViewModel8"):
                weapon.pop("ads_presentation", None); continue
            base = fields.get("worldModel" if world else "gunModel")
            if not base: continue
            parts = composition(fields, world, ads)
            world_names = sorted({weapon["lure"]["animation"].rsplit(".", 1)[1]} if world and weapon.get("lure") else ())
            if world and base == "weapon_parabolic_knife" and parts:
                evidence["world_proxy"] = "native melee knife proxy; staff attachments belong to its viewmodel"
                parts = []
            signature = {"base": base, "parts": parts, "hidden": hidden, "clips": world_names if world else names}
            digest = hashlib.sha256(json.dumps(signature, sort_keys=True).encode()).hexdigest()[:24]
            identity = "actor.weapon_"+digest
            if digest not in cache:
                assembly = Assembly(native); assembly.add(model_path(base), images_out, fallback, hidden=hidden)
                for part in parts:
                    assembly.add(model_path(part["model"]), images_out, fallback, part["socket"], part["offset"], part["rotation"], hidden)
                raw = source_out / ("weapon_"+digest+".gltf"); assembly.write(raw)
                relative, notes = convert(raw, pack / "actors", fallback, [clips_root / n for n in (world_names if world else names)], translation_offsets=assembly.offsets, native_translations=assembly.native_translations)
                actions = register(identity, relative, notes)
                cache[digest] = (actions, assembly.hidden_triangles)
            actions, hidden_count = cache[digest]
            weapon[{"view": "presentation", "world": "pickup", "ads": "ads_presentation"}[mode]] = identity
            if world and weapon.get("lure"):
                weapon["lure"]["animation"] = actions[world_names[0]]
            if world and weapon.get("mine"):
                weapon["mine"]["presentation"] = identity
            if mode == "view":
                old = weapon.get("animation_actions", {})
                weapon["animation_actions"] = {action: actions.get(fields.get(field), old.get(action)) for action, field in ACTIONS.items()
                                                if fields.get(field) in actions or action in old}
                weapon["animation_actions"] = {action: actions.get(clip.rsplit(".", 1)[1], clip) for action, clip in weapon["animation_actions"].items()}
            evidence["compositions"].append({"mode": mode, "presentation": identity, "parts": parts, "hidden_triangles": hidden_count})
        report.append(evidence)
    (pack / "game.json").write_text(json.dumps(game, indent=2)+"\n", encoding="utf-8")
    (output / "assembly.json").write_text(json.dumps({"weapons": report, "unique_compositions": len(cache),
        "approximation": "Absent native attachment tag uses j_gun for viewmodels and tag_weapon for world rigs. Slot 8 replaces slot 1 at full ADS. Hidden triangles require all positive skin influences to be hidden.",
        "native_translation_baselines": "Recovered XModel.trans is added to keyed XAnim translation before unit conversion; absent channels preserve the authored glTF rest transform.",
        "unavailable_native_audio_aliases": sorted(missing_audio)}, indent=2))
    print(json.dumps({"accounted_weapons": len(report), "assembled_weapons": sum(r["status"] == "assembled" for r in report), "unique_compositions": len(cache)}))
