"""Restore the Origins FX dependency closure and original CreateFX placements."""
import argparse
import json
import re
import shutil
from pathlib import Path


def export(research: Path, checkout: Path):
    path = checkout / "gameplay/origins/fx_catalog.json"
    catalog = json.loads(path.read_text())
    sources = {}
    for source in sorted((research / "rust-pack-20261012/native/assets/fx").glob("*.json")):
        record = json.loads(source.read_text())
        if not record["reference"]:
            sources[record["name"].lstrip(",")] = (source, record["data"])
    scripts = research / "source/client-tomb/clientscripts/mp"
    names = dict(re.findall(
        r'level\._effect\["([^"]+)"\] = loadfx\( "([^"]+)" \)',
        (scripts / "zm_tomb_fx.csc").read_text(),
    ))
    createfx = (scripts / "createfx/zm_tomb_fx.csc").read_text()
    blocks = re.split(r'ent = clientscripts\\mp\\_fx::create(oneshoteffect|exploder)\( "([^"]+)" \);', createfx)
    placements = []
    for kind, key, body in zip(blocks[1::3], blocks[2::3], blocks[3::3]):
        def field(name):
            return re.search(r'ent.v\["' + name + r'"\] = ([^;]+);', body)[1]

        def vector(value):
            match = re.fullmatch(r'\( ([^()]+) \)', value)
            if match:
                return [float(v) for v in match[1].split(",")]
            match = re.fullmatch(r'vectorscale\( \( ([^()]+) \), ([^()]+) \)', value)
            assert match, value
            return [float(v) * float(match[2]) for v in match[1].split(",")]

        placements.append({
            "name": names[key], "origin": vector(field("origin")),
            "angles": vector(field("angles")), "start_time": float(field("delay")),
            "exploder": int(field("exploder")) if kind == "exploder" else None,
        })
    assert len(placements) == 585, "Original Origins CreateFX placement count changed"
    required = set(catalog["effects"]) | {p["name"] for p in placements}
    for script in scripts.rglob("*.csc"):
        # Precache-only reference stubs need no emitter. Require every actual
        # placement and runtime request below, including their child effects.
        required.update(n for n in re.findall(r'loadfx\(\s*"([^"]+)"', script.read_text()) if n in sources)
    missing = []
    for source in (checkout / "src").rglob("*.rs"):
        text = source.read_text()
        for name in re.findall(r'"((?:maps|weapon|misc|impacts|vehicle|explosions|env|system)/[^"\n]+)"', text):
            if name in sources:
                required.add(name)
            elif "{" not in name and not name.endswith("_") and "/fx_" in name:
                missing.append((str(source.relative_to(checkout)), name))
    assert not missing, f"Runtime FX definitions missing: {missing}"
    added = []
    pending = list(required)
    visited = set()
    while pending:
        name = pending.pop().lstrip(",")
        if name in visited:
            continue
        visited.add(name)
        source, effect = sources[name]
        if name not in catalog["effects"]:
            catalog["effects"][name] = effect
            catalog["source_files"][name] = source.name
            added.append(name)
        for element in catalog["effects"][name]["elements"]:
            pending.extend(element[k] for k in ("effect_attached", "effect_emitted", "effect_on_death", "effect_on_impact") if element[k])
            for visual in element["visuals"]:
                if not visual:
                    continue
                if visual.get("effect"):
                    pending.append(visual["effect"])
                for material in ([visual["material"]] if visual.get("material") else visual.get("decal_materials") or []):
                    material = material.lstrip(",")
                    destination = checkout / "target/assets/materials" / (material.replace("*", "_") + ".json")
                    if not destination.is_file():
                        definitions = [research / "evidence/rendering-20261012/source/all" / zone / "materials" / (material + ".json")
                                       for zone in ("code_post_gfx_zm", "common_zm", "patch_zm", "zm_tomb", "zm_tomb_patch")]
                        source_material = next((p for p in reversed(definitions) if p.is_file()), None)
                        assert source_material, material
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(source_material, destination)
                    catalog["materials"].setdefault(material, {})
    catalog["map_effects"] = placements
    catalog["effects"] = dict(sorted(catalog["effects"].items()))
    catalog["materials"] = dict(sorted(catalog["materials"].items()))
    path.write_text(json.dumps(catalog, separators=(",", ":"), allow_nan=False) + "\n")
    print(json.dumps({"effects": len(catalog["effects"]), "materials": len(catalog["materials"]),
                      "ambient_placements": sum(p["exploder"] is None for p in placements),
                      "exploders": sum(p["exploder"] is not None for p in placements), "added": sorted(added)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--research", type=Path, required=True)
    args = parser.parse_args()
    export(args.research, Path(__file__).resolve().parent.parent)
