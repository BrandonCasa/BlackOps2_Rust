import argparse
import hashlib
import json
import struct
import re
from pathlib import Path

from ktx2_textures import MAGIC, decoded_levels, inspect_ktx2
from PIL import Image
import numpy as np


def encode_texture(source, template):
    raw = source.read_bytes()
    levels = decoded_levels(raw)
    while len(levels) < max(levels[0].shape[:2]).bit_length():
        image = Image.fromarray(levels[-1])
        image = image.resize((max(1, image.width // 2), max(1, image.height // 2)), Image.Resampling.BOX)
        levels.append(np.asarray(image))
    sample = template.read_bytes()
    dfd_offset, dfd_size = struct.unpack_from('<2I', sample, 48)
    dfd = sample[dfd_offset:dfd_offset + dfd_size]
    count = len(levels)
    offset = 80 + count * 24
    header = MAGIC + struct.pack('<9I', 43, 1, levels[0].shape[1], levels[0].shape[0], 0, 0, 1, count, 0)
    header += struct.pack('<4I2Q', offset, len(dfd), 0, 0, 0, 0)
    offset += len(dfd)
    index = bytearray()
    payload = bytearray(dfd)
    for level in levels:
        padding = -offset % 4
        payload.extend(bytes(padding))
        offset += padding
        part = level.tobytes()
        index.extend(struct.pack('<3Q', offset, len(part), len(part)))
        payload.extend(part)
        offset += len(part)
    result = header + index + payload
    inspect_ktx2(result)
    return result


def prepare(recovery, extraction, pack):
    effects = {}
    for zone in ['common_zm', 'patch_zm', 'zm_tomb']:
        for source in sorted((recovery / zone / 'native_fx').rglob('*.json')):
            effect = json.loads(source.read_text())
            for element in effect['elements']:
                element['atlas'][:6] = [v & 255 for v in element['atlas'][:6]]
            effect['source_sha256'] = hashlib.sha256(source.read_bytes()).hexdigest()
            effects[effect['name']] = effect
    material_sources = {}
    images = {}
    roots = sorted(extraction.glob('zone/*/*'))
    roots.sort(key=lambda p: (p.name in ['common_zm', 'patch_zm', 'zm_tomb'], p.name == 'zm_tomb'))
    for root in roots:
        for source in (root / 'materials').rglob('*.json'):
            material_sources[source.relative_to(root / 'materials').with_suffix('').as_posix()] = source
        for source in (root / 'images').glob('*.dds'):
            images[source.stem] = source
    required = set()
    references = set()
    for effect in effects.values():
        for element in effect['elements']:
            if element['type'] <= 6 or element['type'] == 11:
                required.update(element['visuals'])
            if element['type'] == 12:
                references.update(element['visuals'])
            references.update(element[key] for key in ['impact', 'death', 'emitted', 'attached'] if element[key])
    missing_effects = sorted(references - effects.keys())
    if missing_effects:
        raise ValueError('Missing original effect definitions: ' + ', '.join(missing_effects))
    game = json.loads((pack / 'game.json').read_text())
    scenes = [p['scene'] for p in game['content']['presentations'] if p.get('scene')]
    models = {}
    for model in sorted({v.lstrip(',') for effect in effects.values() for e in effect['elements'] if e['type'] == 7 for v in e['visuals']}):
        matches = [s for s in scenes if s.endswith('/' + model + '_lod0.gltf#Scene0')]
        if not matches:
            raise ValueError('Missing original FX model: ' + model)
        models[model] = min(matches, key=lambda s: ('/zm_tomb/' not in s, s))
    template = next(p for p in (pack / 'textures').glob('*.ktx2') if struct.unpack_from('<I', p.read_bytes(), 12)[0] == 43)
    destination = pack / 'effects'
    destination.mkdir(exist_ok=True)
    materials = {}
    textures = {}
    missing_materials = []
    for name in sorted(required):
        source = material_sources.get(name.lstrip(','))
        if source is None:
            missing_materials.append(name)
            continue
        native = json.loads(source.read_text())
        color = next((t for t in native.get('textures', []) if t.get('semantic') == 'colorMap'), None)
        if color is None or color['image'] not in images:
            missing_materials.append(name)
            continue
        image = images[color['image']]
        source_hash = hashlib.sha256(image.read_bytes()).hexdigest()
        texture = f'effects/{source_hash[:32]}.ktx2'
        target = pack / texture
        if not target.exists():
            target.write_bytes(encode_texture(image, template))
        textures[texture] = {'source_sha256': source_hash, 'sha256': hashlib.sha256(target.read_bytes()).hexdigest()}
        blend = native.get('stateBits', [{}])[0]
        materials[name] = {'texture': texture, 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                           'native': native, 'blend': blend}
    if missing_materials:
        raise ValueError('Missing original FX color maps: ' + ', '.join(missing_materials))
    manifest = {'effects': effects, 'materials': materials, 'textures': textures, 'models': models}
    target = destination / 'native.nativefx'
    target.write_text(json.dumps(manifest, separators=(',', ':'), allow_nan=False) + '\n', encoding='utf-8')
    validate(pack)
    print(json.dumps({'effects': len(effects), 'materials': len(materials), 'textures': len(textures), 'manifest_bytes': target.stat().st_size}))


def validate(pack, runtime=None):
    manifest = json.loads((pack / 'effects/native.nativefx').read_text())
    for texture, identity in manifest['textures'].items():
        raw = (pack / texture).read_bytes()
        inspect_ktx2(raw)
        if hashlib.sha256(raw).hexdigest() != identity['sha256']:
            raise ValueError('FX texture identity changed: ' + texture)
    for material in manifest['materials'].values():
        path = material['texture'].removeprefix('origins/')
        if path not in manifest['textures']:
            raise ValueError('Untracked FX texture: ' + path)
    for model, scene in manifest['models'].items():
        if not (pack / scene.removeprefix('origins/').split('#')[0]).is_file():
            raise ValueError('Missing FX model scene: ' + model)
    for effect in manifest['effects'].values():
        for element in effect['elements']:
            if element['type'] <= 6 or element['type'] == 11:
                if any(name not in manifest['materials'] for name in element['visuals']):
                    raise ValueError('FX material closure incomplete: ' + effect['name'])
            refs = element['visuals'] if element['type'] == 12 else []
            refs += [element[key] for key in ['impact', 'death', 'emitted', 'attached'] if element[key]]
            if any(name not in manifest['effects'] for name in refs):
                raise ValueError('FX effect closure incomplete: ' + effect['name'])
    game = json.loads((pack / 'game.json').read_text())
    requested = set()
    for weapon in game['content']['weapons']:
        for key in ['projectile_trail','projectile_effect']:
            name = weapon.get('mechanics',{}).get(key)
            if name:
                requested.add(name)
    if runtime:
        for path in runtime.rglob('*.rs'):
            requested.update(re.findall(r'"((?:maps|weapon|misc|impacts|vehicle)/[^"\n]*fx_[^"\n]+)"',path.read_text()))
    missing = []
    for name in requested:
        if '{' in name:
            pattern = re.sub(r'\\\{[^}]*\\\}', '[^/]+', re.escape(name))
            if not any(re.fullmatch(pattern,key) for key in manifest['effects']):
                missing.append(name)
        elif name not in manifest['effects']:
            missing.append(name)
    if missing:
        raise ValueError('Runtime FX definitions missing: ' + ', '.join(sorted(missing)))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--recovery', type=Path)
    parser.add_argument('--extraction', type=Path)
    parser.add_argument('--pack', type=Path, default=Path('assets/origins'))
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--runtime', type=Path, help='check literal Rust and weapon-definition FX requests')
    args = parser.parse_args()
    if args.check:
        validate(args.pack, args.runtime)
    else:
        if args.recovery is None or args.extraction is None:
            parser.error('--recovery and --extraction are required for preparation')
        prepare(args.recovery, args.extraction, args.pack)


if __name__ == '__main__':
    main()
