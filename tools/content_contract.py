"""Version-two stable IDs and typed content dependency validation.

Used by preparation, import, and packaging; never enumerates optional raw inputs.
"""
import math
import re

TABLES = {
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
    "generators": ("generator", "id placement zones price capture_seconds radius"),
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
            require(isinstance(row, dict) and set(row) == set(fields.split()), f"invalid {table} fields")
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
                    natural(row[key], key != "souls")
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
            if "zone" in row:
                resolve(owner, row["zone"], "zone")
            if table == "weapons":
                resolve(owner, row["upgraded"], "weapon", True)
                require(row["category"] in ("bullet", "melee", "explosive", "fire", "ice", "lightning", "wind", "environmental", "crush"), "invalid damage category")
            elif table == "perks":
                resolve(owner, row["icon"], "ui", True)
            elif table == "powerups":
                number(row["duration"])
            elif table == "placements":
                resolve(owner, row["definition"])
                require(type(row["yaw"]) in (int, float) and math.isfinite(row["yaw"]), "invalid placement yaw")
            elif table == "interactions":
                resolve(owner, row["placement"], "placement")
                action = row["action"]
                require(isinstance(action, dict), "invalid interaction action")
                schemas = {"door": ("connection", "connection", {"kind", "connection", "price"}),
                           "weapon": ("weapon", "weapon", {"kind", "weapon", "price"}),
                           "perk": ("perk", "perk", {"kind", "perk"}),
                           "generator": ("generator", "generator", {"kind", "generator"}),
                           "quest": ("quest", "quest", {"kind", "quest", "event"}),
                           "pickup": ("item", "any", {"kind", "item"})}
                require(action.get("kind") in schemas, "unknown interaction kind")
                key, kind, fields = schemas[action["kind"]]
                require(set(action) == fields, "invalid interaction payload")
                resolve(owner, action[key], kind)
                if "price" in action:
                    natural(action["price"])
                if "event" in action:
                    require(isinstance(action["event"], str) and action["event"], "invalid interaction event")
            elif table == "zone_connections":
                resolve(owner, row["from"], "zone")
                resolve(owner, row["to"], "zone")
            elif table == "navigation_links":
                resolve(owner, row["from"], "node")
                resolve(owner, row["to"], "node")
                resolve(owner, row["connection"], "connection", True)
                require(row["traversal"] in ("walk", "climb", "jump", "rise", "portal"), "invalid traversal")
            elif table == "generators":
                resolve(owner, row["placement"], "placement")
                identities(owner, row["zones"], "zone")
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
            elif table == "ui_assets":
                path(row["path"])
            elif table == "cinematics":
                require(isinstance(row["frames"], list) and row["frames"], "empty cinematic")
                for frame in row["frames"]:
                    path(frame)
                resolve(owner, row["audio"], "sound", True)
    return dependencies
