"""Convert resolved BO2 weapon fields into typed, reusable runtime definitions."""
import hashlib
import json
import re
from pathlib import Path

try:
    from .prepare_gameplay_content import external
except ImportError:
    from prepare_gameplay_content import external

ACTIONS = {
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
        candidate = name.removesuffix("_zm") + "_upgraded_zm"
        upgrade = "weapon." + candidate if "upgraded" not in name and candidate in identities else None
        recoil_pitch = sorted([number("hipViewKickPitchMin") * 0.01, number("hipViewKickPitchMax") * 0.01])
        recoil_yaw = sorted([number("hipViewKickYawMin") * 0.01, number("hipViewKickYawMax") * 0.01])
        key = f.get("displayName", "")
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
                "ads_fov": max(1., min(179., number("adsZoomFov1", 50.))),
                "ads_seconds": [positive("adsTransInTime", 0.2), positive("adsTransOutTime", 0.2)],
                "movement": [positive("moveSpeedScale", 1.), positive("adsMoveSpeedScale", 0.5)],
                "empty_reload": positive("reloadEmptyTime", 1.), "shell_reload": f.get("segmentedReload") == "1",
                "shell_seconds": positive("reloadAddTime", 0.5), "equip_seconds": positive("raiseTime", 0.5),
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
    content["definitions"] = sorted(set(content["definitions"]) | ammunition)
    (pack / "game.json").write_text(json.dumps(game, indent=2) + "\n", encoding="utf-8")
    report = {"weapons": len(weapons), "strings": len(strings), "ammunition_groups": len(ammunition),
        "missing_presentations": unresolved,
        "approximations": ["Native view-kick values use a 0.01 reconstruction scale.",
                           "Penetration classes map to tunable material depths.",
                           "Zero action times use bounded runtime defaults; script-only damage needs dedicated effects."]}
    (inspection / "solo-content/arsenal-preparation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "missing_presentations"}))
