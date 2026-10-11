import argparse
import hashlib
import json
from pathlib import Path

from prepare_actors import append_clips
from xanim import decode


def prepare(pack, inspection, patch):
    game = json.loads((pack / 'game.json').read_text())
    model = 'p6_zm_vending_diesel_magic'
    rows = [p for p in game['content']['presentations'] if p.get('scene') and p['scene'].endswith('/' + model + '_lod0.gltf#Scene0')]
    row = min(rows, key=lambda p: ('/zm_tomb/' not in p['scene'], p['scene']))
    relative = Path(row['scene'].split('#')[0])
    document = json.loads((pack / relative).read_text())
    binary_path = relative.parent / document['buffers'][0]['uri']
    original = (pack / binary_path).read_bytes()
    binary = bytearray(original)
    names = ['o_zombie_dlc4_vending_diesel_' + suffix for suffix in ['turn_on', 'turn_off', 'ballspin_loop', 'on_idle']]
    source = inspection / 'solo-content/source/xanim'
    existing = {a['name'] for a in document.get('animations', [])}
    append_clips(document, binary, [source / name for name in names if name not in existing], in_place=True, preserve_native=True)
    if binary[:len(original)] != original:
        raise ValueError('Original vendor mesh data changed')
    document['buffers'][0]['byteLength'] = len(binary)
    (pack / relative).write_text(json.dumps(document, separators=(',', ':')) + '\n')
    (pack / binary_path).write_bytes(binary)
    animations = []
    durations = {}
    for index, animation in enumerate(document['animations']):
        if animation['name'] not in names:
            continue
        decoded = decode((source / animation['name']).read_bytes())
        durations[animation['name']] = decoded['duration']
        animations.append({'id': 'animation.' + model + '.' + animation['name'], 'path': relative.as_posix() + '#Animation' + str(index), 'skeleton': row['id'], 'notifications': decoded['notifications']})
    result = {'animations': animations, 'presentations': [{'id': row['id'], 'animations': [a['id'] for a in animations]}], 'durations': durations,
              'files': {path.as_posix(): hashlib.sha256((pack / path).read_bytes()).hexdigest() for path in [relative, binary_path]}}
    patch.parent.mkdir(parents=True, exist_ok=True)
    patch.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'animations': len(animations), 'durations': durations}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--inspection', type=Path, required=True)
    parser.add_argument('--patch', type=Path, required=True)
    args = parser.parse_args()
    prepare(args.pack, args.inspection, args.patch)
