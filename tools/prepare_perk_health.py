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
    machine = next(e for e in source["entities"] if e["fields"].get("targetname") == "zm_perk_machine" and e["fields"].get("script_noteworthy") == "specialty_armorvest")
    bottle = next(w for w in source["weapons"] if w["source"] == "zombie_perk_bottle_jugg")
    model = machine["fields"]["model"] + "_on"
    relative, _ = convert(inspection / "game-prepare/models/model_export" / (model + "_lod0.gltf"), pack / "props", inspection / "solo-content/actor-images", [], preparation=config)
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    c = game["content"]
    perk, presentation = "perk.juggernog", "prop." + model
    price = int(re.search(r'case "specialty_armorvest":\s*case "specialty_armorvest_upgrade":\s*cost = (\d+);', text)[1])
    native_maximum = int(re.search(r'set_zombie_var\( "zombie_perk_juggernaut_health", (\d+) \)', text)[1])
    f = bottle["fields"]
    rows = {"perks": {"id":perk,"generator":"generator.generator_nml_right","name":"Juggernog","price":price,"icon":None,"presentation":presentation,"maximum_health":250,
            "drink":{"bottle":"weapon."+bottle["source"],"seconds":float(f["firstRaiseTime"]),"lower_seconds":float(f["dropTime"]),"activation_delay":0.5}},
        "presentations":{"id":presentation,"scene":"props/"+relative+"#Scene0","animations":[],"sounds":[]},
        "placements":{"id":"placement.juggernog","definition":perk,"position":machine["position"],"yaw":math.radians(float(machine["fields"]["angles"].split()[1])),"zone":"zone.start"},
        "interactions":{"id":"interaction.juggernog","placement":"placement.juggernog","action":{"kind":"perk","perk":perk},"reach":2.,"hold_seconds":0.,"hidden_prompt":False}}
    for table, row in rows.items():
        c[table][:] = [v for v in c[table] if v["id"] != row["id"]]
        c[table].append(row)
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n")
    report = {"perk":rows["perks"],"machine":machine,"native_script_maximum":native_maximum,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),
        "approximations":["250 effective health reconstructs the five-hit ordinary-zombie behavior; retained script maximum 160 and native armorvest arithmetic are unresolved.","Shared machine bounds and zone.start reconstruct pending collision/zone metadata."]}
    (inspection / "solo-content/perk-health-preparation.json").write_text(json.dumps(report,indent=2))
    print(json.dumps(report["perk"]))
