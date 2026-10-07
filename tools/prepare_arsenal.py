"""Convert resolved BO2 weapon fields into typed, reusable runtime definitions."""
import hashlib
import json
import math
import csv
import re
from pathlib import Path

try:
    from .prepare_gameplay_content import external
    from .xanim import decode
    from .prepare_ads import optics
except ImportError:
    from prepare_gameplay_content import external
    from xanim import decode
    from prepare_ads import optics

ACTIONS = {
    "prime": "holdFireAnim",
    "idle": "idleAnim", "fire": "fireAnim", "reload": "reloadAnim", "reload_empty": "reloadEmptyAnim",
    "reload_start": "reloadStartAnim", "reload_end": "reloadEndAnim", "equip": "raiseAnim",
    "unequip": "dropAnim", "melee": "meleeAnim", "ads_enter": "adsUpAnim", "ads_leave": "adsDownAnim",
    "sprint_enter": "sprintInAnim", "sprint_loop": "sprintLoopAnim", "sprint_leave": "sprintOutAnim",
    "prone_enter": "crawlInAnim", "prone_loop": "crawlForwardAnim", "prone_leave": "crawlOutAnim",
    "dive_enter": "dtp_in", "dive_loop": "dtp_loop", "dive_leave": "dtp_out",
}


def localized_strings(directory):
    result = {}
    # Patch strings take precedence over base; map overrides take precedence last.
    files = sorted(directory.glob("*.str"), key=lambda p: ("tomb" in p.name, "patch" in p.name, p.name))
    for path in files:
        text = path.read_text(encoding="utf-8-sig")
        for key, value in re.findall(r'REFERENCE\s+(\S+)\s+LANG_ENGLISH\s+("(?:[^"\\]|\\.)*")', text):
            if key.startswith('"'):
                key = json.loads(key)
            result[key] = json.loads(value)
    return result


def ads_pose(fields, inspection):
    """Retain the authored torso trajectory in its parent-local native basis."""
    import numpy as np
    clips = inspection / "solo-content/source/xanim"
    idle_path, ads_path = [clips / fields.get(k, "") for k in ("idleAnim", "adsUpAnim")]
    if not idle_path.is_file() or not ads_path.is_file():
        return []
    idle = decode(idle_path.read_bytes())["tracks"].get("tag_torso", {})
    clip = decode(ads_path.read_bytes())
    torso = clip["tracks"].get("tag_torso", {})
    if "translation" not in torso or "rotation" not in torso:
        return []
    translation, rotation = torso["translation"], torso["rotation"]
    base_t = idle.get("translation", translation)[1][0]
    base_q = idle.get("rotation", rotation)[1][0]
    result = []
    for time in sorted(set([0., clip["duration"], *translation[0], *rotation[0]])):
        t = np.asarray([np.interp(time, translation[0], translation[1][:, i]) for i in range(3)])
        q = np.asarray([np.interp(time, rotation[0], rotation[1][:, i]) for i in range(4)])
        q /= np.linalg.norm(q)
        vector = base_q[3] * q[:3] - q[3] * base_q[:3] - np.cross(base_q[:3], q[:3])
        delta_q = np.asarray([*vector, base_q[3] * q[3] + np.dot(base_q[:3], q[:3])])
        delta_q /= np.linalg.norm(delta_q)
        result.append({"fraction": float(time / clip["duration"]),
                       "translation": ((t - base_t) * .0254).tolist(), "rotation": delta_q.tolist()})
    return result


def prepare_ads(config):
    """Refresh only ADS metadata in an already assembled external runtime pack."""
    inspection = external(config["assets"]["inspection_output"])
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    fields = {"weapon." + w["source"]: w["fields"] for w in source["weapons"]}
    pack = external(config["game"]["asset_pack"])
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    for weapon in game["content"]["weapons"]:
        f = fields[weapon["id"]]
        m = weapon["mechanics"]
        m["ads_zoom_fraction"] = [min(1., max(0., float(f[k] if f.get(k) is not None else 1.))) for k in ("adsZoomInFrac", "adsZoomOutFrac")]
        m["ads_pose"] = ads_pose(f, inspection)
        m["ads_sight"] = optics(f, pack, m["ads_pose"], weapon["id"].removeprefix("weapon."))
    (pack / "game.json").write_text(json.dumps(game, indent=2) + "\n", encoding="utf-8")


def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    source = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    pack = external(config["game"]["asset_pack"])
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    content = game["content"]
    strings = localized_strings(inspection / "solo-content/localized/english/localizedstrings")
    content["localized_strings"] = [{"id": "string." + key, "key": key, "text": value} for key, value in sorted(strings.items())]
    identities = {w["source"] for w in source["weapons"]}
    presentations = {p["id"] for p in content["presentations"]}
    clips = {a["id"] for a in content["animations"]}
    weapons, ammunition = [], set()
    unresolved = []
    for record in source["weapons"]:
        f, name = record["fields"], record["source"]
        def number(key, fallback=0.):
            return float(f.get(key) or fallback)
        def integer(key, fallback=0):
            return int(number(key, fallback))
        def positive(key, fallback):
            return max(number(key, fallback), fallback if number(key, fallback) <= 0 else 0.)
        delivery = {"bullet": "hitscan", "projectile": "projectile", "melee": "melee"}.get(f["weaponType"], "equipment")
        magazine = integer("clipSize")
        clip_relative = f.get("ammoCountClipRelative") == "1"
        capacity = integer("maxAmmo") * (magazine if clip_relative else 1)
        starting = min(capacity, integer("startAmmo") * (magazine if clip_relative else 1))
        ammo_name = f.get("ammoName") or name
        ammo_id = "ammo." + hashlib.sha256(ammo_name.encode()).hexdigest()[:20]
        ammunition.add(ammo_id)
        pattern = f["fireType"]
        fire = {"mode": {"Single Shot": "semi", "Full Auto": "automatic", "Charge Shot": "charge"}.get(pattern, "burst")}
        if fire["mode"] == "burst":
            fire["shots"] = int(pattern.split("-", 1)[0])
        category = {"hitscan": "bullet", "projectile": "explosive", "melee": "melee", "equipment": "explosive"}[delivery]
        for element, category_name in (("staff_fire", "fire"), ("staff_water", "ice"), ("staff_air", "wind"), ("staff_lightning", "lightning")):
            if name.startswith(element):
                category = category_name
        model = f.get("gunModel", "")
        presentation = "actor." + model if "actor." + model in presentations else None
        world_model = f.get("worldModel", "")
        pickup = "actor." + world_model if "actor." + world_model in presentations else None
        actions = {}
        for action, field in ACTIONS.items():
            clip_id = "animation." + model + "." + f.get(field, "")
            if clip_id in clips:
                actions[action] = clip_id
        if delivery == "melee":
            # Knife tracks animate the hands, not the rigid knife mesh.
            clip_id = "animation.c_zom_dempsey_viewhands." + f.get("meleeAnim", "")
            if clip_id in clips:
                actions["melee"] = clip_id
        candidate = name.removesuffix("_zm") + "_upgraded_zm"
        upgrade = "weapon." + candidate if "upgraded" not in name and candidate in identities else None
        recoil_pitch = sorted([number("hipViewKickPitchMin") * 0.01, number("hipViewKickPitchMax") * 0.01])
        recoil_yaw = sorted([number("hipViewKickYawMin") * 0.01, number("hipViewKickYawMax") * 0.01])
        key = f.get("displayName", "")
        # These tracks are local to tag_view's converted basis. Keep parent-local
        # coordinates and derive a delta from the authored idle torso pose.
        ads_translation = [0., 0., 0.]
        ads_rotation = [0., 0., 0., 1.]
        native_clips = inspection / "solo-content/source/xanim"
        idle_path, ads_path = [native_clips / f.get(k, "") for k in ("idleAnim", "adsUpAnim")]
        if idle_path.is_file() and ads_path.is_file():
            idle = decode(idle_path.read_bytes())["tracks"].get("tag_torso", {})
            ads = decode(ads_path.read_bytes())["tracks"].get("tag_torso", {})
            if "translation" in ads:
                base = idle.get("translation", ads["translation"])[1][0]
                ads_translation = ((ads["translation"][1][-1] - base) * 0.0254).tolist()
            if "rotation" in ads:
                a, b = idle.get("rotation", ads["rotation"])[1][0], ads["rotation"][1][-1]
                # inverse(a) * b, native normalized quaternions in xyzw order.
                import numpy as np
                vector = a[3] * b[:3] - b[3] * a[:3] - np.cross(a[:3], b[:3])
                ads_rotation = [*vector.tolist(), float(a[3] * b[3] + np.dot(a[:3], b[:3]))]
        weapon = {
            "id": "weapon." + name, "name": strings.get(key) or key or name,
            "localization_key": key or None, "damage": integer("meleeDamage") if delivery == "melee" else integer("damage"),
            "category": category, "magazine": magazine, "reserve": capacity,
            "fire_interval": positive("fireTime", 0.1), "reload_seconds": positive("reloadTime", 1.),
            "upgraded": upgrade, "presentation": presentation, "pickup": pickup,
            "ammo_group": ammo_id, "animation_actions": actions,
            "mechanics": {
                "delivery": delivery, "fire": fire, "starting_reserve": starting,
                "minimum_damage": integer("minDamage"),
                "damage_range": sorted([number("maxDamageRange") * 0.0254, number("minDamageRange") * 0.0254]),
                "hit_multipliers": [number(k, 1.) for k in ("locHead", "locTorsoUpper", "locTorsoMid", "locRightArmUpper", "locRightLegLower")],
                "pellets": max(1, integer("shotCount")),
                "penetration": {"none": 0., "small": 0.15, "medium": 0.3, "large": 0.5}.get(f.get("penetrateType"), 0.),
                "recoil_pitch": recoil_pitch, "recoil_yaw": recoil_yaw,
                "recoil_recovery": positive("hipViewKickCenterSpeed", 800.) * 0.01,
                "spread": [number(k) for k in ("hipSpreadStandMin", "hipSpreadDuckedMin", "hipSpreadProneMin", "adsSpread")],
                # BO2 authored horizontal FOV uses its reference 4:3 aspect.
                "ads_fov": max(1., math.degrees(2 * math.atan(math.tan(math.radians(max(1., min(179., number("adsZoomFov1", 50.)))) / 2) * 0.75))),
                "ads_seconds": [positive("adsTransInTime", 0.2), positive("adsTransOutTime", 0.2)],
                "ads_zoom_fraction": [min(1., max(0., number(k, 1.))) for k in ("adsZoomInFrac", "adsZoomOutFrac")],
                "ads_pose": ads_pose(f, inspection),
                "ads_sight": optics(f, pack, ads_pose(f, inspection), name),
                "ads_enabled": f.get("aimDownSight") == "1",
                "ads_translation": ads_translation, "ads_rotation": ads_rotation,
                "burst_delay": number("burstFireDelay"),
                "movement": [positive("moveSpeedScale", 1.), min(1., positive("adsMoveSpeedScale", 0.5))],
                "empty_reload": positive("reloadEmptyTime", 1.), "shell_reload": f.get("segmentedReload") == "1",
                "shell_seconds": positive("reloadAddTime", 0.5), "equip_seconds": positive("raiseTime", 0.5),
                "reload_start_seconds": number("reloadStartTime"), "reload_start_insert": number("reloadStartAddTime"),
                "reload_end_seconds": number("reloadEndTime"),
                "unequip_seconds": positive("dropTime", 0.3), "dual_wield": f.get("dualWield") == "1",
                "projectile_speed": number("projectileSpeed") * 0.0254,
                "projectile_lifetime": positive("projectileLifetime", 10.),
                "explosion_radius": number("explosionRadius") * 0.0254,
                "explosion_damage": [integer("explosionInnerDamage"), integer("explosionOuterDamage")],
                "melee_seconds": [number("meleeDelay"), number("meleeTime")], "fuse_seconds": number("fuseTime"),
            },
        }
        if delivery in ("hitscan", "projectile") and (weapon["damage"] == 0 or magazine == 0):
            # Native script-only weapons are equipment commands, never fake gun ammo.
            weapon["mechanics"]["delivery"] = "equipment"
        if model and not presentation:
            unresolved.append({"weapon": name, "missing_model": model})
        weapons.append(weapon)
    content["weapons"] = weapons
    # Retained decompilation is an offline research input, never a runtime script.
    script = external(config.get("preparation", {}).get("weapon_script", inspection / "damage-life-state-03/decompiled/t6/zm_tomb.gsc"))
    prices = {}
    for name, price, ammo in re.findall(r'add_zombie_weapon\(\s*"([^"]+)"\s*,[^,]*,[^,]*,\s*(\d+)\s*,[^,]*,[^,]*,\s*(undefined|\d+)', script.read_text()):
        prices[name] = [int(price), None if ammo == "undefined" else int(ammo)]
    table = external(config.get("preparation", {}).get("weapon_prices", inspection / "mechanics-rounds-02/table-dump/mp/zombiemode.csv"))
    rows = list(csv.reader(table.read_text().splitlines()))
    native_points = {row[0]: int(row[1]) for row in rows if row[0].startswith("zombie_score_") and row[1]}
    game.setdefault("tuning", {})["combat_points"] = [native_points["zombie_score_" + key] for key in
        ("damage_normal", "kill_1player", "bonus_head", "bonus_torso", "bonus_melee")]
    for row in rows:
        if row[0] in prices:
            for column in (1, 2):
                if row[column]:
                    prices[row[0]][column-1] = math.ceil(int(row[column]) / 10) * 10
    # Map initialization applies these overrides after the common CSV lookup.
    for name, price in re.findall(r'change_weapon_cost\(\s*"([^"]+)"\s*,\s*(\d+)\s*\)', script.read_text()):
        prices[name] = [int(price), math.ceil(int(int(price) * 0.5) / 10) * 10]
    catalog = {w["id"]: w for w in weapons}
    wall_placements, wall_interactions = [], []
    for entity in source["entities"]:
        fields = entity["fields"]
        name = fields.get("zombie_weapon_upgrade")
        if fields.get("targetname") != "weapon_upgrade" or "weapon." + str(name) not in catalog:
            continue
        weapon = catalog["weapon." + name]
        if weapon["mechanics"]["delivery"] != "hitscan":
            continue # Offhand equipment gets its dedicated inventory/action path.
        if name not in prices:
            raise ValueError("Missing native wall price: " + name)
        price, ammo = prices[name]
        placement = "placement.wall." + entity["id"].removeprefix("entity.")
        wall_placements.append({"id": placement, "definition": weapon["id"], "position": entity["position"],
                                "yaw": math.radians(float(fields.get("angles", "0 0 0").split()[1])), "zone": "zone.start"})
        wall_interactions.append({"id": "interaction.wall." + entity["id"].removeprefix("entity."),
                                  "placement": placement, "action": {"kind": "weapon", "weapon": weapon["id"], "price": price,
                                  "ammo_price": ammo if ammo is not None else math.ceil(int(price * 0.5) / 10) * 10},
                                  "reach": 2., "hold_seconds": 0., "hidden_prompt": False})
    content["placements"] = [p for p in content["placements"] if not p["id"].startswith("placement.wall.")] + wall_placements
    content["interactions"] = [p for p in content["interactions"] if not p["id"].startswith("interaction.wall.")] + wall_interactions
    content["definitions"] = sorted(set(content["definitions"]) | ammunition)
    (pack / "game.json").write_text(json.dumps(game, indent=2) + "\n", encoding="utf-8")
    report = {"weapons": len(weapons), "strings": len(strings), "ammunition_groups": len(ammunition),
        "wall_buys": len(wall_interactions), "weapon_script_sha256": hashlib.sha256(script.read_bytes()).hexdigest(),
        "weapon_price_table_sha256": hashlib.sha256(table.read_bytes()).hexdigest(),
        "missing_presentations": unresolved,
        "approximations": ["Native view-kick values use a 0.01 reconstruction scale.",
                           "ADS horizontal FOV uses the 4:3 reference aspect; movement scales are capped at one.",
                           "Penetration classes map to tunable material depths.",
                           "Zero action times use bounded runtime defaults; script-only damage needs dedicated effects."]}
    (inspection / "solo-content/arsenal-preparation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "missing_presentations"}))
