"""Qualified neural preparation with semantic channels and original mip tails.

DeepBump uses ONNX Runtime CPU/CUDA; Remacri uses the optional Spandrel adapter.
Model weights, candidates and
receipts remain external. This command never writes into its source pack.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import time

import numpy as np
from PIL import Image

from material_textures import color_levels, inspect_dds, normal_levels, response_levels, rgba_dds, rgba_levels
from preparation_resources import PreparationResources
from deepbump_inference import DeepBumpSession

MODEL_SHA256 = "e1a73bd89c2da1ae494774746398689048b5a892bd9653e146713f9df8bca86a"
MODEL_CREDIT = "4x Remacri by FoolhardyVEVO; CC-BY-NC-SA-4.0; https://openmodeldb.info/models/4x-Remacri"
DEEPBUMP_UPSCALE_SHA256 = "cf89d0ac5f7a73c8c172b8d92bd0e1fc3d5385bddef2fab15146f84a61477703"
DEEPBUMP_CREDIT = "DeepBump by HugoTini; GPL-3.0; https://github.com/HugoTini/DeepBump"
RECIPE_VERSION = 2


class ConstantField:
    def upscale(self, rgb, repeat_u=True, repeat_v=True):
        if not np.all(rgb == rgb[0, 0]):
            raise ValueError("Constant-field preparation received varying material data")
        return np.tile(rgb[0, 0], (rgb.shape[0]*4, rgb.shape[1]*4, 1))


class Remacri:
    def __init__(self, weights, tile=256, halo=64, pause_file=None):
        import torch
        import spandrel
        if hashlib.sha256(weights.read_bytes()).hexdigest() != MODEL_SHA256:
            raise ValueError("The requested original Remacri model checksum does not match")
        self.torch, self.tile, self.halo, self.pause_file = torch, tile, halo, pause_file
        torch.set_num_threads(1)
        # Full precision prevents invented precision-dependent texture changes.
        self.model = spandrel.ModelLoader().load_from_file(weights).eval()
        if self.model.scale != 4 or self.model.input_channels != 3:
            raise ValueError("Expected the original RGB 4x ESRGAN Remacri model")
        self.on_gpu = False
        self.resources = PreparationResources(pause_file, exit_for_shared_slot=True)

    def ready(self):
        if self.resources.refresh(gpu_admission=not self.on_gpu, gpu_owned=self.on_gpu):
            self.resources.wait(gpu_admission=not self.on_gpu, gpu_owned=self.on_gpu, on_pause=self.offload)
        if not self.on_gpu:
            self.resources.wait(gpu_admission=True)
            self.model.cuda()
            self.on_gpu = True

    def offload(self):
        if self.on_gpu:
            self.model.cpu()
            self.torch.cuda.empty_cache()
            self.on_gpu = False

    def upscale(self, rgb, repeat_u=True, repeat_v=True):
        height, width = rgb.shape[:2]
        if np.all(rgb == rgb[0, 0]):
            # Constant material fields must stay constant; the RGB model's
            # learned border response has no physical meaning for them.
            return np.tile(rgb[0, 0], (height * 4, width * 4, 1))
        result = np.empty((height * 4, width * 4, 3), np.float32)
        for y in range(0, height, self.tile):
            for x in range(0, width, self.tile):
                self.ready()
                y_end, x_end = min(y + self.tile, height), min(x + self.tile, width)
                ys = np.arange(y - self.halo, y_end + self.halo)
                xs = np.arange(x - self.halo, x_end + self.halo)
                ys = ys % height if repeat_v else np.clip(ys, 0, height - 1)
                xs = xs % width if repeat_u else np.clip(xs, 0, width - 1)
                tile = rgb[ys[:, None], xs[None, :]]
                tensor = self.torch.from_numpy(np.ascontiguousarray(tile.transpose(2, 0, 1))).unsqueeze(0).cuda()
                with self.torch.inference_mode():
                    output = self.model(tensor).float().clamp(0, 1)
                crop = output[0, :, self.halo*4:(self.halo+y_end-y)*4,
                              self.halo*4:(self.halo+x_end-x)*4]
                result[y*4:y_end*4, x*4:x_end*4] = crop.cpu().numpy().transpose(1, 2, 0)
                del tensor, output, crop
        return result


class DeepBumpUpscaler:
    """Original DeepBump ONNX weights with deterministic contextual tile edges.

    Its published wrapper has nonoverlapping tiles and random edge shuffling.
    Here every output cell comes from a 256-pixel context with a retained halo,
    so periodic textures and atlas borders remain deterministic.
    """
    def __init__(self, weights, tile=256, halo=64, pause_file=None, device="cpu"):
        if hashlib.sha256(weights.read_bytes()).hexdigest() != DEEPBUMP_UPSCALE_SHA256:
            raise ValueError("DeepBump upscale model checksum mismatch")
        if not 0 < halo < 128:
            raise ValueError("DeepBump 256-pixel model requires a halo below 128")
        self.backend = DeepBumpSession(weights, pause_file, device)
        self.halo, self.tile, self.pause_file = halo, 256 - 2*halo, pause_file

    @property
    def on_gpu(self):
        return self.backend.device == "cuda" and self.backend.session is not None

    def offload(self):
        self.backend.offload()

    def upscale(self, rgb, repeat_u=True, repeat_v=True):
        height, width = rgb.shape[:2]
        if np.all(rgb == rgb[0, 0]):
            return np.tile(rgb[0, 0], (height*4, width*4, 1))
        result = np.empty((height*4, width*4, 3), np.float32)
        for y in range(0, height, self.tile):
            for x in range(0, width, self.tile):
                ys = np.arange(y-self.halo, y-self.halo+256)
                xs = np.arange(x-self.halo, x-self.halo+256)
                ys = ys % height if repeat_v else np.clip(ys, 0, height-1)
                xs = xs % width if repeat_u else np.clip(xs, 0, width-1)
                tile = rgb[ys[:, None], xs[None, :]]
                tensor = np.ascontiguousarray(tile.transpose(2, 0, 1)[None], np.float32)
                output = self.backend.run(tensor)
                if output.shape != (1, 3, 1024, 1024):
                    raise ValueError("Unexpected DeepBump 4x output shape")
                y_end, x_end = min(y+self.tile, height), min(x+self.tile, width)
                crop = output[0, :, self.halo*4:(self.halo+y_end-y)*4,
                              self.halo*4:(self.halo+x_end-x)*4]
                result[y*4:y_end*4, x*4:x_end*4] = np.clip(crop.transpose(1, 2, 0), 0, 1)
        return result


def preserve_low_frequency(candidate, source):
    """Keep the original texel average, while retaining inferred subtexel detail."""
    height, width, channels = source.shape
    result = candidate.reshape(height, 4, width, 4, channels).copy()
    for _ in range(8):
        error = source - result.mean(axis=(1, 3))
        result += error[:, None, :, None, :]
        np.clip(result, 0, 1, out=result)
    return result.reshape(height*4, width*4, channels)


def upscale_channel(model, pixels, entry):
    values = pixels.astype(np.float32) / 255.
    role = entry["role"]
    samplers = entry.get("samplers", [20])
    repeat_u, repeat_v = not any(x & 32 for x in samplers), not any(x & 64 for x in samplers)
    def scalar(plane):
        if np.ptp(plane) == 0:
            return np.full((plane.shape[0]*4, plane.shape[1]*4), plane[0, 0], np.float32)
        rgb = np.repeat(plane[..., None], 3, axis=2)
        # RGB-trained inference is applied to each packed scalar separately;
        # material fields can never contaminate one another as RGB colors.
        up = model.upscale(rgb, repeat_u, repeat_v).mean(axis=2, keepdims=True)
        return preserve_low_frequency(up, plane[..., None])[..., 0]
    if role in ("color", "emissive"):
        rgb = model.upscale(values[..., :3], repeat_u, repeat_v)
        rgb = preserve_low_frequency(rgb, values[..., :3])
        # Authored coverage is geometrical data, including opaque zero alpha.
        # Preserve every source coverage cell exactly at the finer resolution.
        alpha = np.repeat(np.repeat(values[..., 3], 4, axis=0), 4, axis=1)
        result = np.dstack((rgb, alpha))
    elif role == "normal":
        rgb = model.upscale(values[..., :3], repeat_u, repeat_v)
        rgb = preserve_low_frequency(rgb, values[..., :3])
        normal = rgb * 2 - 1
        normal[..., 2] = np.maximum(normal[..., 2], 1e-4)
        normal /= np.maximum(np.linalg.norm(normal, axis=2, keepdims=True), 1e-6)
        result = np.dstack((normal*.5+.5, np.ones(normal.shape[:2], np.float32)))
    elif role == "response":
        # Occlusion stays neutral; no baked darkness is inferred from albedo.
        if np.any(pixels[..., 0] != 255):
            raise ValueError("Canonical realtime response requires neutral occlusion")
        result = np.dstack((np.ones((pixels.shape[0]*4, pixels.shape[1]*4), np.float32),
                            scalar(values[..., 1]), scalar(values[..., 2]), scalar(values[..., 3])))
    elif role in ("control", "mask", "coating", "auxiliary"):
        result = np.stack([scalar(values[..., channel]) for channel in range(4)], axis=2)
    else:
        raise ValueError(f"Unimplemented texture role: {role}")
    return np.rint(np.clip(result, 0, 1) * 255).astype(np.uint8)


def top_levels(pixels, entry):
    if entry["role"] == "normal":
        normal = pixels[..., :3].astype(np.float32)/255*2-1
        levels, _ = normal_levels(normal)
    elif entry["role"] == "response":
        levels = response_levels(pixels, [])
    else:
        levels = color_levels(pixels, entry["color_space"] == "srgb")
    return levels[:2]


def source_levels(pack, masters, entry):
    path = pack / entry["uri"]
    raw = path.read_bytes()
    info = inspect_dds(raw)
    if (info["width"], info["height"]) != (entry["width"], entry["height"]):
        raise ValueError(f"Source dimensions changed after inventory: {entry['uri']}")
    if entry.get("source_runtime_sha256") not in (None, info["sha256"]):
        raise ValueError(f"Source texture changed after inventory: {entry['uri']}")
    if info["dxgi"] in (28, 29) and raw[84:88] == b"DX10":
        return rgba_levels(raw), path, info["sha256"], info["sha256"]
    master = masters / path.name
    if not master.is_file():
        raise ValueError(f"Lossless canonical master unavailable: {entry['uri']}")
    master_raw = master.read_bytes()
    record = inspect_dds(master_raw)
    if (record["width"], record["height"], record["mip_count"]) != (
            info["width"], info["height"], info["mip_count"]):
        raise ValueError("Canonical master does not match runtime texture dimensions")
    return rgba_levels(master_raw), master, record["sha256"], info["sha256"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--masters", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--upscaler", choices=("remacri", "deepbump"), default="deepbump")
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu",
                        help="DeepBump backend; Remacri retains its CUDA adapter")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--select", default="")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--tile", type=int, default=256)
    parser.add_argument("--halo", type=int, default=64)
    parser.add_argument("--pause-file", type=Path)
    parser.add_argument("--constant-only", action="store_true",
                        help="Prepare single-texel material fields without loading inference models")
    parser.add_argument("--previews", action="store_true",
                        help="Write external PNG evidence for selected qualification batches")
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parents[1]
    if args.output.resolve().is_relative_to(workspace) or args.output.resolve().is_relative_to(args.pack.resolve()):
        raise ValueError("Candidate output must be external and separate from source assets")
    args.output.mkdir(parents=True, exist_ok=True)
    entries = json.loads(args.inventory.read_text())["textures"]
    if args.constant_only:
        entries = [entry for entry in entries if entry["width"] == entry["height"] == 1]
    if args.select:
        entries = [x for x in entries if re.search(args.select, x["uri"] + " " + " ".join(x["materials"]))]
    entries = sorted(entries, key=lambda e: (e["role"] != "color", e["uri"]))
    if args.limit:
        entries = entries[:args.limit]
    model = None
    model_hash = MODEL_SHA256 if args.upscaler == "remacri" else DEEPBUMP_UPSCALE_SHA256
    credit = MODEL_CREDIT if args.upscaler == "remacri" else DEEPBUMP_CREDIT
    resources = PreparationResources(args.pause_file, exit_for_shared_slot=True)
    for index, entry in enumerate(entries):
        resources.wait(gpu_owned=model is not None and model.on_gpu,
                       on_pause=model.offload if model is not None else None)
        output = args.output / entry["uri"]
        receipt = output.with_suffix(".json")
        output.parent.mkdir(parents=True, exist_ok=True)
        levels, source, source_hash, runtime_hash = source_levels(args.pack, args.masters, entry)
        if entry["scale"] not in (1, 4):
            raise ValueError("Qualified models provide exactly 4x preparation")
        if max(entry["width"], entry["height"]) * entry["scale"] > 16384:
            raise ValueError(f"Candidate exceeds Bevy's supported texture extent: {entry['uri']}")
        recipe = dict(version=RECIPE_VERSION, model_sha256=model_hash, upscaler=args.upscaler,
                      device=args.device if args.upscaler == "deepbump" else "cuda",
                      tile=args.tile, halo=args.halo, scale=entry["scale"], role=entry["role"],
                      color_space=entry["color_space"], samplers=sorted(entry.get("samplers", [20])))
        recipe_hash = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()
        if receipt.exists() and output.exists():
            existing = json.loads(receipt.read_text())
            if existing["source_sha256"] == source_hash and existing.get("recipe_sha256") == recipe_hash:
                if hashlib.sha256(output.read_bytes()).hexdigest() == existing["output_sha256"]:
                    if args.previews and not output.with_suffix(".candidate.png").exists():
                        Image.fromarray(levels[0]).save(output.with_suffix(".source.png"))
                        Image.fromarray(rgba_levels(output.read_bytes())[0]).save(output.with_suffix(".candidate.png"))
                    print(f"{index+1}/{len(entries)} cached {entry['uri']}", flush=True)
                    continue
        started = time.monotonic()
        if entry["scale"] == 1:
            cooked = levels
            pixels = levels[0]
        else:
            constant = np.all(levels[0] == levels[0][0, 0])
            if constant:
                pixels = upscale_channel(ConstantField(), levels[0], entry)
            else:
                if model is None:
                    model_type = Remacri if args.upscaler == "remacri" else DeepBumpUpscaler
                    options = dict(device=args.device) if args.upscaler == "deepbump" else {}
                    model = model_type(args.model, args.tile, args.halo, args.pause_file, **options)
                pixels = upscale_channel(model, levels[0], entry)
            resources.wait(gpu_owned=model is not None and model.on_gpu,
                           on_pause=model.offload if model is not None else None)
            # Retain the original chain verbatim below the new detail levels.
            # Distant material appearance and authored alpha mip coverage persist.
            cooked = top_levels(pixels, entry) + levels
        raw = rgba_dds(cooked, entry["color_space"] == "srgb")
        temporary = output.with_suffix(".tmp")
        temporary.write_bytes(raw)
        temporary.replace(output)
        record = dict(uri=entry["uri"], role=entry["role"], scale=entry["scale"],
            source_sha256=source_hash, model_sha256=model_hash, credit=credit, upscaler=args.upscaler,
            source_runtime_sha256=runtime_hash,
            recipe=recipe, recipe_sha256=recipe_hash,
            output_sha256=hashlib.sha256(raw).hexdigest(), width=pixels.shape[1], height=pixels.shape[0],
            mip_count=len(cooked), source_mips_retained=True, seconds=time.monotonic()-started,
            original_coverage_retained=entry["role"] in ("color", "emissive"))
        record["constant_field"] = bool(np.all(levels[0] == levels[0][0, 0]))
        receipt.write_text(json.dumps(record, indent=2) + "\n")
        # Qualification previews are evidence, never runtime source selection.
        if args.previews:
            Image.fromarray(levels[0]).save(output.with_suffix(".source.png"))
            Image.fromarray(pixels).save(output.with_suffix(".candidate.png"))
        print(f"{index+1}/{len(entries)} {entry['role']} {entry['uri']} {record['width']}x{record['height']}", flush=True)
    print("Bounded batch complete; process exit releases inference resources", flush=True)


if __name__ == "__main__":
    main()
