"""Restore weapon alias levels from the exported original packed sound records."""
import argparse
import json
from pathlib import Path


def repair(assets, catalog):
    weapons = json.loads((assets / "source-weapons.json").read_text())
    audio = catalog["audio"]
    aliases = {name.lower(): name for name in audio["aliases"]}
    sounds = {sound["id"]: sound for sound in catalog["sounds"]}
    required = {"zmb_melee_whoosh_plr", "zmb_melee_staff_upgraded_plr", "fly_riotshield_zm_swing",
                "wpn_one_inch_punch_plr", "wpn_one_inch_punch_fire_plr", "wpn_one_inch_punch_air_plr",
                "wpn_one_inch_punch_ice_plr", "wpn_one_inch_punch_lightning_plr"}
    for weapon in weapons.values():
        required.update(value.lower() for key, value in weapon["info"].items()
                        if "sound" in key.lower() and "map" not in key.lower() and value)
        required.update(note[6:].lower() for clip in weapon["spec"]["clips"].values()
                        for note, _ in clip["notes"] if note.startswith("sndnt#"))
        for line in weapon["info"].get("notetrackSoundMap", "").splitlines():
            tokens = line.split()
            if len(tokens) == 2:
                required.add(tokens[1].lower())
    pending = list(required)
    while pending:
        key = pending.pop()
        if key not in aliases:
            continue
        for native in audio["aliases"][aliases[key]]["native"]:
            secondary = native.get("secondary")
            if secondary and secondary.lower() not in required:
                required.add(secondary.lower())
                pending.append(secondary.lower())
    # Same registration order as the game: map and patch aliases override common banks.
    records = {}
    for bank in ("code_post_gfx_zm", "en_code_post_gfx_zm", "common_zm", "en_common_zm",
                 "patch_zm", "dlc3_load_zm", "dlc4_load_zm", "zm_tomb", "zm_tomb_patch", "en_zm_tomb"):
        for path in (assets / "audio/source" / bank / "sound_aliases").glob("*.json"):
            record = json.loads(path.read_text())
            records[record["name"].lower()] = record["entries"]
    corrected = []
    for key in sorted(required & aliases.keys()):
        native = audio["aliases"][aliases[key]]["native"]
        entries = records[key]
        assert len(native) == len(entries), (key, "source alias entry count")
        for index, (sound, original) in enumerate(zip(native, entries)):
            assert sound["id"] == original["id"], (key, index, "source alias identity")
            minimum, maximum = original["volMin"] / 65535., original["volMax"] / 65535.
            definition = sounds.get(sound["sound"])
            if definition is not None:
                definition["volume"] = minimum
            sound["volume_max"] = maximum
        corrected.append(aliases[key])
    return {"source_aliases": len(corrected), "undefined_source_aliases": sorted(required - aliases.keys())}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    catalog = json.loads(args.catalog.read_text())
    report = repair(args.assets, catalog)
    args.catalog.write_text(json.dumps(catalog, indent=2) + "\n", encoding="utf-8")
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Restored original levels for {report['source_aliases']} weapon aliases")
