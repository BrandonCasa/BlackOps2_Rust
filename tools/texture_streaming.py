"""Assemble standard DDS bootstrap tails and a validated range-streaming index."""
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import struct

try:
    from .material_textures import inspect_dds
except ImportError:
    from material_textures import inspect_dds


def mip_ranges(raw):
    record = inspect_dds(raw)
    cursor = 148 if raw[84:88] == b"DX10" else 128
    for mip in record["mips"]:
        mip["offset"] = cursor
        cursor += mip["bytes"]
    return record


def tail(raw, record, first):
    """Retain bytes verbatim, updating only the standard DDS dimensions."""
    if not 0 <= first < record["mip_count"]:
        raise ValueError("Invalid first resident mip")
    start = record["mips"][first]
    header_bytes = 148 if raw[84:88] == b"DX10" else 128
    header = bytearray(raw[:header_bytes])
    struct.pack_into("<II", header, 12, start["height"], start["width"])
    struct.pack_into("<I", header, 28, record["mip_count"] - first)
    # DDSD_PITCH and DDSD_LINEARSIZE use different meanings.
    flags = struct.unpack_from("<I", header, 8)[0]
    if flags & 0x80000:
        struct.pack_into("<I", header, 20, start["bytes"])
    elif flags & 8:
        struct.pack_into("<I", header, 20, start["bytes"] // start["height"])
    return bytes(header) + raw[start["offset"]:]


def assemble(pack, textures, bootstrap_extent=128, provenance=None, resources=None, meshes=None, density_policy=None):
    destination = pack / "textures/streaming"
    destination.mkdir(parents=True, exist_ok=True)
    entries = []
    full_sources = {}
    for texture in textures:
        if resources is not None:
            resources.wait()
        path = pack / texture["uri"]
        raw = path.read_bytes()
        record = mip_ranges(raw)
        first = next((m["level"] for m in record["mips"]
                      if max(m["width"], m["height"]) <= bootstrap_extent), record["mip_count"] - 1)
        full = destination / path.name
        if full in full_sources and full_sources[full] != record["sha256"]:
            raise ValueError("Distinct streaming sources share a filename")
        full_sources[full] = record["sha256"]
        # Move the immutable cooked file rather than duplicating an entire BC
        # chain. Candidate/cache hard links remain immutable; the bootstrap is
        # a newly created, independent file at the original URI.
        path.replace(full)
        bootstrap = tail(raw, record, first)
        path.write_bytes(bootstrap)
        entries.append(dict(uri=texture["uri"], full_uri=full.relative_to(pack).as_posix(),
            width=record["width"], height=record["height"], format=record["dxgi"],
            bootstrap_mip=first, header_bytes=148 if raw[84:88] == b"DX10" else 128,
            bytes=len(raw), sha256=record["sha256"],
            bootstrap_sha256=hashlib.sha256(bootstrap).hexdigest(),
            color_space=texture["color_space"],
            texels_per_meter=(texture.get("minimum_axis_texels_per_meter", texture.get("minimum_texels_per_meter")) or 0.) * texture.get("scale", 1),
            protected_closeup=texture.get("protected_closeup", False),
            mips=[{key: mip[key] for key in ("level", "width", "height", "offset", "bytes", "sha256")}
                  for mip in record["mips"]]))
    document = dict(version=2 if meshes is not None else 1, textures=entries, models=provenance or {})
    if meshes is not None:
        scales = {entry["uri"]: entry.get("scale", 1) for entry in textures}
        document["meshes"] = {key: [dict(region, textures={uri: density * scales[uri]
            for uri, density in region["textures"].items()}) for region in regions]
            for key, regions in meshes.items()}
    if density_policy is not None:
        document["density_policy"] = density_policy
    (pack / "texture-streaming.json").write_text(json.dumps(document, separators=(",", ":")) + "\n")
    return document


def validate(pack, resources=None):
    """Importer hook: verify the complete closure, headers and every byte range."""
    index = pack / "texture-streaming.json"
    if not index.exists():
        return {}
    document = json.loads(index.read_text())
    if document.get("version") not in (1, 2) or "textures" not in document or set(document) - {"version", "textures", "models", "meshes", "density_policy"}:
        raise ValueError("Unsupported texture streaming index")
    if document["version"] == 2 and not isinstance(document.get("meshes"), dict):
        raise ValueError("Streaming version 2 requires local surface regions")
    hashes = {index.relative_to(pack).as_posix(): hashlib.sha256(index.read_bytes()).hexdigest()}
    seen = set()
    for item in document["textures"]:
        if resources is not None:
            resources.wait()
        if item["uri"] in seen:
            raise ValueError("Duplicate streaming texture")
        seen.add(item["uri"])
        for key in ("uri", "full_uri"):
            uri = PurePosixPath(item[key])
            if uri.is_absolute() or ".." in uri.parts or uri.suffix != ".dds" or "\\" in item[key] or ":" in item[key]:
                raise ValueError("Invalid streaming DDS dependency")
            path = (pack / item[key]).resolve()
            if not path.is_relative_to(pack.resolve()):
                raise ValueError("Streaming dependency escapes pack")
        raw = (pack / item["full_uri"]).read_bytes()
        record = mip_ranges(raw)
        if record["mip_count"] != max(record["width"], record["height"]).bit_length():
            raise ValueError("Streaming requires a complete mip chain")
        if not 0 <= item["bootstrap_mip"] < record["mip_count"]:
            raise ValueError("Invalid bootstrap mip range")
        if not math.isfinite(item["texels_per_meter"]) or item["texels_per_meter"] < 0:
            raise ValueError("Invalid physical streaming density")
        if item["color_space"] not in ("srgb", "linear"):
            raise ValueError("Invalid streaming color space")
        if len(raw) != item["bytes"] or record["sha256"] != item["sha256"]:
            raise ValueError("Streaming DDS checksum mismatch")
        if (record["width"], record["height"], record["dxgi"]) != (
                item["width"], item["height"], item["format"]):
            raise ValueError("Streaming DDS descriptor mismatch")
        expected = [{key: mip[key] for key in ("level", "width", "height", "offset", "bytes", "sha256")}
                    for mip in record["mips"]]
        if expected != item["mips"]:
            raise ValueError("Streaming DDS ranges do not cover its mip chain")
        initial = (pack / item["uri"]).read_bytes()
        if initial != tail(raw, record, item["bootstrap_mip"]):
            raise ValueError("Bootstrap texture differs from full DDS tail")
        if hashlib.sha256(initial).hexdigest() != item["bootstrap_sha256"]:
            raise ValueError("Bootstrap DDS checksum mismatch")
        hashes[item["uri"]] = item["bootstrap_sha256"]
        hashes[item["full_uri"]] = item["sha256"]
    for mesh, regions in document.get("meshes", {}).items():
        scene, marker, label = mesh.partition("#")
        if not marker or not label.startswith("Mesh") or "/Primitive" not in label or \
                not (pack/scene).resolve().is_relative_to(pack.resolve()) or not (pack/scene).is_file():
            raise ValueError("Invalid streaming mesh dependency")
        for region in regions:
            if set(region) != {"center", "half_extents", "textures"} or \
                    len(region["center"]) != 3 or len(region["half_extents"]) != 3 or \
                    any(not math.isfinite(v) for v in region["center"] + region["half_extents"]) or \
                    any(v < 0 for v in region["half_extents"]):
                raise ValueError("Invalid streaming surface bounds")
            if any(uri not in seen or not math.isfinite(value) or value < 0
                   for uri, value in region["textures"].items()):
                raise ValueError("Invalid local streaming density")
    policy = document.get("density_policy")
    if policy is not None:
        if set(policy) != {"target", "metric", "cap_to_device", "pending_textures", "results"} or \
                not math.isfinite(policy["target"]) or policy["target"] <= 0 or \
                policy["metric"] not in ("area", "minimum-axis") or type(policy["cap_to_device"]) is not bool:
            raise ValueError("Invalid streaming density policy")
        pending = set(policy["pending_textures"])
        results = set()
        by_uri = {entry["uri"]: entry for entry in document["textures"]}
        for result in policy["results"]:
            uri = result["uri"]
            if uri in results or uri not in seen or uri in pending:
                raise ValueError("Duplicate or unknown density result")
            results.add(uri)
            density = result["density"]
            if density is not None and (not math.isfinite(density) or density < 0):
                raise ValueError("Invalid achieved density")
            if result["applicable"] and (density is None or density < policy["target"]):
                if not result["exception"] or not policy["cap_to_device"] or \
                        max(by_uri[uri]["width"], by_uri[uri]["height"]) != 16384:
                    raise ValueError("Unreported density shortfall")
            elif result["exception"]:
                raise ValueError("Spurious density exception")
        if results | pending != seen or len(pending) != len(policy["pending_textures"]):
            raise ValueError("Density policy does not cover every streamed map")
    return hashes
