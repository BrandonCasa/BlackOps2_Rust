"""Prepare throwable tuning from retained resolved weapon definitions."""
import json
import re
import math
import hashlib
from prepare_gameplay_content import external

def prepare_prime_clips(content, rows, inspection, pack):
    from prepare_actors import convert
    equipment=[w for w in content["weapons"] if w["id"] in (
        "weapon.frag_grenade_zm","weapon.sticky_grenade_zm","weapon.cymbal_monkey_zm","weapon.claymore_zm")]
    primes={rows[w["id"]]["fields"]["holdFireAnim"] for w in equipment}
    # Reconstructed camera framing; native joint tracks and attachment sockets stay intact.
    for w in equipment: w["view_offset"]=[0.,.14,-.15]
    models={rows[w["id"]]["fields"]["gunModel"]:{rows[w["id"]]["fields"]["holdFireAnim"]} for w in equipment}
    models.update({"c_zom_"+name+"_viewhands":primes for name in ("dempsey","nikolai","richtofen","takeo")})
    for model,extra in models.items():
        identity="actor."+model
        existing=sorted((a for a in content["animations"] if a["skeleton"]==identity),
            key=lambda a:int(a["path"].rsplit("Animation",1)[1]))
        names=[a["id"].rsplit(".",1)[1] for a in existing]
        names+=sorted(extra-set(names))
        relative,notes=convert(inspection / "game-prepare/models/model_export" / (model+"_lod0.gltf"),
            pack / "actors",inspection / "solo-content/actor-images",
            [inspection / "solo-content/source/xanim" / name for name in names])
        generated={}
        references=[]
        for index,(name,notifications) in enumerate(notes):
            clip="animation."+model+"."+name
            references.append(clip)
            generated[clip]={"id":clip,"path":"actors/"+relative+f"#Animation{index}",
                "skeleton":identity,"notifications":notifications}
        content["animations"][:]=[generated.pop(a["id"],a) for a in content["animations"]]
        content["animations"].extend(generated.values())
        presentation=next(p for p in content["presentations"] if p["id"]==identity)
        presentation["animations"]=references
        for w in equipment:
            if rows[w["id"]]["fields"]["gunModel"]==model:
                clip="animation."+model+"."+rows[w["id"]]["fields"]["holdFireAnim"]
                if clip not in references: raise ValueError("missing equipment prime animation: "+clip)
                w["animation_actions"]["prime"]=clip

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    rows = {r["id"].replace("weapons.","weapon."):r for r in source["weapons"]}
    prepare_prime_clips(game["content"],rows,inspection,pack)
    receipts = []
    round_script=inspection / "damage-life-state-06/decompiled/t6/_zm.gsc"
    round_text=round_script.read_text().split("award_grenades_for_survivors()\n{",1)[1].split("\nai_calculate_health",1)[0]
    awards=[int(v) for v in re.findall(r'setweaponammoclip\( lethal_grenade, (\d+) \)',round_text)]
    if awards != [0,2,3,4]: raise ValueError("native survivor equipment award changed")
    receipts.append({"round_script_sha256":hashlib.sha256(round_script.read_bytes()).hexdigest(),
        "round_award":"two lethal charges up to four at round start; tactical stock unchanged"})
    for weapon in game["content"]["weapons"]:
        if weapon["id"] not in ("weapon.frag_grenade_zm","weapon.sticky_grenade_zm","weapon.cymbal_monkey_zm"): continue
        row = rows[weapon["id"]]; f = row["fields"]
        if f["offhandSlot"]!="Tactical grenade" and int(f["maxAmmo"])!=4:
            raise ValueError("survivor lethal award requires native four-charge capacity")
        weapon["throwable"] = {
            "slot":"tactical" if f["offhandSlot"]=="Tactical grenade" else "lethal",
            "round_award":0 if f["offhandSlot"]=="Tactical grenade" else 2,
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
    from prepare_actors import convert
    monkey_model="t6_wpn_zmb_monkey_bomb_world"
    relative,notifications=convert(inspection / "game-prepare/models/model_export" / (monkey_model+"_lod0.gltf"),
        pack / "actors",inspection / "solo-content/actor-images",[inspection / "solo-content/source/xanim/o_monkey_bomb"])
    identity="actor."+monkey_model
    content["presentations"][:]=[p for p in content["presentations"] if p["id"]!=identity]
    content["animations"][:]=[a for a in content["animations"] if a["skeleton"]!=identity]
    clip="animation."+monkey_model+".o_monkey_bomb"
    content["animations"].append({"id":clip,"path":"actors/"+relative+"#Animation0","skeleton":identity,"notifications":notifications[0][1]})
    content["presentations"].append({"id":identity,"scene":"actors/"+relative+"#Scene0","animations":[clip],"sounds":[]})
    monkey=next(w for w in content["weapons"] if w["id"]=="weapon.cymbal_monkey_zm")
    monkey["name"]="Monkey Bomb"
    monkey["lure"]={"range":1536*.0254,"capacity":96,"spacing":45*.0254,"animation":clip}
    monkey_script=inspection / "solo-content/monkey-recovery/decompiled/t6/_zm_weap_cymbal_monkey.gsc"
    receipts.append({"weapon":monkey["id"],"script_sha256":hashlib.sha256(monkey_script.read_bytes()).hexdigest(),"lure":monkey["lure"],"activation":"stationary after native prime/flight; reset native fuse on activation"})
    model="t6_wpn_claymore_world"
    relative,_=convert(inspection / "game-prepare/models/model_export" / (model+"_lod0.gltf"),
        pack / "actors",inspection / "solo-content/actor-images",[])
    presentation="actor."+model
    content["presentations"][:]=[p for p in content["presentations"] if p["id"]!=presentation]
    content["presentations"].append({"id":presentation,"scene":"actors/"+relative+"#Scene0","animations":[],"sounds":[]})
    claymore=next(w for w in content["weapons"] if w["id"]=="weapon.claymore_zm")
    row=rows[claymore["id"]]; f=row["fields"]
    native_mines=inspection / "solo-content/claymore-recovery/decompiled/t6/_zm_weap_claymore.gsc"
    script_text=native_mines.read_text()
    mine_price=int(re.search(r'self.zombie_cost\s*=\s*(\d+)',script_text)[1])
    limit=int(re.search(r'claymores_max_per_player\s*=\s*(\d+)',script_text)[1])
    claymore["mine"]={"stock":int(f["startAmmo"]),"limit":limit,"place_seconds":float(f["holdFireTime"]),
        "recovery_seconds":float(f["fireTime"]),"trigger_radius":96*.0254,"trigger_cos":math.cos(math.radians(70)),
        "minimum_forward":20*.0254,"alert_seconds":.4,"unsafe_seconds":.1,"radius":float(f["explosionRadius"])*.0254,
        "inner_damage":int(f["explosionInnerDamage"]),"outer_damage":int(f["explosionOuterDamage"]),
        "blast_cos":math.cos(math.radians(float(f["damageConeAngle"]))),"presentation":presentation}
    receipts.append({"weapon":claymore["id"],"source_sha256":row["sha256"],
        "script_sha256":hashlib.sha256(native_mines.read_bytes()).hexdigest(),"tuning":claymore["mine"]})
    content["placements"][:]=[p for p in content["placements"] if not p["id"].startswith("placement.mine.")]
    content["interactions"][:]=[i for i in content["interactions"] if not i["id"].startswith("interaction.mine.")]
    mine_buys=0
    for entity in source["entities"]:
        f=entity["fields"]
        if f.get("targetname")!="claymore_purchase":continue
        mine_buys+=1
        suffix=entity["id"].removeprefix("entity.");placement="placement.mine."+suffix
        content["placements"].append({"id":placement,"definition":claymore["id"],"position":entity["position"],
            "yaw":math.radians(float(f["angles"].split()[1])),"zone":"zone.start"})
        content["interactions"].append({"id":"interaction.mine."+suffix,"placement":placement,"action":{
            "kind":"weapon","weapon":claymore["id"],"price":mine_price,"ammo_price":None},"reach":2.,"hold_seconds":0.,"hidden_prompt":False})
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
    report={"definitions":receipts,"semtex_buys":buys,"semtex_price":price,"claymore_buys":mine_buys,"claymore_price":mine_price,"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"approximations":["Claymore placement resolves ground directly rather than a deployed mine flight; 8 cm offset and 6 cm clearance reconstruct support. Detection uses target movement-capsule centers; blast uses a retained-angle dot cone and linear radial damage, not recovered native damageconetrace arithmetic. Lasers, retrieval, enemy damage/chain and EMP interactions remain pending.","4.5 cm sweep radius and default-surface bounce reconstruct collision; material-specific responses remain pending.","Grenade gravity uses shared game tuning; radial damage is linear between retained inner/outer values, with world obstruction.","Sticky actor collision uses movement capsules; attachment follows actor roots, not animated skin vertices. Native zone gating remains pending."]}
    (inspection / "solo-content/equipment-preparation.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report))
