"""Prepare throwable tuning from retained resolved weapon definitions."""
import json
import re
import math
import hashlib
from prepare_gameplay_content import external

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    rows = {r["id"].replace("weapons.","weapon."):r for r in source["weapons"]}
    receipts = []
    for weapon in game["content"]["weapons"]:
        if weapon["id"] not in ("weapon.frag_grenade_zm","weapon.sticky_grenade_zm"): continue
        row = rows[weapon["id"]]; f = row["fields"]
        weapon["throwable"] = {
            "initial":int(f["startAmmo"]), "capacity":int(f["maxAmmo"]),
            "prime_seconds":float(f["holdFireTime"]), "throw_seconds":float(f["fireTime"]),
            "fuse_seconds":float(f["fuseTime"]), "speed":float(f["projectileSpeed"])*.0254,
            "up_speed":float(f["projectileSpeedRelativeUp"])*.0254,
            "radius":float(f["explosionRadius"])*.0254,
            "inner_radius":float(f["explosionRadiusMin"])*.0254,
            "inner_damage":int(f["explosionInnerDamage"]), "outer_damage":int(f["explosionOuterDamage"]),
            "tangent_bounce":float(f["parallelDefaultBounce"]),
            "normal_bounce":float(f["perpendicularDefaultBounce"]),
            "collision_radius":.045,
            "cook_in_hand":f["cookOffHold"]=="1",
            "sticky":f["stickiness"]=="Stick to all",
        }
        receipts.append({"weapon":weapon["id"],"source_sha256":row["sha256"],"tuning":weapon["throwable"]})
    script = external(config.get("preparation",{}).get("weapon_script", inspection / "damage-life-state-03/decompiled/t6/zm_tomb.gsc"))
    price_match=re.search(r'add_zombie_weapon\(\s*"sticky_grenade_zm"\s*,[^,]*,[^,]*,\s*(\d+)',script.read_text())
    if price_match is None: raise ValueError("native Semtex price missing")
    price=int(price_match.group(1))
    content=game["content"]
    content["placements"][:]=[p for p in content["placements"] if not p["id"].startswith("placement.lethal.")]
    content["interactions"][:]=[i for i in content["interactions"] if not i["id"].startswith("interaction.lethal.")]
    buys=0
    for entity in source["entities"]:
        f=entity["fields"]
        if f.get("targetname")!="weapon_upgrade" or f.get("zombie_weapon_upgrade")!="sticky_grenade_zm":continue
        suffix=entity["id"].removeprefix("entity."); placement="placement.lethal."+suffix
        content["placements"].append({"id":placement,"definition":"weapon.sticky_grenade_zm","position":entity["position"],
            "yaw":math.radians(float(f["angles"].split()[1])),"zone":"zone.start"})
        content["interactions"].append({"id":"interaction.lethal."+suffix,"placement":placement,"action":{
            "kind":"weapon","weapon":"weapon.sticky_grenade_zm","price":price,"ammo_price":price},
            "reach":2.,"hold_seconds":0.,"hidden_prompt":False})
        buys+=1
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    report={"definitions":receipts,"semtex_buys":buys,"semtex_price":price,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"approximations":["4.5 cm sweep radius and default-surface bounce reconstruct collision; material-specific responses remain pending.","Grenade gravity uses shared game tuning; radial damage is linear between retained inner/outer values, with world obstruction.","Sticky actor collision uses movement capsules; attachment follows actor roots, not animated skin vertices. Native zone gating remains pending."]}
    (inspection / "solo-content/equipment-preparation.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report))
