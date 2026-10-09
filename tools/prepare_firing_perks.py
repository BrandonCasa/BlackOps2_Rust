import hashlib
import json
from prepare_gameplay_content import external

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    script = inspection / "damage-life-state-03/decompiled/t6/_zm_perks.gsc"
    text = script.read_text()
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    prepared = []
    for name, title, bottle, specialty, scale, copies, spread in [
        ("double_tap", "Double Tap II", "doubletap", "rof", 0.75, 2, 1.0),
        ("deadshot", "Deadshot Daiquiri", "deadshot", "deadshot", 1.0, 1, 0.65),
    ]:
        assert 'case "specialty_' + specialty + '":' in text
        weapon = next(w for w in source["weapons"] if w["source"] == "zombie_perk_bottle_" + bottle)
        row = {"id":"perk."+name,"name":title,"price":2000 if name=="double_tap" else 1500,"icon":None,
               "firing":{"interval_scale":scale,"bullet_copies":copies,"hip_spread_scale":spread},
               "drink":{"bottle":"weapon."+weapon["source"],"seconds":float(weapon["fields"]["firstRaiseTime"]),"lower_seconds":float(weapon["fields"]["dropTime"]),"activation_delay":0.5}}
        game["content"]["perks"][:] = [p for p in game["content"]["perks"] if p["id"] != row["id"]]
        game["content"]["perks"].append(row)
        prepared.append({"perk":row,"bottle_sha256":weapon["sha256"]})
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n")
    (inspection / "solo-content/firing-perks-preparation.json").write_text(json.dumps({"prepared":prepared,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),
        "approximations":["Native engine numeric effects are tunable reconstructions: 4/3 firing rate and 0.65 hip spread.","Eligible hitscan bullet pellets duplicate their direction; explosive, equipment and melee delivery do not duplicate.","Mouse aim remains raw, with no aim assist or head snapping; ADS spread and sensitivity are unchanged.","No standalone Origins vending placements are invented; Wunderfizz/challenge acquisition, icons and audio remain unfinished."]},indent=2))
    print(json.dumps(prepared))
