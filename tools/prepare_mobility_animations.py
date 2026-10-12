import argparse
import copy
import hashlib
import json
from pathlib import Path

from prepare_actors import append_clips
from prepare_gameplay_content import external
from prepare_weapon_assemblies import composition


ACTIONS = {
    'prone_enter': 'crawlInAnim', 'prone_loop': 'crawlForwardAnim',
    'prone_back': 'crawlBackAnim', 'prone_right': 'crawlRightAnim',
    'prone_left': 'crawlLeftAnim', 'prone_leave': 'crawlOutAnim',
    'prone_enter_empty': 'crawlEmptyInAnim', 'prone_loop_empty': 'crawlEmptyForwardAnim',
    'prone_back_empty': 'crawlEmptyBackAnim', 'prone_right_empty': 'crawlEmptyRightAnim',
    'prone_left_empty': 'crawlEmptyLeftAnim', 'prone_leave_empty': 'crawlEmptyOutAnim',
    'dive_enter': 'dtp_in', 'dive_loop': 'dtp_loop', 'dive_leave': 'dtp_out',
    'dive_enter_empty': 'dtp_empty_in', 'dive_loop_empty': 'dtp_empty_loop',
    'dive_leave_empty': 'dtp_empty_out'}
TIMINGS = {
    'crawl_seconds': ('crawlInTime', 'crawlForwardTime', 'crawlBackTime',
                      'crawlRightTime', 'crawlLeftTime', 'crawlOutTime', 'crawlOutFireTime'),
    'dive_seconds': ('dtpInTime', 'dtpLoopTime', 'dtpOutTime')}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def prepare(base, output, inspection, report_path):
    output, report_path = external(output), external(report_path)
    original_game = (base / 'game.json').read_bytes()
    game = json.loads(original_game)
    content = game['content']
    presentations = {row['id']: row for row in content['presentations']}
    recovered_path = inspection / 'solo-content/recovered-gameplay.json'
    recovered_bytes = recovered_path.read_bytes()
    fields = {'weapon.' + row['source']: row['fields']
              for row in json.loads(recovered_bytes)['weapons']}
    source = inspection / 'solo-content/source/xanim'
    native_roots = [inspection / 'weapon-assembly/native-models' / mode / 'model_export'
                    for mode in ('patch', 'shared', 'base')]
    native_cache = {}

    def native(model):
        if model not in native_cache:
            path = next((root / (model + '_lod0.gltf') for root in native_roots
                         if (root / (model + '_lod0.gltf')).is_file()), None)
            if path is None:
                raise ValueError('Missing recovered native skeleton: ' + model)
            doc = json.loads(path.read_bytes())
            joints = {joint for skin in doc.get('skins', []) for joint in skin['joints']}
            native_cache[model] = {doc['nodes'][joint]['name']:
                                   doc['nodes'][joint]['extras']['origins_native_translation']
                                   for joint in joints}
        return native_cache[model]

    required, baselines, offsets = {}, {}, {}
    all_names = set()
    for weapon in content['weapons']:
        row = fields[weapon['id']]
        names = {row[field] for field in ACTIONS.values() if row.get(field)}
        all_names.update(names)
        for mode in ('presentation', 'ads_presentation'):
            rig = weapon.get(mode)
            if not rig or not names:
                continue
            required.setdefault(rig, set()).update(names)
            baseline = dict(native(row['gunModel']))
            translation_offsets = {}
            for part in composition(row, ads=mode == 'ads_presentation'):
                table = native(part['model'])
                for bone, value in table.items():
                    baseline.setdefault(bone, value)
                if any(part['offset']):
                    raw = next(root / (part['model'] + '_lod0.gltf') for root in native_roots
                               if (root / (part['model'] + '_lod0.gltf')).is_file())
                    doc = json.loads(raw.read_bytes())
                    joints = {joint for skin in doc.get('skins', []) for joint in skin['joints']}
                    children = {child for joint in joints
                                for child in doc['nodes'][joint].get('children', [])}
                    for joint in joints - children:
                        bone = doc['nodes'][joint]['name']
                        if bone not in native(row['gunModel']):
                            translation_offsets[bone] = part['offset']
            if rig in baselines and baselines[rig] != baseline:
                raise ValueError('Shared presentation has conflicting native baseline: ' + rig)
            baselines[rig], offsets[rig] = baseline, translation_offsets
    for character in ('dempsey', 'nikolai', 'richtofen', 'takeo'):
        model = 'c_zom_' + character + '_viewhands'
        required['actor.' + model] = all_names.copy()
        baselines['actor.' + model] = native(model)
    report = {'source': {'path': str(recovered_path), 'sha256': digest(recovered_bytes)},
              'base_game_sha256': digest(original_game), 'files': [], 'rigs': [],
              'native_clips': {name: digest((source / name).read_bytes()) for name in sorted(all_names)},
              'action_fields': ACTIONS, 'timing_fields': TIMINGS}
    for rig, names in sorted(required.items()):
        presentation = presentations[rig]
        relative = presentation['scene'].split('#')[0]
        original = (base / relative).read_bytes()
        doc = json.loads(original)
        binary_relative = (Path(relative).parent / doc['buffers'][0]['uri']).as_posix()
        original_binary = (base / binary_relative).read_bytes()
        binary = bytearray(original_binary)
        before = copy.deepcopy(doc)
        existing = {animation['name']: index for index, animation in enumerate(doc.get('animations', []))}
        missing = sorted(names - existing.keys())
        notes = append_clips(doc, binary, [source / name for name in missing],
                             native_translations=baselines.get(rig), translation_offsets=offsets.get(rig))
        if [name for name, _ in notes] != missing:
            raise ValueError('Native animation does not bind to rig: ' + rig)
        for index, (name, notifications) in enumerate(notes, len(existing)):
            identity = 'animation.' + rig.removeprefix('actor.') + '.' + name
            if any(animation['id'] == identity for animation in content['animations']):
                raise ValueError('Animation ID exists without canonical clip: ' + identity)
            content['animations'].append({'id': identity, 'path': relative + f'#Animation{index}',
                                          'skeleton': rig, 'notifications': notifications})
            presentation['animations'].append(identity)
            sounds = [{'seconds': note['seconds'], 'alias': note['event'].split('#', 1)[1]}
                      for note in notifications if note['event'].startswith('sndnt#')]
            if sounds:
                available = [sound for sound in sounds if sound['alias'] in game['audio']['aliases']]
                if available:
                    game['audio']['timelines'][identity] = available
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
            target.write_text(json.dumps(doc, separators=(',', ':')), encoding='utf-8')
            (output / binary_relative).write_bytes(binary)
            for path, previous in ((relative, original), (binary_relative, original_binary)):
                report['files'].append({'path': path, 'before': digest(previous),
                                        'after': digest((output / path).read_bytes())})
        report['rigs'].append({'rig': rig, 'appended': len(missing), 'existing': len(existing)})
    for weapon in content['weapons']:
        row = fields[weapon['id']]
        rig = weapon.get('presentation')
        if rig:
            for action, field in ACTIONS.items():
                if row.get(field):
                    weapon['animation_actions'][action] = 'animation.' + rig.removeprefix('actor.') + '.' + row[field]
        for key, names in TIMINGS.items():
            weapon['mechanics'][key] = [float(row.get(name) or 0) for name in names]
    output.mkdir(parents=True, exist_ok=True)
    (output / 'game.json').write_text(json.dumps(game, separators=(',', ':')) + '\n', encoding='utf-8')
    report['files'].append({'path': 'game.json', 'before': digest(original_game),
                            'after': digest((output / 'game.json').read_bytes())})
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--base-pack', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--inspection', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    report = prepare(args.base_pack, args.output, args.inspection, args.report)
    print(json.dumps({'rigs': len(report['rigs']), 'appended': sum(rig['appended'] for rig in report['rigs']),
                      'changed_files': len(report['files'])}))


if __name__ == '__main__':
    main()
