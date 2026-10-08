"""Build-time BCn derived data; never retain uncompressed neural candidates."""
import hashlib
import io
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time

import numpy as np
from PIL import Image

from material_textures import inspect_dds, rgba_dds

ENCODER_SHA256 = "dcfdec10244e02cf5037fba089c55fb7e1326b1c8181742d77d15fa5cb5eef06"
FORMATS = {"BC7_UNORM_SRGB": 99, "BC7_UNORM": 98, "BC5_UNORM": 83, "BC4_UNORM": 80,
           "R8G8B8A8_UNORM_SRGB": 29, "R8G8B8A8_UNORM": 28}


def target_format(entry):
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


def write_master(path, levels, srgb):
    # Avoid rgba_dds's full-chain join: a 16K chain already owns 1.33 GiB.
    header = bytearray(rgba_dds([levels[-1]], srgb)[:148])
    height, width = levels[0].shape[:2]
    struct.pack_into("<III", header, 12, height, width, width*4)
    struct.pack_into("<I", header, 28, len(levels))
    struct.pack_into("<I", header, 108, 0x401008 if len(levels) > 1 else 0x1000)
    with path.open("wb") as stream:
        stream.write(header)
        for level in levels:
            stream.write(memoryview(np.ascontiguousarray(level)).cast("B"))


def qualify(raw, levels, role):
    """Compare each compressed mip with its semantic master in row chunks."""
    info = inspect_dds(raw)
    cursor, squared, count, maximum = 148, 0., 0, 0.
    for mip, original in zip(info["mips"], levels, strict=True):
        header = bytearray(raw[:148])
        struct.pack_into("<II", header, 12, mip["height"], mip["width"])
        struct.pack_into("<I", header, 28, 1)
        with Image.open(io.BytesIO(header + raw[cursor:cursor+mip["bytes"]])) as image:
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
                error = np.abs(a[..., :1 if role == "coating" else 4]-b[..., :1 if role == "coating" else 4])
            squared += float(np.square(error, dtype=np.float64).sum())
            count += error.size
            maximum = max(maximum, float(error.max()))
    rms = math.sqrt(squared/count)
    # These are block-codec qualification ceilings, not equivalence claims.
    # Never silently fall back to massive RGBA8 files for failed BC candidates.
    rms_limit, max_limit = (2., 30.) if role == "normal" else (.025, .25)
    if rms > rms_limit or maximum > max_limit:
        raise ValueError(f"{role} block compression failed quality limits: rms={rms:.6f}, max={maximum:.6f}")
    return dict(rms=rms, maximum=maximum, units="degrees" if role == "normal" else "unit-channel",
                rms_limit=rms_limit, maximum_limit=max_limit, checked_mips=len(levels))


class CompressedTextureCache:
    def __init__(self, root, encoder, device="cuda", resources=None):
        self.root, self.encoder = root.resolve(), encoder.resolve()
        self.device = device
        self.resources = resources
        if hashlib.sha256(encoder.read_bytes()).hexdigest() != ENCODER_SHA256:
            raise ValueError("DirectXTex encoder differs from the pinned may2026 executable")
        self.root.mkdir(parents=True, exist_ok=True)

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

    def store(self, key, levels, entry, source_raw):
        fmt = target_format(entry)
        path = self.root/key[:2]/(key+".dds")
        path.parent.mkdir(parents=True, exist_ok=True)
        original = inspect_dds(source_raw)
        prefix = int(math.log2(entry["scale"]))
        with tempfile.TemporaryDirectory(prefix="cook-", dir=self.root) as scratch:
            scratch = Path(scratch)
            master = scratch/"master.dds"
            write_master(master, levels, entry["color_space"] == "srgb")
            if fmt.startswith("BC"):
                output = scratch/"compressed"
                output.mkdir()
                args = [str(self.encoder), "-nologo", "-y", "-dx10", "-sepalpha",
                        "-f", fmt, "-o", str(output), str(master)]
                args[1:1] = ["-gpu", "0"] if self.device == "cuda" else ["-nogpu"]
                if fmt.startswith("BC7"):
                    args[1:1] = ["-bc", "x"]
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
                raw = (output/master.name).read_bytes()
            else:
                raw = master.read_bytes()
            info = inspect_dds(raw)
            if (info["width"], info["height"], info["mip_count"], info["dxgi"]) != (
                    entry["width"]*entry["scale"], entry["height"]*entry["scale"], len(levels), FORMATS[fmt]):
                raise ValueError("Compression changed the material extent, transfer or mip chain")
            quality = qualify(raw, levels, entry["role"])
            # If the canonical source is already in our target format, append
            # its exact authored compressed tail instead of recompressing it.
            retained = original["dxgi"] == info["dxgi"]
            if retained:
                cut = 148+sum(m["bytes"] for m in info["mips"][:prefix])
                raw = raw[:cut]+source_raw[148:]
                info = inspect_dds(raw)
            record = dict(cache_key=key, output_sha256=info["sha256"], format=fmt,
                compression_device=self.device,
                encoder_sha256=ENCODER_SHA256, compressed=fmt.startswith("BC"), quality=quality,
                source_mips_retained=retained, source_mips_recompressed=not retained,
                retained_mip_sha256=[m["sha256"] for m in info["mips"][prefix:]],
                master_mip_sha256=[hashlib.sha256(memoryview(level).cast("B")).hexdigest() for level in levels[prefix:]])
            temporary = path.with_suffix(".tmp")
            temporary.write_bytes(raw)
            temporary.replace(path)
            receipt = path.with_suffix(".json")
            temporary = receipt.with_suffix(".tmp")
            temporary.write_text(json.dumps(record, indent=2)+"\n")
            temporary.replace(receipt)
        return path, record


def materialize(cached, output):
    temporary = output.with_suffix(".tmp")
    temporary.unlink(missing_ok=True)
    try:
        temporary.hardlink_to(cached)
    except OSError:
        shutil.copyfile(cached, temporary)
    temporary.replace(output)
