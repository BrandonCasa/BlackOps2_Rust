import argparse
import hashlib
import json
from pathlib import Path


def current_scenes(game):
    content = game['content']
    weapons = {entry['id']: entry for entry in content['weapons']}
    presentations = {entry['id']: entry for entry in content['presentations']}
    scenes = {game['scene'].split('#')[0], 'actors/c_zom_dempsey_viewhands_lod0.gltf'}
    used_weapons = {'weapon.c96_zm', 'weapon.frag_grenade_zm', 'weapon.knife_zm',
                    game['solo_revive']['downed_weapon']}
    used_weapons.update(w['id'] for w in weapons.values()
                        if w.get('throwable') or w.get('mine') or w.get('lure'))
    used_perks, ids = set(), set()
    for interaction in content['interactions']:
        action = interaction['action']
        if action['kind'] == 'weapon':
            used_weapons.add(action['weapon'])
        elif action['kind'] == 'perk':
            used_perks.add(action['perk'])
    for perk in content['perks']:
        if perk['id'] in used_perks:
            ids.add(perk.get('presentation'))
            if perk.get('drink'):
                used_weapons.add(perk['drink']['bottle'])
    for weapon_id in used_weapons:
        weapon = weapons[weapon_id]
        ids.update(weapon.get(key) for key in ('presentation', 'pickup', 'ads_presentation'))
    for enemy in content['enemies']:
        if enemy.get('family') in ('german', 'crusader'):
            ids.add(enemy.get('presentation'))
            ids.update(part['presentation'] for part in enemy.get('parts', []))
    for generator in content['generators']:
        ids.add(generator['presentation'])
    for barrier in content['barriers']:
        ids.update(board['presentation'] for board in barrier['boards'])
    for door in game['access']['doors']:
        ids.update(prop['presentation'] for prop in door['props'])
    for identity in ids - {None}:
        scene = presentations[identity].get('scene')
        if scene:
            scenes.add(scene.split('#')[0])
    return dict(version=1, policy='implemented-origins-presentation', scenes=sorted(scenes),
                weapons=sorted(used_weapons), perks=sorted(used_perks))


def main():
    parser = argparse.ArgumentParser(description='Derive texture preparation scope from implemented Origins presentation paths.')
    parser.add_argument('pack', type=Path)
    parser.add_argument('receipt', type=Path)
    args = parser.parse_args()
    pack = args.pack.resolve()
    if args.receipt.resolve().is_relative_to(pack) or args.receipt.resolve().is_relative_to(Path(__file__).resolve().parents[1]):
        raise ValueError('Scope receipt must remain external to input and workspace')
    raw = (pack/'game.json').read_bytes()
    result = current_scenes(json.loads(raw))
    for uri in result['scenes']:
        path = (pack/uri).resolve()
        if not path.is_relative_to(pack) or not path.is_file():
            raise ValueError(f'Missing current scene: {uri}')
    result['source_game_sha256'] = hashlib.sha256(raw).hexdigest()
    args.receipt.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
