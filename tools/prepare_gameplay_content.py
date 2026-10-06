"""Read-only BO2 gameplay recovery, preserving source fields and patch precedence.

This manifest is an external preparation intermediate, not a runtime pack.
"""
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import struct

WORKSPACE = Path(__file__).resolve().parents[1]


def external(path):
    path = Path(path).resolve()
    if path == WORKSPACE or WORKSPACE in path.parents:
        raise ValueError("gameplay recovery output must remain external")
    return path


def info_string(path, expected):
    """OAT exports complete resolved definitions as BO2 key/value info strings."""
    raw = path.read_bytes()
    tokens = raw.decode("utf-8-sig").rstrip("\r\n").split("\\")
    if tokens[0] != expected or len(tokens) % 2 != 1:
        raise ValueError(f"malformed {expected}: {path.name}")
    fields = {}
    for key, value in zip(tokens[1::2], tokens[2::2]):
        if not key or key in fields:
            raise ValueError(f"duplicate/empty source field: {path.name}/{key}")
        fields[key] = value
    return {"source": path.name, "sha256": hashlib.sha256(raw).hexdigest(), "fields": fields}


def recover(config):
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    output = external(settings.get("gameplay_output", inspection / "solo-content"))
    source = output / "source"
    installation = Path(config["assets"]["installation_root"])
    zone_root = installation / "zone/all"
    unlinker = Path(settings.get("unlinker", inspection / "tooling/OAT/v0.33.0/Unlinker.exe"))
    # Base exports are complete. Patch exports replace same-named files only after
    # the base/common pools are loaded, resolving native cross-zone references.
    common, base, patch = [zone_root / name for name in ("common_zm.ff", "zm_tomb.ff", "zm_tomb_patch.ff")]
    for path in (common, base, patch, unlinker):
        if not path.is_file():
            raise ValueError(f"required local recovery input missing: {path.name}")
    source.mkdir(parents=True, exist_ok=True)
    arguments = [str(unlinker), "--no-color", "--include-assets", "weapon,attachment,attachmentunique,xanim", "-o", str(source)]
    shared_patch = zone_root / "patch_zm.ff"
    for name, loads, target in (("base", [common], base), ("shared", [common], shared_patch), ("patch", [common, shared_patch, base], patch)):
        command = arguments + [item for path in loads for item in ("--load", str(path))] + [str(target)]
        with (output / f"recover-{name}.log").open("wb") as log:
            subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
        log_text = (output / f"recover-{name}.log").read_text(encoding="utf-8", errors="replace")
        if "ERROR:" in log_text or not re.search(r"Finished with \d+ warnings, 0 errors", log_text):
            raise ValueError(f"OAT reported extraction errors; inspect recover-{name}.log")
    # Shared patched view/world models include the Mark II referenced by Origins.
    command = [str(unlinker), "--no-color", "--include-assets", "xmodel", "--model-format", "GLTF",
               "--load", str(common), "-o", str(output / "shared"), str(shared_patch)]
    with (output / "recover-shared-models.log").open("wb") as log:
        subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
    log_text = (output / "recover-shared-models.log").read_text(encoding="utf-8", errors="replace")
    if "ERROR:" in log_text or not re.search(r"Finished with \d+ warnings, 0 errors", log_text):
        raise ValueError("OAT reported shared model extraction errors")
    for zone_name in ("en_code_post_gfx_zm", "en_common_zm", "en_patch_zm", "en_dlc3_load_zm", "en_dlc4_load_zm", "en_zm_tomb"):
        localized_zone = installation / "zone/english" / (zone_name + ".ff")
        if not localized_zone.is_file():
            raise ValueError(f"missing original English zone: {zone_name}")
        command = [str(unlinker), "--no-color", "--include-assets", "localize", "-o", str(output / "localized"), str(localized_zone)]
        log_path = output / f"recover-{zone_name}.log"
        with log_path.open("wb") as log:
            subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
        log_text = log_path.read_text(encoding="utf-8", errors="replace")
        if "ERROR:" in log_text or not re.search(r"Finished with \d+ warnings, 0 errors", log_text):
            raise ValueError(f"OAT reported localized string extraction errors: {zone_name}")
    tables = {}
    for table, header in (("weapons", "WEAPONFILE"), ("attachment", "ATTACHMENTFILE"), ("attachmentunique", "ATTACHMENTUNIQUEFILE")):
        rows = []
        for path in sorted((source / table).glob("*")):
            if path.is_file():
                record = info_string(path, header)
                record["id"] = f"{table}.{path.name}"
                rows.append(record)
        if not rows:
            raise ValueError(f"empty recovered source table: {table}")
        tables[table] = rows
    entity_path = Path(settings.get("entities", inspection / "game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents"))
    entity_bytes = entity_path.read_bytes()
    text = entity_bytes.decode("utf-8-sig")
    records = []
    occurrences = {}
    for block in re.findall(r"\{([^{}]*)\}", text):
        pairs = re.findall(r'"([^"\n]+)"\s+"([^"\n]*)"', block)
        fields = dict(pairs)
        if len(fields) != len(pairs):
            raise ValueError("duplicate map entity field")
        digest = hashlib.sha256(json.dumps(fields, sort_keys=True).encode()).hexdigest()[:24]
        # Authored GUIDs are not unique in this map. Content hashes disambiguate
        # differing records; identical repeated records retain occurrence identity.
        identity = "entity." + fields.get("guid", "anonymous").lower() + "." + digest
        occurrence = occurrences.get(identity, 0)
        occurrences[identity] = occurrence + 1
        identity += f".{occurrence}"
        record = {"id": identity, "fields": fields}
        if "origin" in fields:
            coordinates = [float(x) for x in fields["origin"].split()]
            if len(coordinates) != 3 or not all(math.isfinite(x) for x in coordinates):
                raise ValueError("invalid map entity origin")
            x, y, z = coordinates
            record["position"] = [x * 0.0254, z * 0.0254, -y * 0.0254]
        records.append(record)
    tables["entities"] = records
    tables["navigation_nodes"] = [r for r in records if r["fields"].get("classname") == "node_pathnode"]
    animation_records = []
    for path in sorted((source / "xanim").glob("*")):
        if not path.is_file():
            continue
        raw = path.read_bytes()
        if len(raw) < 10:
            raise ValueError(f"truncated exported animation: {path.name}")
        version, frames, bones, flags, asset_type, rate = struct.unpack_from("<HHHBBH", raw)
        if version != 19:
            raise ValueError(f"unsupported exported BO2 animation version: {path.name}")
        animation_records.append({"id": "animation.source." + path.name, "source": path.name,
            "sha256": hashlib.sha256(raw).hexdigest(), "frames": frames, "bones": bones,
            "flags": flags, "asset_type": asset_type, "frame_rate": rate})
    tables["animations"] = animation_records
    tables["source_precedence"] = [p.name for p in (common, base, shared_patch, patch)]
    tables["entities_sha256"] = hashlib.sha256(entity_bytes).hexdigest()
    destination = output / "recovered-gameplay.json"
    destination.write_text(json.dumps(tables, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: len(value) for key, value in tables.items() if isinstance(value, list)}))
    return destination
