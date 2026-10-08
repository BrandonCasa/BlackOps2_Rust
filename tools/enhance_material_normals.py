"""DeepBump normal candidates with authored macro structure and original mips.

Generates tangent detail only; it never infers occlusion or modifies geometry.
Weights and candidates remain external, with reproducible model provenance.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path

import numpy as np
from PIL import Image

from material_textures import normal_levels, response_levels, rgba_dds, rgba_levels
from deepbump_inference import DeepBumpSession

NORMAL_MODEL_SHA256 = "3a2ababc5fa652b19040a0f3d639b71826368e52ad805cf70ffb0e7c96a54d67"


class DeepBumpNormals:
    def __init__(self, weights, pause_file=None, device="cpu"):
        if hashlib.sha256(weights.read_bytes()).hexdigest() != NORMAL_MODEL_SHA256:
            raise ValueError("DeepBump normal model checksum mismatch")
        # The normal model returns 256-square data, unlike the 4x upscaler's
        # fixed 1024-square graph output. Use the native dynamic run path.
        self.backend = DeepBumpSession(weights, pause_file, device, cuda_graph=False)

    def infer(self, color, repeat_u=True, repeat_v=True):
        # The model is trained on grayscale texture structure, encoded as RGB
        # tangent normals. Keep the published 256-pixel input size unchanged.
        gray = color[..., :3].mean(axis=2).astype(np.float32)
        height, width = gray.shape
        result = np.empty((height, width, 3), np.float32)
        halo, core = 64, 128
        for y in range(0, height, core):
            for x in range(0, width, core):
                ys, xs = np.arange(y-halo, y-halo+256), np.arange(x-halo, x-halo+256)
                ys = ys % height if repeat_v else np.clip(ys, 0, height-1)
                xs = xs % width if repeat_u else np.clip(xs, 0, width-1)
                tile = np.ascontiguousarray(gray[ys[:, None], xs[None, :]][None, None])
                output = self.backend.run(tile)
                if output.shape != (1, 3, 256, 256):
                    raise ValueError("Unexpected DeepBump normal output shape")
                y_end, x_end = min(y+core, height), min(x+core, width)
                crop = output[0, :, halo:halo+y_end-y, halo:halo+x_end-x]
                result[y:y_end, x:x_end] = crop.transpose(1, 2, 0)*2-1
        result[..., 2] = np.maximum(result[..., 2], 1e-3)
        result /= np.maximum(np.linalg.norm(result, axis=2, keepdims=True), 1e-6)
        return result


def resize_normal(normal, width, height):
    channels = [np.asarray(Image.fromarray(normal[..., i]).resize((width, height), Image.Resampling.BILINEAR))
                for i in range(3)]
    result = np.stack(channels, axis=2)
    return result / np.maximum(np.linalg.norm(result, axis=2, keepdims=True), 1e-6)


def reorient(authored, detail):
    # Reoriented normal mapping retains the authored orientation rather than
    # flattening both maps through color interpolation.
    t = authored + [0., 0., 1.]
    u = detail * [-1., -1., 1.]
    result = t * (np.sum(t*u, axis=2, keepdims=True) / np.maximum(t[..., 2:3], 1e-6)) - u
    return result / np.maximum(np.linalg.norm(result, axis=2, keepdims=True), 1e-6)


def paired_response(source, variances, destination):
    """Keep authored fields and lower mips, filtering new GGX detail footprints."""
    source_raw = source.read_bytes()
    original = rgba_levels(source_raw)
    if np.any(original[0][..., 0] != 255):
        raise ValueError("Realtime response must retain neutral occlusion")
    pixels = np.repeat(np.repeat(original[0], 4, axis=0), 4, axis=1)
    levels = response_levels(pixels, variances)[:2] + original
    raw = rgba_dds(levels)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(raw)
    return dict(source_sha256=hashlib.sha256(source_raw).hexdigest(),
                output_sha256=hashlib.sha256(raw).hexdigest(),
                width=pixels.shape[1], height=pixels.shape[0], original_mips_retained=True)


def main():
    from scipy.ndimage import gaussian_filter
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--albedo", type=Path, required=True)
    parser.add_argument("--authored-normal", type=Path)
    parser.add_argument("--authored-response", type=Path,
                        help="Optional lossless response master for matched GGX mip filtering")
    parser.add_argument("--response-output", type=Path)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scale", type=int, default=4)
    parser.add_argument("--strength", type=float, default=.25)
    parser.add_argument("--sampler", type=int, default=20)
    parser.add_argument("--pause-file", type=Path)
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu")
    args = parser.parse_args()
    if bool(args.authored_response) != bool(args.response_output):
        raise ValueError("Provide both the authored response and its candidate output")
    if args.scale != 4 or not 0 <= args.strength <= 1:
        raise ValueError("Current preparation contract is 4x with unit-bounded detail strength")
    workspace = Path(__file__).resolve().parents[1]
    if args.output.resolve().is_relative_to(workspace):
        raise ValueError("Normal candidates must remain external until pack qualification")
    if args.response_output and args.response_output.resolve().is_relative_to(workspace):
        raise ValueError("Response candidates must remain external until pack qualification")
    inputs = [path.resolve() for path in (args.albedo, args.authored_normal, args.authored_response) if path]
    outputs = [path.resolve() for path in (args.output, args.response_output) if path]
    if len(set(outputs)) != len(outputs) or set(inputs) & set(outputs):
        raise ValueError("Candidate outputs must not replace any input")
    color_raw = args.albedo.read_bytes()
    color = np.asarray(Image.open(io.BytesIO(color_raw)).convert("RGB"), np.float32)/255.
    source_hash = hashlib.sha256(color_raw).hexdigest()
    authored_levels = None
    authored_hash = None
    if args.authored_normal:
        authored_raw = args.authored_normal.read_bytes()
        authored_hash = hashlib.sha256(authored_raw).hexdigest()
        authored_levels = rgba_levels(authored_raw)
        width, height = authored_levels[0].shape[1]*4, authored_levels[0].shape[0]*4
        authored = authored_levels[0][..., :3].astype(np.float32)/255*2-1
        authored = resize_normal(authored, width, height)
        if color.shape[:2] != (height, width):
            color = np.asarray(Image.fromarray(np.rint(color*255).astype(np.uint8)).resize(
                (width, height), Image.Resampling.LANCZOS), np.float32)/255.
    else:
        width, height = color.shape[1], color.shape[0]
        authored = np.zeros((height, width, 3), np.float32)
        authored[..., 2] = 1
    model = DeepBumpNormals(args.model, args.pause_file, args.device)
    inferred = model.infer(color, not args.sampler & 32, not args.sampler & 64)
    slopes = inferred[..., :2] / np.maximum(inferred[..., 2:3], .1)
    if authored_levels is not None:
        # Keep only detail finer than the original texture's structure. Do not
        # turn painted stains, logos or authored shading into large geometry.
        modes = ("wrap" if not args.sampler & 64 else "nearest",
                 "wrap" if not args.sampler & 32 else "nearest")
        for channel in range(2):
            slopes[..., channel] -= gaussian_filter(slopes[..., channel], sigma=8., mode=modes)
    slopes *= args.strength
    detail = np.dstack((slopes, np.ones((height, width), np.float32)))
    detail /= np.maximum(np.linalg.norm(detail, axis=2, keepdims=True), 1e-6)
    fused = reorient(authored, detail)
    levels, variances = normal_levels(fused)
    if authored_levels is not None:
        levels = levels[:2] + authored_levels
    raw = rgba_dds(levels)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(raw)
    Image.fromarray(levels[0]).save(args.output.with_suffix(".png"))
    record = dict(version=1, source_color_sha256=source_hash,
        authored_normal_sha256=authored_hash,
        model_sha256=NORMAL_MODEL_SHA256, model_source="https://github.com/HugoTini/DeepBump",
        model_commit="fad19ba87daed12b1d0410a57e74f3d79e82f78d", credit="DeepBump by HugoTini; GPL-3.0",
        strength=args.strength, width=width, height=height, authored_macro_structure_retained=authored_levels is not None,
        output_sha256=hashlib.sha256(raw).hexdigest(), approximation="learned tangent detail from intrinsic color")
    if args.authored_response:
        record["response"] = paired_response(args.authored_response, variances, args.response_output)
    args.output.with_suffix(".json").write_text(json.dumps(record, indent=2)+"\n")
    print(json.dumps(record), flush=True)


if __name__ == "__main__":
    main()
