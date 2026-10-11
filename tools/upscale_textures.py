import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import time
from concurrent.futures import ThreadPoolExecutor

import numpy as np
from PIL import Image

from material_textures import inspect_dds, inspect_texture, rgba_levels
from ktx2_textures import decoded_levels
from preparation_resources import PreparationResources
from deepbump_inference import DeepBumpSession
from compressed_texture_cache import NativeTextureCache, mask_recipe, materialize, open_validated_texture, stored_bytes

DEEPBUMP_UPSCALE_SHA256 = "cf89d0ac5f7a73c8c172b8d92bd0e1fc3d5385bddef2fab15146f84a61477703"
DEEPBUMP_CREDIT = "DeepBump by HugoTini; GPL-3.0; https://github.com/HugoTini/DeepBump"
RECIPE_VERSION = 6


class DeepBumpUpscaler:
    def __init__(self, weights, tile=256, halo=64, pause_file=None, device="cpu", saturate=True, cuda_graph=True,
                 math_mode="fp32", prefer_nhwc=False):
        if hashlib.sha256(weights.read_bytes()).hexdigest() != DEEPBUMP_UPSCALE_SHA256:
            raise ValueError("DeepBump upscale model checksum mismatch")
        if tile != 256 or not 0 < halo < 128:
            raise ValueError("DeepBump 256-pixel model requires a halo below 128")
        self.backend = DeepBumpSession(weights, pause_file, device, saturate, cuda_graph,
                                      math_mode, prefer_nhwc)
        self.executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="deepbump")
        self.halo, self.tile, self.pause_file = halo, 256 - 2*halo, pause_file

    @property
    def on_gpu(self):
        return self.backend.device == "cuda" and self.backend.session is not None

    def offload(self):
        self.backend.offload()

    def upscale(self, rgb, repeat_u=True, repeat_v=True, scale=4, entry=None, progress=""):
        height, width = rgb.shape[:2]
        if scale not in (2, 4):
            raise ValueError("DeepBump stages retain 4x output or reduce it to 2x")
        if np.all(rgb == rgb[0, 0]):
            return np.tile(rgb[0, 0], (height*scale, width*scale, 1))
        result = np.empty((height*scale, width*scale, 3), np.float32)
        started = reported = time.monotonic()
        total = math.ceil(height/self.tile)*math.ceil(width/self.tile)
        completed = 0
        print(f"Inference {progress}: {width}x{height} -> {width*scale}x{height*scale}, "
              f"{total} contextual tiles", flush=True)

        def tensors():
            for y in range(0, height, self.tile):
                for x in range(0, width, self.tile):
                    ys = np.arange(y-self.halo, y-self.halo+256)
                    xs = np.arange(x-self.halo, x-self.halo+256)
                    ys = ys % height if repeat_v else np.clip(ys, 0, height-1)
                    xs = xs % width if repeat_u else np.clip(xs, 0, width-1)
                    tile = rgb[ys[:, None], xs[None, :]]
                    tensor = np.ascontiguousarray(tile.transpose(2, 0, 1)[None], np.float32)
                    yield y, x, tensor

        pending = iter(tensors())
        current = next(pending, None)
        future = self.executor.submit(self.backend.run, current[2]) if current else None
        while current is not None:
            y, x, _ = current
            output = future.result()
            upcoming = next(pending, None)
            future = self.executor.submit(self.backend.run, upcoming[2]) if upcoming else None
            if output.shape != (1, 3, 1024, 1024):
                raise ValueError("Unexpected DeepBump 4x output shape")
            y_end, x_end = min(y+self.tile, height), min(x+self.tile, width)
            crop = output[0, :, self.halo*4:(self.halo+y_end-y)*4,
                          self.halo*4:(self.halo+x_end-x)*4]
            crop = np.clip(crop.transpose(1, 2, 0), 0, 1)
            if not np.isfinite(crop).all():
                raise ValueError("DeepBump produced nonfinite material data")
            if scale == 2:
                rgba = np.empty((*crop.shape[:2], 4), np.uint8)
                rgba[..., :3] = np.rint(crop*255).astype(np.uint8)
                rgba[..., 3] = 255
                crop = downsample(rgba, entry)[..., :3].astype(np.float32)/255
            result[y*scale:y_end*scale, x*scale:x_end*scale] = crop
            completed += 1
            now = time.monotonic()
            if now-reported >= 30 or completed == total:
                print(f"Inference {progress}: {completed}/{total} tiles, "
                      f"{now-started:.1f}s elapsed", flush=True)
                reported = now
            current = upcoming
        return result


def preserve_low_frequency(candidate, source):
    height, width, channels = source.shape
    scale = candidate.shape[0]//height
    result = candidate.reshape(height, scale, width, scale, channels)
    def preserve_rows(start):
        block = result[start:start+16]
        for _ in range(8):
            error = source[start:start+16] - block.mean(axis=(1, 3))
            block += error[:, None, :, None, :]
            np.clip(block, 0, 1, out=block)
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 1) as workers:
        list(workers.map(preserve_rows, range(0, height, 16)))
    return result.reshape(height*scale, width*scale, channels)


def upscale_channel(model, pixels, entry, scale=4):
    values = pixels.astype(np.float32) / 255.
    role = entry["role"]
    samplers = entry.get("samplers", [20])
    addresses = {(x & 32, x & 64) for x in samplers}
    if len(addresses) != 1:
        raise ValueError(f"{entry['uri']}: conflicting clamp/repeat uses; split the material recipe explicitly")
    if role not in ("color", "emissive"):
        width, height = pixels.shape[1]*scale, pixels.shape[0]*scale
        result = np.zeros((height, width, 4), np.uint8)
        result[..., 3] = 255
        def resized(plane):
            pad = np.pad(plane, ((1, 1), (0, 0)), mode="edge" if samplers[0]&64 else "wrap")
            pad = np.pad(pad, ((0, 0), (1, 1)), mode="edge" if samplers[0]&32 else "wrap")
            enlarged = Image.fromarray(pad).resize(((pixels.shape[1]+2)*scale,
                         (pixels.shape[0]+2)*scale), Image.Resampling.BILINEAR)
            return np.asarray(enlarged)[scale:height+scale, scale:width+scale]
        if role == "normal":
            z = np.maximum(values[..., 2]*2-1, .05)
            planes = [resized((values[..., i]*2-1)/z) for i in range(2)]
            for start in range(0, height, 32):
                unit = np.dstack((planes[0][start:start+32], planes[1][start:start+32],
                                  np.ones((min(32, height-start), width), np.float32)))
                unit /= np.maximum(np.linalg.norm(unit, axis=2, keepdims=True), 1e-6)
                result[start:start+32, :, :3] = np.rint(np.clip(unit*.5+.5, 0, 1)*255).astype(np.uint8)
        else:
            if role == "response":
                if np.any(pixels[..., 0] != 255):
                    raise ValueError("Realtime response requires neutral occlusion")
                channels = (1, 2, 3)
                result[..., 0] = 255
            elif role in ("control", "coating", "mask", "auxiliary"):
                channels = (0, 1) if role == "control" else (0,) if role == "coating" else range(4)
            else:
                raise ValueError(f"Unsupported physical field role: {role}")
            for channel in channels:
                plane = values[..., channel]**4 if role == "response" and channel == 1 else values[..., channel]
                plane = resized(plane)
                for start in range(0, height, 32):
                    rows = plane[start:start+32]
                    if role == "response" and channel == 1:
                        rows = np.maximum(rows, 0)**.25
                    result[start:start+32, :, channel] = np.rint(np.clip(rows, 0, 1)*255).astype(np.uint8)
                del plane
        return result
    repeat_u, repeat_v = not any(x & 32 for x in samplers), not any(x & 64 for x in samplers)
    result = np.empty((pixels.shape[0]*scale, pixels.shape[1]*scale, 4), np.uint8)
    def encode(values, channel):
        for start in range(0, values.shape[0], 32):
            block = values[start:start+32]
            result[start:start+32, :, channel] = np.rint(np.clip(block, 0, 1)*255).astype(np.uint8)
    if role in ("color", "emissive"):
        rgb = model.upscale(values[..., :3], repeat_u, repeat_v, scale=scale, entry=entry,
                            progress=f"{entry['uri']} {role} RGB")
        rgb = preserve_low_frequency(rgb, values[..., :3])
        encode(rgb, slice(0, 3))
        result[..., 3] = np.repeat(np.repeat(pixels[..., 3], scale, axis=0), scale, axis=1)
    return result


def top_levels(pixels, entry, count=None):
    levels = [pixels]
    while levels[-1].shape[:2] != (1, 1) and (count is None or len(levels) < count):
        levels.append(downsample(levels[-1], entry))
    return levels


def downsample(pixels, entry):
    height, width = pixels.shape[:2]
    nh, nw = max(1, height//2), max(1, width//2)
    output = np.empty((nh, nw, 4), np.uint8)
    from material_textures import reduced
    for start in range(0, nh, 32):
        rows = pixels[start*2:min((start+32)*2, height)].astype(np.float32)/255
        if entry["role"] == "normal":
            normals = rows[..., :3]*2-1
            mean = reduced(normals)
            mean /= np.maximum(np.linalg.norm(mean, axis=2, keepdims=True), 1e-6)
            encoded = np.dstack((mean*.5+.5, np.ones(mean.shape[:2], np.float32)))
        else:
            if entry["color_space"] == "srgb":
                rgb = rows[..., :3]
                rows[..., :3] = np.where(rgb <= .04045, rgb/12.92, ((rgb+.055)/1.055)**2.4)
            if entry["role"] == "response":
                rows[..., 1] **= 4
            encoded = reduced(rows)
            if entry["color_space"] == "srgb":
                rgb = encoded[..., :3]
                encoded[..., :3] = np.where(rgb <= .0031308, rgb*12.92, 1.055*np.maximum(rgb, 0)**(1/2.4)-.055)
            if entry["role"] == "response":
                encoded[..., 1] = np.maximum(encoded[..., 1], 0)**.25
        output[start:start+len(encoded)] = np.rint(np.clip(encoded, 0, 1)*255).astype(np.uint8)
    return output


def prepare_levels(model, levels, entry):
    scale = entry["scale"]
    if scale < 1 or scale & (scale-1):
        raise ValueError("Density output scale must be a positive power of two")
    if entry.get("density_applicable") and (
            entry["minimum_texels_per_meter"] * scale < entry["density_floor"] and not entry.get("density_exception")):
        raise ValueError(f"Candidate cannot satisfy density floor: {entry['uri']}")
    if max(entry["width"], entry["height"]) * scale > 16384:
        raise ValueError(f"Strict density floor exceeds Bevy's texture limit: {entry['uri']}")
    if scale == 1:
        return levels, 0
    pixels = levels[0]
    retained_scale, stages = 1, 0
    while retained_scale < scale:
        step = 4 if scale//retained_scale >= 4 else 2
        print(f"Stage {stages+1} {entry['uri']}: retained scale {retained_scale} -> "
              f"{retained_scale*step}, role {entry['role']}", flush=True)
        pixels = upscale_channel(model, pixels, entry, scale=step)
        retained_scale *= step
        stages += 1
    added = int(math.log2(scale))
    generated = top_levels(pixels, entry, count=added)
    return generated[:added] + levels, stages


def source_levels(pack, masters, entry):
    path = pack / entry["uri"]
    raw = path.read_bytes()
    info = inspect_texture(raw)
    if (info["width"], info["height"]) != (entry["width"], entry["height"]):
        raise ValueError(f"Source dimensions changed after inventory: {entry['uri']}")
    if entry.get("source_runtime_sha256") not in (None, info["sha256"]):
        raise ValueError(f"Source texture changed after inventory: {entry['uri']}")
    if info["dxgi"] in (28, 29, 49, 61):
        levels = decoded_levels(raw)
        if entry["role"] == "normal" and info["dxgi"] == 49:
            for level in levels:
                xy = level[..., :2].astype(np.float32)/127.5-1
                z = np.sqrt(np.maximum(1-(xy*xy).sum(axis=2), 0))
                level[..., 2] = np.rint((z*.5+.5)*255).astype(np.uint8)
        return levels, path, info["sha256"], info["sha256"]
    master = masters / path.with_suffix(".dds").name
    if not master.is_file():
        raise ValueError(f"Lossless canonical master unavailable: {entry['uri']}")
    master_raw = master.read_bytes()
    expected = "pbr-" + hashlib.sha256(master_raw + entry["role"].encode()).hexdigest()[:32]
    if path.stem != expected:
        raise ValueError(f"Lossless master is not the content-addressed source: {entry['uri']}")
    record = inspect_dds(master_raw)
    if (record["width"], record["height"], record["mip_count"]) != (
            info["width"], info["height"], info["mip_count"]):
        raise ValueError("Canonical master does not match runtime texture dimensions")
    return rgba_levels(master_raw), master, record["sha256"], info["sha256"]


def main():
    parser = argparse.ArgumentParser(description='Qualified neural preparation with semantic channels and original mip tails.\n\nDeepBump uses ONNX Runtime CPU/CUDA for every inferred detail level.\nModel weights, candidates and\nreceipts remain external. This command never writes into its source pack.\n')
    parser.add_argument("--pack", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--masters", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--upscaler", choices=("deepbump",), default="deepbump")
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cuda",
                        help="Explicit DeepBump inference backend")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--preparation-executable", type=Path, required=True)
    parser.add_argument("--cache", type=Path, help="External compressed derived-data cache; defaults beside candidates")
    parser.add_argument("--polite", action="store_true", help="Opt into low-priority resource-pressure yielding")
    parser.add_argument("--no-cuda-graph", action="store_true", help="Disable fixed-buffer CUDA graph replay")
    parser.add_argument("--cuda-math", choices=("fp32", "tf32"), default="fp32",
                        help="CUDA convolution arithmetic; original float32 model/input/output stay pinned")
    parser.add_argument("--prefer-nhwc", action="store_true", help="Use ORT's documented NHWC CUDA kernel preference")
    parser.add_argument("--select", default="")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--tile", type=int, default=256)
    parser.add_argument("--halo", type=int, default=64)
    parser.add_argument("--pause-file", type=Path)
    parser.add_argument("--constant-only", action="store_true",
                        help="Prepare single-texel material fields without loading inference models")
    parser.add_argument("--previews", action="store_true",
                        help="Write external PNG evidence for selected qualification batches")
    parser.add_argument("--audit-select", help="Detailed numerical audits only for this real representative regex; all maps retain integrity checks")
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parents[1]
    if args.output.resolve().is_relative_to(workspace) or args.output.resolve().is_relative_to(args.pack.resolve()) or args.pack.resolve().is_relative_to(args.output.resolve()):
        raise ValueError("Candidate output must be external and separate from source assets")
    args.output.mkdir(parents=True, exist_ok=True)
    cache_root = args.cache or args.output.parent/"compressed-texture-cache"
    if cache_root.resolve().is_relative_to(workspace) or cache_root.resolve().is_relative_to(args.pack.resolve()) or args.pack.resolve().is_relative_to(cache_root.resolve()):
        raise ValueError("Derived-data cache must be external and separate from source assets")
    resources = PreparationResources(args.pause_file, exit_for_shared_slot=True, saturate=not args.polite)
    cache = NativeTextureCache(cache_root, args.preparation_executable, resources)
    entries = json.loads(args.inventory.read_text())["textures"]
    if args.constant_only:
        entries = [entry for entry in entries if entry["width"] == entry["height"] == 1]
    if args.select:
        entries = [x for x in entries if re.search(args.select, x["uri"] + " " + " ".join(x["materials"]))]
    entries = sorted(entries, key=lambda e: (e["role"] != "color", e["uri"]))
    if args.limit:
        entries = entries[:args.limit]
    impossible = [entry["uri"] for entry in entries if not entry.get("achievable", True) and not entry.get("density_exception")]
    if impossible:
        raise ValueError(f"Strict density preflight rejected {len(impossible)} maps above 16384 pixels: "
                         + ", ".join(impossible[:5]))
    extents = [stored_bytes(entry) for entry in entries]
    scratch = 2*max((math.ceil(entry["width"]*entry["height"]*entry["scale"]**2*16/3)+148 for entry in entries), default=0)
    required = sum(max(0, size-(args.output/entry["uri"]).stat().st_size)
                   if (args.output/entry["uri"]).exists() else size
                   for entry, size in zip(entries, extents)) + scratch
    free = shutil.disk_usage(args.output).free
    if required > free:
        raise ValueError(f"Candidate storage preflight needs {required/2**30:.2f} GiB; only {free/2**30:.2f} GiB free. "
                         "Use an external volume with sufficient space or an explicit bounded selection.")
    cache_required = sum(extents)+scratch
    if cache_root.resolve().anchor != args.output.resolve().anchor and cache_required > shutil.disk_usage(cache_root).free:
        raise ValueError(f"Compressed cache needs {cache_required/2**30:.2f} GiB free on its volume")
    print(f"Compressed candidate budget {sum(extents)/2**30:.2f} GiB; temporary raw chain at most {scratch/2**30:.2f} GiB", flush=True)
    model = None
    model_hash = DEEPBUMP_UPSCALE_SHA256
    credit = DEEPBUMP_CREDIT
    for index, entry in enumerate(entries):
        resources.wait(gpu_owned=model is not None and model.on_gpu,
                       on_pause=model.offload if model is not None else None)
        output = args.output / entry["uri"]
        receipt = output.with_suffix(".json")
        output.parent.mkdir(parents=True, exist_ok=True)
        levels, source, source_hash, runtime_hash = source_levels(args.pack, args.masters, entry)
        if entry["scale"] < 1 or entry["scale"] & (entry["scale"]-1):
            raise ValueError("Density preparation requires a power-of-two output scale")
        if max(entry["width"], entry["height"]) * entry["scale"] > 16384:
            raise ValueError(f"Candidate exceeds Bevy's supported texture extent: {entry['uri']}")
        recipe = dict(version=RECIPE_VERSION, model_sha256=model_hash, upscaler=args.upscaler,
                      device=args.device,
                      backend=cache.backend, container="ktx2", mip_owner="semantic-preparation-and-authored-tail",
                      cuda_graph=not args.no_cuda_graph,
                      tile=args.tile, halo=args.halo, scale=entry["scale"], role=entry["role"],
                      color_space=entry["color_space"], samplers=sorted(entry.get("samplers", [20])))
        if entry["role"] not in ("color", "emissive"):
            recipe["semantic_filter_version"] = 2
        if args.cuda_math != "fp32" or args.prefer_nhwc:
            recipe.update(cuda_math=args.cuda_math, prefer_nhwc=args.prefer_nhwc)
        if entry["role"] in ("control", "coating"):
            recipe["sampled_channels"] = [0, 1] if entry["role"] == "control" else [0]
        recipe["mask_cutoffs"] = entry.get("mask_cutoffs", [])
        if entry.get("mask_cutoffs"):
            recipe["mask_compression"] = mask_recipe(entry)
        recipe.update(density_floor=entry.get("density_floor"), density_metric=entry.get("density_metric"))
        recipe["density_exception"] = entry.get("density_exception", False)
        recipe["source_density"] = entry.get("minimum_texels_per_meter")
        audit = args.audit_select is None or bool(re.search(args.audit_select, entry["uri"]))
        if not audit:
            recipe["detailed_audit"] = False
        recipe_hash = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()
        cache_key = hashlib.sha256((source_hash+runtime_hash+recipe_hash).encode()).hexdigest()
        neural_recipe = {key: value for key, value in recipe.items() if key not in
                         ("backend", "container", "mip_owner", "detailed_audit",
                          "mask_compression", "mask_cutoffs")}
        prepared_key = hashlib.sha256((source_hash+runtime_hash+
                       json.dumps(neural_recipe, sort_keys=True)).encode()).hexdigest()
        if receipt.exists() and output.exists():
            existing = json.loads(receipt.read_text())
            compatible = existing.get("recipe_sha256") == recipe_hash
            if not audit and existing.get("quality", {}).get("audited", True):
                stronger = {key: value for key, value in existing.get("recipe", {}).items()
                            if key != "detailed_audit"}
                requested = {key: value for key, value in recipe.items() if key != "detailed_audit"}
                compatible |= stronger == requested
            if args.cuda_math == "tf32" and existing.get("recipe", {}).get("cuda_math", "fp32") == "fp32":
                precise = {key: value for key, value in existing.get("recipe", {}).items()
                           if key not in ("cuda_math", "prefer_nhwc")}
                requested = {key: value for key, value in recipe.items()
                             if key not in ("cuda_math", "prefer_nhwc")}
                compatible |= precise == requested
            if existing["source_sha256"] == source_hash and compatible:
                if hashlib.sha256(output.read_bytes()).hexdigest() == existing["output_sha256"]:
                    if args.previews and not output.with_suffix(".candidate.png").exists():
                        Image.fromarray(levels[0]).save(output.with_suffix(".source.png"))
                        with open_validated_texture(output.read_bytes()) as preview:
                            preview.convert("RGBA").save(output.with_suffix(".candidate.png"))
                    print(f"{index+1}/{len(entries)} cached {entry['uri']}", flush=True)
                    continue
        started = time.monotonic()
        cached = cache.find(cache_key)
        stages = math.ceil(math.log2(entry["scale"])/2)
        if cached is None:
            cooked = cache.load_prepared(prepared_key, entry)
            if cooked is not None and [hashlib.sha256(memoryview(level).cast("B")).hexdigest()
                    for level in cooked[int(math.log2(entry["scale"])):]] != [
                    hashlib.sha256(memoryview(level).cast("B")).hexdigest() for level in levels]:
                raise ValueError("Recovered semantic candidate changed authored source mip data")
            if cooked is None:
                if entry["scale"] > 1 and entry["role"] in ("color", "emissive") and model is None:
                    model = DeepBumpUpscaler(args.model, args.tile, args.halo, args.pause_file, device=args.device,
                                             saturate=not args.polite, cuda_graph=not args.no_cuda_graph,
                                             math_mode=args.cuda_math, prefer_nhwc=args.prefer_nhwc)
                cooked, stages = prepare_levels(model, levels, entry)
                cache.save_prepared(prepared_key, cooked, entry)
            else:
                print(f"Reused unchanged semantic master {entry['uri']}", flush=True)
            resources.wait(gpu_owned=model is not None and model.on_gpu,
                           on_pause=model.offload if model is not None else None)
            cached = cache.store(cache_key, cooked, entry, (args.pack/entry["uri"]).read_bytes(),
                                 audit, prepared_key)
            del cooked
        materialize(cached[0], output)
        raw = output.read_bytes()
        info = inspect_texture(raw)
        record = dict(uri=entry["uri"], role=entry["role"], scale=entry["scale"],
            source_sha256=source_hash, model_sha256=model_hash, credit=credit, upscaler=args.upscaler,
            source_runtime_sha256=runtime_hash,
            recipe=recipe, recipe_sha256=recipe_hash,
            output_sha256=info["sha256"], width=info["width"], height=info["height"],
            mip_count=info["mip_count"], seconds=time.monotonic()-started,
            original_coverage_retained_before_compression=entry["role"] in ("color", "emissive"))
        record.update(cached[1])
        record.update(inference_stages=stages, density_floor=entry.get("density_floor"),
                      density_metric=entry.get("density_metric"),
                      output_texels_per_meter=entry.get("output_texels_per_meter"),
                      density_applicable=entry.get("density_applicable", True),
                      density_exception=entry.get("density_exception", False))
        record["constant_field"] = bool(np.all(levels[0] == levels[0][0, 0]))
        temporary = receipt.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(record, indent=2) + "\n")
        temporary.replace(receipt)
        if args.previews:
            Image.fromarray(levels[0]).save(output.with_suffix(".source.png"))
            with open_validated_texture(raw) as preview:
                preview.convert("RGBA").save(output.with_suffix(".candidate.png"))
        print(f"{index+1}/{len(entries)} {entry['role']} {entry['uri']} {record['width']}x{record['height']}", flush=True)
    print("Compressed batch complete; process exit releases inference resources", flush=True)


if __name__ == "__main__":
    main()
