# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13", "zstandard>=0.25,<1", "soundfile>=0.13,<0.14"]
# ///
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path, PurePosixPath
import struct
import os
import shutil
import subprocess
import time
import re
import threading
from functools import lru_cache

try:
    from .ktx2_textures import DXGI_TO_VK, TO_LINEAR, TO_SRGB, inspect_ktx2, payloads, decoded_levels
except ImportError:
    from ktx2_textures import DXGI_TO_VK, TO_LINEAR, TO_SRGB, inspect_ktx2, payloads, decoded_levels

try:
    from .material_textures import inspect_dds, rgba_levels, color_levels, normal_levels, response_levels
except ImportError:
    from material_textures import inspect_dds, rgba_levels, color_levels, normal_levels, response_levels


TEXTURES = {'.dds', '.png', '.jpg', '.jpeg', '.ktx2', '.basis', '.tga', '.bmp', '.webp', '.hdr', '.exr', '.tif', '.tiff'}
POINTER = b'version https://git-lfs.github.com/spec/v1'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')
    temporary.replace(path)


def wait_for_slot():
    command = ['powershell', '-NoProfile', '-NonInteractive', '-Command',
               "@(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^(origins.*|renderdoccmd|qrenderdoc|nsys|nsight.*|capture|tracy-capture.*|tracy-csvexport|wpr|xperf)$' }).Count"]
    while int(subprocess.check_output(command).decode().strip()):
        print('Waiting for the shared game/capture slot', flush=True)
        time.sleep(15)


def invoke(executable, manifest, report):
    wait_for_slot()
    outcome = subprocess.run([str(executable), '--prepare-textures', str(manifest), '--report', str(report)],
                             cwd=executable.parent, capture_output=True, text=True)
    if not report.is_file():
        raise RuntimeError(f'Preparation produced no structured report: {outcome.stderr}')
    result = json.loads(report.read_text(encoding='utf-8'))
    if outcome.returncode or not result.get('passed'):
        failures = [r for r in result['items'] if r.get('error')]
        raise RuntimeError(f'Bevy preparation rejected {len(failures)} items: {failures[:5]}')
    return result


@lru_cache(maxsize=4)
def backend_identity(executable):
    path = executable.resolve().parent / 'identity.json'
    if not path.is_file():
        raise ValueError('Texture preparation requires a pin_development.py bundle identity')
    identity = json.loads(path.read_text(encoding='utf-8'))
    for name, expected in identity['binaries'].items():
        if digest(path.parent/name) != expected:
            raise ValueError(f'Pinned preparation binary/DLL changed: {name}')
    if 'prepare-textures' not in identity['source']['features']:
        raise ValueError('The pinned executable excludes the preparation feature')
    return {'backend': 'bevy-0.20.0/CompressedImageSaver/ctt',
            'binaries': identity['binaries'], 'revision': identity['source']['revision'],
            'features': identity['source']['features'],
            'source_snapshot_sha256': hashlib.sha256(json.dumps(identity['source'], sort_keys=True).encode()).hexdigest(),
            'python_sources': {name: digest(Path(__file__).with_name(name)) for name in
                ('prepare_ktx2.py', 'ktx2_textures.py', 'material_textures.py', 'compressed_texture_cache.py')}}


def save_container(parts, fmt, width, height, source, output, executable, work, role, srgb, faces=1):
    work.mkdir(parents=True, exist_ok=True)
    paths = []
    for level, part in enumerate(parts):
        path = work / (str(level)+'.bin')
        path.write_bytes(part)
        paths.append(str(path.resolve()))
    manifest, report = work/'container-batch.json', work/'container-results.json'
    atomic_json(manifest, {'items': [{'id': output.stem, 'input': str(source.resolve()),
        'output': str(output.resolve()), 'role': role, 'is_srgb': srgb,
        'settings': {'is_normal_map': False, 'input_alpha_mode': 'Straight',
                     'output_alpha_mode': 'Straight', 'generate_mipmaps': False},
        'container': {'format': fmt, 'width': width, 'height': height, 'depth': 0,
                      'layers': 0, 'faces': faces, 'payloads': paths}}]})
    invoke(executable.resolve(), manifest, report)
    return inspect_ktx2(output.read_bytes())


def prepare_levels(levels, entry, executable, work, source_raw=None, retained_start=None):
    import numpy as np
    try:
        from .compressed_texture_cache import qualify
        from .material_textures import inspect_texture
    except ImportError:
        from compressed_texture_cache import qualify
        from material_textures import inspect_texture
    role, srgb = entry['role'], entry['color_space'] == 'srgb'
    if work.resolve().is_relative_to(Path(__file__).resolve().parents[1]):
        raise ValueError('Prepared texture artifacts must stay outside the repository')
    source_info = inspect_texture(source_raw) if source_raw is not None else None
    channels = 2 if role == 'normal' else 1 if role == 'coating' else 4
    if source_info and ((role == 'normal' and source_info['dxgi'] not in (49, 83)) or
                        (role == 'coating' and source_info['dxgi'] not in (61, 80))):
        channels = 4
    fmt = 16 if channels == 2 else 9 if channels == 1 else 43 if srgb else 37
    settings = {'is_normal_map': role == 'normal', 'input_alpha_mode': 'Straight',
                'output_alpha_mode': 'Straight', 'generate_mipmaps': False}
    recipe = {'version': 1, 'backend': backend_identity(executable), 'role': role, 'settings': settings,
              'color_space': entry['color_space'], 'mip_owner': 'semantic-preparation',
              'mip_hashes': [hashlib.sha256(memoryview(np.ascontiguousarray(x))).hexdigest() for x in levels],
              'mask_cutoffs': entry.get('mask_cutoffs', []), 'lossless': entry.get('lossless', False),
              'authored_source_sha256': hashlib.sha256(source_raw).hexdigest() if source_raw else None,
              'authored_mip_start': retained_start,
              'encoding': {'native_preferred': 141 if role == 'normal' else 139 if role == 'coating' else 146 if srgb else 145,
                           'lossless': fmt}, 'samplers': entry.get('samplers', [entry.get('sampler', 20)])}
    key = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()
    scratch = work.resolve()/key
    scratch.mkdir(parents=True, exist_ok=True)
    output, receipt = scratch/'texture.ktx2', scratch/'receipt.json'
    if output.exists() and receipt.exists():
        record = json.loads(receipt.read_text())
        if record['recipe'] != recipe or record['output']['sha256'] != digest(output):
            raise ValueError('Canonical preparation cache identity differs')
        inspect_ktx2(output.read_bytes())
        return output, record
    if output.exists():
        raise ValueError('Unqualified preparation output requires a fresh cache directory')
    raw_parts, items = [], []
    for index, level in enumerate(levels):
        part = np.ascontiguousarray(level[..., :channels]).tobytes()
        raw_parts.append(part)
        raw_path = scratch/(str(index)+'.raw')
        raw_path.write_bytes(part)
        items.append({'id': str(index), 'input': str(raw_path), 'output': str(scratch/(str(index)+'.ktx2')),
                      'role': role, 'is_srgb': srgb, 'settings': settings,
                      'raw': {'width': level.shape[1], 'height': level.shape[0], 'channels': channels}})
    width, height = levels[0].shape[1], levels[0].shape[0]
    blocked = width % 4 == 0 and height % 4 == 0
    compression = not entry.get('lossless', False) and blocked
    authored_parts = None
    authored_format = None
    if source_raw is not None:
        if retained_start is None or retained_start < 0 or len(levels)-retained_start != source_info['mip_count']:
            raise ValueError('Authored mip ownership differs from the prepared extent')
        if (levels[retained_start].shape[1], levels[retained_start].shape[0]) != (source_info['width'], source_info['height']):
            raise ValueError('Authored mip extent differs from the prepared chain')
        if source_raw.startswith(b'DDS '):
            cursor = 148 if source_raw[84:88] == b'DX10' else 128
            authored_parts = []
            for mip in source_info['mips']:
                authored_parts.append(source_raw[cursor:cursor+mip['bytes']])
                cursor += mip['bytes']
            authored_format = DXGI_TO_VK[source_info['dxgi']]
        else:
            authored_parts = list(payloads(source_raw, source_info))
            authored_format = source_info['vk_format']
        expected_format = 141 if role == 'normal' else 139 if role == 'coating' else 146 if srgb else 145
        if authored_format != expected_format:
            compression = False
    quality = None
    if compression:
        manifest, report = scratch/'encode-batch.json', scratch/'encode-results.json'
        encode_count = retained_start if authored_parts is not None else len(levels)
        if encode_count:
            atomic_json(manifest, {'items': items[:encode_count]})
            invoke(executable.resolve(), manifest, report)
        encoded = [(scratch/(str(i)+'.ktx2')).read_bytes() for i in range(encode_count)]
        layouts = [inspect_ktx2(raw, complete=False) for raw in encoded]
        if len({layout['vk_format'] for layout in layouts}) > 1:
            raise ValueError('Bevy encoded semantic mip levels with differing formats')
        encoded_fmt = layouts[0]['vk_format'] if layouts else authored_format
        parts = [next(payloads(raw, layout)) for raw, layout in zip(encoded, layouts, strict=True)]
        if authored_parts is not None:
            if encoded_fmt != authored_format:
                raise ValueError('Native encoder changed the required authored mip encoding')
            parts.extend(authored_parts)
        candidate = scratch/'compressed.ktx2'
        save_container(parts, encoded_fmt, width, height, Path(items[0]['input']), candidate,
                       executable, scratch/'compressed-container', role, srgb)
        try:
            quality = qualify(candidate.read_bytes(), levels, role, retained_start, entry.get('mask_cutoffs', []))
        except ValueError:
            compression = False
        if compression:
            candidate.rename(output)
    if not compression:
        if source_raw is not None:
            original_levels = decoded_levels(source_raw)
            raw_parts[retained_start:] = [np.ascontiguousarray(level[..., :channels]).tobytes() for level in original_levels]
        save_container(raw_parts, fmt, width, height, Path(items[0]['input']), output,
                       executable, scratch/'lossless-container', role, srgb)
        quality = qualify(output.read_bytes(), levels, role, retained_start, entry.get('mask_cutoffs', []))
    result = {'recipe': recipe, 'cache_key': key, 'output': inspect_ktx2(output.read_bytes()),
              'quality': quality, 'compression': 'native-BCn' if compression else 'lossless-semantic-data',
              'authored_mips': 'identical-blocks' if compression and source_raw else 'identical-decoded-channels' if source_raw else None}
    if source_raw is not None:
        saved = output.read_bytes()
        if compression:
            if list(payloads(saved))[retained_start:] != authored_parts:
                raise ValueError('Prepared output altered authored mip blocks')
        elif any(not np.array_equal(a[..., :channels], b[..., :channels]) for a, b in
                 zip(decoded_levels(source_raw), decoded_levels(saved)[retained_start:], strict=True)):
            raise ValueError('Prepared output altered authored mip channels')
        result['authored_preservation_checked'] = True
    atomic_json(receipt, result)
    return output, result


def convert(inventory_path, output, executable, selected=()):
    inventory_record = json.loads(inventory_path.read_text(encoding='utf-8-sig'))
    source = Path(inventory_record['root'])
    output = output.resolve()
    if output.is_relative_to(source) or source.is_relative_to(output):
        raise ValueError('Candidate processing must stay separate from its input pack')
    output.mkdir(parents=True, exist_ok=True)
    backend = backend_identity(executable)
    manifest_items, records = [], {}
    for uri, entry in inventory_record['textures'].items():
        if selected and not any(s in uri for s in selected):
            continue
        raw = (source / uri).read_bytes()
        if hashlib.sha256(raw).hexdigest() != entry['source_sha256']:
            raise ValueError(f'Source changed after inventory: {uri}')
        roles = entry['roles']
        if len(entry['color_spaces']) > 1:
            raise ValueError(f'Texture has incompatible transfer consumers: {uri}')
        srgb = entry['color_spaces'] == ['srgb']
        role = roles[0] if len(roles) == 1 else 'shared:' + ','.join(roles) if roles else 'image'
        layout = inspect_dds(raw, allow_non_2d=True) if Path(uri).suffix.lower() == '.dds' else None
        encoding = DXGI_TO_VK[layout['dxgi']] if layout else 43 if srgb else 37
        if entry['color_spaces']:
            encoding = TO_SRGB.get(encoding, encoding) if srgb else TO_LINEAR.get(encoding, encoding)
        premultiplied = layout is not None and raw[84:88] == b'DX10' and (struct.unpack_from('<I', raw, 144)[0] & 7) == 2
        settings = {'is_normal_map': False, 'input_alpha_mode': 'Straight',
                    'output_alpha_mode': 'Premultiplied' if premultiplied else 'Straight', 'generate_mipmaps': False}
        recipe = {'version': 1, 'backend': backend, 'mode': 'lossless-container-migration',
                  'source_sha256': entry['source_sha256'], 'roles': roles, 'color_spaces': entry['color_spaces'],
                  'consumer_semantics_sha256': hashlib.sha256(json.dumps(entry['consumers'], sort_keys=True).encode()).hexdigest(),
                  'mip_owner': entry['mip_owner'], 'settings': settings, 'encoding': encoding}
        key = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()
        scratch = output / 'payloads' / key
        scratch.mkdir(parents=True, exist_ok=True)
        destination = output / 'textures' / (key + '.ktx2')
        if destination.exists():
            checked = inspect_ktx2(destination.read_bytes())
            receipt = output / 'receipts' / (key + '.json')
            old = json.loads(receipt.read_text()) if receipt.exists() else None
            if not old or old.get('recipe') != recipe or old['output']['sha256'] != checked['sha256']:
                raise ValueError(f'Unqualified cached KTX2 output: {uri}')
            records[uri] = old
            continue
        if Path(uri).suffix.lower() in ('.png', '.jpg', '.jpeg'):
            import numpy as np
            from PIL import Image
            with Image.open(source/uri) as image:
                pixels = np.asarray(image.convert('RGBA'))
            if not roles and 'normal' in Path(uri).stem:
                role, srgb = 'normal', False
            elif not roles:
                role, srgb = 'color', True
                for consumer in entry['consumers']:
                    document_path = source/consumer['document']
                    if document_path.suffix != '.gltf':
                        continue
                    document = json.loads(document_path.read_bytes())
                    image_indices = {i for i, image in enumerate(document.get('images', []))
                                     if (PurePosixPath(consumer['document']).parent/image.get('uri', '')).as_posix() == uri}
                    textures = {i for i, t in enumerate(document.get('textures', [])) if t.get('source') in image_indices}
                    if any(m.get('normalTexture', {}).get('index') in textures for m in document.get('materials', [])):
                        role, srgb = 'normal', False
                    break
            cutoffs = sorted({c['presentation']['cutoff'] for c in entry['consumers']
                              if isinstance(c.get('presentation'), dict) and c['presentation'].get('alpha') == 'mask'} |
                             {c['mask_cutoff'] for c in entry['consumers'] if c.get('mask_cutoff') is not None})
            if len(cutoffs) > 1:
                raise ValueError(f'Image has multiple cutoffs requiring explicit semantic mip preparation: {uri}')
            if role == 'normal':
                normal = pixels[..., :3].astype(np.float32)/127.5-1
                levels, _ = normal_levels(normal)
                levels[0] = pixels.copy()
            elif role == 'response':
                levels = response_levels(pixels, [])
            else:
                levels = color_levels(pixels, srgb, cutoffs[0] if cutoffs else None)
                levels[0] = pixels.copy()
            parts = [level.tobytes() for level in levels]
            width, height = pixels.shape[1], pixels.shape[0]
            save_container(parts, 43 if srgb else 37, width, height, source/uri, destination,
                           executable, scratch, role, srgb)
            records[uri] = {'recipe': recipe, 'preparation_mode': 'lossless-source-image-and-semantic-mips',
                            'cache_key': key, 'source': entry, 'candidate_path': str(destination),
                            'source_mips': [{'level': index, 'surface': 0, 'sha256': hashlib.sha256(part).hexdigest()}
                                            for index, part in enumerate(parts)]}
            continue
        if Path(uri).suffix.lower() != '.dds':
            raise ValueError(f'Unsupported original preparation input: {uri}')
        if layout['mip_count'] != max(layout['width'], layout['height']).bit_length():
            raise ValueError(f'Incomplete authored mip chain needs explicit semantic preparation: {uri}')
        if layout['dxgi'] not in DXGI_TO_VK:
            raise ValueError(f'No Bevy-supported KTX2 format for DDS {layout["dxgi"]}: {uri}')
        fmt = encoding
        if not entry['color_spaces']:
            srgb = fmt in TO_LINEAR
        cursor = 148 if raw[84:88] == b'DX10' else 128
        mip_data = defaultdict(list)
        source_hashes = []
        for mip in layout['mips']:
            data = raw[cursor:cursor+mip['bytes']]
            cursor += mip['bytes']
            source_hashes.append({'level': mip['level'], 'surface': mip['surface'], 'sha256': mip['sha256']})
            mip_data[mip['level']].append(data)
        paths = []
        for level in range(layout['mip_count']):
            path = scratch / (str(level) + '.bin')
            path.write_bytes(b''.join(mip_data[level]))
            paths.append(str(path))
        item = {'id': key, 'input': str((source/uri).resolve()), 'output': str(destination),
                'role': role, 'is_srgb': srgb, 'settings': settings,
                'container': {'format': fmt, 'width': layout['width'], 'height': layout['height'],
                              'depth': 0, 'layers': 0, 'faces': 6 if layout['dimension'] == 'cube' else 1,
                              'payloads': paths}}
        manifest_items.append(item)
        records[uri] = {'recipe': recipe, 'cache_key': key, 'source': entry,
                        'source_mips': source_hashes, 'candidate_path': str(destination)}
    manifest = output / 'batch.json'
    report = output / 'bevy-results.json'
    if manifest_items:
        atomic_json(manifest, {'items': manifest_items})
        invoke(executable.resolve(), manifest, report)
    for uri, record in records.items():
        raw = Path(record['candidate_path']).read_bytes()
        info = inspect_ktx2(raw)
        source = record['source_mips']
        got = []
        for level, data in enumerate(payloads(raw, info)):
            surfaces = [m for m in source if m['level'] == level]
            stride = len(data)//len(surfaces)
            for index, mip in enumerate(surfaces):
                sha = hashlib.sha256(data[index*stride:(index+1)*stride]).hexdigest()
                if sha != mip['sha256']:
                    raise ValueError(f'Authored mip payload changed: {uri} mip {level} surface {index}')
                got.append({**mip, 'sha256': sha})
        record['output'] = info
        record['qualification'] = {'authored_mips_identical': True, 'decoded_error': 0,
                                   'mask_classification_changes': 0, 'checked_surfaces': len(got)}
        atomic_json(output/'receipts'/(record['cache_key']+'.json'), record)
    result = {'version': 1, 'complete': set(records) == set(inventory_record['textures']),
              'inventory': str(inventory_path.resolve()), 'textures': records,
              'encoder_identity': digest(executable.resolve())}
    atomic_json(output/'conversion.json', result)
    return {'converted': len(records), 'complete': result['complete']}


def preserve_source(source, role, srgb, executable, work):
    try:
        from .material_textures import inspect_dds
    except ImportError:
        from material_textures import inspect_dds
    raw = source.read_bytes()
    layout = inspect_dds(raw, allow_non_2d=True)
    entry = {'source_sha256': layout['sha256'], 'source_bytes': len(raw), 'source_container': 'dds',
             'canonical_sources': [{'path': str(source.resolve()), 'sha256': layout['sha256'],
                                    'stage': 'retained-native-source'}], 'missing_master': False,
             'roles': [role], 'color_spaces': ['srgb' if srgb else 'linear'], 'consumers': [],
             'mip_owner': 'authored-source'}
    recipe = {'entry': entry, 'backend': backend_identity(executable)}
    key = hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest()
    scratch = work.resolve()/key
    scratch.mkdir(parents=True, exist_ok=True)
    manifest = scratch/'inventory.json'
    atomic_json(manifest, {'root': str(source.resolve().parent), 'textures': {source.name: entry}})
    convert(manifest, scratch/'candidate', executable)
    record = json.loads((scratch/'candidate/conversion.json').read_text())['textures'][source.name]
    return Path(record['candidate_path']), record


def runtime_image(pixels, role, srgb, destination, config, cutoffs=()):
    import numpy as np
    try:
        from .material_textures import color_levels, normal_levels, response_levels
        from .compressed_texture_cache import materialize
    except ImportError:
        from material_textures import color_levels, normal_levels, response_levels
        from compressed_texture_cache import materialize
    executable = Path(config['materials']['preparation_executable'])
    work = Path(config['assets']['inspection_output'])/'runtime-ktx2-cache'
    cutoffs = sorted(set(cutoffs))
    if len(cutoffs) > 1:
        raise ValueError('Multiple material cutoffs require an explicit jointly qualified mip recipe')
    if role == 'normal':
        levels, _ = normal_levels(pixels[..., :3].astype(np.float32)/127.5-1)
    elif role == 'response':
        levels = response_levels(pixels, [])
    else:
        levels = color_levels(pixels, srgb, cutoffs[0] if cutoffs else None)
    levels[0] = pixels.copy()
    output, record = prepare_levels(levels, {'role': role, 'color_space': 'srgb' if srgb else 'linear',
                                            'mask_cutoffs': cutoffs, 'lossless': True}, executable, work)
    materialize(output, destination)
    return record


def process_recipes(recipes, output, executable):
    from PIL import Image
    import numpy as np
    results = []
    for item in json.loads(recipes.read_text())['items']:
        try:
            source = Path(item['input']).resolve()
            raw = source.read_bytes()
            entry = item['semantic']
            if source.suffix == '.dds':
                levels = rgba_levels(raw)
            else:
                with Image.open(source) as image:
                    pixels = np.asarray(image.convert('RGBA'))
                if entry['role'] == 'normal':
                    levels, _ = normal_levels(pixels[..., :3].astype(np.float32)/127.5-1)
                elif entry['role'] == 'response':
                    levels = response_levels(pixels, [])
                else:
                    cutoffs = entry.get('mask_cutoffs', [])
                    if len(cutoffs) > 1:
                        raise ValueError('Multiple cutoffs require an explicit semantic master')
                    levels = color_levels(pixels, entry['color_space'] == 'srgb', cutoffs[0] if cutoffs else None)
                levels[0] = pixels.copy()
            authored = Path(item['authored_input']).resolve().read_bytes() if item.get('authored_input') else None
            path, record = prepare_levels(levels, entry, executable, output/'cache', authored,
                                          item.get('authored_mip_start'))
            results.append({'id': item['id'], 'input': str(source), 'input_sha256': hashlib.sha256(raw).hexdigest(),
                            'output': str(path), 'record': record, 'error': None})
        except Exception as error:
            results.append({'id': item['id'], 'error': str(error)})
    result = {'passed': all(r['error'] is None for r in results), 'items': results}
    atomic_json(output/'results.json', result)
    return {'passed': result['passed'], 'items': len(results)}


def inspect_game(identity_path, cases_path, output, port):
    try:
        from .gameplay_remote import Client
        from .verify_weapon_materials import capture, wait_materials
    except ImportError:
        from gameplay_remote import Client
        from verify_weapon_materials import capture, wait_materials
    identity = json.loads(identity_path.read_text())
    cases = json.loads(cases_path.read_text())['cases']
    capture_names = [case['id']+'-'+step['name'] for case in cases for step in case['steps']
                     if step['action'] == 'capture']
    if len({case['id'] for case in cases}) != len(cases) or len(set(capture_names)) != len(capture_names) or any(
            not re.fullmatch(r'[A-Za-z0-9_-]+', name) for name in capture_names):
        raise ValueError('Visual cases require unique plain IDs and capture names')
    if output.exists() or output.resolve().is_relative_to(Path(__file__).resolve().parents[1]):
        raise ValueError('Visual inspection requires a fresh external output directory')
    binary = identity_path.parent/'origins.exe'
    for name, expected in identity['binaries'].items():
        if digest(identity_path.parent/name) != expected:
            raise ValueError(f'Pinned visual executable/DLL changed: {name}')
    root = Path(identity['asset_root'])
    if {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()} != identity['assets'].keys():
        raise ValueError('Pinned visual runtime asset file list changed')
    for uri, expected in identity['assets'].items():
        stat = (root/uri).stat()
        if (stat.st_size, stat.st_mtime_ns) != (expected['size'], expected['mtime_ns']) and digest(root/uri) != expected['sha256']:
            raise ValueError(f'Pinned visual runtime asset changed: {uri}')
    output.mkdir(parents=True)
    client = Client(f'http://127.0.0.1:{port}')
    report = {'passed': False, 'identity_sha256': digest(identity_path), 'cases_sha256': digest(cases_path),
              'script_sha256': digest(Path(__file__)), 'cases': [],
              'asset_identity_check': 'pinned file list, sizes and timestamps; SHA256 rechecked on metadata changes',
              'visual_review': 'pending inspection of actual game captures'}
    wait_for_slot()
    with (output/'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(binary), '--validation-remote', str(port)], cwd=binary.parent,
                                   stdout=log, stderr=subprocess.STDOUT)
        try:
            client.until(lambda s: s['state'] == 'Playing' and s.get('player'), timeout=180)
            report['discovery'] = client.call('rpc.discover')
            report['schema'] = client.call('origins/schema')
            for case in cases:
                client.reset(timeout=60)
                row = {'id': case['id'], 'captures': [], 'material_states': []}
                report['cases'].append(row)
                stop, anchor_errors = threading.Event(), []
                anchor = None
                def hold_position(position):
                    anchored = Client(f'http://127.0.0.1:{port}')
                    while not stop.wait(.05):
                        try:
                            anchored.call('origins/setup', {'position': position})
                        except Exception as error:
                            anchor_errors.append(str(error))
                            return
                try:
                    for step in case['steps']:
                        if anchor_errors:
                            raise RuntimeError(f'Visual position setup failed: {anchor_errors}')
                        action = step['action']
                        if action == 'setup':
                            client.call('origins/setup', step['params'])
                            if case.get('hold_position') and anchor is None and 'position' in step['params']:
                                anchor = threading.Thread(target=hold_position, args=(step['params']['position'],), daemon=True)
                                anchor.start()
                        elif action == 'preview':
                            client.call('origins/preview', step['params'])
                        elif action == 'preview-pose':
                            schema = client.call('registry.schema', {'with_crates': ['bevy_transform']})
                            transforms = [name for name in schema if name.endswith('::Transform')]
                            if len(transforms) != 1:
                                raise ValueError('Preview pose requires one reflected Transform type')
                            previews = client.call('origins/assets')['previews']
                            if len(previews) != 1:
                                raise ValueError('Preview pose requires one active asset preview')
                            row.setdefault('preview_schemas', {})[transforms[0]] = schema[transforms[0]]
                            for field, value in step['fields'].items():
                                client.call('world.mutate_components', {'entity': previews[0]['entity'],
                                            'component': transforms[0], 'path': field, 'value': value})
                        elif action == 'input':
                            client.call('origins/input', {'events': step['events']})
                        elif action == 'release':
                            client.release()
                        elif action == 'wait':
                            state = client.wait_ticks(step['ticks'], timeout=45)
                            if state['state'] != 'Playing':
                                raise RuntimeError(f'Visual scenario left Playing: {state}')
                        elif action == 'materials':
                            row['material_states'].append(wait_materials(client, first=step.get('first', True),
                                                                         world=step.get('world', False), timeout=90,
                                                                         expected=step.get('expected', ())))
                        elif action == 'capture':
                            state = client.state()
                            if state['state'] != 'Playing':
                                raise RuntimeError(f'Visual capture left Playing: {state}')
                            row.setdefault('states', []).append(state)
                            row['captures'].append(capture(client, output/(case['id']+'-'+step['name']+'.png')))
                        else:
                            raise ValueError(f'Unsupported visual scenario action: {action}')
                finally:
                    stop.set()
                    if anchor is not None:
                        anchor.join(timeout=6)
                        if anchor.is_alive():
                            raise RuntimeError('Visual position setup thread did not stop')
                    client.release()
                atomic_json(output/'inspection.json', report)
            report['passed'] = True
        except Exception as error:
            report['error'] = str(error)
            try:
                report['failure_state'] = client.state()
            except Exception as observation_error:
                report['failure_observation_error'] = str(observation_error)
        finally:
            client.shutdown(report)
            try:
                process.wait(timeout=20)
            except subprocess.TimeoutExpired:
                report['passed'] = False
                report['process_exit_error'] = 'Owned validation process did not finish after AppExit'
            atomic_json(output/'inspection.json', report)
    return {'passed': report['passed'], 'cases': len(report['cases']), 'error': report.get('error')}


def inventory(root, master_roots):
    root = root.resolve()
    consumers, embedded, files = defaultdict(list), [], {}
    for path in sorted(root.rglob('*')):
        if path.is_file() and path.suffix.lower() in TEXTURES:
            uri = path.relative_to(root).as_posix()
            with path.open('rb') as stream:
                header = stream.read(148)
            if header.startswith(POINTER):
                raise ValueError(f'Unresolved LFS texture: {uri}')
            files[uri] = {'source_sha256': digest(path), 'source_bytes': path.stat().st_size,
                          'source_container': path.suffix.lower()[1:], 'canonical_sources': []}
            if path.suffix.lower() == '.dds':
                height, width = struct.unpack_from('<II', header, 12)
                levels = max(1, struct.unpack_from('<I', header, 28)[0])
                files[uri].update(width=width, height=height, mip_count=levels,
                                  complete_mips=levels == max(width, height).bit_length())

    def consume(uri, base, record):
        if not isinstance(uri, str):
            return
        if uri.startswith('data:image/'):
            embedded.append({**record, 'encoding': uri.partition(';')[0]})
        elif PurePosixPath(uri).suffix.lower() in TEXTURES:
            options = [uri, (base / uri).as_posix()]
            resolved = next((p for p in options if p in files), None)
            if resolved is None:
                raise ValueError(f'Missing texture dependency {uri!r} from {record}')
            consumers[resolved].append(record)

    def walk(value, base, scene, pointer=''):
        if isinstance(value, dict):
            if 'uri' in value and 'role' in value:
                consume(value['uri'], base, {'document': scene, 'pointer': pointer,
                    **{k: value[k] for k in ('role', 'layer', 'color_space', 'format', 'sampler', 'channels') if k in value}})
            for key, item in value.items():
                walk(item, base, scene, pointer + '/' + str(key))
        elif isinstance(value, list):
            for index, item in enumerate(value):
                walk(item, base, scene, pointer + '/' + str(index))
        elif isinstance(value, str):
            consume(value, base, {'document': scene, 'pointer': pointer})

    for path in sorted(root.rglob('*')):
        if path.suffix.lower() not in {'.gltf', '.json', '.glb'} or not path.is_file():
            continue
        raw = path.read_bytes()
        if raw.startswith(POINTER):
            raise ValueError(f'Unresolved LFS model/metadata: {path}')
        if path.suffix.lower() == '.glb':
            if raw[:4] != b'glTF' or len(raw) < 20:
                raise ValueError(f'Invalid GLB: {path}')
            size, kind = struct.unpack_from('<II', raw, 12)
            if kind != 0x4e4f534a:
                raise ValueError(f'Missing GLB JSON chunk: {path}')
            document = json.loads(raw[20:20+size])
        else:
            document = json.loads(raw)
        scene = path.relative_to(root).as_posix()
        for index, image in enumerate(document.get('images', [])):
            if 'bufferView' in image:
                embedded.append({'document': scene, 'image': index, 'encoding': image.get('mimeType'),
                                 'buffer_view': image['bufferView']})
        walk(document, path.relative_to(root).parent, scene)
        for material in document.get('materials', []):
            for field, role in (('normalTexture', 'normal'), ('occlusionTexture', 'response'),
                                ('emissiveTexture', 'emissive')):
                if field in material:
                    texture = document['textures'][material[field]['index']]
                    image = document['images'][texture['source']]
                    consume(image.get('uri'), path.relative_to(root).parent,
                            {'document': scene, 'material': material.get('name'), 'role': role,
                             'color_space': 'srgb' if role == 'emissive' else 'linear'})
            pbr = material.get('pbrMetallicRoughness', {})
            for field, role in (('baseColorTexture', 'color'), ('metallicRoughnessTexture', 'response')):
                if field in pbr:
                    texture = document['textures'][pbr[field]['index']]
                    image = document['images'][texture['source']]
                    consume(image.get('uri'), path.relative_to(root).parent,
                            {'document': scene, 'material': material.get('name'), 'role': role,
                             'color_space': 'srgb' if role == 'color' else 'linear',
                             'mask_cutoff': material.get('alphaCutoff', .5) if material.get('alphaMode') == 'MASK' else None})
            surface = material.get('extras', {}).get('surface', {})
            for channel in surface.get('channels', []):
                consume(channel.get('uri'), path.relative_to(root).parent,
                    {'document': scene, 'material': material.get('name'), 'family': surface.get('family'),
                     'presentation': surface.get('presentation'), 'provenance': surface.get('provenance'),
                     'native_weapon': surface.get('native_weapon'), **channel})

    for path in (Path(__file__).resolve().parents[1]/'src').rglob('*.rs'):
        for uri in re.findall(r'"([^"\r\n]+\.(?:dds|png|jpg|jpeg|ktx2))"', path.read_text(encoding='utf-8')):
            if uri in files:
                consumers[uri].append({'code': path.relative_to(Path(__file__).resolve().parents[1]).as_posix(),
                                       'role': 'sky' if uri.startswith('sky/') else 'direct-load',
                                       'color_space': 'linear' if uri.startswith('sky/') else 'unknown'})

    candidates = defaultdict(list)
    names = {Path(uri).name for uri in files}
    native_hashes = {record['source_sha256'] for uri, record in files.items() if Path(uri).name.startswith('native-')}
    for master_root in master_roots:
        for path in sorted(master_root.resolve().rglob('*')):
            if not path.is_file() or path.suffix.lower() not in TEXTURES:
                continue
            if path.name in names:
                candidates[path.name].append(path)
            if native_hashes and path.suffix.lower() == '.dds':
                sha256 = digest(path)
                if sha256 in native_hashes:
                    candidates['native-' + sha256 + '.dds'].append(path)
    for uri, record in files.items():
        record['consumers'] = consumers[uri]
        record['roles'] = sorted({c['role'] for c in consumers[uri] if 'role' in c})
        record['color_spaces'] = sorted({c['color_space'] for c in consumers[uri] if 'color_space' in c})
        for candidate in dict.fromkeys(candidates[Path(uri).name]):
            record['canonical_sources'].append({'path': str(candidate), 'sha256': digest(candidate)})
        record['missing_master'] = not record['canonical_sources']
        record['mip_owner'] = 'retained-authored' if Path(uri).name.startswith('native-') or uri.startswith('sky/') else 'prepared-semantic' if Path(uri).name.startswith('pbr-') else 'source-image'
    return {'version': 1, 'root': str(root), 'textures': files, 'embedded_images': embedded,
            'summary': {'textures': len(files), 'containers': dict(Counter(v['source_container'] for v in files.values())),
                        'missing_masters': sum(v['missing_master'] for v in files.values()),
                        'incomplete_chains': sum(v.get('complete_mips') is False for v in files.values()),
                        'unreferenced_files': sum(not v['consumers'] for v in files.values()),
                        'embedded_images': len(embedded)}}


def enrich_inventory(inventory_path, master_roots):
    result = json.loads(inventory_path.read_text(encoding='utf-8-sig'))
    aliases = defaultdict(set)
    for uri, record in result['textures'].items():
        aliases[Path(uri).name].add(uri)
        for consumer in record['consumers']:
            native = consumer.get('native_weapon') or {}
            for binding in native.get('bindings', []):
                if binding.get('uri') == uri:
                    aliases[binding['image']+'.dds'].add(uri)
    for master_root in master_roots:
        for path in master_root.resolve().rglob('*.dds'):
            if path.name not in aliases:
                continue
            raw = path.read_bytes()
            try:
                layout = inspect_dds(raw, allow_non_2d=True)
            except ValueError:
                continue
            for uri in aliases[path.name]:
                record = result['textures'][uri]
                exact = layout['sha256'] == record['source_sha256']
                semantic = Path(uri).name.startswith('pbr-') and layout['dxgi'] in (28, 29)
                dimensions = (record.get('width'), record.get('height')) == (layout['width'], layout['height'])
                if exact or (semantic and dimensions):
                    item = {'path': str(path), 'sha256': layout['sha256'],
                            'stage': 'retained-native-source' if exact else 'canonical-semantic-master',
                            'format': layout['dxgi'], 'mip_count': layout['mip_count']}
                    if item not in record['canonical_sources']:
                        record['canonical_sources'].append(item)
                    record['missing_master'] = False
    result['summary']['missing_masters'] = sum(r['missing_master'] for r in result['textures'].values())
    return result


def assemble(conversion_path, destination, identity_path):
    try:
        from .import_origins import validate_pack
    except ImportError:
        from import_origins import validate_pack
    conversion = json.loads(conversion_path.read_text(encoding='utf-8'))
    if not conversion['complete']:
        raise ValueError('A partial texture conversion cannot be assembled as a complete runtime pack')
    identity = json.loads(identity_path.read_text(encoding='utf-8'))
    source = Path(identity['asset_root']).resolve()
    destination = destination.resolve()
    if destination.exists() or destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError('Assembly requires a fresh external candidate directory')
    if destination.is_relative_to(Path(__file__).resolve().parents[1]):
        raise ValueError('Qualify the complete candidate outside the repository before import')
    names = {uri: str(PurePosixPath(uri).with_suffix('.ktx2')) for uri in conversion['textures']}
    if len(set(names.values())) != len(names):
        raise ValueError('Texture container migration creates a URI collision')

    def rewrite(value, base):
        if isinstance(value, dict):
            uri = value.get('uri')
            original = None
            if isinstance(uri, str):
                original = uri if uri in names else (base/uri).as_posix() if (base/uri).as_posix() in names else None
            result = {key: rewrite(item, base) for key, item in value.items()}
            if original:
                info = conversion['textures'][original]['output']
                for key in ('format', 'width', 'height', 'mip_count', 'sha256'):
                    if key in result:
                        result[key] = info[key]
            return result
        if isinstance(value, list):
            return [rewrite(item, base) for item in value]
        if isinstance(value, str):
            uri, marker, label = value.partition('#')
            if uri in names:
                return names[uri] + (marker+label if marker else '')
            relative = (base/uri).as_posix()
            if relative in names:
                return str(PurePosixPath(names[relative]).relative_to(base)) + (marker+label if marker else '')
        return value

    destination.mkdir(parents=True)
    for index, (uri, expected) in enumerate(identity['assets'].items()):
        path = source/uri
        if uri in names:
            record = conversion['textures'][uri]
            if expected['sha256'] != record['source']['source_sha256']:
                raise ValueError(f'Pinned source texture differs from the conversion inventory: {uri}')
            candidate = Path(record['candidate_path'])
            if digest(candidate) != record['output']['sha256']:
                raise ValueError(f'Qualified candidate changed: {uri}')
            output = destination/names[uri]
            output.parent.mkdir(parents=True, exist_ok=True)
            os.link(candidate, output)
        else:
            if path.suffix.lower() in TEXTURES:
                raise ValueError(f'Texture was omitted from the complete conversion: {uri}')
            if digest(path) != expected['sha256']:
                raise ValueError(f'Pinned nontexture input changed during assembly: {uri}')
            output = destination/uri
            output.parent.mkdir(parents=True, exist_ok=True)
            if path.suffix.lower() in ('.gltf', '.json'):
                document = rewrite(json.loads(path.read_bytes()), PurePosixPath(uri).parent)
                output.write_text(json.dumps(document, separators=(',', ':'))+'\n', encoding='utf-8')
            else:
                os.link(path, output)
        if index and index % 2000 == 0:
            print(f'Assembled {index} runtime assets', flush=True)
    _, _, closure = validate_pack(destination, exact=True, progress=True)
    if len(names) != sum(uri.endswith('.ktx2') for uri in closure):
        raise ValueError('Assembled texture inventory differs from validated dependency closure')
    return {'complete': True, 'textures': len(names), 'files': len(closure), 'sha256': closure,
            'source_identity_sha256': digest(identity_path), 'conversion_sha256': digest(conversion_path)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=('inventory', 'masters', 'process', 'convert', 'assemble', 'inspect-game'))
    parser.add_argument('--pack', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--masters', action='append', type=Path, default=[])
    parser.add_argument('--inventory', type=Path)
    parser.add_argument('--executable', type=Path)
    parser.add_argument('--select', action='append', default=[])
    parser.add_argument('--conversion', type=Path)
    parser.add_argument('--identity', type=Path)
    parser.add_argument('--recipes', type=Path)
    parser.add_argument('--cases', type=Path)
    parser.add_argument('--port', type=int, default=15708)
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(Path(__file__).resolve().parents[1]):
        parser.error('Generated outputs and reports must remain outside the repository')
    if args.pack and args.output.resolve().is_relative_to(args.pack.resolve()):
        parser.error('Reports must stay outside the runtime pack')
    if args.command == 'inventory':
        if not args.pack:
            parser.error('inventory requires --pack')
        result = inventory(args.pack, args.masters)
        atomic_json(args.output, result)
        print(json.dumps(result['summary']))
    elif args.command == 'masters':
        if not args.inventory or not args.masters:
            parser.error('masters requires --inventory and --masters')
        result = enrich_inventory(args.inventory, args.masters)
        atomic_json(args.output, result)
        print(json.dumps(result['summary']))
    elif args.command == 'process':
        if not args.recipes or not args.executable:
            parser.error('process requires --recipes and --executable')
        result = process_recipes(args.recipes, args.output, args.executable)
        print(json.dumps(result))
        if not result['passed']:
            raise SystemExit(1)
    elif args.command == 'convert':
        if not args.inventory or not args.executable:
            parser.error('convert requires --inventory and --executable')
        print(json.dumps(convert(args.inventory, args.output, args.executable, args.select)))
    elif args.command == 'inspect-game':
        if not args.identity or not args.cases:
            parser.error('inspect-game requires --identity and --cases')
        result = inspect_game(args.identity, args.cases, args.output, args.port)
        print(json.dumps(result))
        if not result['passed']:
            raise SystemExit(1)
    else:
        if not args.conversion or not args.identity:
            parser.error('assemble requires --conversion and --identity')
        result = assemble(args.conversion, args.output, args.identity)
        atomic_json(args.output.with_suffix('.validation.json'), result)
        print(json.dumps({k: v for k, v in result.items() if k != 'sha256'}))


if __name__ == '__main__':
    main()
