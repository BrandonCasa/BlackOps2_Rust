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
    machine = next(e for e in source["entities"] if e["fields"].get("targetname") == "zm_perk_machine" and e["fields"].get("script_noteworthy") == "specialty_longersprint")
    bottle = next(w for w in source["weapons"] if w["source"] == "zombie_perk_bottle_marathon")
    model = machine["fields"]["model"] + "_on"
    relative, _ = convert(inspection / "game-prepare/models/model_export" / (model + "_lod0.gltf"), pack / "props", inspection / "solo-content/actor-images", [])
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    content = game["content"]
    perk, presentation = "perk.stamin_up", "prop." + model
    price = int(re.search(r'case "specialty_longersprint":\s*case "specialty_longersprint_upgrade":\s*cost = (\d+);', text)[1])
    fields = bottle["fields"]
    rows = {"perks":{"id":perk,"generator":"generator.generator_nml_left","name":"Stamin-Up","price":price,"icon":None,"presentation":presentation,"sprint":{"duration_scale":2.0,"speed_scale":1.07},
            "drink":{"bottle":"weapon."+bottle["source"],"seconds":float(fields["firstRaiseTime"]),"lower_seconds":float(fields["dropTime"]),"activation_delay":0.5}},
        "presentations":{"id":presentation,"scene":"props/"+relative+"#Scene0","animations":[],"sounds":[]},
        "placements":{"id":"placement.stamin_up","definition":perk,"position":machine["position"],"yaw":math.radians(float(machine["fields"]["angles"].split()[1])),"zone":"zone.start"},
        "interactions":{"id":"interaction.stamin_up","placement":"placement.stamin_up","action":{"kind":"perk","perk":perk},"reach":2.,"hold_seconds":0.,"hidden_prompt":False}}
    for table,row in rows.items():
        content[table][:] = [r for r in content[table] if r["id"] != row["id"]]
        content[table].append(row)
    game["tuning"].update({"sprint_seconds":4., "sprint_minimum":1., "sprint_recovery_rate":1., "sprint_recovery_delay":0.})
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n")
    report = {"perk":rows["perks"],"machine":machine,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"bottle_sha256":bottle["sha256"],
        "approximations":["Double endurance and 7 percent uncrouched speed reconstruct Stamin-Up engine modifiers; native arithmetic is unrecovered.","Base four-second endurance, one second recovered per second and release-to-rearm exhaustion are explicit sprint reconstructions.","Machine collision/zone metadata and original audio remain unfinished or reconstructed."]}
    (inspection / "solo-content/stamin-up-preparation.json").write_text(json.dumps(report,indent=2))
    print(json.dumps(report["perk"]))
