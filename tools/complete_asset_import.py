import argparse
import copy
import csv
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
from threading import Lock

import numpy as np

from prepare_actors import append_clips
from prepare_weapon_assemblies import transform
from xanim import decode

COPIED = set()


def accessor(document, binary, index):
    item = document['accessors'][index]
    view = document['bufferViews'][item['bufferView']]
    dtype = np.dtype({5121: 'u1', 5123: '<u2', 5125: '<u4', 5126: '<f4'}[item['componentType']])
    width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[item['type']]
    return np.ndarray((item['count'], width), dtype, binary,
                      view.get('byteOffset', 0) + item.get('byteOffset', 0),
                      (view.get('byteStride', width * dtype.itemsize), dtype.itemsize)).copy()


def append_accessor(document, binary, values):
    values = np.asarray(values, dtype='<f4')
    binary.extend(b'\0' * (-len(binary) % 4))
    view = len(document['bufferViews'])
    document['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': values.nbytes})
    binary.extend(values.tobytes())
    index = len(document['accessors'])
    document['accessors'].append({'bufferView': view, 'componentType': 5126, 'count': len(values), 'type': 'VEC4'})
    return index


def replace_attribute(document, binary, index, values):
    if index is None:
        return append_accessor(document, binary, values)
    item = document['accessors'][index]
    view = document['bufferViews'][item['bufferView']]
    values = np.asarray(values, dtype='<f4')
    if item['componentType'] != 5126 or item['type'] != 'VEC4' or item['count'] != len(values) or view.get('byteStride', 16) != 16:
        return append_accessor(document, binary, values)
    offset = view.get('byteOffset', 0) + item.get('byteOffset', 0)
    binary[offset:offset + values.nbytes] = values.tobytes()
    for key, value in (('min', values.min(axis=0)), ('max', values.max(axis=0))):
        if key in item:
            item[key] = value.tolist()
    return index


def unpack_direction(words):
    values = np.stack([(words >> shift & 1023).astype(np.int32) for shift in (0, 10, 20)], axis=1)
    values -= 2 * (values & 512)
    lengths = np.linalg.norm(values, axis=1, keepdims=True)
    return np.divide(values, lengths, out=np.zeros_like(values, dtype=float), where=lengths > 0)

def write_json(path, document):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, separators=(',', ':')) + '\n', encoding='utf-8')


def copy_file(source, target):
    key = (source, target)
    if key in COPIED:
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    if not target.exists():
        shutil.copyfile(source, target)
    elif digest(source) != digest(target):
        raise ValueError('Asset identity collision: ' + str(target))
    COPIED.add(key)


class Importer:
    def __init__(self, inspection, output, base):
        self.inspection, self.output = inspection, output
        self.pack = output / 'pack'
        if not self.pack.exists():
            shutil.copytree(base, self.pack)
        self.game = json.loads((self.pack / 'game.json').read_bytes())
        self.library = inspection / 'bo2-library-prepared'
        self.manifest = json.loads((self.library / 'library.json').read_bytes())
        self.models = {}
        priority = {'zm_tomb_patch': 5, 'zm_tomb': 4, 'patch_zm': 3, 'common_zm': 2}
        for model in sorted(self.manifest['models'], key=lambda m: priority.get(m['zone'].split('/')[-1], 0)):
            self.models[model['identity']] = model
        self.materials = {}
        active = [base / 'map.gltf'] + sorted((base / 'actors').glob('*.gltf')) + sorted((base / 'props').glob('*.gltf'))
        for path in active:
            for material in json.loads(path.read_bytes()).get('materials', []):
                self.materials.setdefault(material['name'], material)
        self.report = {'model_entries': 0, 'model_lods': 0, 'map_models': 0,
                       'map_lods': 0, 'actor_scenes': [], 'animations': []}
        if (output / 'coverage.json').exists():
            self.report = json.loads((output / 'coverage.json').read_bytes())

    def distances(self, model):
        path = self.inspection / 'bo2-library-extraction' / model['zone'] / 'xmodel' / (model['identity'] + '.json')
        native = json.loads(path.read_bytes())
        values = [float(v['distance']) * .0254 for v in native['lods'][:-1]]
        if len(values) != model['source_lods'] - 1 or any(b < a for a, b in zip([0.] + values, values)):
            raise ValueError('Invalid authored LOD distances: ' + model['identity'])
        return values

    def source(self, model, level):
        return self.library / next(v['uri'] for v in model['variants'] if v['lod'] == level)

    def runtime_source(self, model, level):
        identity = hashlib.sha256((model['zone'] + '/' + model['identity']).encode()).hexdigest()[:24]
        variant = Path(next(v['uri'] for v in model['variants'] if v['lod'] == level))
        return self.pack / 'library' / variant.parent / identity / variant.name

    def library_material(self, material):
        material = copy.deepcopy(material)
        for key in ('normalTexture', 'occlusionTexture', 'emissiveTexture'):
            material.pop(key, None)
        for key in ('baseColorTexture', 'metallicRoughnessTexture'):
            material.get('pbrMetallicRoughness', {}).pop(key, None)
        for channel in material.get('extras', {}).get('surface', {}).get('channels', []):
            relative = channel['uri']
            channel['uri'] = 'library/' + relative
            copy_file(self.library / relative, self.pack / channel['uri'])
        return material

    def catalog(self):
        self.report['model_entries'], self.report['model_lods'] = 0, 0
        presentations = self.game['content']['presentations']
        presentations[:] = [p for p in presentations if not p['id'].startswith('source.model.')]
        for model in self.manifest['models']:
            for variant in model['variants']:
                source = self.library / variant['uri']
                target = self.runtime_source(model, variant['lod'])
                relative = target.relative_to(self.pack).as_posix()
                document = json.loads(source.read_bytes())
                document['materials'] = [self.library_material(m) for m in document.get('materials', [])]
                document['textures'], document['images'], document['samplers'] = [], [], []
                for buffer in document.get('buffers', []):
                    copy_file(source.parent / buffer['uri'], target.parent / buffer['uri'])
                write_json(target, document)
                identity = hashlib.sha256((model['zone'] + '/' + model['identity']).encode()).hexdigest()[:24]
                presentations.append({'id': 'source.model.' + identity + '.lod' + str(variant['lod']),
                                      'scene': relative + '#Scene0', 'animations': [], 'sounds': []})
                self.report['model_lods'] += 1
            self.report['model_entries'] += 1
            if self.report['model_entries'] % 250 == 0:
                print('Runtime model catalog: ' + str(self.report['model_entries']), flush=True)

    def add_geometry(self, target, binary, source_path, material_map, joint_map=None, placement=None, skip_materials=()):
        source = json.loads(source_path.read_bytes())
        buffers = [(source_path.parent / b['uri']).read_bytes() for b in source['buffers']]
        accessor_map = {}
        positions = {p['attributes']['POSITION'] for m in source['meshes'] for p in m['primitives']}
        vectors = {p['attributes'][key] for m in source['meshes'] for p in m['primitives']
                   for key in ('NORMAL', 'TANGENT') if key in p['attributes']}
        dtypes = {5120: 'i1', 5121: 'u1', 5122: '<i2', 5123: '<u2', 5125: '<u4', 5126: '<f4'}
        widths = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}
        for index, old in enumerate(source['accessors']):
            view = source['bufferViews'][old['bufferView']]
            dtype, width = np.dtype(dtypes[old['componentType']]), widths[old['type']]
            values = np.ndarray((old['count'], width), dtype, buffers[view['buffer']],
                view.get('byteOffset', 0) + old.get('byteOffset', 0),
                (view.get('byteStride', width * dtype.itemsize), dtype.itemsize)).copy()
            accessor = copy.deepcopy(old)
            accessor.pop('byteOffset', None)
            if placement is not None and index in positions:
                values = (np.column_stack((values, np.ones(len(values)))) @ placement.T)[:, :3].astype('<f4')
                accessor['min'], accessor['max'] = values.min(axis=0).tolist(), values.max(axis=0).tolist()
            elif placement is not None and index in vectors:
                values[:, :3] = values[:, :3] @ np.linalg.inv(placement[:3, :3])
                values[:, :3] /= np.linalg.norm(values[:, :3], axis=1, keepdims=True)
            binary.extend(b'\0' * (-len(binary) % 4))
            accessor['bufferView'] = len(target['bufferViews'])
            target['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': values.nbytes})
            binary.extend(values.tobytes())
            accessor_map[index] = len(target['accessors'])
            target['accessors'].append(accessor)
        globals_ = {}
        def visit(index, parent):
            globals_[index] = parent @ transform(target['nodes'][index])
            for child in target['nodes'][index].get('children', []):
                visit(child, globals_[index])
        if joint_map is not None:
            for index in target['scenes'][target.get('scene', 0)]['nodes']:
                visit(index, np.eye(4))
        skin_map = {}
        for index, old in enumerate(source.get('skins', [])):
            if joint_map is None:
                continue
            joints = [joint_map[source['nodes'][j]['name']] for j in old['joints']]
            matrices = np.asarray([np.linalg.inv(globals_[j]).T.flatten() for j in joints], dtype='<f4')
            binary.extend(b'\0' * (-len(binary) % 4))
            view = len(target['bufferViews'])
            target['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': matrices.nbytes})
            binary.extend(matrices.tobytes())
            accessor = len(target['accessors'])
            target['accessors'].append({'bufferView': view, 'componentType': 5126, 'count': len(joints), 'type': 'MAT4'})
            skin_map[index] = len(target.setdefault('skins', []))
            target['skins'].append({'joints': joints, 'inverseBindMatrices': accessor})
        result = []
        for old in source['nodes']:
            if 'mesh' not in old:
                continue
            primitives = []
            for primitive in source['meshes'][old['mesh']]['primitives']:
                material = source['materials'][primitive['material']]
                if material['name'] in skip_materials:
                    continue
                if material['name'] not in material_map:
                    cooked = self.materials.get(material['name'])
                    if cooked is None:
                        cooked = self.library_material(material)
                    material_map[material['name']] = len(target['materials'])
                    target['materials'].append(copy.deepcopy(cooked))
                p = copy.deepcopy(primitive)
                p['attributes'] = {key: accessor_map[value] for key, value in p['attributes'].items()
                                   if joint_map is not None or not key.startswith(('JOINTS_', 'WEIGHTS_'))}
                p['indices'] = accessor_map[p['indices']]
                p['material'] = material_map[material['name']]
                primitives.append(p)
            if not primitives:
                continue
            node = {key: copy.deepcopy(old[key]) for key in ('name', 'translation', 'rotation', 'scale', 'matrix') if key in old}
            node['mesh'] = len(target['meshes'])
            target['meshes'].append({'name': source['meshes'][old['mesh']].get('name', old.get('name', 'lod')), 'primitives': primitives})
            if 'skin' in old and joint_map is not None:
                node['skin'] = skin_map[old['skin']]
            result.append(node)
        return result

    def map_lods(self):
        path = self.pack / 'map.gltf'
        doc = json.loads(path.read_bytes())
        doc['asset']['generator'] = doc['asset'].get('generator', 'Origins').removesuffix('; native packed vertex frames v2')
        binary = bytearray((path.parent / doc['buffers'][0]['uri']).read_bytes())
        scene = doc['scenes'][0]['nodes']
        scene[:] = [i for i in scene if all(doc['nodes'][i].get('extras', {}).get(kind, {}).get('level', 0) == 0
                                           for kind in ('prop_lod', 'mesh_lod'))]
        self.report['map_models'], self.report['map_lods'] = 0, 0
        groups = {}
        for index in scene:
            node = doc['nodes'][index]
            extras = node.get('extras', {})
            extras.pop('prop_lod', None)
            if extras.get('static_instance') and 'mesh' in node:
                groups.setdefault(extras['source_model'], []).append(index)
        materials = {m['name']: i for i, m in enumerate(doc['materials'])}
        for name, indices in sorted(groups.items()):
            model = self.models[name]
            distances = self.distances(model)
            placements, emitting = {}, set()
            for index in indices:
                node = doc['nodes'][index]
                if node['extras']['fixture_emitter']:
                    emitting.update(doc['materials'][p['material']]['name'] for p in doc['meshes'][node['mesh']]['primitives'])
                    continue
                if distances:
                    node['extras']['mesh_lod'] = {'level': 0, 'distances': distances}
                placement = {k: node[k] for k in ('matrix', 'translation', 'rotation', 'scale') if k in node}
                placements[json.dumps(placement, sort_keys=True)] = placement
            for level in range(1, model['source_lods']):
                nodes = self.add_geometry(doc, binary, self.source(model, level), materials, skip_materials=emitting)
                for placement in placements.values():
                    for node in nodes:
                        placed = copy.deepcopy(node)
                        placed.update(placement)
                        placed['name'] = 'static_instance'
                        placed['extras'] = {'static_instance': True, 'source_model': name, 'fixture_emitter': False,
                                            'mesh_lod': {'level': level, 'distances': distances}}
                        scene.append(len(doc['nodes']))
                        doc['nodes'].append(placed)
            self.report['map_models'] += 1
            self.report['map_lods'] += model['source_lods']
        doc['buffers'][0]['byteLength'] = len(binary)
        (path.parent / doc['buffers'][0]['uri']).write_bytes(binary)
        write_json(path, doc)
        self.game['render_meshes'] = sum('mesh' in doc['nodes'][i] for i in scene)
        print('Complete map LODs: ' + str(self.report['map_lods']), flush=True)

    def actor_lods(self):
        paths = list((self.pack / 'actors').glob('*.gltf')) + list((self.pack / 'props').glob('*.gltf'))
        for path in sorted(paths):
            doc = json.loads(path.read_bytes())
            initial_nodes = len(doc['nodes'])
            model_name = path.stem.removesuffix('_lod0')
            if path.stem.endswith('_lod0') and model_name in self.models:
                components = [(model_name, None)]
            else:
                components = sorted({(n['name'].split('_lod0_')[0], n['name'].split('_lod0_')[0] + '_lod0_')
                    for n in doc['nodes'] if 'mesh' in n and '_lod0_' in n.get('name', '')
                    and n.get('extras', {}).get('mesh_lod', {}).get('level', 0) == 0})
            if not components:
                continue
            if any(name not in self.models for name, _ in components):
                raise ValueError('Actor component omitted from native model inventory: ' + str(components))
            levels = max(self.models[name]['source_lods'] for name, _ in components)
            if levels == 1:
                continue
            entry = {'uri': path.relative_to(self.pack).as_posix(), 'levels': levels,
                     'models': [n for n, _ in components]}
            if any(n.get('extras', {}).get('mesh_lod', {}).get('level', 0) > 0 for n in doc['nodes']):
                if entry not in self.report['actor_scenes']:
                    self.report['actor_scenes'].append(entry)
                continue
            distances = self.distances(max((self.models[name] for name, _ in components), key=lambda m: m['source_lods']))
            doc['asset']['generator'] = doc['asset'].get('generator', 'Origins').removesuffix('; native packed vertex frames v2')
            binary_path = path.parent / doc['buffers'][0]['uri']
            binary = bytearray(binary_path.read_bytes())
            materials = {m['name']: i for i, m in enumerate(doc['materials'])}
            joints = {doc['nodes'][j]['name']: j for s in doc.get('skins', []) for j in s['joints']}
            for n in doc['nodes']:
                if 'mesh' in n:
                    n.setdefault('extras', {})['mesh_lod'] = {'level': 0, 'distances': distances}
            for name, prefix in components:
                model = self.models[name]
                source0 = self.source(model, 0)
                original = json.loads(source0.read_bytes())
                current_meshes = [n['mesh'] for n in doc['nodes'][:initial_nodes] if 'mesh' in n and (prefix is None or n.get('name', '').startswith(prefix))]
                placement = self.alignment(doc, binary, current_meshes, source0, original)
                for level in range(1, levels):
                    nodes = self.add_geometry(doc, binary, self.source(model, min(level, model['source_lods'] - 1)),
                                              materials, joints if joints else None, placement)
                    for node in nodes:
                        node['name'] = name + '_lod' + str(level) + '_' + node.get('name', 'mesh')
                        node['extras'] = {'mesh_lod': {'level': level, 'distances': distances}}
                        doc['scenes'][doc.get('scene', 0)]['nodes'].append(len(doc['nodes']))
                        doc['nodes'].append(node)
            doc['buffers'][0]['byteLength'] = len(binary)
            binary_path.write_bytes(binary)
            write_json(path, doc)
            if entry not in self.report['actor_scenes']:
                self.report['actor_scenes'].append(entry)
            print('Actor LODs: ' + path.name + ' (' + str(levels) + ')', flush=True)

    def alignment(self, doc, binary, meshes, source_path, source):
        def positions(document, index, buffers):
            a = document['accessors'][index]
            v = document['bufferViews'][a['bufferView']]
            return np.ndarray((a['count'], 3), '<f4', buffers[v['buffer']],
                v.get('byteOffset', 0) + a.get('byteOffset', 0), (v.get('byteStride', 12), 4)).copy()
        buffers = [(source_path.parent / b['uri']).read_bytes() for b in source['buffers']]
        targets = [p for i in meshes for p in doc['meshes'][i]['primitives']]
        for mesh in source['meshes']:
            for primitive in mesh['primitives']:
                points = positions(source, primitive['attributes']['POSITION'], buffers)
                name = source['materials'][primitive['material']]['name']
                for target in targets:
                    if doc['materials'][target['material']]['name'] != name:
                        continue
                    destination = positions(doc, target['attributes']['POSITION'], [binary])
                    if len(points) != len(destination):
                        source_indices = accessor(source, buffers[0], primitive['indices']).flatten()
                        target_indices = accessor(doc, binary, target['indices']).flatten()
                        if len(source_indices) != len(target_indices):
                            continue
                        matched_points = points[source_indices]
                        destination = destination[target_indices]
                    else:
                        matched_points = points
                    indices = np.linspace(0, len(matched_points) - 1, min(len(matched_points), 128), dtype=int)
                    a, b = matched_points[indices].astype(float), destination[indices].astype(float)
                    ca, cb = a.mean(axis=0), b.mean(axis=0)
                    u, _, vt = np.linalg.svd((a-ca).T @ (b-cb))
                    rotation = vt.T @ u.T
                    if np.linalg.det(rotation) < 0:
                        vt[-1] *= -1
                        rotation = vt.T @ u.T
                    result = np.eye(4)
                    result[:3, :3] = rotation
                    result[:3, 3] = cb - rotation @ ca
                    if np.max(np.abs(matched_points @ rotation.T + result[:3, 3] - destination)) < .0003:
                        return result
        raise ValueError('Cannot preserve existing actor component placement: ' + str(source_path))

    def animations(self, records):
        self.report['animations'] = []
        content = self.game['content']
        presentations = content['presentations']
        presentations[:] = [p for p in presentations if not p['id'].startswith('source.animation.')]
        content['animations'][:] = [a for a in content['animations'] if not a['id'].startswith('animation.source.')]
        by_hash = {}
        for record in records:
            by_hash.setdefault(record['sha256'], []).append(record)
        for index, (sha, sources) in enumerate(by_hash.items()):
            record = sources[0]
            source = self.output / record['uri']
            clip = decode(source.read_bytes())
            nodes = [{'name': 'native_animation', 'children': list(range(1, len(clip['tracks']) + 1))}]
            nodes.extend({'name': name} for name in clip['tracks'])
            doc = {'asset': {'version': '2.0', 'generator': 'Origins complete native animation import'},
                   'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': nodes,
                   'meshes': [], 'materials': [], 'accessors': [], 'bufferViews': [],
                   'skins': [{'joints': list(range(1, len(nodes)))}] if len(nodes) > 1 else [],
                   'extras': {'native_animation': {'source_sha256': sha, 'sources': [{'zone': r['zone'], 'name': r['name']} for r in sources],
                              'asset_type': clip['asset_type'], 'looped': clip['looped'], 'duration': clip['duration'],
                              'channel_presence': clip['channel_presence'], 'notifications': clip['notifications']}}}
            binary = bytearray()
            tracks = clip['tracks']
            saved_delta = clip['delta']
            if not tracks:
                nodes.append({'name': 'native_timeline'})
                nodes[0]['children'].append(1)
                doc['skins'] = [{'joints': [1]}]
            staging = self.output / 'animation-input' / sha / record['name']
            staging.parent.mkdir(parents=True, exist_ok=True)
            copy_file(source, staging)
            notes = append_clips(doc, binary, [staging], preserve_native=True)
            if not notes:
                doc['animations'] = [{'name': record['name'], 'channels': [], 'samplers': []}]
            animation = doc['animations'][0]
            def add_track(node, target, times, values):
                pair = []
                for data, kind in ((np.asarray(times, dtype='<f4').reshape(-1, 1), 'SCALAR'),
                                   (np.asarray(values, dtype='<f4'), 'VEC4' if target == 'rotation' else 'VEC3')):
                    binary.extend(b'\0' * (-len(binary) % 4))
                    view = len(doc['bufferViews'])
                    doc['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': data.nbytes})
                    binary.extend(data.tobytes())
                    pair.append(len(doc['accessors']))
                    doc['accessors'].append({'bufferView': view, 'componentType': 5126, 'count': len(data), 'type': kind,
                                             'min': data.min(axis=0).tolist(), 'max': data.max(axis=0).tolist()})
                animation['channels'].append({'sampler': len(animation['samplers']), 'target': {'node': node, 'path': target}})
                animation['samplers'].append({'input': pair[0], 'output': pair[1], 'interpolation': 'LINEAR'})
            if saved_delta:
                delta = len(nodes)
                nodes.append({'name': 'native_root_motion'})
                nodes[0]['children'].append(delta)
                for target, (times, values) in saved_delta.items():
                    if target == 'translation':
                        values = values * .0254
                    add_track(delta, target, times, values)
            if not animation['channels']:
                add_track(1, 'translation', [0., clip['duration']], [[0., 0., 0.], [0., 0., 0.]])
            aliases = sorted({r['name'] for r in sources}, key=str.casefold)
            doc['animations'] = [dict(animation, name=name) for name in aliases]
            relative = 'animations/' + sha + '.gltf'
            doc['buffers'] = [{'uri': sha + '.bin', 'byteLength': len(binary)}]
            write_json(self.pack / relative, doc)
            (self.pack / 'animations' / (sha + '.bin')).write_bytes(binary)
            identity = 'source.animation.' + sha
            ids = []
            for alias_index, name in enumerate(aliases):
                animation_id = 'animation.source.' + sha + '.' + str(alias_index)
                ids.append(animation_id)
                content['animations'].append({'id': animation_id, 'path': relative + '#Animation' + str(alias_index),
                                              'skeleton': identity, 'notifications': [n for n in clip['notifications'] if n['event']]})
            presentations.append({'id': identity, 'scene': relative + '#Scene0', 'animations': ids, 'sounds': []})
            self.report['animations'].append({'sha256': sha, 'sources': sources, 'path': relative})
            if index % 250 == 0:
                print('Runtime animations: ' + str(index + 1) + '/' + str(len(by_hash)), flush=True)

    def finish(self):
        scene_path, label = self.game['scene'].split('#')
        document = json.loads((self.pack / scene_path).read_bytes())
        pending = list(document['scenes'][int(label.removeprefix('Scene'))]['nodes'])
        visited = set()
        while pending:
            index = pending.pop()
            if index in visited:
                raise ValueError('Multiply parented runtime map node')
            visited.add(index)
            pending.extend(document['nodes'][index].get('children', []))
        self.game['render_meshes'] = sum('mesh' in document['nodes'][i] for i in visited)
        self.report['model_entries'] = len(self.manifest['models'])
        self.report['model_lods'] = sum(len(m['variants']) for m in self.manifest['models'])
        if 'native_model_attributes' in self.report:
            self.report['native_model_attributes'] = self.report['model_lods']
        unique = {}
        for presentation in self.game['content']['presentations']:
            previous = unique.setdefault(presentation['id'], presentation)
            if previous != presentation:
                raise ValueError('Conflicting source presentation: ' + presentation['id'])
        self.game['content']['presentations'] = list(unique.values())
        for animation in self.game['content']['animations']:
            if animation['id'].startswith('animation.source.'):
                animation['notifications'] = [n for n in animation['notifications'] if n['event']]
        write_json(self.pack / 'game.json', self.game)
        write_json(self.output / 'coverage.json', self.report)

    def complete_frames(self, verify_only=False, catalog=True, scene_prefix=None):
        zones = {}
        cache = {}
        zone_lock = Lock()

        def source_frames(model, level):
            key = model['zone'], model['identity'], level
            if key in cache:
                return cache[key]
            with zone_lock:
                if model['zone'] not in zones:
                    folder = self.output / 'packed-frames' / model['zone']
                    zones[model['zone']] = (json.loads((folder / 'model-lod-frames.json').read_bytes()),
                                            (folder / 'model-lod-frames.bin').read_bytes())
            metadata, raw = zones[model['zone']]
            surfaces = metadata[model['identity']][level]['surfaces']
            path = self.source(model, level)
            doc = json.loads(path.read_bytes())
            binary = (path.parent / doc['buffers'][0]['uri']).read_bytes()
            primitives = [p for m in doc['meshes'] for p in m['primitives']]
            if len(primitives) != len(surfaces):
                raise ValueError('Native surface count mismatch: ' + str(key))
            vertices, used, base = [], [], 0
            for primitive, surface in zip(primitives, surfaces):
                def material_identity(name):
                    name = name.lstrip(',')
                    return name[4:] if name.startswith('mlv/') else name[3:] if name.startswith('mc/') else name
                if material_identity(doc['materials'][primitive['material']]['name']) != material_identity(surface['material']):
                    raise ValueError('Native material correspondence mismatch: ' + str(key))
                indices = accessor(doc, binary, primitive['indices']).flatten().astype(np.int64) - base
                native_indices = np.frombuffer(raw, '<u2', surface['triangles'] * 3, surface['index_offset'])
                if not np.array_equal(np.sort(indices), np.sort(native_indices)):
                    raise ValueError('Native topology mismatch: ' + str(key))
                vertices.append(np.frombuffer(raw, dtype=[('tangent', '<u4'), ('sign', '<f4'),
                    ('color', '<u4'), ('normal', '<u4'), ('position', '<f4', (3,))], count=surface['vertices'], offset=surface['vertex_offset']))
                used.extend((indices + base).tolist())
                base += surface['vertices']
            vertex = np.concatenate(vertices)
            used = np.unique(used)
            normal = accessor(doc, binary, primitives[0]['attributes']['NORMAL'])
            native_normal = unpack_direction(vertex['normal'])
            if len(normal) != len(vertex):
                raise ValueError('Native vertex count mismatch: ' + str(key))
            positions = accessor(doc, binary, primitives[0]['attributes']['POSITION'])
            bases = [np.array([[1., 0., 0.], [0., 0., -1.], [0., 1., 0.]]), np.eye(3)]
            rotation = min(bases, key=lambda basis: np.max(np.abs(vertex['position'] @ basis * .0254 - positions), initial=0))
            if np.max(np.abs(native_normal[used] @ rotation - normal[used]), initial=0) > .005:
                raise ValueError('Native coordinate basis mismatch: ' + str(key))
            tangent = unpack_direction(vertex['tangent']) @ rotation
            sign = np.sign(vertex['sign'])
            if (np.linalg.norm(tangent[used], axis=1) < .9).any() or (sign[used] == 0).any():
                raise ValueError('Invalid referenced native tangent: ' + str(key))
            unused = np.linalg.norm(tangent, axis=1) < .9
            tangent[unused] = [1., 0., 0.]
            sign[sign == 0] = 1
            tangent = np.column_stack((tangent, sign)).astype('<f4')
            colors = np.stack([(vertex['color'] >> shift & 255) / 255 for shift in (0, 8, 16, 24)], axis=1).astype('<f4')
            if not np.allclose(vertex['position'] @ rotation * .0254, positions, atol=.00002, rtol=.000002):
                raise ValueError('Native vertex position mismatch: ' + str(key))
            cache[key] = positions, tangent, colors
            return cache[key]

        def update(path, selections):
            doc = json.loads(path.read_bytes())
            marker = '; native packed vertex frames v2'
            if verify_only:
                for _, model, level in selections(doc):
                    source_frames(model, level)
                return
            if doc['asset'].get('generator', '').endswith(marker):
                return
            binary_path = path.parent / doc['buffers'][0]['uri']
            binary = bytearray(binary_path.read_bytes())
            done = {}
            for mesh, model, level in selections(doc):
                points, tangents, colors = source_frames(model, level)
                for primitive in doc['meshes'][mesh]['primitives']:
                    position = primitive['attributes']['POSITION']
                    key = position, model['zone'], model['identity'], level
                    if key not in done:
                        target = accessor(doc, binary, position)
                        if len(target) != len(points):
                            raise ValueError('Native attribute vertex count mismatch: ' + str(path))
                        a, b = points.astype(float), target.astype(float)
                        rotation = np.eye(3)
                        if not np.array_equal(a, b):
                            u, _, vt = np.linalg.svd((a-a.mean(axis=0)).T @ (b-b.mean(axis=0)))
                            rotation = u @ vt
                            if np.linalg.det(rotation) < 0:
                                u[:, -1] *= -1
                                rotation = u @ vt
                        if np.max(np.abs((a-a.mean(axis=0)) @ rotation - (b-b.mean(axis=0))), initial=0) > .0003:
                            raise ValueError('Native attribute placement mismatch: ' + str(path))
                        values = tangents.copy()
                        values[:, :3] = values[:, :3] @ rotation
                        done[key] = (replace_attribute(doc, binary, primitive['attributes'].get('TANGENT'), values),
                                     replace_attribute(doc, binary, primitive['attributes'].get('COLOR_0'), colors))
                    primitive['attributes']['TANGENT'], primitive['attributes']['COLOR_0'] = done[key]
            doc['buffers'][0]['byteLength'] = len(binary)
            doc['asset']['generator'] = doc['asset'].get('generator', 'Origins').replace('; native packed vertex frames', '') + marker
            binary_path.write_bytes(binary)
            write_json(path, doc)

        def complete_model(model):
            for variant in model['variants']:
                update(self.runtime_source(model, variant['lod']),
                       lambda doc, m=model, level=variant['lod']: [(i, m, level) for i in range(len(doc['meshes']))])
        unique_models = {(m['zone'], m['identity']): m for m in self.manifest['models']}
        if catalog:
            with ThreadPoolExecutor(max_workers=8) as pool:
                for index, _ in enumerate(pool.map(complete_model, unique_models.values())):
                    if index % 250 == 0:
                        print('Native model attributes: ' + str(index + 1), flush=True)
        if scene_prefix is None:
            update(self.pack / 'map.gltf', lambda doc: [
                (n['mesh'], self.models[n['extras']['source_model']], n['extras']['mesh_lod']['level'])
                for n in doc['nodes'] if 'mesh' in n and n.get('extras', {}).get('mesh_lod', {}).get('level', 0) > 0])
        for entry in self.report['actor_scenes']:
            if scene_prefix is not None and not entry['uri'].startswith(scene_prefix):
                continue
            def actor_selection(doc, entry=entry):
                result = []
                for node in doc['nodes']:
                    level = node.get('extras', {}).get('mesh_lod', {}).get('level', 0)
                    if 'mesh' not in node or level == 0:
                        continue
                    name = next(n for n in entry['models'] if node['name'].startswith(n + '_lod' + str(level) + '_'))
                    model = self.models[name]
                    result.append((node['mesh'], model, min(level, model['source_lods'] - 1)))
                return result
            update(self.pack / entry['uri'], actor_selection)
        self.report['native_model_attributes'] = self.report['model_lods']
        self.report['native_vertex_positions'] = self.report['model_lods']

    def close_textures(self):
        referenced = {p.split('#', 1)[0] for kind in ('ui_assets', 'sounds')
                      for item in self.game['content'].get(kind, [])
                      for p in [item.get('path', '')] if p}
        referenced.update(p.split('#', 1)[0] for item in self.game['content'].get('cinematics', [])
                          for p in item['frames'])
        def references(path):
            doc = json.loads(path.read_bytes())
            found = {(path.parent / i['uri']).relative_to(self.pack).as_posix()
                     for i in doc.get('images', []) if 'uri' in i}
            found.update(c['uri'] for m in doc.get('materials', [])
                         for c in m.get('extras', {}).get('surface', {}).get('channels', []))
            return found
        with ThreadPoolExecutor(max_workers=8) as pool:
            for found in pool.map(references, self.pack.rglob('*.gltf')):
                referenced.update(found)
        camos = self.pack / 'weapon_camos.json'
        if camos.exists():
            referenced.update(c['uri'] for s in json.loads(camos.read_bytes())['surfaces'].values()
                              for c in s['channels'])
        retired = []
        for path in (self.pack / 'textures').rglob('*'):
            if path.is_file() and path.relative_to(self.pack).as_posix() not in referenced:
                relative = path.relative_to(self.pack)
                target = self.output / 'retired-runtime' / relative
                copy_file(path, target)
                path.unlink()
                retired.append(relative.as_posix())
        write_json(self.output / 'retired-textures.json', retired)
        print('Archived unreferenced prepared textures: ' + str(len(retired)), flush=True)


def retain_animations(output, workspace):
    records = json.loads((output / 'extraction.json').read_bytes())
    root = workspace / 'assets/source/animations'
    catalog = []
    for record in records:
        relative = record['sha256'] + '.xanim'
        copy_file(output / record['uri'], root / relative)
        catalog.append({key: record[key] for key in ('zone', 'name', 'sha256')} | {'uri': relative})
    write_json(root / 'catalog.json', {'version': 1, 'animations': catalog})


def extract_frames(installation, inspection, output):
    from export_instances import export
    tooling = inspection / 'tooling/OAT/surface-7d027e8/source'
    reader_root = output / 'frame-reader'
    callback = Path(__file__).with_name('model_lod_frames.cpp')
    export(tooling, installation / 'zone/all/code_post_gfx_zm.ff', reader_root, callback=callback)
    expected_callback = digest(callback)
    for zone in sorted((installation / 'zone').rglob('*.ff')):
        target = output / 'packed-frames' / zone.relative_to(installation).with_suffix('')
        target.mkdir(parents=True, exist_ok=True)
        expected = {'source_sha256': digest(zone), 'callback_sha256': expected_callback}
        stamp = target / 'receipt.json'
        if not stamp.exists() or json.loads(stamp.read_bytes()) != expected:
            with (target / 'export.log').open('wb') as log:
                subprocess.run([str(reader_root / 'Unlinker.exe'), '--no-color', '--list', str(zone)],
                               cwd=target, stdout=log, stderr=subprocess.STDOUT, check=True)
            write_json(stamp, expected)
        print('Authored LOD frames: ' + zone.stem, flush=True)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def extract(installation, inspection, output):
    unlinker = inspection / 'tooling/OAT/v0.33.0/Unlinker.exe'
    records = []
    for zone in sorted((installation / 'zone').rglob('*.ff')):
        identity = zone.relative_to(installation).with_suffix('').as_posix()
        target = output / 'source' / identity
        target.mkdir(parents=True, exist_ok=True)
        stamp = target / 'receipt.json'
        expected = {'source_sha256': digest(zone), 'reader_sha256': digest(unlinker)}
        if not stamp.is_file() or json.loads(stamp.read_bytes()) != expected:
            with (target / 'extract.log').open('wb') as log:
                subprocess.run([str(unlinker), '--no-color', '--include-assets', 'xanim',
                                '--output-folder', str(target), str(zone)],
                               stdout=log, stderr=subprocess.STDOUT, check=True)
            text = (target / 'extract.log').read_text(errors='replace')
            if 'ERROR:' in text or '0 errors' not in text:
                raise ValueError('Animation extraction failed: ' + identity)
            stamp.write_text(json.dumps(expected, indent=2) + '\n')
        listing = inspection / 'bo2-library-extraction' / identity / 'zone_source' / (zone.stem + '.zone')
        names = {row[1] for row in csv.reader(listing.read_text(encoding='utf-8-sig').splitlines())
                 if len(row) > 1 and row[0] == 'xanim' and row[1]}
        recovered = {p.name.casefold(): p for p in (target / 'xanim').rglob('*') if p.is_file()}
        for name in sorted(names):
            path = recovered.get(name.casefold())
            if path is None:
                raise ValueError('Missing declared source animation: ' + identity + '/' + name)
            records.append({'zone': identity, 'name': name, 'sha256': digest(path),
                            'uri': path.relative_to(output).as_posix()})
        print(identity + ': ' + str(len(names)) + ' animations', flush=True)
    (output / 'extraction.json').write_text(json.dumps(records, indent=2) + '\n')
    return records


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--installation', type=Path, required=True)
    parser.add_argument('--inspection', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--prepare', action='store_true')
    parser.add_argument('--retain', action='store_true')
    parser.add_argument('--frames', action='store_true')
    parser.add_argument('--verify-frames', action='store_true')
    parser.add_argument('--base', type=Path, default=Path(__file__).resolve().parents[1] / 'assets/origins')
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if output.is_relative_to(workspace) or output.is_relative_to(args.installation.resolve()):
        raise ValueError('Preparation output must be external to the project and original game')
    output.mkdir(parents=True, exist_ok=True)
    if args.frames:
        extract_frames(args.installation.resolve(), args.inspection.resolve(), output)
    elif args.verify_frames:
        importer = Importer(args.inspection.resolve(), output, args.base.resolve())
        importer.complete_frames(verify_only=True)
        write_json(output / 'native-geometry-validation.json', {'passed': True, 'model_lods': importer.report['model_lods']})
    elif args.retain:
        retain_animations(output, workspace)
    elif args.prepare:
        importer = Importer(args.inspection.resolve(), output, args.base.resolve())
        stamp = output / 'preparation-stages.json'
        completed = json.loads(stamp.read_bytes()) if stamp.exists() else []
        for name, action in [('catalog', importer.catalog), ('map', importer.map_lods),
                             ('actors', importer.actor_lods),
                             ('animations', lambda: importer.animations(json.loads((output / 'extraction.json').read_bytes()))),
                             ('frames', importer.complete_frames)]:
            if name not in completed:
                action()
                importer.finish()
                completed.append(name)
                write_json(stamp, completed)
        importer.finish()
        importer.close_textures()
    else:
        extract(args.installation.resolve(), args.inspection.resolve(), output)


if __name__ == '__main__':
    main()
