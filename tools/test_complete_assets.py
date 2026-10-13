from test_support import asset_bytes
import argparse
import json
from pathlib import Path
import time

from test_support import Client, write_report


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
    game = json.loads(asset_bytes(pack / 'game.json'))
    cases = []
    for relative in ('actors/c_zom_tomb_german_body_1a_lod0.gltf',
                     'actors/t6_wpn_smg_mp40_world_lod0.gltf',
                     'props/p6_anim_zm_barricade_board_01_lod0.gltf',
                     'props/zombie_vending_jugg_on_lod0.gltf',
                     'props/fxanim_zom_tomb_generator_pump_mod_lod0.gltf'):
        presentation = next(p for p in game['content']['presentations'] if p.get('scene') == relative + '#Scene0')
        doc = json.loads(asset_bytes(pack / relative))
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
                        and json.loads(asset_bytes(pack / a['path'].split('#')[0]))['animations'][0]['name'] == 'ai_zombie_walk_v1')
    source = next(p for p in game['content']['presentations'] if animation_id in p['animations'])
    doc = json.loads(asset_bytes(pack / source['scene'].split('#')[0]))
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
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    report = {'passed': False}
    client = Client()
    try:
        report.update(runtime(client, args.assets.resolve()))
        report['passed'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'complete-assets-runtime.json', report)
        print(json.dumps({'report': str(path), 'passed': report['passed']}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
