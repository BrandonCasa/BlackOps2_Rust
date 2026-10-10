import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import shutil

from material_textures import inspect_dds
from resident_textures import assemble
from preparation_resources import PreparationResources
from import_origins import validate_pack
from upscale_textures import DEEPBUMP_UPSCALE_SHA256, RECIPE_VERSION, source_levels
from compressed_texture_cache import ENCODER_SHA256, FORMATS, MASK_MARGINS, mask_recipe, materialize, quality_limits, sampled_channels, target_format


def verify_quality(entry, record, info, source_info):
    quality = record.get("quality", {})
    if quality.get("output_sha256") != info["sha256"]:
        raise ValueError("Candidate quality/integrity receipt refers to different bytes")
    retained = source_info["dxgi"] == info["dxgi"]
    if record.get("source_mips_retained") != retained or record.get("source_mips_recompressed") != (not retained):
        raise ValueError("Candidate source-tail classification differs from its actual codec")
    if record.get("mask_compression") != mask_recipe(entry):
        raise ValueError("Candidate mask encoder recipe differs from actual material cutoffs")
    if not quality.get("audited", True):
        if quality.get("policy") != "representative-material-types" or quality.get("checked_mips") != 0:
            raise ValueError("Unaudited candidate lacks the explicit representative-type policy")
        return False
    rows = quality.get("mips", [])
    prefix = int(math.log2(entry["scale"]))
    rms_limit, maximum_limit = quality_limits(entry["role"])
    if quality.get("output_sha256") != info["sha256"] or \
            quality.get("checked_mips") != info["mip_count"] or len(rows) != info["mip_count"] or \
            quality.get("rms_limit") != rms_limit or quality.get("maximum_limit") != maximum_limit or \
            quality.get("units") != ("degrees" if entry["role"] == "normal" else "unit-channel"):
        raise ValueError("Candidate quality receipt differs from its hashed chain or strict ceilings")
    legacy_count = info["mip_count"]-prefix if retained else 0
    if quality.get("retained_source_mips") != legacy_count or \
            quality.get("generated_mips") != info["mip_count"]-legacy_count:
        raise ValueError("Candidate quality receipt misclassifies generated/retained mips")
    cutoffs = entry.get("mask_cutoffs", [])
    for index, (row, mip) in enumerate(zip(rows, info["mips"], strict=True)):
        legacy = retained and index >= prefix
        if row.get("level") != index or row.get("retained_source_mip") != legacy:
            raise ValueError("Candidate quality receipt has incorrect mip ownership")
        values = (row.get("rms"), row.get("maximum"))
        if any(not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0 for value in values):
            raise ValueError("Candidate quality error must be finite and nonnegative")
        pixels = mip["width"]*mip["height"]
        components = 1 if entry["role"] == "normal" else len(sampled_channels(entry["role"]))
        if row.get("samples") != pixels*components:
            raise ValueError("Candidate quality receipt omits semantic channel samples")
        coverage = row.get("alpha_coverage", [])
        if [result.get("cutoff") for result in coverage] != cutoffs:
            raise ValueError("Candidate quality receipt omits an actual material cutoff")
        for result in coverage:
            counts = [result.get(key) for key in ("reference_covered", "decoded_covered", "changed")]
            if any(type(count) is not int or not 0 <= count <= pixels for count in counts) or \
                    abs(counts[0]-counts[1]) > counts[2]:
                raise ValueError("Candidate mask coverage counts are inconsistent")
        passed = values[0] <= rms_limit and values[1] <= maximum_limit and \
                 all(result["changed"] == 0 for result in coverage)
        visual_review = entry["role"] == "response" and quality.get("numerical_policy") == "visual-review"
        if row.get("generated_limits_passed") != passed or \
                (not legacy and not passed and not visual_review) or \
                (not legacy and any(result["changed"] for result in coverage)):
            raise ValueError("Candidate has an unqualified generated/recompressed mip")
    if record.get("mask_compression") != mask_recipe(entry):
        raise ValueError("Candidate mask encoder recipe differs from actual material cutoffs")
    for level, margin in record.get("mask_encoder_margins", {}).items():
        index = int(level)
        if not cutoffs or not 0 <= index < info["mip_count"] or \
                (retained and index >= prefix) or type(margin) is not int or margin not in MASK_MARGINS[1:]:
            raise ValueError("Candidate mask margin changed an unowned or invalid mip")
    return True


def prepare(source, candidates, inventory, destination, receipt, pause_file=None, allow_partial=False, masters=None,
            representative_audits=None):
    resources = PreparationResources(pause_file, saturate=True)
    source, candidates, destination = source.resolve(), candidates.resolve(), destination.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if destination.is_relative_to(workspace) or destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError("Preparation output must be external and separate from input pack")
    if destination.exists():
        raise ValueError("Use a fresh preparation destination; source packs stay untouched")
    if receipt.resolve().is_relative_to(workspace) or any(receipt.resolve().is_relative_to(root) for root in (source, candidates, destination)):
        raise ValueError("Assembly receipt must remain external and separate from its inputs/output")
    if (source/"texture-streaming.json").exists():
        raise ValueError("Candidate assembly requires the canonical unstreamed source pack")
    inventory_document = json.loads(inventory.read_text())
    entries = copy.deepcopy(inventory_document["textures"])
    source_hashes = {**inventory_document["source_scenes"], **inventory_document["source_buffers"],
                     "game.json": inventory_document["source_game_sha256"]}
    for uri, digest in source_hashes.items():
        resources.wait()
        if hashlib.sha256((source/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Canonical pack changed after density inventory: {uri}")
    metadata = {}
    provenance = {}
    adopted = []
    pending = []
    audited = set()
    unaudited = set()
    for entry in entries:
        resources.wait()
        path = candidates / entry["uri"]
        if allow_partial and not path.with_suffix(".json").exists():
            pending.append(entry["uri"])
            entry["scale"] = 1
            entry["output_texels_per_meter"] = entry["minimum_texels_per_meter"]
            entry["density_exception"] = False
            continue
        record = json.loads(path.with_suffix(".json").read_text())
        if record["scale"] != entry["scale"] or record["uri"] != entry["uri"]:
            raise ValueError("Candidate does not match density inventory")
        source_hash = hashlib.sha256((source/entry["uri"]).read_bytes()).hexdigest()
        if record.get("source_runtime_sha256") != source_hash:
            raise ValueError(f"Source changed since candidate cooking: {entry['uri']}")
        if record["role"] != entry["role"] or record.get("upscaler") != "deepbump":
            raise ValueError("Migration requires the selected DeepBump material recipe")
        recipe = record.get("recipe", {})
        if entry["role"] not in ("color", "emissive") and recipe.get("semantic_filter_version") != 2:
            raise ValueError("Data fields require independent physical filtering; recook this candidate")
        if recipe.get("version") != RECIPE_VERSION or record.get("density_floor") != entry.get("density_floor") or \
                record.get("density_metric") != entry.get("density_metric"):
            raise ValueError("Candidate requires the current strict-density recipe")
        if record.get("model_sha256") != DEEPBUMP_UPSCALE_SHA256 or recipe.get("model_sha256") != DEEPBUMP_UPSCALE_SHA256 or \
                recipe.get("encoder_sha256") != ENCODER_SHA256 or record.get("encoder_sha256") != ENCODER_SHA256 or \
                hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest() != record.get("recipe_sha256"):
            raise ValueError("Candidate model, compression encoder or recipe checksum mismatch")
        if recipe.get("compression") != target_format(entry) or recipe.get("scale") != entry["scale"] or \
                recipe.get("role") != entry["role"] or recipe.get("color_space") != entry["color_space"] or \
                recipe.get("samplers") != sorted(entry.get("samplers", [20])) or \
                recipe.get("mask_cutoffs") != entry.get("mask_cutoffs", []) or \
                recipe.get("mask_compression") != mask_recipe(entry):
            raise ValueError("Candidate compression or material recipe differs from inventory")
        if recipe.get("cuda_math", "fp32") not in ("fp32", "tf32") or \
                type(recipe.get("prefer_nhwc", False)) is not bool or \
                (entry["role"] in ("control", "coating") and
                 recipe.get("sampled_channels") != list(sampled_channels(entry["role"]))):
            raise ValueError("Candidate math or consumed-channel recipe is invalid")
        if masters is None:
            raise ValueError("Assembly requires canonical masters to verify source mip receipts")
        levels, _, master_hash, _ = source_levels(source, masters, entry)
        if record.get("source_sha256") != master_hash or record.get("master_mip_sha256") != [
                hashlib.sha256(memoryview(level).cast("B")).hexdigest() for level in levels]:
            raise ValueError("Candidate source mip hashes differ from canonical masters")
        del levels
        if record.get("density_exception", False) != entry.get("density_exception", False):
            raise ValueError("Candidate density exception differs from inventory")
        if record.get("output_texels_per_meter") != entry.get("output_texels_per_meter"):
            raise ValueError("Candidate achieved density differs from inventory")
        if entry.get("density_exception") and (not inventory_document.get("cap_to_device") or
                max(entry["width"], entry["height"])*entry["scale"] != 16384):
            raise ValueError("Density exception requires the explicit device cap policy")
        if entry.get("density_applicable") and entry["minimum_texels_per_meter"] * entry["scale"] < entry["density_floor"] and not entry.get("density_exception"):
            raise ValueError(f"Candidate fails the strict density floor: {entry['uri']}")
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != record["output_sha256"]:
            raise ValueError(f"Candidate changed after cooking: {entry['uri']}")
        info = inspect_dds(raw)
        if info["dxgi"] != FORMATS[target_format(entry)] or record.get("format") != target_format(entry):
            raise ValueError("Candidate was not cooked to its required build-time block format")
        if (info["width"], info["height"]) != (entry["width"]*entry["scale"], entry["height"]*entry["scale"]):
            raise ValueError("Upscaled texture dimensions differ from the preparation contract")
        if info["mip_count"] != max(info["width"], info["height"]).bit_length():
            raise ValueError("Candidate requires a complete mip chain")
        source_info = inspect_dds((source/entry["uri"]).read_bytes())
        (audited if verify_quality(entry, record, info, source_info) else unaudited).add(entry["uri"])
        prefix = int(math.log2(entry["scale"]))
        if [mip["sha256"] for mip in info["mips"][prefix:]] != record.get("retained_mip_sha256"):
            raise ValueError("Candidate changed retained source mip data")
        if record.get("source_mips_retained") and [mip["sha256"] for mip in info["mips"][prefix:]] != [
                mip["sha256"] for mip in source_info["mips"]]:
            raise ValueError("Candidate did not retain the original compressed mip tail")
        metadata[entry["uri"]] = info
        adopted.append(entry["uri"])
        provenance[record["model_sha256"]] = dict(credit=record["credit"], upscaler=record["upscaler"])
    type_receipt = None
    if unaudited or representative_audits is not None:
        if representative_audits is None:
            raise ValueError("Representative-only cooking requires an explicit complete type-audit manifest")
        type_receipt = json.loads(representative_audits.read_text())
        representatives = type_receipt.get("representatives", {})
        types = {"color", "cutout", "normal", "response", "control", "coating", "constant", "emissive"}
        if set(representatives) != types:
            raise ValueError("Representative audits must cover every prepared material type")
        by_uri = {entry["uri"]: entry for entry in entries}
        for kind, uri in representatives.items():
            entry = by_uri.get(uri)
            if entry is None or uri not in audited or \
                    (kind == "constant" and not entry.get("constant_field")) or \
                    (kind == "cutout" and (entry["role"] != "color" or not entry.get("mask_cutoffs"))) or \
                    (kind not in ("constant", "cutout") and entry["role"] != kind):
                raise ValueError(f"Missing actual qualified representative for {kind}")
    def copy_file(source_file, destination_file):
        resources.wait()
        relative = Path(source_file).relative_to(source).as_posix()
        if relative in metadata:
            materialize(candidates/relative, Path(destination_file))
            return destination_file
        return shutil.copy2(source_file, destination_file)
    requested_bytes = sum(path.stat().st_size for path in source.rglob("*") if path.is_file()) + \
                      sum((candidates/uri).stat().st_size for uri in adopted)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if requested_bytes > shutil.disk_usage(destination.parent).free:
        raise ValueError(f"Assembly needs at least {requested_bytes/2**30:.2f} GiB free")
    shutil.copytree(source, destination, copy_function=copy_file)
    policy = dict(target=inventory_document.get("world_density_floor"), metric=inventory_document.get("density_metric"),
                  cap_to_device=inventory_document.get("cap_to_device", False),
                  pending_textures=pending,
                  results=[dict(uri=entry["uri"], density=entry["output_texels_per_meter"],
                                applicable=entry["density_applicable"], exception=entry.get("density_exception", False))
                           for entry in entries if entry["uri"] in adopted])
    resident_index = assemble(destination, entries, provenance=provenance, resources=resources,
                            meshes=inventory_document.get("meshes"), density_policy=policy)
    resident = {entry["uri"]: entry for entry in resident_index["textures"]}
    scene_hashes = {}
    for original in sorted(source.rglob("*.gltf")):
        resources.wait()
        document = json.loads(original.read_text())
        original_geometry = {key: copy.deepcopy(document.get(key)) for key in
            ("nodes", "meshes", "accessors", "bufferViews", "buffers", "skins", "animations", "scenes")}
        for material in document.get("materials", []):
            surface = material.get("extras", {}).get("surface", {})
            if surface.get("version") != 2:
                raise ValueError("Upscaling requires the canonical version-2 material pack")
            for channel in surface["channels"]:
                uri = channel["uri"]
                if uri not in resident:
                    continue
                entry = resident[uri]
                channel.update(width=entry["width"], height=entry["height"],
                    mip_count=entry["mip_count"],
                    format={28: "R8G8B8A8_UNORM", 29: "R8G8B8A8_UNORM_SRGB", 80: "BC4_UNORM",
                            83: "BC5_UNORM", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB"}[entry["format"]])
        for key, original_value in original_geometry.items():
            if document.get(key) != original_value:
                raise ValueError(f"Texture preparation changed scene geometry: {original.name}")
        relative = original.relative_to(source)
        (destination / relative).write_text(json.dumps(document, separators=(",", ":")) + "\n")
        scene_hashes[relative.as_posix()] = hashlib.sha256(original.read_bytes()).hexdigest()
    _, _, complete_closure = validate_pack(destination, exact=True)
    buffer_hashes = {}
    for original in sorted(source.rglob("*.bin")):
        resources.wait()
        relative = original.relative_to(source)
        digest = hashlib.sha256(original.read_bytes()).hexdigest()
        if hashlib.sha256((destination / relative).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Mesh buffer changed: {relative}")
        buffer_hashes[relative.as_posix()] = digest
    result = dict(version=1, texture_count=len(entries), resident_textures=len(resident),
                  validated_files=len(complete_closure),
                  runtime_bytes=sum((destination/uri).stat().st_size for uri in complete_closure),
                  bulk_cook_complete=not pending,
                  visual_status="pending actual-game multimodal review",
                  adopted_textures=adopted, pending_textures=pending,
                  upgraded_textures=sum(entry["scale"] > 1 for entry in entries),
                  density_metric=inventory_document.get("density_metric"),
                  world_density_floor=inventory_document.get("world_density_floor"),
                  close_density_floor=inventory_document.get("close_density_floor"),
                  density_exceptions=[dict(uri=entry["uri"], achieved=entry["output_texels_per_meter"],
                                           requested=entry["density_floor"])
                                      for entry in entries if entry.get("density_exception") and entry["uri"] in adopted],
                  source_scenes=scene_hashes, unchanged_buffers=buffer_hashes,
                  source_game_sha256=hashlib.sha256((source/"game.json").read_bytes()).hexdigest(),
                  models=provenance, representative_audits=type_receipt,
                  detailed_audited_textures=sorted(audited), integrity_only_textures=sorted(unaudited))
    for uri, digest in source_hashes.items():
        resources.wait()
        if hashlib.sha256((source/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Canonical source changed during assembly: {uri}")
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result[key] for key in ("texture_count", "resident_textures")}), flush=True)


def main():
    parser = argparse.ArgumentParser(description='Assemble qualified candidates into the existing PBR pack without geometry edits.')
    for name in ("source", "candidates", "inventory", "destination", "receipt", "masters"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--pause-file", type=Path)
    parser.add_argument("--allow-partial", action="store_true",
                        help="Qualification only: retain and explicitly report maps without candidates")
    parser.add_argument("--representative-audits", type=Path,
                        help="Explicit real per-type numerical/visual qualification manifest")
    args = parser.parse_args()
    prepare(args.source, args.candidates, args.inventory, args.destination, args.receipt, args.pause_file,
            args.allow_partial, args.masters, args.representative_audits)


if __name__ == "__main__":
    main()
