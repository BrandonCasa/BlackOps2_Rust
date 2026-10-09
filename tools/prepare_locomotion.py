import hashlib
import json
from pathlib import Path
import re
import subprocess

from prepare_actors import append_clips
from prepare_gameplay_content import external
from xanim import decode

STATES = ("walk", "walk_crawl", "run", "run_crawl", "sprint", "sprint_crawl",
          "super_sprint", "super_sprint_crawl", "stumpy")


def state_clips(text):
    states = {}
    for state in STATES:
        matches = re.findall(r"(?m)^zm_move_" + state + r"\s*:[^\n]*notify move_anim\s*\{([^}]*)\}", text)
        if len(matches) != 1:
            raise ValueError(f"missing or duplicate native locomotion state: {state}")
        names = matches[0].split()
        if not names or any(not re.fullmatch(r"ai_zombie_[a-z0-9_]+", n) for n in names):
            raise ValueError(f"invalid native locomotion substate: {state}")
        states[state] = names
    return states


def prepare(config):
    base = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    settings = config.get("preparation", {})
    recovery = external(base / "solo-content/locomotion-recovery")
    asd = external(Path(settings.get("locomotion_asd", recovery / "animstatedefs/zm_tomb_basic.asd")))
    if not asd.is_file():
        if "locomotion_asd" in settings:
            raise ValueError("configured native locomotion definition is missing")
        recovery.mkdir(parents=True, exist_ok=True)
        zone = Path(config["assets"]["installation_root"]) / "zone/all"
        unlinker = Path(settings.get("unlinker", base / "tooling/OAT/v0.33.0/Unlinker.exe"))
        with (recovery / "recovery.log").open("w") as log:
            subprocess.run([str(unlinker), "--no-color", "--include-assets", "rawfile",
                            "--load", str(zone / "common_zm.ff"), "--load", str(zone / "patch_zm.ff"),
                            "-o", str(recovery), str(zone / "zm_tomb.ff")], check=True, stdout=log, stderr=log)
    states = state_clips(asd.read_text())
    names = list(dict.fromkeys(n for clips in states.values() for n in clips))
    source = base / "solo-content/source/xanim"
    decoded = {n: decode((source / n).read_bytes()) for n in names}
    if any(not c["looped"] or c["duration"] <= 0 for c in decoded.values()):
        raise ValueError("native locomotion requires finite looping clips")
    game_path = pack / "game.json"
    game = json.loads(game_path.read_text(encoding="utf-8-sig"))
    content = game["content"]
    rigs = {e["presentation"] for e in content["enemies"]}
    rigs.update(p["presentation"] for e in content["enemies"] for p in e.get("parts", []))
    rows = []
    for identity in sorted(rigs):
        presentation = next(p for p in content["presentations"] if p["id"] == identity)
        relative = presentation["scene"].split("#")[0]
        path = pack / relative
        document = json.loads(path.read_text())
        if len(document["buffers"]) != 1:
            raise ValueError("locomotion append requires one canonical actor buffer")
        buffer_path = path.parent / document["buffers"][0]["uri"]
        binary = bytearray(buffer_path.read_bytes())
        if len(binary) != document["buffers"][0]["byteLength"]:
            raise ValueError("canonical actor buffer length mismatch")
        existing = {a["name"]: i for i, a in enumerate(document.get("animations", []))}
        if len(existing) != len(document.get("animations", [])):
            raise ValueError("duplicate canonical actor clip names")
        missing = [n for n in names if n not in existing]
        before = len(existing)
        notes = append_clips(document, binary, [source / n for n in missing], in_place=True)
        if [n for n, _ in notes] != missing:
            raise ValueError(f"native locomotion does not animate recovered rig: {identity}")
        model = identity.removeprefix("actor.")
        for index, (name, notifications) in enumerate(notes, before):
            aid = "animation." + model + "." + name
            if any(a["id"] == aid for a in content["animations"]):
                raise ValueError("animation identity already exists without canonical clip")
            content["animations"].append({"id": aid, "path": relative + f"#Animation{index}",
                                          "skeleton": identity, "notifications": notifications})
            presentation["animations"].append(aid)
        if missing:
            document["buffers"][0]["byteLength"] = len(binary)
            buffer_path.write_bytes(binary)
            path.write_text(json.dumps(document, separators=(",", ":")), encoding="utf-8")
        for name in names:
            aid = "animation." + model + "." + name
            index = next(i for i, a in enumerate(document["animations"]) if a["name"] == name)
            matches = [a for a in content["animations"] if a["id"] == aid]
            if len(matches) != 1 or matches[0]["path"] != relative + f"#Animation{index}" or aid not in presentation["animations"]:
                raise ValueError("native locomotion binding disagrees with prepared rig")
        rows.append({"rig": identity, "previous_clips": before, "appended_clips": len(missing),
                     "clips": len(document["animations"]), "buffer_bytes": len(binary)})
    for enemy in content["enemies"]:
        model = enemy["presentation"].removeprefix("actor.")
        enemy["locomotion_states"] = {s: ["animation." + model + "." + n for n in clips]
                                     for s, clips in states.items()}
    game_path.write_text(json.dumps(game, indent=2) + "\n", encoding="utf-8")
    report = {"states": states, "unique_clips": len(names), "rigs": rows,
              "definition_sha256": hashlib.sha256(asd.read_bytes()).hexdigest(),
              "source_clips": {n: {"sha256": hashlib.sha256((source / n).read_bytes()).hexdigest(),
                                   "duration": decoded[n]["duration"]} for n in names},
              "approximation": "Native poses and substate order/duplicates; deterministic per-actor selection replaces opaque engine randomness. Collider owns translation; existing gait speeds remain unchanged. Crawl/stumpy/super-sprint states are prepared for future gameplay consumers."}
    recovery.mkdir(parents=True, exist_ok=True)
    (recovery / "preparation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({"unique_clips": len(names), "states": len(states), "rigs": len(rigs),
                      "appended_bindings": sum(r["appended_clips"] for r in rows)}))
