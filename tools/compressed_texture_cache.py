"""Build-time BCn cache with one recoverable failed semantic candidate."""
import hashlib
import io
import json
import math
from contextlib import contextmanager
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time

import numpy as np
from PIL import Image

from material_textures import inspect_dds, rgba_dds, rgba_levels

ENCODER_SHA256 = "dcfdec10244e02cf5037fba089c55fb7e1326b1c8181742d77d15fa5cb5eef06"
FORMATS = {"BC7_UNORM_SRGB": 99, "BC7_UNORM": 98, "BC5_UNORM": 83, "BC4_UNORM": 80,
           "R8G8B8A8_UNORM_SRGB": 29, "R8G8B8A8_UNORM": 28}
MASK_MARGINS = (0, 2, 4, 8, 16)


def mask_recipe(entry):
    return (dict(version=1, alpha_weight=4., margin_attempts=list(MASK_MARGINS))
            if entry.get("mask_cutoffs") else None)


class CoverageError(ValueError):
    def __init__(self, level, message):
        super().__init__(message)
        self.level = level


def quality_limits(role):
    return (2., 30.) if role == "normal" else (.025, .25)


def sampled_channels(role):
    # Prepared material declarations and native raster/ray consumers agree.
    return {"control": (0, 1), "coating": (0,), "emissive": (0, 1, 2)}.get(role, (0, 1, 2, 3))


@contextmanager
def open_validated_dds(raw):
    info = inspect_dds(raw)
    if max(info["width"], info["height"]) > 16384:
        raise ValueError("DDS decode exceeds the approved renderer extent")
    previous = Image.MAX_IMAGE_PIXELS
    try:
        # Raise only for this verified extent; keep a finite ceiling and restore
        # Pillow's prior guard even when decoding or qualification fails.
        Image.MAX_IMAGE_PIXELS = max(previous or 0, info["width"]*info["height"])
        with Image.open(io.BytesIO(raw)) as image:
            yield image
    finally:
        Image.MAX_IMAGE_PIXELS = previous


def target_format(entry):
    # Explicit preparation choice for a bounded reauthored material set whose
    # packed response exceeds the BC7 error ceiling. Never silently fall back.
    if entry.get('lossless', False):
        return "R8G8B8A8_UNORM_SRGB" if entry["color_space"] == "srgb" else "R8G8B8A8_UNORM"
    # A constant 1x1 field occupies four bytes; BC requires a sixteen/eight-byte
    # block and a block-aligned resource extent. Keep tiny fields explicit.
    width, height = entry["width"]*entry["scale"], entry["height"]*entry["scale"]
    if width % 4 or height % 4 or not entry.get("density_applicable", True):
        return "R8G8B8A8_UNORM_SRGB" if entry["color_space"] == "srgb" else "R8G8B8A8_UNORM"
    if entry["role"] == "normal":
        return "BC5_UNORM"
    if entry["role"] == "coating":
        return "BC4_UNORM"
    return "BC7_UNORM_SRGB" if entry["color_space"] == "srgb" else "BC7_UNORM"


def stored_bytes(entry):
    width, height = entry["width"]*entry["scale"], entry["height"]*entry["scale"]
    fmt = target_format(entry)
    total = 148
    while True:
        total += (math.ceil(width/4)*math.ceil(height/4)*(8 if fmt == "BC4_UNORM" else 16)
                  if fmt.startswith("BC") else width*height*4)
        if width == height == 1:
            return total
        width, height = max(1, width//2), max(1, height//2)


def write_master(path, levels, srgb, cutoffs=(), margins=None, generated_count=None, channels=None):
    # Avoid rgba_dds's full-chain join: a 16K chain already owns 1.33 GiB.
    header = bytearray(rgba_dds([levels[-1]], srgb)[:148])
    height, width = levels[0].shape[:2]
    struct.pack_into("<III", header, 12, height, width, width*4)
    struct.pack_into("<I", header, 28, len(levels))
    struct.pack_into("<I", header, 108, 0x401008 if len(levels) > 1 else 0x1000)
    with path.open("wb") as stream:
        stream.write(header)
        for index, level in enumerate(levels):
            margin = (margins or {}).get(index, 0)
            constrained = margin and cutoffs and (generated_count is None or index < generated_count)
            unused = set(range(4))-set(channels) if channels is not None else set()
            if not constrained and not unused:
                stream.write(memoryview(np.ascontiguousarray(level)).cast("B"))
                continue
            # Constrain the encoder near actual surface cutoffs. Keep each
            # reference pixel on its original side of every cutoff, and bound
            # scratch to rows. Final quality still compares UNCHANGED levels.
            for start in range(0, len(level), 32):
                rows = level[start:start+32].copy()
                for channel in unused:
                    rows[..., channel] = 255 if channel == 3 else 0
                if not constrained:
                    stream.write(memoryview(rows).cast("B"))
                    continue
                reference = rows[..., 3].astype(np.int16)
                alpha = reference.copy()
                low, high = np.zeros_like(alpha), np.full_like(alpha, 255)
                for cutoff in cutoffs:
                    threshold = math.ceil(cutoff*255)
                    below = reference < threshold
                    alpha = np.where(below, np.minimum(alpha, threshold-1-margin),
                                     np.maximum(alpha, threshold+margin))
                    low = np.where(below, low, np.maximum(low, threshold))
                    high = np.where(below, np.minimum(high, threshold-1), high)
                rows[..., 3] = np.clip(alpha, low, high).astype(np.uint8)
                stream.write(memoryview(rows).cast("B"))


def qualify(raw, levels, role, legacy_start=None, cutoffs=()):
    """Compare each compressed mip with its semantic master in row chunks."""
    info = inspect_dds(raw)
    cursor, rows = 148, []
    rms_limit, max_limit = quality_limits(role)
    for index, (mip, original) in enumerate(zip(info["mips"], levels, strict=True)):
        squared, count, maximum = 0., 0, 0.
        coverage = {float(cutoff): dict(reference_covered=0, decoded_covered=0, changed=0)
                    for cutoff in cutoffs}
        header = bytearray(raw[:148])
        struct.pack_into("<II", header, 12, mip["height"], mip["width"])
        struct.pack_into("<I", header, 28, 1)
        with open_validated_dds(header + raw[cursor:cursor+mip["bytes"]]) as image:
            decoded = np.asarray(image.convert("RGBA"))
        cursor += mip["bytes"]
        for start in range(0, len(original), 32):
            a = original[start:start+32].astype(np.float32)/255
            b = decoded[start:start+32].astype(np.float32)/255
            if role == "normal":
                def unit(values):
                    xy = values[..., :2]*2-1
                    n = np.dstack((xy, np.sqrt(np.maximum(1-(xy*xy).sum(axis=2), 0))))
                    return n/np.maximum(np.linalg.norm(n, axis=2, keepdims=True), 1e-6)
                error = np.rad2deg(np.arccos(np.clip((unit(a)*unit(b)).sum(axis=2), -1, 1)))
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
        # Retained source bytes cannot acquire a new codec error. Record their
        # master differences separately; reject generated/recompressed errors.
        if not retained and not numerical_passed:
            raise ValueError(f"{role} mip {index} block compression failed quality/coverage limits: "
                             f"rms={rms:.6f}, max={maximum:.6f}, alpha={coverage}")
        if not retained and not coverage_passed:
            raise CoverageError(index, f"{role} mip {index} block compression changed mask classification: {coverage}")
    return dict(output_sha256=info["sha256"],
                units="degrees" if role == "normal" else "unit-channel",
                rms_limit=rms_limit, maximum_limit=max_limit, checked_mips=len(levels),
                sampled_channels=list(sampled_channels(role)),
                generated_mips=sum(not row["retained_source_mip"] for row in rows),
                retained_source_mips=sum(row["retained_source_mip"] for row in rows),
                mips=rows)


class CompressedTextureCache:
    def __init__(self, root, encoder, device="cuda", resources=None):
        self.root, self.encoder = root.resolve(), encoder.resolve()
        self.device = device
        self.resources = resources
        if hashlib.sha256(encoder.read_bytes()).hexdigest() != ENCODER_SHA256:
            raise ValueError("DirectXTex encoder differs from the pinned may2026 executable")
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
        # Only this task-owned keyed intermediate is removed after acceptance.
        for suffix in (".dds", ".json", ".encoded.dds"):
            (self.root/"pending"/(key+suffix)).unlink(missing_ok=True)

    def find(self, key):
        path = self.root/key[:2]/(key+".dds")
        receipt = path.with_suffix(".json")
        if path.exists() and receipt.exists():
            record = json.loads(receipt.read_text())
            raw = path.read_bytes()
            if record.get("cache_key") == key and hashlib.sha256(raw).hexdigest() == record.get("output_sha256"):
                inspect_dds(raw)
                return path, record
        return None

    def store(self, key, levels, entry, source_raw, audit=True, prepared_key=None):
        fmt = target_format(entry)
        path = self.root/key[:2]/(key+".dds")
        path.parent.mkdir(parents=True, exist_ok=True)
        original = inspect_dds(source_raw)
        prefix = int(math.log2(entry["scale"]))
        retained = original["dxgi"] == FORMATS[fmt]
        cutoffs = entry.get("mask_cutoffs", [])
        with tempfile.TemporaryDirectory(prefix="cook-", dir=self.root) as scratch:
            scratch = Path(scratch)
            master = scratch/"master.dds"
            output = scratch/"compressed"
            output.mkdir()

            def encode(margins):
                write_master(master, levels, entry["color_space"] == "srgb", cutoffs, margins,
                             prefix if retained else None, sampled_channels(entry["role"]))
                if not fmt.startswith("BC"):
                    return master.read_bytes()
                args = [str(self.encoder), "-nologo", "-y", "-dx10", "-sepalpha",
                        "-f", fmt, "-o", str(output), str(master)]
                args[1:1] = ["-gpu", "0"] if self.device == "cuda" else ["-nogpu"]
                if fmt.startswith("BC7"):
                    args[1:1] = ["-bc", "x"]
                    if cutoffs:
                        args[1:1] = ["-aw", "4"]
                with subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True) as process:
                    while process.poll() is None:
                        if self.resources is not None and self.resources.refresh(force=True) and self.resources.shared_slot_busy:
                            process.terminate()
                            try:
                                process.wait(timeout=5)
                            except subprocess.TimeoutExpired:
                                process.kill()
                                process.wait()
                            self.resources.wait()
                        time.sleep(.2)
                    log = process.stdout.read()
                    if process.returncode:
                        raise RuntimeError(log)
                return (output/master.name).read_bytes()

            margins = {}
            while True:
                raw = encode(margins)
                info = inspect_dds(raw)
                if (info["width"], info["height"], info["mip_count"], info["dxgi"]) != (
                        entry["width"]*entry["scale"], entry["height"]*entry["scale"], len(levels), FORMATS[fmt]):
                    raise ValueError("Compression changed the material extent, transfer or mip chain")
                # Preserve exact authored compressed tails before qualification.
                if retained:
                    cut = 148+sum(m["bytes"] for m in info["mips"][:prefix])
                    raw = raw[:cut]+source_raw[148:]
                    info = inspect_dds(raw)
                    if [m["sha256"] for m in info["mips"][prefix:]] != [m["sha256"] for m in original["mips"]]:
                        raise ValueError("Authored compressed mip tail was not retained byte-identically")
                if not audit:
                    quality = dict(output_sha256=info["sha256"], audited=False,
                                   policy="representative-material-types", checked_mips=0)
                    break
                if prepared_key is not None:
                    # A rejected codec can be inspected/replaced without
                    # discarding expensive, unchanged neural semantic levels.
                    (self.root/"pending"/(prepared_key+".encoded.dds")).write_bytes(raw)
                try:
                    quality = qualify(raw, levels, entry["role"], prefix if retained else None, cutoffs)
                    quality["audited"] = True
                    break
                except CoverageError as error:
                    previous = margins.get(error.level, 0)
                    if previous == MASK_MARGINS[-1] or not fmt.startswith("BC7"):
                        raise
                    margins[error.level] = MASK_MARGINS[MASK_MARGINS.index(previous)+1]
                    print(f"Mask mip {error.level} retries encoder margin {margins[error.level]}/255; "
                          "semantic reference remains unchanged", flush=True)
            record = dict(cache_key=key, output_sha256=info["sha256"], format=fmt,
                compression_device=self.device,
                encoder_sha256=ENCODER_SHA256, compressed=fmt.startswith("BC"), quality=quality,
                source_mips_retained=retained, source_mips_recompressed=not retained,
                retained_mip_sha256=[m["sha256"] for m in info["mips"][prefix:]],
                master_mip_sha256=[hashlib.sha256(memoryview(level).cast("B")).hexdigest() for level in levels[prefix:]])
            if cutoffs:
                record["mask_compression"] = mask_recipe(entry)
                record["mask_encoder_margins"] = margins
            temporary = path.with_suffix(".tmp")
            temporary.write_bytes(raw)
            temporary.replace(path)
            receipt = path.with_suffix(".json")
            temporary = receipt.with_suffix(".tmp")
            temporary.write_text(json.dumps(record, indent=2)+"\n")
            temporary.replace(receipt)
            if prepared_key is not None:
                self.release_prepared(prepared_key)
        return path, record


def materialize(cached, output):
    temporary = output.with_suffix(".tmp")
    temporary.unlink(missing_ok=True)
    try:
        temporary.hardlink_to(cached)
    except OSError:
        shutil.copyfile(cached, temporary)
    temporary.replace(output)
