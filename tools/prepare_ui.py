# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///
import argparse
import hashlib
import json
from pathlib import Path
import struct

import numpy as np
from PIL import Image, ImageDraw

from ktx2_textures import MAGIC, inspect_ktx2, decoded_levels
from material_textures import color_levels


PERKS = {
    "quick_revive": "quickrevive", "juggernog": "juggernaut",
    "speed_cola": "fastreload", "stamin_up": "marathon", "mule_kick": "mulekick",
    "double_tap": "doubletap", "deadshot": "ads", "electric_cherry": "cherry",
    "phd_flopper": "divetonuke",
}


def encode(levels):
    dfd = struct.pack("<IIHH4B4B8B", 92, 0, 2, 88, 1, 1, 2, 0, 0, 0, 0, 0,
                      4, 0, 0, 0, 0, 0, 0, 0)
    for index, channel in enumerate((0, 1, 2, 15)):
        dfd += struct.pack("<HBB4BII", index * 8, 7, channel, 0, 0, 0, 0, 0, 255)
    offset = 80 + 24 * len(levels)
    header = MAGIC + struct.pack("<9I4I2Q", 43, 1, levels[0].shape[1], levels[0].shape[0],
        0, 0, 1, len(levels), 0, offset, len(dfd), 0, 0, 0, 0)
    cursor = offset + len(dfd)
    indices, payload = b"", b""
    for pixels in levels:
        part = np.ascontiguousarray(pixels).tobytes()
        indices += struct.pack("<3Q", cursor, len(part), len(part))
        payload += part
        cursor += len(part)
    raw = header + indices + dfd + payload
    inspect_ktx2(raw)
    if not np.array_equal(decoded_levels(raw)[0], levels[0]):
        raise ValueError("UI container changed source pixels")
    return raw


def prepare(extraction, recovery, pack, output, gameplay):
    if output.resolve().is_relative_to(Path(__file__).resolve().parent.parent):
        raise ValueError("Preparation receipts must be external")
    output.mkdir(parents=True, exist_ok=True)
    selected = {}
    zones = ["zone/english/en_code_post_gfx_zm", "zone/all/common_zm", "zone/all/ui_zm", "zone/all/patch_zm",
             "zone/all/dlc4_load_zm", "zone/all/zm_tomb", "zone/all/zm_tomb_patch"]
    for zone in zones:
        for path in sorted((extraction / zone / "images").glob("*.dds")):
            name = path.stem
            if name.startswith(("hud_", "zom_hud_", "zm_hud_", "chalkmarks_", "tomb_medal_",
                                "zm_capture_", "tomb_capture_", "tomb_spinner_", "scorebar_zom_", "reticle_", "specialty_")) or name in (
                    "waypoint_arrow_tomb", "menu_zm_cac_grad_stretch",
                    "gamefonts_pc_720", "scorebar_zom_5", "menu_zm_map_tomb",
                    "menu_zm_map_tomb_blur", "loadscreen_zombie_le_tombeau", "hit_direction", "damage_feedback",
                    "damage_feedback_glass", "overlay_low_health", "directional_damage_feedback",
                    "blood_overlay", "vignette"):
                selected[name] = path
    if "specialty_cherry_zombies" not in selected:
        cherry = next(extraction.glob("zone/all/zm_tomb/images/minimap_icon_electric_cherry.dds"), None)
        if cherry:
            selected["specialty_cherry_zombies"] = cherry
    material_receipt = []
    for name in ("reticle_side_small", "reticle_center_cross", "reticle_m203", "scorebar_zom_1"):
        for zone in zones:
            material = extraction / zone / "materials" / (name + ".json")
            if not material.exists():
                continue
            data = json.loads(material.read_bytes())
            color = next(texture["image"] for texture in data["textures"] if texture["name"] == "colorMap")
            image = extraction / zone / "images" / (color + ".dds")
            if image.exists():
                selected[name] = image
                material_receipt.append({"id": "ui." + name, "material": str(material),
                    "material_sha256": hashlib.sha256(material.read_bytes()).hexdigest()})
    rows, receipt, previews = [], material_receipt, []
    for name, path in sorted(selected.items()):
        source = path.read_bytes()
        if not source.startswith(b"DDS "):
            raise ValueError("Original UI image is not hydrated: " + str(path))
        if struct.unpack_from("<I", source, 80)[0] == 2:
            height, width = struct.unpack_from("<2I", source, 12)
            pixels = np.full((height, width, 4), 255, np.uint8)
            pixels[..., 3] = np.frombuffer(source, np.uint8, width * height, 128).reshape(height, width)
        else:
            with Image.open(path) as image:
                pixels = np.array(image.convert("RGBA"))
        levels = color_levels(pixels, True)
        raw = encode(levels)
        relative = "ui/" + name + ".ktx2"
        destination = pack / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if not destination.exists() or destination.read_bytes() != raw:
            destination.write_bytes(raw)
        rows.append({"id": "ui." + name, "path": relative})
        receipt.append({"id": rows[-1]["id"], "source": str(path),
                        "source_sha256": hashlib.sha256(source).hexdigest(),
                        "runtime_sha256": hashlib.sha256(raw).hexdigest(),
                        "width": pixels.shape[1], "height": pixels.shape[0]})
        previews.append((name, Image.fromarray(pixels)))
    atlas = next(row for row in rows if row["id"] == "ui.gamefonts_pc_720")
    atlas_image = next(image for name, image in previews if name == "gamefonts_pc_720")
    for style in ("normal", "small", "bold", "big", "extraSmall", "italic", "extraBig", "smallItalic"):
        path = recovery / "en_code_post_gfx_zm/fonts/720" / (style + "Font.json")
        font = json.loads(path.read_bytes())
        receipt.append({"id": "ui.font." + style, "source": str(path),
                        "source_sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        width, height = atlas_image.size
        rows.append({"id": "ui.font." + style, "path": atlas["path"], "font": {
            "height": font["pixelHeight"], "atlas": [width, height],
            "glyphs": [{"codepoint": ord(g["letter"]) if isinstance(g["letter"], str) else g["letter"], "offset": [g["x0"], g["y0"]],
                "advance": g["dx"], "size": [g["pixelWidth"], g["pixelHeight"]],
                "rect": [g["s0"] * width, g["t0"] * height, g["s1"] * width, g["t1"] * height]}
                for g in font["glyphs"]]}})
    required = ["hud_tomb_zom_num_" + str(i) for i in range(10)]
    required += ["chalkmarks_hellcatraz_" + str(i) for i in range(1, 6)]
    required += ["specialty_" + name + "_zombies" for name in PERKS.values()]
    missing = [name for name in required if name not in selected]
    if missing:
        raise ValueError("Missing native UI images: " + ", ".join(missing))
    game_path = pack / "game.json"
    game = json.loads(game_path.read_bytes())
    source = json.loads(gameplay.read_bytes())
    weapons = {weapon["source"]: weapon for weapon in source["weapons"]}
    hud_receipt = []
    for weapon in game["content"]["weapons"]:
        native = weapons.get(weapon["id"].removeprefix("weapon."))
        if native is None:
            continue
        fields = native["fields"]
        def image_id(field):
            name = fields.get(field, "")
            if not name:
                return None
            if name not in selected:
                raise ValueError("Missing native weapon reticle: " + name)
            return "ui." + name
        weapon["hud"] = {"center": image_id("reticleCenter"), "side": image_id("reticleSide"),
                         "center_size": float(fields.get("reticleCenterSize", 0)),
                         "side_size": float(fields.get("reticleSideSize", 0)),
                         "side_position": float(fields.get("hipReticleSidePos", 0)),
                         "minimum_offset": float(fields.get("reticleMinOfs", 0)),
                         "keep_when_ads": fields.get("keepCrosshairWhenADS", "0") == "1",
                         "low_ammo_fraction": float(fields.get("lowAmmoWarningThreshold", 0.25))}
        hud_receipt.append({"id":weapon["id"],"native_sha256":native["sha256"],"hud":weapon["hud"]})
    identities = {row["id"] for row in rows}
    game["content"]["ui_assets"] = [row for row in game["content"]["ui_assets"]
                                     if row["id"] not in identities] + rows
    for perk in game["content"]["perks"]:
        name = PERKS.get(perk["id"].removeprefix("perk."))
        if name:
            perk["icon"] = "ui.specialty_" + name + "_zombies"
    game_path.write_text(json.dumps(game, separators=(",", ":")) + "\n", encoding="utf-8")
    sheet = Image.new("RGB", (960, ((len(previews) + 5) // 6) * 120), (34, 34, 34))
    draw = ImageDraw.Draw(sheet)
    for index, (name, image) in enumerate(previews):
        image.thumbnail((144, 86))
        x, y = index % 6 * 160, index // 6 * 120
        sheet.paste(image, (x + (160 - image.width) // 2, y), image)
        draw.text((x + 3, y + 87), name.replace("zom_hud_craftable_", "").replace("specialty_", "")[:23], fill="white")
    sheet.save(output / "original-ui.png")
    (output / "preparation.json").write_text(json.dumps(receipt, indent=2) + "\n")
    (output / "weapons.json").write_text(json.dumps(hud_receipt, indent=2) + "\n")
    print(f"Prepared {len(selected)} original UI images and eight original bitmap fonts")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--extraction", required=True, type=Path)
    parser.add_argument("--recovery", required=True, type=Path)
    parser.add_argument("--pack", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--gameplay", required=True, type=Path)
    args = parser.parse_args()
    prepare(args.extraction, args.recovery, args.pack, args.output, args.gameplay)
