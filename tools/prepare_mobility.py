import argparse
import hashlib
import json
from pathlib import Path
import re
import struct


NATIVE = {
    'dive_startup': ('dtp_startup_delay', .001),
    'dive_minimum_speed': ('dtp_min_speed', .0254),
    'dive_exhaustion': ('dtp_exhaustion_window', .001),
    'dive_slide': ('dtp_max_slide_duration', .001),
    'dive_recovery': ('dtp_post_move_pause', .001),
}


def native_tuning(research):
    path = research / 'exports/functions/008a/008ad170.c'
    text = path.read_text()
    values = {}
    for key, (name, scale) in NATIVE.items():
        match = re.search(r'FUN_00757df0\("' + name + r'",(0x[0-9a-f]+),', text)
        if match is None:
            raise ValueError('Missing native mobility default: ' + name)
        value = struct.unpack('<f', struct.pack('<I', int(match[1], 16)))[0]
        values[key] = value * scale
    return values


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--research', type=Path, required=True)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    values = native_tuning(args.research)
    path = args.pack / 'game.json'
    game = json.loads(path.read_bytes())
    game['tuning'].setdefault('mobility', {}).update(values)
    path.write_text(json.dumps(game, separators=(',', ':')) + '\n', encoding='utf-8')
    entries = ('008a/008ad170', '008a/008ada20', '008a/008adac0', '008a/008ad690',
               '008a/008ad7f0', '008a/008ad8b0', '008b/008be7e0', '008b/008ba200',
               '008b/008b64d0')
    evidence = {}
    for entry in entries:
        source = args.research / ('exports/functions/' + entry + '.c')
        evidence[entry.rsplit('/', 1)[1]] = {
            'path': str(source.resolve()), 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()}
    receipt = {'native_tuning': values, 'sources': evidence,
               'game_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
               'limitations': ['Airborne trajectory, slope friction and collision response still use the reconstructed motor.',
                               'Native entry gates and ground slide/post-move durations are retained.']}
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'native_tuning': values, 'receipt': str(args.receipt)}))


if __name__ == '__main__':
    main()
