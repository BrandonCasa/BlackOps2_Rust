import argparse
import hashlib
import json
from pathlib import Path
import re


def native_pistols(research):
    source = research / 'source/server-patch/maps/mp/zombies/_zm.gsc'
    return native_pistols_from_text(source.read_text())


def native_pistols_from_text(text):
    body = re.search(r'(?m)^last_stand_pistol_rank_init\(\)\s*\{(.*?)^\}', text, re.S)
    if body is None:
        raise ValueError('Missing native last-stand pistol initialization')
    ranks, replacement = [], None
    for line in body[1].splitlines():
        match = re.search(r'level\.pistol_values\[level\.pistol_values\.size\] = "([^"]+)";', line)
        if match:
            ranks.append({'weapon': 'weapon.' + match[1], 'rank': len(ranks)})
        if 'level.pistol_value_solo_replace_below = level.pistol_values.size - 1;' in line:
            replacement = len(ranks) - 1
    if replacement is None or replacement < 0 or replacement >= len(ranks) - 1:
        raise ValueError('Missing native solo pistol replacement threshold')
    if len({row['weapon'] for row in ranks}) != len(ranks):
        raise ValueError('Duplicate native last-stand pistol')
    return ranks, replacement


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--research', type=Path, required=True)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    ranks, replacement = native_pistols(args.research)
    path = args.pack / 'game.json'
    original = path.read_bytes()
    game = json.loads(original)
    definitions = {weapon['id'] for weapon in game['content']['weapons']}
    profile = game['solo_revive']
    profile['pistol_ranks'] = [row for row in ranks if row['weapon'] in definitions]
    profile['pistol_replace_below'] = replacement
    if not any(row['weapon'] == profile['downed_weapon'] and row['rank'] > replacement
               for row in profile['pistol_ranks']):
        raise ValueError('Solo default pistol is absent from native high-rank pistols')
    if path.read_bytes() != original:
        raise RuntimeError('Runtime pack changed during solo preparation')
    path.write_text(json.dumps(game, separators=(',', ':')) + '\n', encoding='utf-8')
    sources = [args.research / ('source/' + name) for name in (
        'server-patch/maps/mp/zombies/_zm.gsc',
        'server-patch/maps/mp/zombies/_zm_laststand.gsc',
        'server-patch/maps/mp/zombies/_zm_perks.gsc',
        'server-tomb/maps/mp/zm_tomb.gsc')]
    receipt = {
        'sources': {str(source.resolve()): hashlib.sha256(source.read_bytes()).hexdigest()
                    for source in sources},
        'base_game_sha256': hashlib.sha256(original).hexdigest(),
        'game_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
        'native_rank': ranks, 'profile': profile,
        'functions': ['last_stand_pistol_rank_init', 'last_stand_best_pistol',
                      'last_stand_compare_pistols', 'last_stand_pistol_swap',
                      'last_stand_save_pistol_ammo', 'last_stand_restore_pistol_ammo']}
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'native_pistols': len(ranks),
                      'available_pistols': len(profile['pistol_ranks']),
                      'replace_below': replacement, 'receipt': str(args.receipt)}))


if __name__ == '__main__':
    main()
