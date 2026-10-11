import json
import math
import numpy as np
from PIL import Image
from prepare_actors import convert
from prepare_gameplay_content import external
from prepare_origins import entities, presentation
from xanim import decode

CLIPS = [f'fxanim_zom_tomb_generator_{name}_anim' for name in
         ('start', 'up_idle', 'down_idle', 'end', 'fluid_up', 'fluid_down',
          'fluid_rotate_up', 'fluid_rotate_down')]


def prepare(config):
    base = external(config['assets']['inspection_output'])
    pack = external(config['game']['asset_pack'])
    game = json.loads((pack / 'game.json').read_text())
    c = game['content']
    source = base / 'solo-content/source/xanim'
    records = entities(base / 'game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents')
    coord = lambda value: (np.array(list(map(float, value.split())))[[0, 2, 1]] * [.0254, .0254, -.0254]).tolist()
    identity = 'presentation.generator.pump'
    relative, notes = convert(base / 'game-prepare/models/model_export/fxanim_zom_tomb_generator_pump_mod_lod0.gltf',
                              pack / 'props', base / 'solo-content/actor-images', [source / n for n in CLIPS], preparation=config)
    surfaces = json.loads((base / 'game-prepare/instances/model-surfaces.json').read_text())
    retained = {s['material']: s for model in surfaces.values() for s in model}
    prop = pack / 'props' / relative
    model = json.loads(prop.read_text())
    alpha = []
    for mat in model['materials']:
        native = retained.get(mat['name'])
        if native:
            state = presentation(native, native['family'])
            mat['alphaMode'] = 'MASK' if state['alpha'] == 'mask' else 'OPAQUE' if state['alpha'] == 'opaque' else 'BLEND'
            if mat['alphaMode'] == 'MASK': mat['alphaCutoff'] = state['cutoff']
            evidence = 'retained material state'
        else:
            index = mat['pbrMetallicRoughness']['baseColorTexture']['index']
            path = model['images'][model['textures'][index]['source']]['uri']
            from ktx2_textures import decoded_levels
            alpha_pixels = decoded_levels((prop.parent/path).read_bytes())[0][..., 3]
            lo, hi = int(alpha_pixels.min()), int(alpha_pixels.max())
            if lo < 255: mat['alphaMode'] = 'BLEND'
            evidence = 'source texture alpha reconstruction'
        alpha.append({'material': mat['name'], 'mode': mat.get('alphaMode', 'OPAQUE'), 'evidence': evidence})
    prop.write_text(json.dumps(model, separators=(',', ':')))
    assert [n for n, _ in notes] == CLIPS
    c['presentations'][:] = [p for p in c['presentations'] if p['id'] != identity]
    c['animations'][:] = [a for a in c['animations'] if a['skeleton'] != identity]
    refs = []
    for index, (name, notifications) in enumerate(notes):
        aid = 'animation.generator.' + name
        refs.append(aid)
        c['animations'].append({'id': aid, 'path': 'props/' + relative + f'#Animation{index}',
                                'skeleton': identity, 'notifications': notifications})
    c['presentations'].append({'id': identity, 'scene': 'props/' + relative + '#Scene0', 'animations': refs, 'sounds': []})
    for table, prefix in (('placements', 'placement.generator.'), ('interactions', 'interaction.generator.')):
        c[table][:] = [p for p in c[table] if not p['id'].startswith(prefix)]
    c['generators'] = []
    for e in sorted((r for r in records if r.get('targetname') == 's_generator'), key=lambda r: int(r['script_int'])):
        name = e['script_noteworthy']
        number = int(e['script_int'])
        models = [r for r in records if r.get('classname') == 'script_model' and r.get('targetname') == name]
        assert len(models) == 1, name
        p = coord(e['origin'])
        nearest = min((n for n in c['navigation_nodes'] if n['zone'] != 'zone.unassigned'),
                      key=lambda n: np.linalg.norm(np.array(n['position']) - p))
        gid = 'generator.' + name
        pid = 'placement.generator.' + name
        c['placements'].append({'id': pid, 'definition': gid, 'position': p,
                                'yaw': math.radians(float(e['angles'].split()[1])), 'zone': nearest['zone']})
        c['interactions'].append({'id': 'interaction.generator.' + name, 'placement': pid,
                                 'action': {'kind': 'generator', 'generator': gid}, 'reach': 3.2512,
                                 'hold_seconds': 0., 'hidden_prompt': False})
        holes = [coord(r['origin']) for r in records if r.get('targetname') == e['target']]
        assert len(holes) == 4, name
        c['generators'].append({'id': gid, 'placement': pid, 'zones': [nearest['zone']], 'number': number,
                                'price': 200, 'capture_seconds': 12., 'radius': 5.588,
                                'decay_seconds': 20., 'shutdown_seconds': 40., 'recovery_seconds': 10.,
                                'presentation': identity, 'model_position': coord(models[0]['origin']),
                                'emergence': holes, 'clip_seconds': [decode((source / n).read_bytes())['duration'] for n in CLIPS],
                                'capture_sound': 'sound.generator.capture', 'active_sound': 'sound.generator.active'})
    assert len(c['generators']) == 6
    (pack / 'game.json').write_text(json.dumps(game, indent=2) + '\n')
    from prepare_audio import prepare as prepare_audio
    prepare_audio(config)
    report = {'generators': 6, 'native_clips': CLIPS, 'emergence_points': 24,
              'material_alpha': alpha,
              'source': 'zm_tomb_capture_zones.gsc and authored map entities',
              'approximations': 'Nearest classified navigation zone; native audio uses reconstructed mixer attenuation; use reach follows native trigger box extent.'}
    (base / 'solo-content/generators-preparation.json').write_text(json.dumps(report, indent=2))
    print(report)
