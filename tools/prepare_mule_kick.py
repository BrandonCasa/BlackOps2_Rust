import hashlib
import json
import math
import re
from prepare_actors import convert
from prepare_gameplay_content import external

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    script = inspection / "damage-life-state-03/decompiled/t6/_zm_perks.gsc"
    text = script.read_text()
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    machine = next(e for e in source["entities"] if e["fields"].get("targetname") == "zm_perk_machine" and e["fields"].get("script_noteworthy") == "specialty_additionalprimaryweapon")
    bottle = next(w for w in source["weapons"] if w["source"] == "zombie_perk_bottle_additionalprimaryweapon")
    model = machine["fields"]["model"]
    relative, _ = convert(inspection / "game-prepare/models/model_export" / (model + "_lod0.gltf"), pack / "props", inspection / "solo-content/actor-images", [])
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    content = game["content"]
    perk, presentation = "perk.mule_kick", "prop." + model
    price = int(re.search(r'case "specialty_additionalprimaryweapon":\s*case "specialty_additionalprimaryweapon_upgrade":\s*cost = (\d+);', text)[1])
    fields = bottle["fields"]
    rows = {"perks":{"id":perk,"name":"Mule Kick","price":price,"icon":None,"presentation":presentation,"third_weapon":True,
            "drink":{"bottle":"weapon."+bottle["source"],"seconds":float(fields["firstRaiseTime"]),"lower_seconds":float(fields["dropTime"]),"activation_delay":0.5}},
        "presentations":{"id":presentation,"scene":"props/"+relative+"#Scene0","animations":[],"sounds":[]},
        "placements":{"id":"placement.mule_kick","definition":perk,"position":machine["position"],"yaw":math.radians(float(machine["fields"]["angles"].split()[1])),"zone":"zone.start"},
        "interactions":{"id":"interaction.mule_kick","placement":"placement.mule_kick","action":{"kind":"perk","perk":perk},"reach":2.,"hold_seconds":0.,"hidden_prompt":False}}
    for table,row in rows.items():
        content[table][:] = [r for r in content[table] if r["id"] != row["id"]]
        content[table].append(row)
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n")
    native_limit = int(re.search(r"level.additionalprimaryweapon_limit = (\d+);", text)[1])
    if native_limit != 3: raise ValueError("unsupported native third-weapon limit")
    report = {"native_limit":native_limit,"perk":rows["perks"],"machine":machine,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"bottle_sha256":bottle["sha256"],
        "approximations":["Stable third-slot assignment reconstructs native primary-list ordering and survives replacement/upgrade in that slot.","Uses the authored p6 machine model; separate power-state visuals remain unfinished.","Generator gating, machine collision/zone metadata and original audio remain unfinished or reconstructed."]}
    (inspection / "solo-content/mule-kick-preparation.json").write_text(json.dumps(report,indent=2))
    print(json.dumps(report["perk"]))
