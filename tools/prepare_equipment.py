"""Prepare throwable tuning from retained resolved weapon definitions."""
import json
from prepare_gameplay_content import external

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    rows = {r["id"].replace("weapons.","weapon."):r for r in source["weapons"]}
    receipts = []
    for weapon in game["content"]["weapons"]:
        if weapon["id"] != "weapon.frag_grenade_zm": continue
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
        }
        receipts.append({"weapon":weapon["id"],"source_sha256":row["sha256"],"tuning":weapon["throwable"]})
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    report={"definitions":receipts,"approximations":["4.5 cm sweep radius and default-surface bounce reconstruct collision; material-specific responses remain pending.","Grenade gravity uses shared game tuning; radial damage is linear between retained inner/outer values, with world obstruction."]}
    (inspection / "solo-content/equipment-preparation.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report))
