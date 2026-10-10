import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import time

import numpy as np

from complete_asset_import import accessor
from gameplay_remote import Client, write_report
from xanim import decode


def audit(output):
    pack = output / 'pack'
    report = json.loads((output / 'coverage.json').read_bytes())
    records = json.loads((output / 'extraction.json').read_bytes())
    recovered = {r['sha256']: r for r in records}
    assert len(report['animations']) == len(recovered)
    def verify_animation(entry):
        channels = 0
        record = recovered[entry['sha256']]
        raw = (output / record['uri']).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == entry['sha256']
        native = decode(raw)
        path = pack / entry['path']
        doc = json.loads(path.read_bytes())
        binary = (path.parent / doc['buffers'][0]['uri']).read_bytes()
        metadata = doc['extras']['native_animation']
        assert metadata['asset_type'] == native['asset_type']
        assert metadata['looped'] == native['looped']
        assert metadata['notifications'] == native['notifications']
        assert metadata['channel_presence'] == native['channel_presence']
        aliases = {r['name'] for r in entry['sources']}
        assert {a['name'] for a in doc['animations']} == aliases
        expected = {(bone, kind): (times, values * (.0254 if kind == 'translation' else 1))
                    for bone, tracks in native['tracks'].items() for kind, (times, values) in tracks.items()
                    if native['channel_presence'][bone][kind]}
        expected.update({('native_root_motion', kind): (times, values * (.0254 if kind == 'translation' else 1))
                         for kind, (times, values) in (native['delta'] or {}).items()})
        actual = {}
        animation = doc['animations'][0]
        for channel in animation['channels']:
            target = channel['target']
            key = doc['nodes'][target['node']]['name'], target['path']
            sampler = animation['samplers'][channel['sampler']]
            actual[key] = accessor(doc, binary, sampler['input']).flatten(), accessor(doc, binary, sampler['output'])
        if expected:
            assert actual.keys() == expected.keys(), (entry['path'], actual.keys() ^ expected.keys())
            for key, (times, values) in expected.items():
                assert np.allclose(actual[key][0], times, atol=1e-6)
                assert np.allclose(actual[key][1], values, atol=1e-6), (entry['path'], key)
                channels += 1
        else:
            assert len(actual) == 1
        return {n.casefold() for n in aliases}, channels
    names, channels = set(), 0
    with ThreadPoolExecutor(max_workers=8) as pool:
        for aliases, count in pool.map(verify_animation, report['animations']):
            names.update(aliases)
            channels += count
    assert names == {r['name'].casefold() for r in records}
    assert report['native_model_attributes'] == report['model_lods']
    assert report['native_vertex_positions'] == report['model_lods']
    game = json.loads((pack / 'game.json').read_bytes())
    source_models = [p for p in game['content']['presentations'] if p['id'].startswith('source.model.')]
    assert len(source_models) == report['model_lods']
    assert len({p['scene'] for p in source_models}) == report['model_lods']
    map_doc = json.loads((pack / game['scene'].split('#')[0]).read_bytes())
    placed = {}
    for index in map_doc['scenes'][int(game['scene'].split('#Scene')[1])]['nodes']:
        node = map_doc['nodes'][index]
        extras = node.get('extras', {})
        if extras.get('static_instance') and 'mesh' in node:
            placed.setdefault(extras['source_model'], set()).add(extras.get('mesh_lod', {}).get('level', 0))
    assert len(placed) == report['map_models']
    assert sum(len(levels) for levels in placed.values()) == report['map_lods']
    for entry in report['actor_scenes']:
        doc = json.loads((pack / entry['uri']).read_bytes())
        levels = {n['extras']['mesh_lod']['level'] for n in doc['nodes']
                  if 'mesh' in n and 'mesh_lod' in n.get('extras', {})}
        assert levels == set(range(entry['levels'])), entry['uri']
    return {'declarations': len(records), 'animation_names': len(names), 'animation_payloads': len(recovered),
            'native_animation_channels': channels, 'model_entries': report['model_entries'],
            'model_lods': report['model_lods'], 'map_models': report['map_models'],
            'map_lods': report['map_lods'], 'actor_lod_scenes': len(report['actor_scenes'])}


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
