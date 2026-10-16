"""Restore native RGBA channels and complete mip chains from the pinned source SDK."""
import argparse
import hashlib
import io
import json
import struct
import sys
from pathlib import Path


SOURCE_ZONES = ("code_post_gfx_zm", "common_zm", "patch_zm", "zm_tomb", "zm_tomb_patch")


def source_walks(zone, current=None):
    from scripts.cod import backends

    zone = Path(zone)
    for name in SOURCE_ZONES:
        path = zone.with_name(name + ".ff")
        walk = current if current is not None and path == zone else backends.for_fastfile(str(path)).walk(str(path))
        walk.check()
        yield walk


def _dds(fmt, width, height, faces, mips, pixels):
    from scripts.cod.iw6.images import chain_size, dds

    if len(pixels) != chain_size(fmt, width, height, mips) * faces:
        raise ValueError(f"Source DXGI {fmt} {width}x{height}, {mips} mips/{faces} faces has an invalid payload size")
    output = bytearray(dds(fmt, width, height, faces, b""))
    if mips > 1:
        flags, = struct.unpack_from("<I", output, 8)
        caps, = struct.unpack_from("<I", output, 108)
        struct.pack_into("<I", output, 8, flags | 0x20000)
        struct.pack_into("<I", output, 28, mips)
        struct.pack_into("<I", output, 108, caps | 0x400008)
    return bytes(output) + pixels


def _dds_info(data):
    if len(data) < 128 or data[:4] != b"DDS ":
        raise ValueError("Invalid DDS header")
    height, width = struct.unpack_from("<2I", data, 12)
    mips = max(1, struct.unpack_from("<I", data, 28)[0])
    faces = 6 if struct.unpack_from("<I", data, 112)[0] & 0x200 else 1
    dx10 = data[84:88] == b"DX10"
    header = 148 if dx10 else 128
    if len(data) < header:
        raise ValueError("Incomplete DDS DX10 header")
    fmt = struct.unpack_from("<I", data, 128)[0] if dx10 else None
    masks = struct.unpack_from("<4I", data, 92) if not dx10 else None
    if masks == (255, 65280, 16711680, 4278190080):
        fmt = 28
    return {"width": width, "height": height, "mips": mips, "faces": faces,
            "format": fmt, "header_bytes": header}, data[header:]


def _sha(data):
    return hashlib.sha256(data).hexdigest()


def _write(path, data, write):
    changed = not path.is_file() or path.read_bytes() != data
    if write and changed:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    if write and path.read_bytes() != data:
        raise ValueError(f"Staged bytes differ from the source: {path}")
    return changed


def _sources(walks, assets):
    from scripts.cod.t6 import ipak
    from scripts.cod.t6.dumpers.image import LOADDEF, NOMIPMAPS, CUBEMAP, VOLMAP

    # A later concrete zone definition owns the name, including when it changes
    # format. Reference-only records never override an earlier definition.
    selected, unavailable, zones = {}, {}, []
    for walk in walks:
        zones.append(str(walk.path))
        for asset in walk.by_type("image"):
            if asset.name.startswith(","):
                continue
            record = asset.record
            raw = record.captured("LoadImageData")
            head = LOADDEF.unpack_from(raw) if raw is not None and len(raw) >= LOADDEF.size else None
            fmt = head[2] if head else None
            streamed = bool(record["streamedPartCount"])
            destination = assets / "images" / (asset.name.replace("*", "_") + ".dds")
            selected[asset.name] = None
            unavailable.pop(asset.name, None)
            if fmt != 28 and (not streamed or destination.is_file()):
                continue
            item = {"name": asset.name, "source_zone": str(walk.path), "format": fmt,
                    "width": record["width"], "height": record["height"]}
            if head and head[3]:
                _, flags, fmt, size = head
                if flags & VOLMAP:
                    raise ValueError(f"Required raw RGBA image {asset.name} is a source volume")
                item.update(faces=6 if flags & CUBEMAP else 1,
                            mips=1 if flags & NOMIPMAPS else max(item["width"], item["height"]).bit_length(),
                            pixels=bytes(raw[LOADDEF.size:LOADDEF.size + size]), source="loaddef")
            elif streamed:
                item.update(name_hash=record["hash"], data_hash=record["streamedParts[0].hash"],
                            source="ipak")
            else:
                unavailable[asset.name] = {**item, "reason": "No source load-definition payload or streamed part"}
                continue
            selected[asset.name] = item

    packs_by_folder, images = {}, []
    for item in selected.values():
        if item is None:
            continue
        if item["source"] == "ipak":
            folder = Path(item["source_zone"]).parent
            if folder not in packs_by_folder:
                packs_by_folder[folder] = ipak.Packs(folder)
            packs = packs_by_folder[folder]
            source = packs.iwi(item["name_hash"], item["data_hash"])
            if source is None:
                raise ValueError(f"Required image {item['name']} is absent from the source packs")
            if (source.width, source.height) != (item["width"], item["height"]) or source.flags & ipak.VOLMAP:
                raise ValueError(f"Unexpected source dimensions/volume for {item['name']}")
            levels = [ipak.level(source, k) for k in range(ipak.levels(source))]
            fmt, width, height, faces, _ = levels[0]
            if item["format"] is not None and fmt != item["format"]:
                raise ValueError(f"Source zone/IWI format mismatch for {item['name']}: {item['format']} vs {fmt}")
            for k, level in enumerate(levels):
                if level[:4] != (fmt, max(1, width >> k), max(1, height >> k), faces) or len(level[4]) % faces:
                    raise ValueError(f"Unexpected source mip {k} for {item['name']}")
            # IWI levels are mip-major; DDS stores each face's whole chain.
            pixels = b"".join(level[4][face * (len(level[4]) // faces):(face + 1) * (len(level[4]) // faces)]
                              for face in range(faces) for level in levels)
            item.update(format=fmt, faces=faces, mips=len(levels), pixels=pixels,
                        iwi_format=source.fmt, pack=str(packs.index[(item["name_hash"], item["data_hash"])].path))
        item["dds"] = _dds(item["format"], item["width"], item["height"], item["faces"], item["mips"], item["pixels"])
        images.append(item)
    return images, zones, list(unavailable.values())


def sync_model_aliases(assets, names, *, write=True):
    assets = Path(assets)
    canonical = {}
    for name in names:
        stem = name.replace("*", "_")
        for extension in (".dds", ".png"):
            path = assets / "images" / (stem + extension)
            if path.is_file():
                canonical[path.name] = path
    aliases = []
    for path in (assets / "model_export").rglob("*"):
        if path.is_file() and path.name in canonical:
            data = canonical[path.name].read_bytes()
            aliases.append({"path": str(path.relative_to(assets)), "changed": _write(path, data, write),
                            "sha256": _sha(data)})
    return aliases


def stage_source_images(walks, assets, *, write=True):
    from PIL import Image

    assets = Path(assets)
    sources, zones, unavailable = _sources(walks, assets)
    images = []
    for source in sources:
        path = assets / "images" / (source["name"].replace("*", "_") + ".dds")
        before = path.read_bytes() if path.is_file() else None
        before_info, before_pixels = _dds_info(before) if before is not None else (None, None)
        entry = {key: value for key, value in source.items() if key not in ("pixels", "dds")}
        entry.update(payload_bytes=len(source["pixels"]), source_payload_sha256=_sha(source["pixels"]),
                     before=before_info, before_payload_equal=before_pixels == source["pixels"],
                     before_dds_sha256=_sha(before) if before is not None else None,
                     dds_changed=_write(path, source["dds"], write))
        result = path.read_bytes() if path.is_file() else None
        info, pixels = _dds_info(result) if result is not None else (None, None)
        verified = (info is not None
                    and all(info[key] == source[key] for key in ("width", "height", "mips", "faces"))
                    and (source["format"] != 28 or info["format"] == 28)
                    and pixels == source["pixels"])
        if write and not verified:
            raise ValueError(f"Source dimensions/mips/faces/payload differ after staging {source['name']}")
        entry.update(after=info, payload_verified=verified,
                     dds_sha256=_sha(result) if result is not None else None, png_changed=False)
        if source["format"] == 28:
            # PNG is the first face's finest level; DDS retains every face/mip.
            png = io.BytesIO()
            Image.frombytes("RGBA", (source["width"], source["height"]),
                            source["pixels"][:source["width"] * source["height"] * 4]).save(png, format="PNG")
            png_path = path.with_suffix(".png")
            entry["png_changed"] = _write(png_path, png.getvalue(), write)
            png_verified = False
            if png_path.is_file():
                with Image.open(png_path) as decoded:
                    png_verified = (decoded.mode == "RGBA"
                                    and decoded.size == (source["width"], source["height"])
                                    and decoded.tobytes() == source["pixels"][:source["width"] * source["height"] * 4])
            if write and not png_verified:
                raise ValueError(f"PNG changed source RGBA pixels for {source['name']}")
            entry["png_pixels_verified"] = png_verified
        images.append(entry)
    aliases = sync_model_aliases(assets, [item["name"] for item in sources], write=write)
    return {"mode": "restage" if write else "audit", "source_zones": zones,
            "images": images, "aliases": aliases, "unavailable": unavailable,
            "counts": {"images": len(images), "rgba": sum(item["format"] == 28 for item in images),
                       "dds_changed": sum(item["dds_changed"] for item in images),
                       "png_changed": sum(item["png_changed"] for item in images),
                       "aliases_changed": sum(item["changed"] for item in aliases)}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tool", type=Path)
    parser.add_argument("--zone", type=Path)
    parser.add_argument("--assets", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--audit", action="store_true")
    mode.add_argument("--sync-aliases", action="store_true")
    args = parser.parse_args()
    if args.sync_aliases:
        report = json.loads(args.report.read_text(encoding="utf-8"))
        report["final_alias_sync"] = sync_model_aliases(args.assets, [item["name"] for item in report["images"]])
        summary = {"aliases": len(report["final_alias_sync"]),
                   "aliases_changed": sum(item["changed"] for item in report["final_alias_sync"])}
    else:
        if args.tool is None or args.zone is None:
            parser.error("--tool and --zone are required for source staging")
        sys.path.insert(0, str(args.tool.resolve()))
        report = stage_source_images(source_walks(args.zone), args.assets, write=not args.audit)
        summary = report["counts"]
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
