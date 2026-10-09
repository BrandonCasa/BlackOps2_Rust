import csv
import hashlib
import json
import re
from pathlib import Path
import numpy as np
from prepare_gameplay_content import external
from xanim import decode


def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    retained = inspection / "mechanics-rounds-02"
    table_path = retained / "table-dump/mp/zombiemode.csv"
    script_path = retained / "decompile/decompiled/t6/_zm.gsc"
    utility_path = retained / "decompile/decompiled/t6/_zm_utility.gsc"
    rows = list(csv.reader(table_path.read_text().splitlines()))
    column = rows[0].index("MEDIUM")
    table = {r[0]: float(r[column] or r[1]) for r in rows[1:] if r and not r[0].startswith("//")}
    script = script_path.read_text()
    utility = utility_path.read_text()
    def variable(name):
        if name in table:
            return table[name]
        match = re.search(r'set_zombie_var\( "' + name + r'", ([\d.]+)', script)
        if not match:
            raise ValueError("missing native round variable " + name)
        return float(match[1])
    for text, fragment in [(script, "if ( i >= 10 )"), (script, "timer * 0.95"),
            (script, "0.5 * level.zombie_vars"), (utility, "level.zombie_move_speed + 35"),
            (utility, "rand <= 35"), (utility, "rand <= 70")]:
        if fragment not in text:
            raise ValueError("native round algorithm changed: " + fragment)
    speeds = []
    root = inspection / "solo-content/source/xanim"
    for clip in ["ai_zombie_walk_v1", "ai_zombie_run_v2", "ai_zombie_sprint_v1"]:
        animation = decode((root / clip).read_bytes())
        if animation["delta"] is None:
            raise ValueError("locomotion clip has no native root delta")
        delta = animation["delta"]["translation"][1]
        speeds.append(float(np.linalg.norm((delta[-1] - delta[0])[:2]) * 0.0254 / animation["duration"]))
    profile = {
        "health_start":int(variable("zombie_health_start")),
        "health_increase":int(variable("zombie_health_increase")),
        "health_multiplier":variable("zombie_health_increase_multiplier"),
        "exponential_round":10, "population_base":int(variable("zombie_max_ai")),
        "solo_population_increment":0.5 * variable("zombie_ai_per_player"),
        "population_round_divisor":5,"population_acceleration_round":10,"population_acceleration":0.15,
        "early_population":[0.25,0.3,0.5,0.7,0.9],"ordinary_cap":24,"actor_cap":31,
        "spawn_seconds":variable("zombie_spawn_delay"),"spawn_decay":0.95,"minimum_spawn_seconds":0.08,
        "first_spawn_seconds":2.,"between_round_seconds":variable("zombie_between_round_time"),
        "speed_multiplier":int(variable("zombie_move_speed_multiplier")),"speed_random_band":35,
        "gait_thresholds":[35,70],"gait_speeds":speeds,
    }
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    game["rounds"] = profile
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    report = {"profile":profile,"sources":{p.name:hashlib.sha256(p.read_bytes()).hexdigest()
        for p in [table_path,script_path,utility_path]},"scope":"solo Original ordinary rounds",
        "approximations":["Collider speeds use one recovered locomotion clip per gait rather than blending every native variant.",
        "Fractional script division supplies population multiplier, inferred from retained Divide opcode and reconstructed earlier-engine VM semantics; map callbacks and Easy profile remain pending."]}
    (inspection / "solo-content/round-preparation.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
    print(json.dumps(report))
