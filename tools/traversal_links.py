from collections import Counter
import hashlib
import json
import math
from pathlib import Path

SCALE = .0254


def position(point):
    return [point[0] * SCALE, point[2] * SCALE, -point[1] * SCALE]


def load(config):
    from prepare_gameplay_content import external
    from export_instances import export
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    source = external(settings.get("navigation", inspection / "solo-content/traversal-links-recovery/map-navigation.json"))
    if not source.exists():
        export(external(settings.get("oat_source", inspection / "tooling/OAT/surface-7d027e8/source")),
               Path(config["assets"]["installation_root"]) / "zone/all/zm_tomb.ff", source.parent,
               callback=Path(__file__).with_name("map_navigation.cpp"))
    nodes = json.loads(source.read_text(encoding="utf-8"))
    if not nodes or [n["index"] for n in nodes] != list(range(len(nodes))):
        raise ValueError("incomplete native navigation indices")
    fields = {"type", "position", "animation", "spawn_flags", "targetname", "link_name", "noteworthy", "target", "angle", "forward", "radius", "minimum_use_distance_squared", "overlap", "links"}
    if any(not fields <= set(n) for n in nodes):
        raise ValueError("native endpoint constants missing; re-export with tools/map_navigation.cpp into a fresh external directory")
    return source, nodes


def endpoint(node):
    return {"index": node["index"], "type": node["type"], "spawn_flags": node["spawn_flags"],
            "position": position(node["position"]), "yaw": math.radians(node["angle"]),
            "forward": [node["forward"][0], 0., -node["forward"][1]],
            "radius": node["radius"] * SCALE,
            "minimum_use_distance_squared": node["minimum_use_distance_squared"] * SCALE * SCALE,
            "overlap": node["overlap"],
            **{key: node[key] for key in ("targetname", "link_name", "noteworthy", "target")}}


def negotiation(start, end, link):
    if not link["negotiation"]:
        return {"traversal": "walk", "source_animation": None}
    if (start["type"], end["type"]) != (17, 18) or not start["animation"]:
        raise ValueError("negotiation is missing its authored begin/end/animation")
    alias = start["animation"]
    if alias.startswith(("zm_jump_up_", "zm_jump_down_", "zm_jump_across_")):
        kind = "jump"
    elif alias.startswith(("zm_alcove_", "zm_scaffold_", "zm_vault_barbedwire_", "zm_mantle_over_")):
        kind = "climb"
    else:
        raise ValueError("unsupported native negotiation animation: " + alias)
    return {"traversal": kind, "source_animation": alias,
            "negotiation": {"cost": link["distance"] * SCALE, "flags": link["flags"],
                            "disconnect_count": link["disconnect"], "start": endpoint(start), "end": endpoint(end)}}


def validate(row, require):
    native = row.get("negotiation")
    if native is None:
        return
    require(row["traversal"] in ("climb", "jump") and bool(row.get("source_animation")), "negotiation missing traversal animation")
    require(isinstance(native, dict) and set(native) == {"cost", "flags", "disconnect_count", "start", "end"}, "invalid negotiation fields")
    require(type(native["cost"]) in (int, float) and math.isfinite(native["cost"]) and native["cost"] > 0, "invalid negotiation cost")
    for key in ("flags", "disconnect_count"):
        require(type(native[key]) is int and 0 <= native[key] <= 255, "invalid negotiation byte")
    for key, kind, identity in (("start", 17, "from"), ("end", 18, "to")):
        node = native[key]
        fields = {"index", "type", "spawn_flags", "position", "yaw", "forward", "radius", "minimum_use_distance_squared", "overlap", "targetname", "link_name", "noteworthy", "target"}
        require(isinstance(node, dict) and set(node) == fields, "invalid negotiation endpoint fields")
        require(type(node["index"]) is int and 0 <= node["index"] < 65535 and node["type"] == kind
                and row[identity] == "node.native." + str(node["index"]), "negotiation endpoint identity mismatch")
        require(type(node["spawn_flags"]) is int and -2**31 <= node["spawn_flags"] < 2**31, "invalid negotiation spawn flags")
        for name in ("position", "forward"):
            require(isinstance(node[name], list) and len(node[name]) == 3 and all(type(v) in (int, float) and math.isfinite(v) for v in node[name]), "invalid negotiation vector")
        require(node["forward"][1] == 0, "negotiation heading must be horizontal")
        for name in ("yaw", "radius", "minimum_use_distance_squared"):
            require(type(node[name]) in (int, float) and math.isfinite(node[name]) and (name == "yaw" or node[name] >= 0), "invalid negotiation scalar")
        require(abs(node["forward"][0] - math.cos(node["yaw"])) < .0001
                and abs(node["forward"][2] + math.sin(node["yaw"])) < .0001, "negotiation angle/forward mismatch")
        require(isinstance(node["overlap"], list) and len(node["overlap"]) == 2 and all(type(v) is int and -32768 <= v < 32768 for v in node["overlap"]), "invalid negotiation overlaps")
        for name in ("targetname", "link_name", "noteworthy", "target"):
            require(isinstance(node[name], str) and len(node[name]) < 128, "invalid negotiation source string")


def prepare(config):
    from prepare_gameplay_content import external
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    source, nodes = load(config)
    pack = external(config["game"]["asset_pack"])
    path = pack / "game.json"
    game = json.loads(path.read_text(encoding="utf-8-sig"))
    content = game["content"]
    by_id = {row["id"]: row for row in content["navigation_nodes"]}
    links = {row["id"]: row for row in content["navigation_links"]}
    if len(by_id) != len(nodes) or len(links) != sum(len(n["links"]) for n in nodes):
        raise ValueError("runtime/native navigation closure mismatch")
    aliases, costs, kinds, flags = Counter(), Counter(), Counter(), Counter()
    consumed, scripts = set(), {}
    script_root = external(settings.get("traversal_scripts", inspection / "solo-content/traversal-recovery/decompiled/t6"))
    def require(ok, reason):
        if not ok:
            raise ValueError(reason)
    for node in nodes:
        origin = by_id["node.native." + str(node["index"])]
        require(all(abs(a-b) < .0001 for a,b in zip(origin["position"], position(node["position"]), strict=True)), "native navigation position changed")
        for ordinal, native in enumerate(node["links"]):
            identity = f'link.native.{node["index"]}.{ordinal}'
            row = links[identity]
            consumed.add(identity)
            require(row["from"] == origin["id"] and row["to"] == "node.native." + str(native["to"]), "native link direction changed")
            if not native["negotiation"]:
                require(row.get("negotiation") is None and row["traversal"] == "walk", "unexpected native walking negotiation")
                continue
            prepared = negotiation(node, nodes[native["to"]], native)
            alias = prepared["source_animation"]
            require(row["source_animation"] == alias, "native traversal alias changed")
            require(all(alias in enemy.get("traversals", {}) for enemy in content["enemies"]), "missing actor traversal profile")
            script = script_root / (alias + ".gsc")
            require(script.is_file(), "missing recovered traversal script: " + alias)
            scripts[script.name] = hashlib.sha256(script.read_bytes()).hexdigest()
            row.update(prepared)
            validate(row, require)
            for endpoint_key, node_key in (("start", "from"), ("end", "to")):
                require(all(abs(a-b) < .0001 for a,b in zip(row["negotiation"][endpoint_key]["position"], by_id[row[node_key]]["position"], strict=True)), "traversal endpoint position changed")
            aliases[alias] += 1; costs[native["distance"]] += 1; kinds[row["traversal"]] += 1; flags[native["flags"]] += 1
    require(consumed == set(links) and bool(aliases), "incomplete native link extraction")
    shared = script_root / "zm_shared.gsc"
    require(shared.is_file(), "missing recovered zombie traversal implementation")
    scripts[shared.name] = hashlib.sha256(shared.read_bytes()).hexdigest()
    path.write_text(json.dumps(game, indent=2) + "\n", encoding="utf-8")
    receipt = {"native_nodes": len(nodes), "directed_links": len(links), "negotiations": sum(aliases.values()),
               "aliases": dict(sorted(aliases.items())), "kinds": dict(kinds), "native_costs": dict(costs), "flags": dict(flags),
               "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "script_sha256": scripts,
               "scope": "Compiled pathlink order/direction, begin/end constants, flags and source cost retained. Units convert inches/Z-up to meters/Y-up and angles to radians. Zone overrides, barriers, blockers and actor files preserved.",
               "approximation": "Unknown native flag bits remain data. Avian still projects/admits endpoints; fitted root motion and supported landing correction remain practical reconstruction. Native engine path arithmetic is unrecovered; scripted height overrides are not executed."}
    destination = external(settings.get("traversal_links_receipt", inspection / "solo-content/traversal-links-preparation.json"))
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt))
