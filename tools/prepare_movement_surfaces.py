import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_path(source, name):
    relative = ('generated/_' + name[1:name.index('(')] if name.startswith('*') else name) + '.json'
    return source / relative


def source_flags(source, name):
    definition = json.loads(source_path(source, name).read_bytes())
    flags = definition.get('surfaceFlags')
    if definition.get('_game') != 't6' or type(flags) is not int or not 0 <= flags <= 0xffffffff:
        raise ValueError('Invalid original material surface flags: ' + name)
    return flags


def prepare(source, pack, receipt):
    path = pack / 'map.gltf'
    document = json.loads(path.read_bytes())
    records = []
    for material in document['materials']:
        name = material['name']
        native = source_path(source, name)
        flags = source_flags(source, name)
        material.setdefault('extras', {}).setdefault('surface', {})['flags'] = flags
        records.append({'material': name, 'flags': flags, 'source_sha256': digest(native)})
    path.write_text(json.dumps(document, separators=(',', ':')), encoding='utf-8')
    result = {'materials': records, 'slick_materials': [r['material'] for r in records if r['flags'] & 2],
              'map_sha256': digest(path), 'source_root': str(source.resolve())}
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'materials': len(records), 'slick_materials': result['slick_materials'],
                      'receipt': str(receipt)}))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-materials', type=Path, required=True)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    prepare(args.source_materials, args.pack, args.receipt)


if __name__ == '__main__':
    main()
