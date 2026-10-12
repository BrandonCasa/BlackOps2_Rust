import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

from prepare_mobility import float_default


SOURCES = ('008b/008b64d0', '008b/008ba200', '008b/008bea40', '008b/008be7e0',
           '008b/008bef20', '008b/008be2b0', '008c/008c64b0', '008c/008c6170',
           '008c/008c2760', '008c/008c2a70', '008c/008cb330', '008c/008c9d80',
           '007b/007b3b60', '005f/005fced0', '008b/008ba550', '008b/008ba970',
           '0060/0060d7f0', '0060/0060e0e0', '008c/008c86e0')


def native_tuning(research):
    def source(entry):
        return (research / ('exports/functions/' + entry + '.c')).read_text()
    dvars = source('008b/008b64d0')
    walk = int(re.search(r'FUN_00757cf0\("g_speed",(0x[0-9a-f]+),',
                         source('005f/005fced0'))[1], 16) * .0254
    ground = source('008c/008c64b0')
    for value in (9, 12, 19):
        if f'fVar12 = {value}.0;' not in ground:
            raise ValueError('Native stance acceleration changed')
    image = (research / 'input/t6zm.unpacked.exe').read_bytes()
    pe = struct.unpack_from('<I', image, 0x3c)[0]
    optional = pe + 24
    base = struct.unpack_from('<I', image, optional + 28)[0]
    sections = optional + struct.unpack_from('<H', image, pe + 20)[0]
    def scalar(address):
        for i in range(struct.unpack_from('<H', image, pe + 6)[0]):
            _, rva, size, offset = struct.unpack_from('<4I', image, sections + 40 * i + 8)
            if base + rva <= address < base + rva + size:
                return struct.unpack_from('<f', image, offset + address - base - rva)[0]
        raise ValueError('Native constant outside image')
    crouch, prone = scalar(0x00b15ce8), scalar(0x00b0f6f0)
    if not (.649 < crouch < .651 and .149 < prone < .151):
        raise ValueError('Native stance speed constants changed')
    steps = source('008c/008cb330')
    step_threshold = float(re.search(r'([0-9.]+) < ABS\(local_f4\)', steps)[1]) * .0254
    step_min = float(re.search(r'local_100\)\s*\*\s*0\.8\s*\+\s*([0-9.]+)', steps)[1])
    physics = {'ground_acceleration': 9., 'crouch_acceleration': 12., 'prone_acceleration': 19.,
               'air_acceleration': 1., 'friction': float_default(dvars, 'friction'),
               'stop_speed': float_default(dvars, 'stopspeed') * .0254,
               'strafe_scale': float_default(dvars, 'player_strafeSpeedScale'),
               'backward_scale': float_default(dvars, 'player_backSpeedScale'),
               'sprint_strafe_scale': float_default(dvars, 'player_sprintStrafeSpeedScale'),
               'ground_normal_min': struct.unpack('<f', struct.pack('<I', 0x3f333333))[0],
               'ground_probe': .0254, 'step_down': 9. * .0254, 'skin_width': .00635,
               'step_slowdown_height': step_threshold, 'step_speed_min': step_min}
    tuning = {'walk_speed': walk, 'sprint_speed': walk * float_default(dvars, 'player_sprintSpeedScale'),
              'crouch_speed': walk * crouch, 'step_height': 18. * .0254, 'physics': physics}
    mobility = {'prone_speed': walk * prone, 'prone_step': 10. * .0254}
    return tuning, mobility


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--research', type=Path, required=True)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    if args.receipt.resolve().is_relative_to(root):
        raise ValueError('Physics receipts must be external')
    tuning, mobility = native_tuning(args.research)
    path = args.pack / 'game.json'
    game = json.loads(path.read_bytes())
    game['tuning'].update(tuning)
    game['tuning'].setdefault('mobility', {}).update(mobility)
    path.write_text(json.dumps(game, separators=(',', ':')) + '\n', encoding='utf-8')
    sources = {}
    for entry in SOURCES:
        source = args.research / ('exports/functions/' + entry + '.c')
        sources[entry] = {'path': str(source.resolve()), 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()}
    receipt = {'tuning': tuning, 'mobility': mobility, 'sources': sources,
               'native_image_sha256': hashlib.sha256((args.research / 'input/t6zm.unpacked.exe').read_bytes()).hexdigest(),
               'stance_speed_constants': {'crouch': '00b15ce8', 'prone': '00b0f6f0', 'consumer': '008c2760'},
               'projectile_trajectory': {'gravity_type': 6, 'consumer': '008ba550',
                                         'launcher_creation': '0060d7f0', 'throwable_creation': '0060e0e0'},
               'game_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
               'limitations': ['Avian capsule contacts replace the original collision shape and solver.',
                               'One-inch ground probe and quarter-inch skin are Avian tolerances.',
                               'Landing recovery and imported slick flags are exercised by the separate final landing scenario.',
                               'Discrete 60 Hz input and contacts approximate original frame subdivision.']}
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'tuning': tuning, 'mobility': mobility, 'receipt': str(args.receipt)}))


if __name__ == '__main__':
    main()
