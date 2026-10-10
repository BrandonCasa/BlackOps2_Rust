import argparse
import copy
import hashlib
import json
from pathlib import Path
import re

from prepare_actors import append_clips
from prepare_gameplay_content import external
from prepare_weapon_assemblies import composition
from xanim import decode


ADS = {'ads_fire': 'adsFireAnim', 'ads_last_shot': 'adsLastShotAnim', 'ads_rechamber': 'adsRechamberAnim'}
COMBAT = {'walk_melee': 'zm_walk_melee', 'run_melee': 'zm_run_melee',
          'generator_melee': 'zm_generator_melee', 'death': 'zm_death'}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def prepare(base, output, inspection, asd, report_path):
    output, report_path = external(output), external(report_path)
    original_game = (base / 'game.json').read_bytes()
    game = json.loads(original_game)
    content = game['content']
    presentations = {p['id']: p for p in content['presentations']}
    recovered = json.loads((inspection / 'solo-content/recovered-gameplay.json').read_bytes())
    fields = {'weapon.' + w['source']: w['fields'] for w in recovered['weapons']}
    source = inspection / 'solo-content/source/xanim'
    native_roots = [inspection / 'weapon-assembly/native-models' / mode / 'model_export'
                    for mode in ('patch', 'shared', 'base')]
    native_cache = {}

    def native(model):
        if model not in native_cache:
            path = next((r / (model + '_lod0.gltf') for r in native_roots
                         if (r / (model + '_lod0.gltf')).is_file()), None)
            if path is None:
                raise ValueError('Missing recovered native skeleton: ' + model)
            doc = json.loads(path.read_bytes())
            joints = {j for s in doc.get('skins', []) for j in s['joints']}
            native_cache[model] = {doc['nodes'][j]['name']: doc['nodes'][j]['extras']['origins_native_translation']
                                   for j in joints}
        return native_cache[model]

    text = asd.read_text()
    states = {}
    for state, name in COMBAT.items():
        matches = re.findall(r'(?m)^' + name + r'\s*:[^\n]*\s*\{([^}]+)\}', text)
        if len(matches) != 1:
            raise ValueError('Missing or duplicate native state: ' + name)
        states[state] = matches[0].split()
    combat_names = set(n for names in states.values() for n in names)
    required, baselines, offsets = {}, {}, {}
    ads_names = set()
    for weapon in content['weapons']:
        f = fields[weapon['id']]
        names = {f[k] for k in ADS.values() if f.get(k)}
        if not weapon.get('presentation') or not names:
            continue
        ads_names.update(names)
        for mode in ('presentation', 'ads_presentation'):
            rig = weapon.get(mode)
            if not rig:
                continue
            required.setdefault(rig, set()).update(names)
            baseline = dict(native(f['gunModel']))
            translation_offsets = {}
            for part in composition(f, ads=mode == 'ads_presentation'):
                table = native(part['model'])
                for bone, value in table.items():
                    if bone not in baseline:
                        baseline[bone] = value
                if any(part['offset']):
                    raw = next(r / (part['model'] + '_lod0.gltf') for r in native_roots
                               if (r / (part['model'] + '_lod0.gltf')).is_file())
                    doc = json.loads(raw.read_bytes())
                    joints = {j for s in doc.get('skins', []) for j in s['joints']}
                    children = {c for j in joints for c in doc['nodes'][j].get('children', [])}
                    for j in joints - children:
                        bone = doc['nodes'][j]['name']
                        if bone not in native(f['gunModel']):
                            translation_offsets[bone] = part['offset']
            baselines[rig], offsets[rig] = baseline, translation_offsets
    for character in ('dempsey', 'nikolai', 'richtofen', 'takeo'):
        model = 'c_zom_' + character + '_viewhands'
        required['actor.' + model] = ads_names.copy()
        baselines['actor.' + model] = native(model)
    enemy_rigs = {e['presentation'] for e in content['enemies']}
    enemy_rigs.update(p['presentation'] for e in content['enemies'] for p in e['parts'])
    for rig in enemy_rigs:
        required[rig] = combat_names.copy()
    decoded = {n: decode((source / n).read_bytes()) for n in sorted(ads_names | combat_names)}
    report = {'definition': {'path': str(asd), 'sha256': digest(asd.read_bytes())},
              'states': states, 'source_clips': {n: digest((source / n).read_bytes()) for n in decoded},
              'base_game_sha256': digest(original_game), 'files': [], 'rigs': [],
              'limitations': ['Native state substate selection uses deterministic actor variation rather than engine RNG.',
                              'Collider owns actor translation; native ragdoll handoff remains unimplemented.',
                              'ADS fire variants activate at the existing fully aimed threshold.']}
    for rig, names in sorted(required.items()):
        presentation = presentations[rig]
        relative = presentation['scene'].split('#')[0]
        original = (base / relative).read_bytes()
        doc = json.loads(original)
        binary_relative = (Path(relative).parent / doc['buffers'][0]['uri']).as_posix()
        original_binary = (base / binary_relative).read_bytes()
        binary = bytearray(original_binary)
        before = copy.deepcopy(doc)
        existing = {a['name']: i for i, a in enumerate(doc.get('animations', []))}
        missing = sorted(names - existing.keys())
        notes = append_clips(doc, binary, [source / n for n in missing], in_place=rig in enemy_rigs,
                             native_translations=baselines.get(rig), translation_offsets=offsets.get(rig))
        if [n for n, _ in notes] != missing:
            raise ValueError('Native animation does not bind to rig: ' + rig)
        for index, (name, notifications) in enumerate(notes, len(existing)):
            identity = 'animation.' + rig.removeprefix('actor.') + '.' + name
            if any(a['id'] == identity for a in content['animations']):
                raise ValueError('Animation ID exists without canonical clip: ' + identity)
            content['animations'].append({'id': identity, 'path': relative + f'#Animation{index}',
                                          'skeleton': rig, 'notifications': notifications})
            presentation['animations'].append(identity)
        if missing:
            doc['buffers'][0]['byteLength'] = len(binary)
            for key in before.keys() - {'animations', 'accessors', 'bufferViews', 'buffers'}:
                if doc[key] != before[key]:
                    raise ValueError('Animation append changed existing scene data: ' + key)
            if binary[:len(original_binary)] != original_binary:
                raise ValueError('Animation append changed existing buffer data')
            if (base / relative).read_bytes() != original or (base / binary_relative).read_bytes() != original_binary:
                raise ValueError('Concurrent runtime asset edit: ' + relative)
            target = output / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(json.dumps(doc, separators=(',', ':')))
            (output / binary_relative).write_bytes(binary)
            for path, previous in ((relative, original), (binary_relative, original_binary)):
                report['files'].append({'path': path, 'before': digest(previous),
                                        'after': digest((output / path).read_bytes())})
        report['rigs'].append({'rig': rig, 'appended': len(missing), 'existing': len(existing)})
    for weapon in content['weapons']:
        rig = weapon.get('presentation')
        if not rig:
            continue
        for action, field in ADS.items():
            name = fields[weapon['id']].get(field)
            if name:
                weapon['animation_actions'][action] = 'animation.' + rig.removeprefix('actor.') + '.' + name
    for enemy in content['enemies']:
        model = enemy['presentation'].removeprefix('actor.')
        enemy['combat_states'] = {state: [{'animation': 'animation.' + model + '.' + n,
                                           'seconds': decoded[n]['duration'],
                                           'hits': sorted({x['seconds'] for x in decoded[n]['notifications'] if x['event'] == 'fire'})}
                                          for n in names] for state, names in states.items()}
        for action, state in (('attack', 'walk_melee'), ('death', 'death')):
            clip = enemy['combat_states'][state][0]
            enemy['animation_actions'][action] = clip['animation']
            enemy[action + '_seconds'] = clip['seconds']
            if action == 'attack':
                enemy['attack_hits'] = clip['hits']
    output.mkdir(parents=True, exist_ok=True)
    (output / 'game.json').write_text(json.dumps(game, indent=2) + '\n')
    report['files'].append({'path': 'game.json', 'before': digest(original_game),
                            'after': digest((output / 'game.json').read_bytes())})
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--base-pack', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--inspection', type=Path, required=True)
    parser.add_argument('--state-definition', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    report = prepare(args.base_pack, args.output, args.inspection, args.state_definition, args.report)
    print(json.dumps({'rigs': len(report['rigs']), 'appended': sum(r['appended'] for r in report['rigs']),
                      'changed_files': len(report['files'])}))


if __name__ == '__main__':
    main()
