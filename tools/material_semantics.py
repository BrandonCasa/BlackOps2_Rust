import re

import numpy as np
from PIL import Image

LEGACY_DIELECTRIC_F0 = (56 / 255) ** 2

BINDINGS = {
    2695565377: ("color", 0), 3054311504: ("color", 1),
    3054311507: ("color", 2), 3054311506: ("color", 3),
    4030327853: ("color", 0), 1825069840: ("emissive", 0),
    1507003663: ("normal", 0), 2486480606: ("normal", 1),
    2486480605: ("normal", 2), 2485960688: ("normal", 0),
    887934131: ("specular_gloss", 0), 3532022562: ("specular_gloss", 1),
    3532022561: ("specular_gloss", 2), 2351529600: ("specular_gloss", 0),
    3714339118: ("color", 0), 3714374925: ("color", 1), 3714406764: ("color", 2),
}
CHANNELS = {"color": "rgb:base_color,a:coverage", "normal": "rg:unit_normal_xy",
            "response": "r:occlusion,g:perceptual_roughness,b:metallic,a:coating",
            "coating": "r:coating", "control": "rg:core_coating_weights",
            "reflectance": "rgb:dielectric_reflectance,a:unused",
            "emissive": "rgb:emissive,a:unused"}


def uv_selection(surface, layer, role):
    if surface.get("family") == "tile_blend":
        return "primary" if role == "control" else "primary_times_tiles"
    if "effect" in surface:
        return f"primary_times_effect_layer_{layer}_scale"
    if "layers" in surface and layer:
        return f"secondary_layer_{layer}"
    return "primary"


def resolve_bindings(original, surface, identity):
    if not original:
        raise ValueError(f"{identity}: native definition unavailable; recover its material definition first")
    family = original["family"].lstrip(",")
    if not (re.search(r"(?:^|_)lit_sm_", family) or any(t in family for t in
            ("char_cloth_", "char_skin_", "phong_emissive_alcatraz_", "cod7water_", "tile_blend", "unlit_"))):
        raise ValueError(f"{identity}: unsupported shader family {family}; add a reviewed binding recipe")
    result = {}
    for binding in original.get("textures", []):
        key = BINDINGS.get(binding["hash"])
        expected = {"color": 2, "emissive": 2, "normal": 5, "specular_gloss": 8}
        if key is None or binding["semantic"] != expected[key[0]]:
            raise ValueError(f"{identity}: unsupported/contradictory binding hash={binding['hash']} "
                             f"semantic={binding['semantic']} in {family}; resolve by native shader slot")
        if key in result:
            raise ValueError(f"{identity}: duplicate {key} native binding; refusing last-binding-wins")
        sampler = binding.get("sampler")
        if type(sampler) is not int or not 0 <= sampler <= 255:
            raise ValueError(f"{identity}: invalid native sampler for {key}")
        result[key] = dict(binding)
    layers = sorted(layer for role, layer in result if role == "color")
    for layer in layers:
        color_sampler = result["color", layer]["sampler"]
        for role in ("normal", "specular_gloss"):
            binding = result.get((role, layer))
            if binding and (binding["sampler"] & 96) != (color_sampler & 96):
                raise ValueError(f"{identity}: layer {layer} {role}/color addressing differs; "
                                 "provide a reviewed cross-sampler conversion before packing")
        alpha_gloss = bool(re.search(rf"n{layer}x{layer}(?:_|$)", family))
        if alpha_gloss and ("specular_gloss", layer) in result:
            raise ValueError(f"{identity}: layer {layer} declares both color-alpha and specular-alpha gloss")
        if alpha_gloss and layer == 0 and surface.get("presentation", {}).get("alpha") != "opaque":
            raise ValueError(f"{identity}: color alpha cannot be both base coverage and gloss")
    return result


def meanings(family, surface, layer, has_specular):
    alpha_gloss = bool(re.search(rf"n{layer}x{layer}(?:_|$)", family)) and not has_specular
    return dict(color_alpha="gloss" if alpha_gloss else "coverage" if layer == 0 and
                surface.get("presentation", {}).get("alpha") != "opaque" else "reveal" if layer else "unused",
                specular_rgb="authored_legacy_reflectance", specular_alpha="gloss",
                normal_rg="native_signed_slopes_directx_y", color_space="native_gamma2_to_srgb_rgb_linear_alpha",
                data_space="linear", uv=uv_selection(surface, layer, "color"))


def legacy_color(pixels):
    result = pixels.copy()
    linear = (pixels[..., :3].astype(np.float32) / 255) ** 2
    result[..., :3] = np.rint(np.where(linear <= .0031308, linear * 12.92,
        1.055 * linear ** (1 / 2.4) - .055) * 255).astype(np.uint8)
    return result


def specular_response(color, specular, allow_metal, authored=None):
    h, w = color.shape[:2]
    encoded = np.asarray(Image.fromarray(specular).resize((w, h), Image.Resampling.BILINEAR), np.float32) / 255
    f0 = encoded[..., :3] ** 2
    rgb = color[..., :3].astype(np.float32) / 255
    diffuse = np.where(rgb <= .04045, rgb / 12.92, ((rgb + .055) / 1.055) ** 2.4)
    remaining = 1 - f0.max(axis=2)
    metallic = np.zeros((h, w), np.float32)
    if allow_metal:
        brightness = np.sqrt(np.sum(f0 ** 2 * [.299, .587, .114], axis=2))
        b = np.sqrt(np.sum(diffuse ** 2 * [.299, .587, .114], axis=2)) * remaining / (1-LEGACY_DIELECTRIC_F0) + brightness - 2*LEGACY_DIELECTRIC_F0
        c = LEGACY_DIELECTRIC_F0 - brightness
        metallic = np.where(brightness <= LEGACY_DIELECTRIC_F0, 0, np.clip((-b + np.sqrt(np.maximum(b*b-4*LEGACY_DIELECTRIC_F0*c, 0))) / (2*LEGACY_DIELECTRIC_F0), 0, 1))
    authored = authored or {}
    if "metallic" in authored:
        metallic.fill(authored["metallic"])
    for region in authored.get("regions", []):
        if "metallic" in region:
            u0, v0, u1, v1 = region["uv_rect"]
            metallic[round(v0*h):round(v1*h), round(u0*w):round(u1*w)] = region["metallic"]
    dielectric = np.where((metallic > 1e-6)[..., None], LEGACY_DIELECTRIC_F0, np.minimum(f0, .16))
    if allow_metal or np.any(metallic):
        from_diffuse = diffuse * (remaining / np.maximum(1-dielectric.max(axis=2), 1e-6) / np.maximum(1-metallic, 1e-6))[..., None]
        from_specular = (f0 - dielectric * (1-metallic)[..., None]) / np.maximum(metallic, 1e-6)[..., None]
        weight = metallic[..., None] ** 2
        linear = np.clip(from_diffuse * (1-weight) + from_specular * weight, 0, 1)
        color = color.copy()
        color[..., :3] = np.rint(np.where(linear <= .0031308, linear * 12.92,
            1.055 * linear ** (1/2.4) - .055) * 255).astype(np.uint8)
    reflectance = np.full((h, w, 4), 255, np.uint8)
    reflectance[..., :3] = np.rint(np.sqrt(dielectric / .16) * 255).astype(np.uint8)
    return color, metallic, reflectance


def physical_response(color, gloss, normal, recipe, authored=None):
    template = color if color is not None else normal if normal is not None else gloss
    h, w = template.shape[:2]
    profile = dict(recipe, **{k: v for k, v in (authored or {}).items() if k != "regions"})
    if not 0 <= profile["minimum"] <= profile["roughness"] <= profile["maximum"] <= 1:
        raise ValueError("Physical recipe requires ordered unit roughness bounds")
    rough = np.full((h, w), profile["roughness"], np.float32)
    if gloss is not None:
        g = np.asarray(Image.fromarray(gloss[..., 3]).resize((w, h), Image.Resampling.BILINEAR), np.float32)/255
        fitted = (2/(np.exp2(13*g)+2))**.25
        weight = profile.get("gloss_weight", 1.)
        if not 0 <= weight <= 1:
            raise ValueError("Gloss fit weight must be in [0,1]")
        rough = np.clip((1-weight)*rough+weight*fitted,
                        (authored or {}).get("minimum", .089), (authored or {}).get("maximum", 1.))
    fields = np.dstack((np.ones((h, w)), rough,
                        np.full((h, w), profile["metallic"]), np.full((h, w), profile["coating"])))
    for region in (authored or {}).get("regions", []):
        u0, v0, u1, v1 = region["uv_rect"]
        if not 0 <= u0 < u1 <= 1 or not 0 <= v0 < v1 <= 1:
            raise ValueError("Reviewed atlas region must be an ordered unit UV rectangle")
        ys, xs = slice(round(v0*h), round(v1*h)), slice(round(u0*w), round(u1*w))
        for channel, key in ((1, "roughness"), (2, "metallic"), (3, "coating")):
            if key in region:
                fields[ys, xs, channel] = region[key]
    if not np.isfinite(fields).all() or np.any((fields < 0) | (fields > 1)):
        raise ValueError("Physical recipe produced nonfinite/out-of-range response fields")
    return np.rint(fields*255).astype(np.uint8)
