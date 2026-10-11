import hashlib
import json
import math
from contextlib import contextmanager
from pathlib import Path
import shutil
import struct

import numpy as np
from PIL import Image

try:
    from .material_textures import inspect_dds, inspect_texture, rgba_dds, rgba_levels
    from .ktx2_textures import decoded_levels
except ImportError:
    from material_textures import inspect_dds, inspect_texture, rgba_dds, rgba_levels
    from ktx2_textures import decoded_levels

BACKEND = "bevy-0.20.0/CompressedImageSaver/ctt"


def mask_recipe(entry):
    return dict(version=2, exact_classification=True, cutoffs=entry["mask_cutoffs"]) if entry.get("mask_cutoffs") else None


class CoverageError(ValueError):
    def __init__(self, level, message):
        super().__init__(message)
        self.level = level


def quality_limits(role):
    return (2., 30.) if role == "normal" else (.025, .25)


def sampled_channels(role):
    return {"control": (0, 1), "coating": (0,), "emissive": (0, 1, 2), "reflectance": (0, 1, 2)}.get(role, (0, 1, 2, 3))


@contextmanager
def open_validated_texture(raw):
    info = inspect_texture(raw)
    if max(info["width"], info["height"]) > 16384:
        raise ValueError("Texture decode exceeds the approved renderer extent")
    previous = Image.MAX_IMAGE_PIXELS
    try:
        Image.MAX_IMAGE_PIXELS = max(previous or 0, info["width"]*info["height"])
        image = Image.fromarray(decoded_levels(raw)[0])
        try:
            yield image
        finally:
            image.close()
    finally:
        Image.MAX_IMAGE_PIXELS = previous


def stored_bytes(entry):
    width, height = entry["width"]*entry["scale"], entry["height"]*entry["scale"]
    total = 80 + max(width, height).bit_length()*40 + 128
    while True:
        total += width*height*4
        if width == height == 1:
            return total
        width, height = max(1, width//2), max(1, height//2)


def write_master(path, levels, srgb):
    header = bytearray(rgba_dds([levels[-1]], srgb)[:148])
    height, width = levels[0].shape[:2]
    struct.pack_into("<III", header, 12, height, width, width*4)
    struct.pack_into("<I", header, 28, len(levels))
    struct.pack_into("<I", header, 108, 0x401008 if len(levels) > 1 else 0x1000)
    with path.open("wb") as stream:
        stream.write(header)
        for level in levels:
            stream.write(memoryview(np.ascontiguousarray(level)).cast("B"))


def qualify(raw, levels, role, legacy_start=None, cutoffs=()):
    info = inspect_texture(raw)
    decoded_mips = decoded_levels(raw)
    rows = []
    rms_limit, max_limit = quality_limits(role)
    for index, (mip, original, decoded) in enumerate(zip(info["mips"], levels, decoded_mips, strict=True)):
        squared, count, maximum = 0., 0, 0.
        coverage = {float(cutoff): dict(reference_covered=0, decoded_covered=0, changed=0)
                    for cutoff in cutoffs}
        for start in range(0, len(original), 32):
            a = original[start:start+32].astype(np.float32)/255
            b = decoded[start:start+32].astype(np.float32)/255
            if role == "normal":
                def unit(values, two_component=False):
                    xy = values[..., :2]*2-1
                    n = np.dstack((xy, np.sqrt(np.maximum(1-(xy*xy).sum(axis=2), 0)))) if two_component else values[..., :3]*2-1
                    return n/np.maximum(np.linalg.norm(n, axis=2, keepdims=True), 1e-6)
                error = np.rad2deg(np.arccos(np.clip((unit(a)*unit(b, info['dxgi'] in (49, 83))).sum(axis=2), -1, 1)))
            else:
                channels = sampled_channels(role)
                error = np.abs(a[..., channels]-b[..., channels])
            squared += float(np.square(error, dtype=np.float64).sum())
            count += error.size
            maximum = max(maximum, float(error.max()))
            for cutoff, result in coverage.items():
                reference, candidate = a[..., 3] >= cutoff, b[..., 3] >= cutoff
                result["reference_covered"] += int(reference.sum())
                result["decoded_covered"] += int(candidate.sum())
                result["changed"] += int(np.count_nonzero(reference != candidate))
        rms = math.sqrt(squared/count)
        retained = legacy_start is not None and index >= legacy_start
        numerical_passed = rms <= rms_limit and maximum <= max_limit
        coverage_passed = all(value["changed"] == 0 for value in coverage.values())
        passed = numerical_passed and coverage_passed
        rows.append(dict(level=index, rms=rms, maximum=maximum, samples=count,
                         retained_source_mip=retained, generated_limits_passed=passed,
                         alpha_coverage=[dict(cutoff=cutoff, **value) for cutoff, value in coverage.items()]))
        if not retained and not numerical_passed and role != "response":
            raise ValueError(f"{role} mip {index} block compression failed quality/coverage limits: "
                             f"rms={rms:.6f}, max={maximum:.6f}, alpha={coverage}")
        if not retained and not coverage_passed:
            raise CoverageError(index, f"{role} mip {index} block compression changed mask classification: {coverage}")
    return dict(output_sha256=info["sha256"],
                units="degrees" if role == "normal" else "unit-channel",
                rms_limit=rms_limit, maximum_limit=max_limit, checked_mips=len(levels),
                sampled_channels=list(sampled_channels(role)),
                numerical_policy="visual-review" if role == "response" else "strict",
                generated_mips=sum(not row["retained_source_mip"] for row in rows),
                retained_source_mips=sum(row["retained_source_mip"] for row in rows),
                mips=rows)


class NativeTextureCache:
    def __init__(self, root, executable, resources=None):
        try:
            from .prepare_ktx2 import backend_identity
        except ImportError:
            from prepare_ktx2 import backend_identity
        self.root, self.executable = root.resolve(), executable.resolve()
        if self.root.is_relative_to(Path(__file__).resolve().parents[1]):
            raise ValueError("Native derived-data caches must stay outside the repository")
        self.resources = resources
        self.backend = backend_identity(self.executable)
        self.root.mkdir(parents=True, exist_ok=True)

    def load_prepared(self, key, entry):
        path = self.root/"pending"/(key+".dds")
        receipt = path.with_suffix(".json")
        if not path.exists() or not receipt.exists():
            return None
        record, raw = json.loads(receipt.read_text()), path.read_bytes()
        info = inspect_dds(raw)
        if record.get("key") != key or record.get("uri") != entry["uri"] or \
                record.get("sha256") != info["sha256"] or \
                record.get("mip_sha256") != [mip["sha256"] for mip in info["mips"]] or \
                info["dxgi"] != (29 if entry["color_space"] == "srgb" else 28) or \
                (info["width"], info["height"]) != (entry["width"]*entry["scale"], entry["height"]*entry["scale"]) or \
                info["mip_count"] != max(info["width"], info["height"]).bit_length():
            raise ValueError("Pending semantic master differs from its source/recipe/extent receipt")
        return rgba_levels(raw)

    def save_prepared(self, key, levels, entry):
        path = self.root/"pending"/(key+".dds")
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(".tmp")
        write_master(temporary, levels, entry["color_space"] == "srgb")
        raw = temporary.read_bytes()
        info = inspect_dds(raw)
        temporary.replace(path)
        record = dict(key=key, sha256=info["sha256"], uri=entry["uri"],
                      mip_sha256=[mip["sha256"] for mip in info["mips"]])
        receipt = path.with_suffix(".json")
        temporary = receipt.with_suffix(".tmp")
        temporary.write_text(json.dumps(record, indent=2)+"\n")
        temporary.replace(receipt)

    def release_prepared(self, key):
        for suffix in (".dds", ".json"):
            (self.root/"pending"/(key+suffix)).unlink(missing_ok=True)

    def find(self, key):
        receipt = self.root/key[:2]/(key+".json")
        if not receipt.exists():
            return None
        record = json.loads(receipt.read_text())
        path = Path(record["candidate_path"])
        if record.get("cache_key") != key or record.get("backend") != self.backend or not path.is_file():
            raise ValueError("Native texture cache recipe or executable identity changed")
        info = inspect_texture(path.read_bytes())
        if path.suffix != ".ktx2" or info["sha256"] != record["output_sha256"]:
            raise ValueError("Native texture cache output differs from its qualification receipt")
        return path, record

    def store(self, key, levels, entry, source_raw, audit=True, prepared_key=None):
        try:
            from .prepare_ktx2 import prepare_levels, atomic_json
        except ImportError:
            from prepare_ktx2 import prepare_levels, atomic_json
        prefix = int(math.log2(entry["scale"]))
        path, result = prepare_levels(levels, entry, self.executable, self.root/"native", source_raw, prefix)
        info = result["output"]
        result["quality"]["audited"] = True
        record = dict(cache_key=key, candidate_path=str(path), output_sha256=info["sha256"],
                      format=info["format"], backend=self.backend, quality=result["quality"],
                      source_mips_retained=True, source_mips_recompressed=False,
                      authored_mips=result["authored_mips"],
                      authored_preservation_checked=result.get('authored_preservation_checked', False),
                      retained_mip_sha256=[m["sha256"] for m in info["mips"][prefix:]],
                      master_mip_sha256=[hashlib.sha256(memoryview(level).cast("B")).hexdigest() for level in levels[prefix:]],
                      mask_compression=mask_recipe(entry), preparation_recipe=result["recipe"])
        atomic_json(self.root/key[:2]/(key+".json"), record)
        if prepared_key is not None:
            self.release_prepared(prepared_key)
        return path, record


def materialize(cached, output):
    if cached.suffix.lower() != ".ktx2" or output.suffix.lower() != ".ktx2":
        raise ValueError("Runtime texture materialization requires KTX2 paths")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(".tmp")
    temporary.unlink(missing_ok=True)
    try:
        temporary.hardlink_to(cached)
    except OSError:
        shutil.copyfile(cached, temporary)
    temporary.replace(output)
