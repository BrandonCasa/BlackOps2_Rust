"""Prepare source-defined standing-player recovery and immunity settings."""
import hashlib
import json
import re
from prepare_gameplay_content import external

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    source = inspection / "damage-life-state-03/decompiled/t6/_zm_playerhealth.gsc"
    text = source.read_text()
    def variable(name):
        match = re.search(r'level\.'+name+r' = ([\d.]+);',text)
        if not match: raise ValueError("missing native player-health variable " + name)
        return float(match[1])
    for fragment in ["regenrate = 0.1", "wait 0.05", "newhealth = 1", "hurttime = gettime()"]:
        if fragment not in text: raise ValueError("native player-health algorithm changed")
    profile = {"maximum":100, "regular_delay":variable("playerhealth_regularregendelay") / 1000,
        "critical_delay":variable("longregentime") / 1000,"critical_ratio":variable("healthoverlaycutoff"),
        "critical_step_seconds":0.05,"critical_step_fraction":0.1,
        "worthy_damage_ratio":variable("worthydamageratio"),"immunity_regular":variable("invultime_preshield"),
        "immunity_critical_enter":variable("invultime_onshield"),"immunity_critical_repeat":variable("invultime_postshield"),
        "feedback_seconds":0.8}
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    game["player_health"] = profile
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    report = {"profile":profile,"source_sha256":hashlib.sha256(source.read_bytes()).hexdigest(),
        "scope":"standing solo player; perks and self-revival remain pending",
        "approximations":["Recovery uses a 60 Hz fixed-step clock rather than native script/frame-end scheduling.",
            "Peripheral red rectangles reconstruct damage feedback; native blur and overlay texture remain pending.",
            "Base health 100 retains the qualified solo baseline; perk maxima require separate integration."]}
    (inspection / "solo-content/player-health-preparation.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
    print(json.dumps(report))
