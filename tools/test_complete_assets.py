import argparse
import hashlib
import json
from pathlib import Path
import time

from test_support import Client, write_report


def audit(output):
    pack = output.resolve()
    index = json.loads((pack / 'prepared/index.json').read_bytes())
    assert index['schema_version'] == 1
    checked = set()
    gltfs = set()

    def asset(uri, parent=pack):
        if uri.startswith('data:'):
            return None
        candidate = (parent / uri.split('#')[0]).resolve()
        assert candidate.is_relative_to(pack), uri
        assert candidate.is_file(), uri
        checked.add(candidate)
        if candidate.suffix == '.gltf':
            gltfs.add(candidate)
        return candidate

    def material_links(material):
        extras = material.get('extras', {})
        if extras.get('native_record'):
            asset(extras['native_record'])
        surface = extras.get('surface')
        if surface:
            assert surface['version'] == 3
            frames = [surface, *surface.get('animation', {}).get('frames', [])]
            for frame in frames:
                for channel in frame.get('channels', []):
                    assert channel.get('layer', 0) == 0
                    asset(channel['uri'])

    counts = {}
    for category in ('worlds', 'models', 'animations', 'materials', 'images', 'audio', 'cinematics'):
        entries = index[category]
        counts[category] = len(entries)
        for entry in entries:
            if category in ('worlds', 'models', 'animations'):
                assert all(entry[key] for key in ('name', 'zone', 'source', 'source_sha256', 'native_record'))
            elif category == 'materials':
                assert all(entry[key] for key in ('name', 'zone', 'uri', 'native_record'))
            elif category == 'images':
                assert entry['source'] and entry['sha256'] and entry['uris']
                if 'name_hash' in entry:
                    assert isinstance(entry['name_hash'], int) and isinstance(entry['data_hash'], int)
                    assert entry['source_sha256']
                else:
                    assert entry['name']
            elif category == 'audio':
                assert entry['source'] and entry['payload_sha256'] and entry['uri']
                assert isinstance(entry['id'], int) and isinstance(entry['prepared'], bool)
            elif category == 'cinematics':
                assert entry['kind'] == 'cinematic' and entry['path'] and entry['sha256'] and entry['uri']
            if entry.get('native_record'):
                asset(entry['native_record'])
            if entry.get('uri'):
                path = asset(entry['uri'])
                if category == 'materials':
                    material_links(json.loads(path.read_bytes()))
            for uri in entry.get('uris', {}).values():
                asset(uri)
            for lod in entry.get('lods', []):
                asset(lod['uri'])
            if entry.get('curves_uri'):
                asset(entry['curves_uri'])
    for path in sorted(gltfs):
        document = json.loads(path.read_bytes())
        assert document['asset']['version'] == '2.0'
        for collection in ('buffers', 'images'):
            for item in document.get(collection, []):
                if 'uri' in item:
                    asset(item['uri'], path.parent)
        for material in document.get('materials', []):
            material_links(material)
    native = json.loads((pack / 'native/asset-index.json').read_bytes())
    assert native['schema_version'] == 1
    native_records = []
    for entry in native['assets']:
        asset(entry['native_record'])
        native_records.append(entry['native_record'])
    preserved_records = []
    native_counts = {}
    for kind, entries in index['native_art'].items():
        native_counts[kind] = len(entries)
        for entry in entries:
            assert entry['type'] == kind
            assert all(entry[key] for key in ('name', 'zone', 'source', 'source_sha256', 'native_record'))
            asset(entry['native_record'])
            preserved_records.append(entry['native_record'])
    assert sorted(preserved_records) == sorted(native_records), 'Native art manifest differs from preserved asset index'
    for path in (pack / 'native/ipak').glob('*/index.json'):
        inventory = json.loads(path.read_bytes())
        assert inventory['source'] and inventory['source_sha256']
        for entry in inventory['assets']:
            asset(entry['native_uri'])
            for uri in entry.get('ktx2', {}).values():
                asset(uri)
    return {'prepared': counts, 'native_art': native_counts,
            'native_art_records': len(native_records), 'dependency_files': len(checked),
            'gltf_documents': len(gltfs)}


def preview(client, presentation, animation=None, distance=.9):
    client.call('origins/preview', {'presentation': presentation, 'animation': animation, 'distance': distance})
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        result = client.call('origins/assets')['previews']
        if result and result[0]['loaded'] and result[0]['clip_exists']:
            if not animation or result[0]['players']:
                return result[0]
        time.sleep(.05)
    raise TimeoutError(str(result))


def runtime(client, pack):
    client.until(lambda s: s['state'] in ('Playing', 'Paused') and s.get('player'), timeout=180)
    protocol = client.call('origins/schema')
    assert 'origins/assets' in protocol['methods']
    client.reset()
    game = json.loads((pack / 'game.json').read_bytes())
    cases = []
    for relative in ('actors/c_zom_tomb_german_body_1a_lod0.gltf',
                     'actors/t6_wpn_smg_mp40_world_lod0.gltf',
                     'props/p6_anim_zm_barricade_board_01_lod0.gltf',
                     'props/zombie_vending_jugg_on_lod0.gltf',
                     'props/fxanim_zom_tomb_generator_pump_mod_lod0.gltf'):
        presentation = next(p for p in game['content']['presentations'] if p.get('scene') == relative + '#Scene0')
        doc = json.loads((pack / relative).read_bytes())
        distances = next(n['extras']['mesh_lod']['distances'] for n in doc['nodes'] if 'mesh_lod' in n.get('extras', {}))
        probes = [max(.9, distances[0] * .5)] + [(a+b)*.5 for a, b in zip(distances, distances[1:])] + [distances[-1] + 3]
        for level, distance in enumerate(probes):
            preview(client, presentation['id'], distance=distance)
            client.wait_ticks(4)
            after = client.call('origins/assets')['previews'][0]
            assert after['meshes'] and all(m['range'] is not None for m in after['meshes'])
            ranges = {tuple(m['range']['start'] + m['range']['end']) for m in after['meshes']}
            assert len(ranges) == len(distances) + 1
            assert any(m['visible'] for m in after['meshes'])
            assert all(m['range']['start'][1] <= distance < m['range']['end'][0]
                       for m in after['meshes'] if m['visible']), after['meshes']
            cases.append({'presentation': presentation['id'], 'probe_level': level, 'distance': distance,
                          'meshes': after['meshes']})
    animation_id = next(a['id'] for a in game['content']['animations'] if a['id'].startswith('animation.source.')
                        and a['path'].startswith('animations/')
                        and json.loads((pack / a['path'].split('#')[0]).read_bytes())['animations'][0]['name'] == 'ai_zombie_walk_v1')
    source = next(p for p in game['content']['presentations'] if animation_id in p['animations'])
    doc = json.loads((pack / source['scene'].split('#')[0]).read_bytes())
    name = doc['animations'][0]['name']
    before = preview(client, source['id'], animation=name)
    changed = False
    for _ in range(20):
        client.wait_ticks(4)
        after = client.call('origins/assets')['previews'][0]
        if before['bones'] != after['bones']:
            changed = True
            break
    assert changed, name
    cases.append({'animation': name, 'players': after['players'], 'bones_advanced': changed})
    for filename in ('defaultactor_lod0.gltf', 'test_macbeth_chart_lod0.gltf'):
        source = next(p for p in game['content']['presentations'] if p['id'].startswith('source.model.')
                      and p['scene'].endswith('/' + filename + '#Scene0'))
        loaded = preview(client, source['id'], distance=2.)
        client.wait_ticks(4)
        loaded = client.call('origins/assets')['previews'][0]
        assert loaded['meshes'] and any(m['visible'] for m in loaded['meshes'])
        cases.append({'source_model': filename, 'loaded': loaded['loaded'], 'meshes': loaded['meshes']})
    client.call('origins/preview', {})
    client.reset()
    return {'cases': cases, 'protocol': protocol}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--runtime', type=Path)
    parser.add_argument('--identity', type=Path)
    args = parser.parse_args()
    report = {'passed': False}
    client = Client() if args.runtime else None
    try:
        if args.runtime:
            assert args.identity
            report['identity_sha256'] = hashlib.sha256(args.identity.read_bytes()).hexdigest()
            report.update(runtime(client, args.runtime))
        else:
            report.update(audit(args.output))
        report['passed'] = True
    finally:
        if client is not None:
            client.shutdown(report)
        path = write_report(args.output, 'complete-assets-runtime.json' if args.runtime else 'complete-assets-audit.json', report)
        print(json.dumps({'report': str(path), 'passed': report['passed']}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
