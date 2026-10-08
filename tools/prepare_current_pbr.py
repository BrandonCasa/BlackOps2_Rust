"""Finish selected first-person material sets using the existing DeepBump cook.

All four consumed maps are reauthored at 4x with fresh semantic mip chains.
Original inputs and geometry remain immutable; the rest of the pack is deferred.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter

from compressed_texture_cache import CompressedTextureCache, FORMATS, materialize
from enhance_material_normals import DeepBumpNormals, NORMAL_MODEL_SHA256
from import_origins import validate_pack
from material_textures import color_levels, inspect_dds, normal_levels, response_levels, rgba_dds
from preparation_resources import PreparationResources
from texture_streaming import mip_ranges, tail, validate
from upscale_textures import DeepBumpUpscaler, DEEPBUMP_UPSCALE_SHA256, source_levels


PROFILES = {
    'mc/mtl_viewarm_zom_dempsey': 'gloves',
    'mc/mtl_viewarm_zom_dempsey_device': 'device',
    'mc/mtl_t6_wpn_zmb_mc96_camo': 'mauser',
    'mc/mtl_t6_wpn_zmb_mc96_camo2': 'mauser',
    'mc/mtl_t6_attach_bullet': 'bullet',
}


def response_fields(pixels, profile):
    """Reviewed atlas zones plus chromatic wear, not one global metal preset.

    These are practical remaster recipes, not recovered native material masks.
    AO stays neutral because runtime lighting supplies occlusion.
    """
    rgb = pixels[..., :3].astype(np.float32)/255
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    luminance = rgb.mean(axis=2)
    h, w = luminance.shape
    u, v = np.arange(w, dtype=np.float32)[None, :]/w, np.arange(h, dtype=np.float32)[:, None]/h
    rough = np.full((h, w), .58, np.float32)
    metal = np.zeros((h, w), np.float32)
    coat = np.full((h, w), .12, np.float32)
    if profile == 'gloves':
        # Leather gloves, fabric cuff, and the small metal fasteners/band.
        fabric = (u < .53) & (v < .75)
        fastener = (v > .72) & (u > .18) & (u < .58) & (luminance > .34)
        rough[fabric], coat[fabric] = .82, .025
        metal[fastener], rough[fastener], coat[fastener] = .85, .32, .08
    elif profile == 'device':
        hardware = v < .35
        fabric = v > .73
        metal[:] = np.broadcast_to(hardware, (h, w))*.88
        rough[:] = np.where(hardware, .32, np.where(fabric, .83, .57))
        coat[:] = np.where(hardware, .16, np.where(fabric, .02, .10))
    elif profile == 'mauser':
        # Blued steel receiver and barrel with a separate wooden grip island.
        wood = (u > .60) & (u < .81) & (v > .21) & (v < .35)
        metal.fill(.9); rough.fill(.31); coat.fill(.18)
        metal[wood], rough[wood], coat[wood] = 0, .55, .28
    elif profile == 'bullet':
        metal.fill(.96); rough.fill(.28); coat.fill(.06)
    else:
        raise ValueError('Unknown reviewed material profile')
    # Rust/blood/dirt are dielectric. The brass bullet and yellow device metal
    # deliberately use a red-excess test rather than classifying yellow as rust.
    stain = np.clip((r-g*1.25-.025)*6, 0, 1)*np.clip((r-b-.015)*5, 0, 1)
    metal *= 1-stain
    rough += stain*.16
    coat *= 1-stain*.8
    grain = luminance-gaussian_filter(luminance, sigma=5)
    rough += np.clip(-grain*.35, -.08, .10)
    result = np.empty_like(pixels)
    result[..., 0] = 255
    result[..., 1] = np.rint(np.clip(rough, .18, .95)*255).astype(np.uint8)
    result[..., 2] = np.rint(np.clip(metal, 0, 1)*255).astype(np.uint8)
    result[..., 3] = np.rint(np.clip(coat, 0, 1)*255).astype(np.uint8)
    return result


def atomic_json(path, value):
    temporary = path.with_suffix(path.suffix+'.tmp')
    temporary.write_text(json.dumps(value, separators=(',', ':'))+'\n')
    temporary.replace(path)


def prepare(source, streamed, masters, model_path, upscale_model, encoder, cache_root, destination, receipt):
    source, streamed, destination = source.resolve(), streamed.resolve(), destination.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if destination.exists() or any(destination.is_relative_to(root) or root.is_relative_to(destination)
                                   for root in (source, streamed, workspace)):
        raise ValueError('Use a fresh external destination separate from inputs/workspace')
    if receipt.resolve().is_relative_to(destination) or receipt.resolve().is_relative_to(workspace):
        raise ValueError('Keep the preparation receipt external')
    # The source is an already qualified pack. Hash immutable inputs here;
    # run the full semantic/dependency closure once after assembling changes.
    source_hashes = {p.relative_to(streamed).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                     for p in streamed.rglob('*') if p.is_file()}
    if hashlib.sha256((source/'game.json').read_bytes()).hexdigest() != source_hashes['game.json']:
        raise ValueError('Current source and streamed catalog differ')
    scope = dict(materials=PROFILES, scenes=[])
    resources = PreparationResources(exit_for_shared_slot=True, saturate=True)
    resources.wait()
    cache = CompressedTextureCache(cache_root, encoder, 'cuda', resources)
    shutil.copytree(streamed, destination, copy_function=lambda a,b: materialize(Path(a),Path(b)))
    index = json.loads((destination/'texture-streaming.json').read_text())
    # Density completion is explicitly deferred by the user. Preserve useful
    # accepted detail and all other native chains without advertising 2048/m.
    deferred = copy.deepcopy(index.pop('density_policy', None))
    by_uri = {entry['uri']:entry for entry in index['textures']}
    generated, uses, audits, source_inputs, sets = {}, [], {}, {}, {}
    receipt.parent.mkdir(parents=True, exist_ok=True)

    def original_levels(channel):
        info = inspect_dds((source/channel['uri']).read_bytes())
        entry = dict(channel, width=info['width'], height=info['height'])
        levels, _, master_sha, runtime_sha = source_levels(source, masters, entry)
        source_inputs[channel['uri']] = dict(master_sha256=master_sha, runtime_sha256=runtime_sha)
        return levels

    def add_map(levels, role, reference, derivation):
        color_space = 'srgb' if role == 'color' else 'linear'
        raw = rgba_dds(levels, srgb=role == 'color')
        name = 'pbr-'+hashlib.sha256(raw+role.encode()).hexdigest()[:32]+'.dds'
        uri = 'textures/'+name
        if uri not in generated:
            entry = dict(uri=uri, role=role, color_space=color_space, width=levels[0].shape[1],
                         height=levels[0].shape[0], scale=1, density_applicable=True,
                         lossless=role == 'response')
            if role == 'response':
                derivation = dict(derivation, compression_policy='explicit lossless packed response; BC7 small-mip RMS exceeded 0.025')
            key = hashlib.sha256(raw+json.dumps(derivation,sort_keys=True).encode()).hexdigest()
            found = cache.find(key)
            audited = role not in audits
            if found is None or (audited and not found[1]['quality'].get('audited',True)):
                found = cache.store(key, levels, entry, raw, audit=audited)
            cached, record = found
            full_uri = 'textures/streaming/'+name
            full = destination/full_uri
            materialize(cached, full)
            compressed = full.read_bytes()
            info = mip_ranges(compressed)
            bootstrap = next((m['level'] for m in info['mips'] if max(m['width'],m['height']) <=128),
                             info['mip_count']-1)
            boot = tail(compressed, info, bootstrap)
            (destination/uri).write_bytes(boot)
            reference_entry = by_uri[reference['uri']]
            ratio = max(entry['width'],entry['height'])/max(reference_entry['width'],reference_entry['height'])
            stream = dict(uri=uri, full_uri=full_uri, width=info['width'], height=info['height'],
                          format=info['dxgi'],bootstrap_mip=bootstrap,header_bytes=148,bytes=len(compressed),
                          sha256=info['sha256'],bootstrap_sha256=hashlib.sha256(boot).hexdigest(),
                          color_space=color_space,texels_per_meter=reference_entry['texels_per_meter']*ratio,
                          protected_closeup=reference_entry.get('protected_closeup',False),mips=info['mips'])
            stream['mips'] = [{k:m[k] for k in ('level','width','height','offset','bytes','sha256')} for m in info['mips']]
            index['textures'].append(stream)
            by_uri[uri] = stream
            mip = info['mips'][bootstrap]
            metadata = dict(uri=uri,role=role,color_space=color_space,format=record['format'],
                            width=mip['width'],height=mip['height'],mip_count=info['mip_count']-bootstrap,
                            channels={'color':'rgb:base_color,a:coverage','normal':'rg:unit_normal_xy','response':'r:occlusion,g:perceptual_roughness,b:metallic,a:coating',
                                      'coating':'r:coating'}[role])
            generated[uri] = dict(metadata=metadata, derivation=derivation, receipt=record,
                                  semantic_sha256=hashlib.sha256(raw).hexdigest())
            if audited:
                audits[role] = uri
                preview = Image.fromarray(levels[0]); preview.thumbnail((2048, 2048))
                preview.save(receipt.parent/('current-pbr-'+role+'.png'))
        return copy.deepcopy(generated[uri]['metadata'])

    for scene_path in sorted(source.rglob('*.gltf')):
        scene = scene_path.relative_to(source).as_posix()
        original = json.loads(scene_path.read_text())
        if not any(m.get('extras', {}).get('surface', {}).get('identity') in PROFILES
                   for m in original.get('materials', [])):
            continue
        resources.wait()
        scope['scenes'].append(scene)
        document = json.loads((destination/scene).read_text())
        geometry = {k:copy.deepcopy(document.get(k)) for k in
                    ('nodes','meshes','accessors','bufferViews','buffers','skins','animations','scenes')}
        new_bindings = {}
        for number, native in enumerate(original.get('materials', [])):
            native_surface = native.get('extras', {}).get('surface', {})
            identity = native_surface.get('identity')
            if identity not in PROFILES:
                continue
            material = document['materials'][number]
            surface = material['extras']['surface']
            colors = [c for c in native_surface['channels'] if c['role'] == 'color']
            if len(colors) != 1 or colors[0]['layer'] != 0:
                raise ValueError('Selected atlas must have exactly one base layer')
            color = colors[0]
            profile = PROFILES[identity]
            key = (color['uri'], profile, color['sampler'])
            if key not in sets:
                pixels = original_levels(color)[0]
                if np.any(pixels[..., 3] != 255):
                    raise ValueError('Reviewed first-person sets require opaque source coverage')
                sampler = color['sampler']
                print(f'Complete material {identity}: 4x color and all PBR maps', flush=True)
                upscaler = DeepBumpUpscaler(upscale_model, device='cuda')
                try:
                    rgb = upscaler.upscale(pixels[..., :3].astype(np.float32)/255,
                                           not sampler&32, not sampler&64, progress=identity)
                finally:
                    upscaler.offload()
                    upscaler.executor.shutdown(wait=True)
                pixels = np.empty((*rgb.shape[:2], 4), np.uint8)
                pixels[..., :3] = np.rint(np.clip(rgb, 0, 1)*255).astype(np.uint8)
                pixels[..., 3] = 255
                del rgb
                derivation = dict(kind='complete-material-4x', profile=profile,
                                  source_color=source_inputs[color['uri']], sampler=sampler,
                                  upscale_model_sha256=DEEPBUMP_UPSCALE_SHA256,
                                  normal_model_sha256=NORMAL_MODEL_SHA256,
                                  recipe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                                  original_response_maps_retained=False, original_mips_retained=False)
                maps = [add_map(color_levels(pixels, True), 'color', color, derivation)]
                normal_model = DeepBumpNormals(model_path, device='cuda')
                try:
                    unit = normal_model.infer(pixels.astype(np.float32)/255, not sampler&32, not sampler&64)
                finally:
                    normal_model.backend.offload()
                slopes = unit[..., :2]/np.maximum(unit[..., 2:3], .1)
                del unit
                modes = ('nearest' if sampler&64 else 'wrap', 'nearest' if sampler&32 else 'wrap')
                for axis in range(2):
                    slopes[..., axis] -= gaussian_filter(slopes[..., axis], sigma=16, mode=modes)
                # Reauthor learned relief at the new resolution; no source
                # normal, response, coating, or old mip tail is blended back.
                unit = np.dstack((slopes*.25, np.ones(pixels.shape[:2], np.float32)))
                del slopes
                unit /= np.maximum(np.linalg.norm(unit, axis=2, keepdims=True), 1e-6)
                if not np.isfinite(unit).all():
                    raise ValueError('Inferred tangent data is nonfinite')
                levels, variances = normal_levels(unit)
                del unit
                maps.append(add_map(levels, 'normal', color, derivation))
                del levels
                fields = response_fields(pixels, profile)
                levels = response_levels(fields, variances)
                del variances, fields
                maps.append(add_map(levels, 'response', color, derivation))
                coats = []
                for level in levels:
                    coat = np.zeros_like(level)
                    coat[..., 0] = level[..., 3]; coat[..., 3] = 255
                    coats.append(coat)
                maps.append(add_map(coats, 'coating', color, derivation))
                del levels, coats
                for channel in maps:
                    channel.update(layer=0, sampler=sampler)
                sets[key] = maps
                preview = Image.fromarray(pixels); preview.thumbnail((2048, 2048))
                preview.save(receipt.parent/(profile+'-color.png'))
                del pixels
            old_uris = {c['uri'] for c in surface['channels']}
            surface['channels'] = copy.deepcopy(sets[key])
            surface['coating_roughness'] = .2
            surface.setdefault('provenance', {})['selected_material_remaster'] = (
                'Complete 4x DeepBump color; regenerated learned tangent relief; '
                'atlas-aware GGX roughness/metal/coating; neutral realtime AO; all fresh mips')
            new_bindings[number] = (old_uris, color['uri'], [c['uri'] for c in sets[key]])
            uses.append(dict(scene=scene, material=identity, profile=profile,
                             channels=copy.deepcopy(sets[key])))
        for mesh_number, mesh in enumerate(document.get('meshes', [])):
            for primitive_number, primitive in enumerate(mesh['primitives']):
                replacement = new_bindings.get(primitive['material'])
                if replacement is None:
                    continue
                old_uris, reference, new_uris = replacement
                regions = index['meshes'].get(f'{scene}#Mesh{mesh_number}/Primitive{primitive_number}', [])
                for region in regions:
                    density = region['textures'].get(reference)
                    if density is None:
                        raise ValueError('Selected material has no measured source UV density')
                    for uri in old_uris:
                        region['textures'].pop(uri, None)
                    for uri in new_uris:
                        ratio = max(by_uri[uri]['width'], by_uri[uri]['height'])/max(by_uri[reference]['width'], by_uri[reference]['height'])
                        region['textures'][uri] = density*ratio
        if any(document.get(k) != value for k, value in geometry.items()):
            raise ValueError('PBR generation changed authored geometry/skin/animation')
        atomic_json(destination/scene, document)
        print(f'Complete PBR {scene}: {len(generated)} unique maps, {len(uses)} material bindings', flush=True)
    if len(sets) != 4 or set(u['material'] for u in uses) != set(PROFILES):
        raise ValueError('The four complete first-person sets were not all assembled')
    atomic_json(destination/'texture-streaming.json',index)
    validate(destination)
    _,_,closure = validate_pack(destination,exact=True)
    for uri,digest in source_hashes.items():
        if hashlib.sha256((streamed/uri).read_bytes()).hexdigest()!=digest:
            raise ValueError('Prepared source changed during current-channel generation')
    result=dict(version=1,status='complete-selected-material-sets',scope=scope,generated_maps=generated,
                bindings=uses,representative_audits=audits,complete_material_sets=len(sets),
                deferred_density_policy=deferred,source_inputs=source_inputs,validated_files=len(closure),
                sha256=closure,approximation='learned fine relief and reviewed atlas-aware GGX/metal/coating masks; not recovered native physical measurements',
                bulk_density_cook_deferred=True)
    receipt.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(generated_maps=len(generated),bindings=len(uses),validated_files=len(closure))),flush=True)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('source','streamed','masters','model','upscale-model','encoder','cache','destination','receipt'):
        parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args()
    prepare(args.source,args.streamed,args.masters,args.model,args.upscale_model,args.encoder,args.cache,args.destination,args.receipt)


if __name__=='__main__':
    main()
