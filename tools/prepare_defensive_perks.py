"""Prepare retained defensive-perk rules; Originals stay read-only."""
import hashlib
import json
import re
from prepare_gameplay_content import external

def prepare(config):
    inspection=external(config["assets"]["inspection_output"]);pack=external(config["game"]["asset_pack"])
    source=json.loads((inspection/"solo-content/recovered-gameplay.json").read_text())
    game=json.loads((pack/"game.json").read_text(encoding="utf-8-sig"));rows=[];provenance=[]
    for name,title,bottle,script_name in [("electric_cherry","Electric Cherry","cherry","electric_cherry"),("phd_flopper","PhD Flopper","nuke","divetonuke")]:
        script=inspection/"solo-content/defensive-perks-recovery/decompiled/t6"/("_zm_perk_"+script_name+".gsc")
        text=script.read_text();weapon=next(w for w in source["weapons"] if w["source"]=="zombie_perk_bottle_"+bottle)
        price=int(re.search(r'register_perk_basic_info\([^,]+,[^,]+, (\d+),',text)[1])
        row={"id":"perk."+name,"name":title,"price":price,"icon":None,"drink":{"bottle":"weapon."+weapon["source"],"seconds":float(weapon["fields"]["firstRaiseTime"]),"lower_seconds":float(weapon["fields"]["dropTime"]),"activation_delay":0.5}}
        if name=="electric_cherry":
            radius=[float(x)*0.0254 for x in re.search(r'perk_radius = linear_map\( n_fraction, 1.0, 0.0, (\d+), (\d+) \)',text).groups()]
            damage=[int(x) for x in re.search(r'perk_dmg = linear_map\( n_fraction, 1.0, 0.0, (\d+), (\d+) \)',text).groups()]
            cooldown=float(re.search(r'n_cooldown_time = n_reload_time \+ (\d+);',text)[1])
            row["cherry"]={"radius":radius,"damage":damage,"cooldown_extra":cooldown,"stun_seconds":3.0}
        else: row["explosive_protection"]=True
        game["content"]["perks"][:]=[p for p in game["content"]["perks"] if p["id"]!=row["id"]];game["content"]["perks"].append(row);rows.append(row)
        provenance.append({"name":name,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"bottle_sha256":weapon["sha256"]})
    (pack/"game.json").write_text(json.dumps(game,indent=2)+"\n")
    (inspection/"solo-content/defensive-perks-preparation.json").write_text(json.dumps({"perks":rows,"provenance":provenance,"approximations":["Cherry ammunition fraction uses intended floating division; damage rounds to integer.","Native two stun-animation cycles reconstruct as three seconds idle with attacks/movement suppressed.","Bounded blue discharge markers reconstruct missing shock FX; original audio pending.","Cherry native nearest-target limits and 0.1-second sequential impacts are retained; source-specific 40-point deaths enter once-only rewards.","PhD explosive protection is reconstructed as rejected explosive impacts before health/immunity changes; fall protection/dive explosions remain unfinished.","All nine perk catalog entries now exist; Wunderfizz, challenge acquisition, icons/audio and power remain unfinished."]},indent=2))
    print(json.dumps(rows))
