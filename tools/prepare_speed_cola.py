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
    machine = next(e for e in source["entities"] if e["fields"].get("targetname") == "zm_perk_machine" and e["fields"].get("script_noteworthy") == "specialty_fastreload")
    bottle = next(w for w in source["weapons"] if w["source"] == "zombie_perk_bottle_sleight")
    model = machine["fields"]["model"] + "_on"
    relative, _ = convert(inspection / "game-prepare/models/model_export" / (model + "_lod0.gltf"), pack / "props", inspection / "solo-content/actor-images", [], preparation=config)
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    content = game["content"]
    perk, presentation = "perk.speed_cola", "prop." + model
    price = int(re.search(r'case "specialty_fastreload":\s*case "specialty_fastreload_upgrade":\s*cost = (\d+);', text)[1])
    fields = bottle["fields"]
    rows = {"perks":{"id":perk,"generator":"generator.generator_mid_trench","name":"Speed Cola","price":price,"icon":None,"presentation":presentation,"reload_duration_scale":0.5,
            "drink":{"bottle":"weapon."+bottle["source"],"seconds":float(fields["firstRaiseTime"]),"lower_seconds":float(fields["dropTime"]),"activation_delay":0.5}},
        "presentations":{"id":presentation,"scene":"props/"+relative+"#Scene0","animations":[],"sounds":[]},
        "placements":{"id":"placement.speed_cola","definition":perk,"position":machine["position"],"yaw":math.radians(float(machine["fields"]["angles"].split()[1])),"zone":"zone.start"},
        "interactions":{"id":"interaction.speed_cola","placement":"placement.speed_cola","action":{"kind":"perk","perk":perk},"reach":2.,"hold_seconds":0.,"hidden_prompt":False}}
    for table,row in rows.items():
        content[table][:] = [r for r in content[table] if r["id"] != row["id"]]
        content[table].append(row)
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n")
    report = {"perk":rows["perks"],"machine":machine,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"bottle_sha256":bottle["sha256"],
        "approximations":["Half-duration reload tuning reconstructs Speed Cola's engine perk; native engine arithmetic is not recovered.","Fixed-step phase boundaries quantize shell timing.","Machine collision/zone metadata and original audio remain unfinished or reconstructed."]}
    (inspection / "solo-content/speed-cola-preparation.json").write_text(json.dumps(report,indent=2))
    print(json.dumps(report["perk"]))
