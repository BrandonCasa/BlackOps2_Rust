"""Version-two stable IDs and typed content dependency validation.

Used by preparation, import, and packaging; never enumerates optional raw inputs.
"""
import math
import re

TABLES = {
    "barriers": ("barrier", "id placement entry exit boards repair_sound break_sound"),
    "weapons": ("weapon", "id name damage category magazine reserve fire_interval reload_seconds upgraded presentation"),
    "enemies": ("enemy", "id health speed damage presentation"),
    "perks": ("perk", "id name price icon"),
    "powerups": ("powerup", "id duration presentation"),
    "zones": ("zone", "id initially_open"),
    "placements": ("placement", "id definition position yaw zone"),
    "interactions": ("interaction", "id placement action reach hold_seconds hidden_prompt"),
    "zone_connections": ("connection", "id from to initially_open"),
    "navigation_nodes": ("node", "id position zone"),
    "navigation_links": ("link", "id from to traversal connection"),
    "generators": ("generator", "id placement zones price capture_seconds radius number decay_seconds shutdown_seconds recovery_seconds presentation model_position emergence clip_seconds capture_sound active_sound"),
    "robot_routes": ("route", "id points speed cooldown presentation"),
    "tank_routes": ("route", "id points speed cooldown presentation"),
    "weather": ("weather", "id duration snow sound"),
    "digging": ("dig", "id placement snow_only loot"),
    "recipes": ("recipe", "id parts result bench"),
    "staff_puzzles": ("puzzle", "id weapon interactions solution souls"),
    "quests": ("quest", "id initial_stage"),
    "quest_transitions": ("transition", "id quest from to event prerequisites"),
    "presentations": ("presentation", "id scene animations sounds"),
    "sounds": ("sound", "id path volume pitch distance looping"),
    "animations": ("animation", "id path skeleton notifications"),
    "ui_assets": ("ui", "id path"),
    "cinematics": ("cinematic", "id frames frames_per_second audio"),
    "localized_strings": ("string", "id key text"),
}


def initial_catalog():
    """Carry the existing playable starter data forward; unrecovered tables stay empty."""
    content = {table: [] for table in TABLES}
    content["definitions"] = []
    content["zones"] = [{"id": "zone.start", "initially_open": True}]
    content["weapons"] = [{"id": "weapon.mauser", "name": "Mauser C96",
        "damage": 50, "category": "bullet", "magazine": 8, "reserve": 32,
        "fire_interval": 0.2, "reload_seconds": 1.5, "upgraded": None, "presentation": None}]
    content["enemies"] = [{"id": "enemy.zombie", "health": 150, "speed": 1.15,
                           "damage": 50, "presentation": None}]
    return content


def validate_content(content, require):
    require(isinstance(content, dict) and set(content) == set(TABLES) | {"definitions"},
            "incomplete version-two content tables")
    index = {}

    def register(identity, kind):
        require(isinstance(identity, str) and re.fullmatch(r"[A-Za-z0-9_.-]+", identity), "malformed stable ID")
        require(identity not in index, f"duplicate stable ID: {identity}")
        index[identity] = kind

    require(isinstance(content["definitions"], list), "invalid definition IDs")
    for identity in content["definitions"]:
        register(identity, "definition")
    for table, (kind, fields) in TABLES.items():
        require(isinstance(content[table], list), f"invalid content table: {table}")
        for row in content[table]:
            extras = {"mechanics", "pickup", "ammo_group", "localization_key", "animation_actions", "throwable", "mine", "lure", "view_offset"} if table == "weapons" else {"parts", "hit_regions", "animation_actions", "attack_seconds", "attack_hits", "death_seconds", "corpse_seconds", "bounds_margin", "family", "traversals"} if table == "enemies" else {"presentation", "drink", "maximum_health", "reload_duration_scale", "sprint", "third_weapon", "firing", "cherry", "explosive_protection", "fall_protection", "dive"} if table == "perks" else {"blockers","source_animation"} if table=="navigation_links" else set()
            require(isinstance(row, dict) and set(fields.split()) <= set(row) <= set(fields.split()) | extras, f"invalid {table} fields")
            register(row["id"], kind)

    def resolve(owner, identity, kind="any", optional=False):
        if optional and identity is None:
            return
        require(isinstance(identity, str) and identity in index and (kind == "any" or index[identity] == kind),
                f"missing or wrong-kind {kind} reference in {owner}: {identity}")

    def number(value, positive=False):
        require(type(value) in (int, float) and math.isfinite(value) and (value > 0 if positive else value >= 0),
                "invalid content number")

    def natural(value, positive=False):
        require(type(value) is int and (value > 0 if positive else value >= 0), "invalid content integer")

    def vector(value):
        require(isinstance(value, list) and len(value) == 3 and all(type(v) in (int, float) and math.isfinite(v) for v in value), "invalid content position")

    def identities(owner, values, kind="any"):
        require(isinstance(values, list), "invalid content reference list")
        for identity in values:
            resolve(owner, identity, kind)

    dependencies = []
    def path(value):
        require(isinstance(value, str) and value, "empty content asset path")
        base = value.split("#", 1)[0]
        require(not re.search(r"[:\\%\x00-\x1f]", base) and all(p not in ("", ".", "..") for p in base.split("/")), "unsafe content asset path")
        dependencies.append(value)

    for table, rows in content.items():
        if table == "definitions":
            continue
        for row in rows:
            owner = row["id"]
            for key in ("damage", "magazine", "health", "souls"):
                if key in row:
                    uses_ammo = row.get("mechanics", {}).get("delivery", "hitscan") in ("hitscan", "projectile")
                    natural(row[key], key != "souls" and (table != "weapons" or uses_ammo))
            for key in ("reserve", "price"):
                if key in row:
                    natural(row[key])
            for key in ("speed", "fire_interval", "reload_seconds", "reach", "capture_seconds", "radius", "distance", "frames_per_second"):
                if key in row:
                    number(row[key], True)
            for key in ("cooldown", "hold_seconds", "volume"):
                if key in row:
                    number(row[key])
            for key in ("initially_open", "hidden_prompt", "snow", "snow_only", "looping"):
                if key in row:
                    require(type(row[key]) is bool, "invalid content boolean")
            for key in ("name", "initial_stage"):
                if key in row:
                    require(isinstance(row[key], str) and row[key], "empty content label")
            if "presentation" in row:
                resolve(owner, row["presentation"], "presentation", True)
            if "position" in row:
                vector(row["position"])
            if "view_offset" in row:
                vector(row["view_offset"])
                require(all(abs(v)<2 for v in row["view_offset"]),"invalid weapon view offset")
            if "zone" in row:
                resolve(owner, row["zone"], "zone")
            if table == "enemies":
                for key in ("attack_seconds", "death_seconds", "corpse_seconds", "bounds_margin"):
                    if key in row: number(row[key], True)
                hits = row.get("attack_hits", [])
                require(isinstance(hits,list), "invalid enemy attack notifications")
                for value in hits:
                    number(value)
                    require(value <= row.get("attack_seconds",1.8), "enemy attack notification outside duration")
                require(all(a < b for a,b in zip(hits,hits[1:])), "unordered enemy attack notifications")
                require(isinstance(row.get("family", ""),str), "invalid enemy family")
                parts = row.get("parts", [])
                require(isinstance(parts,list), "invalid actor parts")
                for part in parts:
                    require(isinstance(part,dict) and set(part)=={"presentation","socket","root_joint"}, "invalid actor attachment")
                    resolve(owner,part["presentation"],"presentation")
                    require(isinstance(part["socket"],str) and part["socket"], "invalid actor socket")
                    require(part["root_joint"] is None or isinstance(part["root_joint"],str), "invalid partial rig root")
                actions = row.get("animation_actions", {})
                if row.get("throwable") is not None or row.get("mine") is not None:
                    require({"prime", "idle", "fire"} <= set(actions), "missing equipment deployment animation")
                region_ids = set()
                regions = row.get("hit_regions", [])
                require(isinstance(regions,list), "invalid hit regions")
                for hit in regions:
                    require(isinstance(hit,dict) and set(hit)=={"id","region","presentation","start","end","radius"}, "invalid hit volume")
                    require(isinstance(hit["id"],str) and hit["id"] and all(c.isascii() and (c.isalnum() or c in "._-") for c in hit["id"]) and hit["id"] not in region_ids,"invalid or duplicate hit volume id")
                    region_ids.add(hit["id"])
                    require(hit["region"] in {"head","upper_torso","torso","arm","leg"},"invalid hit region")
                    require(all(isinstance(hit[k],str) and hit[k] for k in ("start","end")),"invalid hit bone")
                    number(hit["radius"],True)
                    require(hit["presentation"] in [row["presentation"]]+[p["presentation"] for p in parts],"hit volume references unrelated rig")
                    resolve(owner,hit["presentation"],"presentation")
                require(isinstance(actions,dict) and set(actions)<={"idle","walk","run","sprint","attack","death"},"invalid enemy animation actions")
                for identity in actions.values(): resolve(owner,identity,"animation")
                profiles=row.get("traversals",{});require(isinstance(profiles,dict),"invalid traversal profiles")
                for alias,profile in profiles.items():
                    require(isinstance(alias,str) and 0<len(alias)<128 and isinstance(profile,dict) and set(profile)=={"animation","seconds","motion"},"invalid traversal fields")
                    resolve(owner,profile["animation"],"animation");number(profile["seconds"],True)
                    require(.1<=profile["seconds"]<=30,"invalid traversal duration")
                    motion=profile["motion"];require(isinstance(motion,list) and 2<=len(motion)<=4096,"invalid traversal samples")
                    require(all(isinstance(p,list) and len(p)==4 and all(type(v) in (int,float) and math.isfinite(v) for v in p) and 0<=p[0]<=profile["seconds"]+.001 for p in motion),"invalid traversal curve")
                    require(motion[0][0]==0 and abs(motion[-1][0]-profile["seconds"])<.001 and all(a[0]<b[0] for a,b in zip(motion,motion[1:])),"unordered traversal curve")
            if table == "weapons" and row.get("throwable") is not None:
                t=row["throwable"]
                require(isinstance(t,dict) and set(t)==set("initial capacity round_award prime_seconds throw_seconds fuse_seconds speed up_speed radius inner_radius inner_damage outer_damage tangent_bounce normal_bounce collision_radius cook_in_hand sticky slot".split()),"invalid throwable fields")
                require(type(t["round_award"]) is int and 0<=t["round_award"]<=t["capacity"],"invalid round equipment award")
                require(t["slot"] in ("lethal","tactical"),"invalid throwable slot")
                for k in ("cook_in_hand","sticky"): require(type(t[k]) is bool,"invalid throwable behavior")
                for k in ("initial","capacity","inner_damage","outer_damage"): require(type(t[k]) is int and t[k]>=0,"invalid throwable count/damage")
                require(0<t["capacity"] and t["initial"]<=t["capacity"] and 0<t["outer_damage"]<=t["inner_damage"],"invalid throwable limits")
                for k in ("prime_seconds","throw_seconds","fuse_seconds","speed","radius","collision_radius"): number(t[k],True)
                number(t["up_speed"])
                number(t["inner_radius"])
                require(t["inner_radius"]<t["radius"],"invalid explosion radius")
                for k in ("tangent_bounce","normal_bounce"):
                    number(t[k]);require(t[k]<=1,"invalid bounce ratio")
            if table == "weapons":
                if row.get("lure") is not None:
                    lure=row["lure"]
                    require(isinstance(lure,dict) and set(lure)==set("range capacity spacing animation".split()),"invalid lure fields")
                    for k in ("range","spacing"):number(lure[k],True)
                    require(type(lure["capacity"]) is int and lure["capacity"]>0 and lure["spacing"]<lure["range"],"invalid attraction capacity")
                    require(row.get("throwable",{}).get("slot")=="tactical","lure requires tactical equipment")
                    resolve(owner,lure["animation"],"animation")
                if row.get("mine") is not None:
                    m=row["mine"]
                    require(isinstance(m,dict) and set(m)==set("stock limit place_seconds recovery_seconds trigger_radius trigger_cos minimum_forward alert_seconds unsafe_seconds radius inner_damage outer_damage blast_cos presentation".split()),"invalid mine fields")
                    for k in ("stock","limit","inner_damage","outer_damage"): require(type(m[k]) is int and m[k]>0,"invalid mine counts")
                    require(m["inner_damage"]>=m["outer_damage"],"invalid mine damage")
                    for k in ("place_seconds","recovery_seconds","trigger_radius","minimum_forward","alert_seconds","unsafe_seconds","radius"):number(m[k],True)
                    for k in ("trigger_cos","blast_cos"):require(isinstance(m[k],(float,int)) and -1<=m[k]<=1,"invalid mine cone")
                    resolve(owner,m["presentation"],"presentation")
                resolve(owner, row["upgraded"], "weapon", True)
                require(row["category"] in ("bullet", "melee", "explosive", "fire", "ice", "lightning", "wind", "environmental", "crush"), "invalid damage category")
                resolve(owner, row.get("pickup"), "presentation", True)
                resolve(owner, row.get("ammo_group"), "definition", True)
                actions = row.get("animation_actions", {})
                require(isinstance(actions, dict) and set(actions) <= {"idle", "prime", "fire", "reload", "reload_empty", "reload_start", "reload_end", "equip", "unequip", "melee", "ads_enter", "ads_leave", "sprint_enter", "sprint_loop", "sprint_leave", "prone_enter", "prone_loop", "prone_leave", "dive_enter", "dive_loop", "dive_leave"}, "invalid weapon animation actions")
                for identity in actions.values():
                    resolve(owner, identity, "animation")
                if "localization_key" in row:
                    require(row["localization_key"] is None or isinstance(row["localization_key"], str), "invalid localization key")
                if "mechanics" in row:
                    m = row["mechanics"]
                    required_fields = "delivery fire starting_reserve minimum_damage damage_range hit_multipliers pellets penetration recoil_pitch recoil_yaw recoil_recovery spread ads_fov ads_seconds ads_enabled ads_translation ads_rotation burst_delay movement empty_reload shell_reload shell_seconds equip_seconds unequip_seconds dual_wield projectile_speed projectile_lifetime explosion_radius explosion_damage melee_seconds fuse_seconds".split()
                    required_fields += ["reload_start_seconds", "reload_start_insert", "reload_end_seconds"]
                    require(isinstance(m, dict) and set(m) == set(required_fields), "invalid weapon mechanics fields")
                    for key in ("reload_start_seconds", "reload_start_insert", "reload_end_seconds"):
                        number(m[key])
                    require(m["delivery"] in ("hitscan", "projectile", "melee", "equipment"), "invalid weapon delivery")
                    fire = m["fire"]
                    require(isinstance(fire, dict) and fire.get("mode") in ("semi", "automatic", "burst", "charge"), "invalid fire pattern")
                    require(set(fire) == ({"mode", "shots"} if fire["mode"] == "burst" else {"mode"}), "invalid fire payload")
                    if fire["mode"] == "burst":
                        natural(fire["shots"], True)
                        require(fire["shots"] <= 8, "invalid burst count")
                    for key in ("starting_reserve", "minimum_damage", "pellets"):
                        natural(m[key], key == "pellets")
                    require(m["starting_reserve"] <= row["reserve"] and m["pellets"] <= 64, "invalid weapon ammunition/pellets")
                    for key, width in (("damage_range", 2), ("hit_multipliers", 5), ("recoil_pitch", 2), ("recoil_yaw", 2), ("spread", 4), ("ads_seconds", 2), ("movement", 2), ("explosion_damage", 2), ("melee_seconds", 2)):
                        require(isinstance(m[key], list) and len(m[key]) == width, "invalid weapon vector")
                        for value in m[key]:
                            if key in ("recoil_pitch", "recoil_yaw"):
                                require(type(value) in (int, float) and math.isfinite(value), "invalid recoil")
                            elif key == "explosion_damage":
                                natural(value)
                            else:
                                number(value, key in ("ads_seconds", "movement"))
                    require(all(m[key][0] <= m[key][1] for key in ("damage_range", "recoil_pitch", "recoil_yaw")), "reversed weapon range")
                    for key in ("penetration", "projectile_speed", "explosion_radius", "fuse_seconds"):
                        number(m[key])
                    for key in ("recoil_recovery", "empty_reload", "shell_seconds", "equip_seconds", "unequip_seconds", "projectile_lifetime", "ads_fov"):
                        number(m[key], True)
                    require(1 <= m["ads_fov"] <= 179, "invalid ADS FOV")
                    require(type(m["shell_reload"]) is bool and type(m["dual_wield"]) is bool, "invalid weapon flags")
                    require(type(m["ads_enabled"]) is bool, "invalid ADS flag")
                    vector(m["ads_translation"])
                    require(isinstance(m["ads_rotation"], list) and len(m["ads_rotation"]) == 4 and all(type(x) in (int, float) and math.isfinite(x) for x in m["ads_rotation"]), "invalid ADS rotation")
                    require(abs(sum(x*x for x in m["ads_rotation"]) - 1) < 0.002, "unnormalized ADS rotation")
                    number(m["burst_delay"])
            elif table == "perks":
                if "fall_protection" in row: require(type(row["fall_protection"]) is bool, "invalid fall protection flag")
                if row.get("dive") is not None:
                    d = row["dive"]
                    require(isinstance(d,dict) and set(d)=={"radius","damage","minimum_drop"},"invalid dive fields")
                    number(d["radius"],positive=True);number(d["minimum_drop"],positive=True)
                    require(isinstance(d["damage"],list) and len(d["damage"])==2,"invalid dive damage")
                    for amount in d["damage"]: natural(amount,positive=True)
                    require(d["damage"][0]<=d["damage"][1]<=2147483647,"unordered dive damage")
                if "explosive_protection" in row: require(type(row["explosive_protection"]) is bool, "invalid explosive protection flag")
                if row.get("cherry") is not None:
                    c = row["cherry"]
                    require(isinstance(c, dict) and set(c) == {"radius", "damage", "cooldown_extra", "stun_seconds"}, "invalid cherry fields")
                    require(isinstance(c["radius"],list) and len(c["radius"])==2, "invalid cherry radii")
                    for r in c["radius"]: number(r, positive=True)
                    require(c["radius"][0] < c["radius"][1], "unordered cherry radii")
                    require(isinstance(c["damage"],list) and len(c["damage"])==2, "invalid cherry damage")
                    for d in c["damage"]: natural(d, positive=True)
                    require(c["damage"][0]<=c["damage"][1]<=2147483647, "unordered cherry damage")
                    number(c["cooldown_extra"],positive=True);number(c["stun_seconds"],positive=True)
                if row.get("firing") is not None:
                    firing = row["firing"]
                    require(isinstance(firing, dict) and set(firing) == {"interval_scale", "bullet_copies", "hip_spread_scale"}, "invalid perk firing fields")
                    for key in ("interval_scale", "hip_spread_scale"):
                        number(firing[key], positive=True)
                        require(firing[key] <= 1, "invalid perk firing scale")
                    natural(firing["bullet_copies"], positive=True)
                    require(firing["bullet_copies"] <= 2, "invalid perk bullet copies")
                if "third_weapon" in row: require(type(row["third_weapon"]) is bool, "invalid perk third-weapon flag")
                if row.get("sprint") is not None:
                    sprint = row["sprint"]
                    require(isinstance(sprint, dict) and set(sprint) == {"duration_scale", "speed_scale"}, "invalid perk sprint fields")
                    for value in sprint.values():
                        number(value, positive=True)
                        require(1 <= value <= 4, "invalid perk sprint scale")
                if row.get("reload_duration_scale") is not None:
                    number(row["reload_duration_scale"], positive=True)
                    require(row["reload_duration_scale"] <= 1, "perk reload scale exceeds one")
                if row.get("maximum_health") is not None:
                    natural(row["maximum_health"], positive=True)
                    require(row["maximum_health"] <= 2147483647, "perk maximum exceeds native integer range")
                resolve(owner, row["icon"], "ui", True)
                resolve(owner, row.get("presentation"), "presentation", True)
                drink = row.get("drink")
                if drink is not None:
                    require(isinstance(drink, dict) and set(drink) == {"bottle", "seconds", "lower_seconds", "activation_delay"}, "invalid perk drinking fields")
                    resolve(owner, drink["bottle"], "weapon")
                    for key in ("seconds", "lower_seconds"): number(drink[key], positive=True)
                    number(drink["activation_delay"])
                    require(drink["activation_delay"] <= drink["lower_seconds"], "perk activation exceeds bottle action")
            elif table == "powerups":
                number(row["duration"])
            elif table == "placements":
                resolve(owner, row["definition"])
                require(type(row["yaw"]) in (int, float) and math.isfinite(row["yaw"]), "invalid placement yaw")
            elif table == "interactions":
                resolve(owner, row["placement"], "placement")
                action = row["action"]
                require(isinstance(action, dict), "invalid interaction action")
                schemas = {"barrier": ("barrier", "barrier", {"kind", "barrier"}),
                           "door": ("connection", "connection", {"kind", "connection", "price"}),
                           "weapon": ("weapon", "weapon", {"kind", "weapon", "price"}),
                           "perk": ("perk", "perk", {"kind", "perk"}),
                           "generator": ("generator", "generator", {"kind", "generator"}),
                           "quest": ("quest", "quest", {"kind", "quest", "event"}),
                           "pickup": ("item", "any", {"kind", "item"})}
                require(action.get("kind") in schemas, "unknown interaction kind")
                key, kind, fields = schemas[action["kind"]]
                extras = {"ammo_price"} if action["kind"] == "weapon" else set()
                require(fields <= set(action) <= fields | extras, "invalid interaction payload")
                if action.get("ammo_price") is not None:
                    natural(action["ammo_price"])
                resolve(owner, action[key], kind)
                if "price" in action:
                    natural(action["price"])
                if "event" in action:
                    require(isinstance(action["event"], str) and action["event"], "invalid interaction event")
            elif table == "zone_connections":
                resolve(owner, row["from"], "zone")
                resolve(owner, row["to"], "zone")
            elif table == "barriers":
                resolve(owner,row['placement'],'placement')
                for key in ('entry','exit'): resolve(owner,row[key],'node')
                for key in ('repair_sound','break_sound'): resolve(owner,row[key],'sound')
                require(len(row['boards']) == 6, 'invalid barrier board count')
                for board in row['boards']:
                    resolve(owner,board['presentation'],'presentation')
                    for key in ('repair','pull'): resolve(owner,board[key],'animation')
                    for key in ('repair_seconds','pull_seconds','grab_seconds','tear_seconds'): number(board[key],positive=True)
                    number(board['destroy_seconds'])
                    require(board['destroy_seconds'] < board['tear_seconds'], 'invalid board destruction notetrack')
                    for key in ('grab_clip','tear_clip'): require(isinstance(board[key],str) and board[key], 'invalid board tear alias')
            elif table == "navigation_links":
                resolve(owner, row["from"], "node")
                resolve(owner, row["to"], "node")
                resolve(owner, row["connection"], "connection", True)
                require(row["traversal"] in ("walk", "climb", "jump", "rise", "portal"), "invalid traversal")
                identities(owner,row.get("blockers",[]),"connection")
                require(row.get("source_animation") is None or isinstance(row["source_animation"],str) and 0<len(row["source_animation"])<128,"invalid source traversal animation")
            elif table == "generators":
                resolve(owner, row["placement"], "placement")
                identities(owner, row["zones"], "zone")
                resolve(owner, row['presentation'], 'presentation')
                for key in ('capture_sound','active_sound'): resolve(owner,row[key],'sound')
                require(isinstance(row['number'],int) and 1 <= row['number'] <= 6,'invalid generator number')
                vector(row['model_position'])
                require(len(row['emergence']) == 4,'invalid generator emergence closure')
                for p in row['emergence']: vector(p)
                require(len(row['clip_seconds']) == 8,'incomplete generator animation clocks')
                for value in row['clip_seconds']: number(value,True)
                for key in ('decay_seconds','shutdown_seconds','recovery_seconds'): number(row[key],True)
            elif table in ("robot_routes", "tank_routes"):
                require(isinstance(row["points"], list) and len(row["points"]) >= 2, "incomplete route")
                for point in row["points"]:
                    vector(point)
            elif table in ("weather", "sounds"):
                values = row["duration" if table == "weather" else "pitch"]
                require(isinstance(values, list) and len(values) == 2, "invalid content range")
                number(values[0], True)
                number(values[1], True)
                require(values[1] >= values[0], "reversed content range")
                if table == "weather":
                    resolve(owner, row["sound"], "sound", True)
                else:
                    path(row["path"])
            elif table == "digging":
                resolve(owner, row["placement"], "placement")
                identities(owner, row["loot"])
                require(row["loot"], "empty dig table")
            elif table == "recipes":
                resolve(owner, row["bench"], "placement")
                resolve(owner, row["result"])
                identities(owner, row["parts"])
                require(row["parts"], "empty recipe")
            elif table == "staff_puzzles":
                resolve(owner, row["weapon"], "weapon")
                identities(owner, row["interactions"], "interaction")
                require(row["interactions"] and isinstance(row["solution"], list) and row["solution"] and all(isinstance(x, str) for x in row["solution"]), "empty puzzle")
            elif table == "quest_transitions":
                resolve(owner, row["quest"], "quest")
                identities(owner, row["prerequisites"])
                require(all(isinstance(row[k], str) and row[k] for k in ("from", "to", "event")), "invalid quest transition")
            elif table == "presentations":
                if row["scene"] is not None:
                    path(row["scene"])
                identities(owner, row["animations"], "animation")
                identities(owner, row["sounds"], "sound")
            elif table == "animations":
                path(row["path"])
                resolve(owner, row["skeleton"], "presentation")
                require(isinstance(row["notifications"], list), "invalid animation notifications")
                previous = 0
                for event in row["notifications"]:
                    require(isinstance(event, dict) and set(event) == {"seconds", "event"}, "invalid animation notification")
                    number(event["seconds"])
                    require(event["seconds"] >= previous and isinstance(event["event"], str) and event["event"], "invalid notification order/event")
                    previous = event["seconds"]
            elif table == "localized_strings":
                require(isinstance(row["key"], str) and row["key"] and isinstance(row["text"], str), "invalid localized string")
            elif table == "ui_assets":
                path(row["path"])
            elif table == "cinematics":
                require(isinstance(row["frames"], list) and row["frames"], "empty cinematic")
                for frame in row["frames"]:
                    path(frame)
                resolve(owner, row["audio"], "sound", True)
    return dependencies


def validate_rounds(profile, require):
    integers = {"health_start","health_increase","exponential_round","population_base","population_round_divisor",
        "population_acceleration_round","ordinary_cap","actor_cap","speed_multiplier","speed_random_band"}
    scalars = {"health_multiplier","solo_population_increment","population_acceleration","spawn_seconds",
        "spawn_decay","minimum_spawn_seconds","first_spawn_seconds","between_round_seconds"}
    vectors = {"early_population":5,"gait_thresholds":2,"gait_speeds":3}
    require(isinstance(profile,dict) and set(profile) == integers | scalars | set(vectors),"incomplete round profile")
    for key in integers:
        require(type(profile[key]) is int and 0 < profile[key] <= 2147483647,"invalid round integer")
    for key in scalars:
        value = profile[key]
        require(type(value) in (int,float) and math.isfinite(value) and value > 0,"invalid round scalar")
    for key,size in vectors.items():
        value = profile[key]
        require(isinstance(value,list) and len(value)==size and all(type(v) in (int,float) and math.isfinite(v) and v>0 for v in value),"invalid round vector")
    require(all(type(v) is int and v<=2147483647 for v in profile["gait_thresholds"]),"invalid gait thresholds")
    require(profile["exponential_round"]>=2 and profile["actor_cap"]>=profile["ordinary_cap"]
        and profile["spawn_decay"]<=1 and profile["minimum_spawn_seconds"]<=profile["spawn_seconds"]
        and all(v<=1 for v in profile["early_population"])
        and profile["gait_thresholds"][0]<profile["gait_thresholds"][1]
        and all(a<b for a,b in zip(profile["gait_speeds"],profile["gait_speeds"][1:])),"invalid round profile ordering")


def validate_player_health(profile, require):
    scalars = {"regular_delay","regular_step_fraction","critical_delay","critical_ratio","critical_step_seconds","critical_step_fraction",
        "worthy_damage_ratio","immunity_regular","immunity_critical_enter","immunity_critical_repeat","feedback_seconds"}
    require(isinstance(profile,dict) and set(profile)==scalars | {"maximum"},"incomplete player health profile")
    require(type(profile["maximum"]) is int and 0<profile["maximum"]<=2147483647,"invalid player maximum health")
    for key in scalars:
        require(type(profile[key]) in (int,float) and math.isfinite(profile[key]) and profile[key]>0,"invalid player health scalar")
    require(profile["critical_delay"]>=profile["regular_delay"] and all(profile[k]<=1 for k in
        ["regular_step_fraction","critical_ratio","critical_step_fraction","worthy_damage_ratio"]),"invalid player health thresholds")
