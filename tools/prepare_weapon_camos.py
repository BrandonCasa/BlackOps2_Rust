import argparse
import copy
import csv
import hashlib
import json
from pathlib import Path
import re

try:
    from .prepare_native_weapons import NativeWeapons
except ImportError:
    from prepare_native_weapons import NativeWeapons


def digest(data):
    return hashlib.sha256(data).hexdigest()


def origins_option(gsc, options):
    source = gsc.read_text()
    match = re.search(r'"zm_tomb"\s*==\s*level\.script\s*\)\s*(?:\{\s*)?camo_index\s*=\s*(\d+)', source)
    if not match:
        raise ValueError("Origins Pack-a-Punch camo option missing from original GSC")
    camo_id = int(match[1])
    rows = list(csv.reader(options.read_text().splitlines()))
    records = [r for r in rows if len(r) > 4 and r[0] == str(camo_id) and r[1] == "camo"]
    if len(records) != 1 or records[0][3] != "1":
        raise ValueError("Origins camo option is not a unique native material-table option")
    index = int(records[0][4]) - 1
    if index < 0:
        raise ValueError("Invalid native camo material table index")
    return camo_id, index, records[0][2]


def references(value, presentations):
    result = set()
    if isinstance(value, dict):
        for child in value.values():
            result.update(references(child, presentations))
    elif isinstance(value, list):
        for child in value:
            result.update(references(child, presentations))
    elif isinstance(value, str) and value in presentations:
        result.add(presentations[value]["scene"].split("#")[0])
    return result


def native_tables(source, uniques):
    fields = source["fields"]
    tables = {fields["camo"]} if fields.get("camo") else set()
    for name in fields.get("attachmentUniques", "").splitlines():
        if name not in uniques:
            raise ValueError(f"Native attachment-unique definition unavailable: {name}")
        if camo := uniques[name]["fields"].get("camo"):
            tables.add(camo)
    return sorted(tables)


def prepare(pack, extraction, camos, options, weapons, gsc):
    game = json.loads((pack / "game.json").read_bytes())["content"]
    original = json.loads(weapons.read_bytes())
    sources = {"weapon." + w["source"]: w for w in original["weapons"]}
    uniques = {w["source"]: w for w in original["attachmentunique"]}
    content = {w["id"]: w for w in game["weapons"]}
    presentations = {p["id"]: p for p in game["presentations"]}
    upgraded = {w["upgraded"] for w in content.values() if w.get("upgraded")}
    pending = list(upgraded)
    while pending:
        weapon = pending.pop()
        if weapon not in sources:
            raise ValueError(f"Native upgraded weapon definition unavailable: {weapon}")
        partner = sources[weapon]["fields"].get("DualWieldWeapon")
        if partner and "weapon." + partner not in upgraded:
            partner = "weapon." + partner
            upgraded.add(partner)
            pending.append(partner)
    camo_id, table_index, option_name = origins_option(gsc, options)
    native = NativeWeapons(pack, extraction)
    library = {"version": 1, "camo_id": camo_id, "weapons": {}, "surfaces": {},
               "provenance": {"gsc_sha256": digest(gsc.read_bytes()),
                              "options_sha256": digest(options.read_bytes()),
                              "weapons_sha256": digest(weapons.read_bytes()),
                              "option_name": option_name, "material_table_index": table_index,
                              "tables": {}}}
    scene_materials, source_tables = {}, {}
    for weapon in sorted(upgraded):
        if weapon not in content:
            raise ValueError(f"Upgraded weapon missing from runtime content: {weapon}")
        materials = set()
        for scene in references(content[weapon], presentations):
            if scene not in scene_materials:
                document = json.loads((pack / scene).read_bytes())
                scene_materials[scene] = {m["name"] for m in document.get("materials", [])}
            materials.update(scene_materials[scene])
        overrides = {}
        for table_name in native_tables(sources[weapon], uniques):
            if table_name not in source_tables:
                path = camos / (table_name + ".json")
                raw = path.read_bytes()
                table = json.loads(raw)
                if table_index >= len(table["camoMaterials"]):
                    raise ValueError(f"Native camo table missing Origins option: {table_name}")
                source_tables[table_name] = table["camoMaterials"][table_index]["materials"]
                library["provenance"]["tables"][table_name] = digest(raw)
            for record in source_tables[table_name]:
                flags = {"color": record["useColorMap"], "normal": record["useNormalMap"],
                         "specular": record["useSpecularMap"]}
                constants = record["shaderConsts"]
                if len(constants) != 8:
                    raise ValueError(f"Invalid source camo shader constants: {table_name}")
                for replacement in record["materialOverrides"]:
                    base = replacement["baseMaterial"]
                    if base not in materials:
                        continue
                    key = {"camo_id": camo_id, "camo_table": table_name, "base_material": base,
                           "material": replacement["camoMaterial"], "shader_consts": constants,
                           "replace_flags": flags}
                    identity = "native_camo/" + digest(json.dumps(key, sort_keys=True).encode())
                    if base in overrides and overrides[base] != identity:
                        raise ValueError(f"Conflicting native camo mappings: {weapon}: {base}")
                    overrides[base] = identity
                    if identity in library["surfaces"]:
                        continue
                    zone, (definition, _, _) = native.definition(base)
                    inputs = {b["name"]: b["image"] for b in definition.get("textures", [])}
                    injections = {}
                    for flag, binding, original_binding in (
                        ("color", "Weapon_Color_Map", "colorMap"),
                        ("normal", "Weapon_Normal_Map", "normalMap"),
                        ("specular", "Weapon_Specular_Map", "specularMap"),
                    ):
                        if flags[flag]:
                            if original_binding not in inputs:
                                raise ValueError(f"Source camo requests absent {original_binding}: {base}")
                            injections[binding] = {"image": inputs[original_binding], "source_zone": zone}
                    material = {"name": identity, "extras": {"surface": {
                        "family": "lit", "native_weapon": {"source_zone": zone}}}}
                    native.material(material, source_name=replacement["camoMaterial"], injections=injections)
                    surface = copy.deepcopy(material["extras"]["surface"])
                    surface["native_weapon"].update({k: v for k, v in key.items() if k != "material"})
                    library["surfaces"][identity] = surface
        library["weapons"][weapon] = dict(sorted(overrides.items()))
    output = pack / "weapon_camos.json"
    output.write_text(json.dumps(library, separators=(",", ":")) + "\n")
    return {"camo_id": camo_id, "material_table_index": table_index,
            "weapons": len(library["weapons"]), "materials": len(library["surfaces"]),
            "overrides": sum(len(v) for v in library["weapons"].values()),
            "native_textures": len(native.copied), "textures": native.copied,
            "library_sha256": digest(output.read_bytes())}


def main():
    parser = argparse.ArgumentParser()
    for name in ("pack", "extraction", "camos", "options", "weapons", "gsc"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    report = prepare(args.pack, args.extraction, args.camos, args.options, args.weapons, args.gsc)
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k != "textures"}))


if __name__ == "__main__":
    main()
