"""Prepare retained solo Quick Revive rules and its original Origins machine."""
import hashlib
import json
import math
import re
from prepare_actors import convert
from prepare_gameplay_content import external

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    scripts = {
        "round": inspection / "damage-life-state-06/decompiled/t6/_zm.gsc",
        "perks": inspection / "damage-life-state-03/decompiled/t6/_zm_perks.gsc",
        "map": inspection / "damage-life-state-03/decompiled/t6/zm_tomb.gsc",
    }
    texts = {key: path.read_text() for key, path in scripts.items()}
    duration = float(re.search(r"solo_revive_time = ([\d.]+);", texts["round"])[1])
    bleedout = float(re.search(r'setdvar\( "player_lastStandBleedoutTime", "([\d.]+)"', texts["round"])[1])
    maximum = int(re.search(r"level.solo_lives_given >= (\d+)", texts["perks"])[1])
    price = int(re.search(r'case "specialty_quickrevive_upgrade":\s*if \( solo \)\s*cost = (\d+);', texts["perks"])[1])
    pistol = re.search(r'level.default_solo_laststandpistol = "([^"]+)";', texts["map"])[1]
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    content = game["content"]
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    machines = [e for e in source["entities"] if e["fields"].get("targetname") == "zm_perk_machine"
        and e["fields"].get("script_noteworthy") == "specialty_quickrevive"]
    if len(machines) != 1: raise ValueError("expected one original Quick Revive machine")
    machine = machines[0]
    model = machine["fields"]["model"] + "_on"
    relative, _ = convert(inspection / "game-prepare/models/model_export" / (model + "_lod0.gltf"),
        pack / "props", inspection / "solo-content/actor-images", [])
    presentation = "prop." + model
    perk = "perk.quick_revive"
    content["perks"][:] = [p for p in content["perks"] if p["id"] != perk]
    bottle = next(w for w in source["weapons"] if w["source"] == "zombie_perk_bottle_revive")
    bottle_fields = bottle["fields"]
    activation_delay = float(re.search(r'waittill_notify_or_timeout\( "burp", ([\d.]+) \)', texts["perks"])[1])
    drink = {"bottle":"weapon." + bottle["source"], "seconds":float(bottle_fields["firstRaiseTime"]),
        "lower_seconds":float(bottle_fields["dropTime"]), "activation_delay":activation_delay}
    content["perks"].append({"id":perk,"name":"Quick Revive","price":price,"icon":None,"presentation":presentation,"drink":drink})
    content["presentations"][:] = [p for p in content["presentations"] if p["id"] != presentation]
    content["presentations"].append({"id":presentation,"scene":"props/"+relative+"#Scene0","animations":[],"sounds":[]})
    placement = "placement.quick_revive"
    interaction = "interaction.quick_revive"
    content["placements"][:] = [p for p in content["placements"] if p["id"] != placement]
    content["interactions"][:] = [p for p in content["interactions"] if p["id"] != interaction]
    yaw = math.radians(float(machine["fields"]["angles"].split()[1]))
    content["placements"].append({"id":placement,"definition":perk,"position":machine["position"],"yaw":yaw,"zone":"zone.start"})
    content["interactions"].append({"id":interaction,"placement":placement,"action":{"kind":"perk","perk":perk},
        "reach":2.0,"hold_seconds":0.,"hidden_prompt":False})
    game["solo_revive"] = {"perk":perk,"seconds":duration,"bleedout_seconds":bleedout,"bleedout_poll_seconds":0.05,
        "maximum_purchases":maximum,"downed_weapon":"weapon."+pistol,"reserve_clips":2,
        "downed_height":0.75,"downed_speed":1.0}
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    report = {"profile":game["solo_revive"],"drink":drink,"bottle_sha256":bottle["sha256"],"price":price,"machine":machine,
        "script_sha256":{key:hashlib.sha256(path.read_bytes()).hexdigest() for key,path in scripts.items()},
        "approximations":["60 Hz fixed-step clocks replace native waits and network-frame cleanup.",
            "75 cm downed capsule and 1 m/s crawl reconstruct last-stand movement.",
            "Machine purchase target uses a reconstructed eye-level endpoint/reach."]}
    (inspection / "solo-content/solo-revive-preparation.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
    print(json.dumps({"solo_revive":game["solo_revive"],"price":price}))
