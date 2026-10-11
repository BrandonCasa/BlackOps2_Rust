from pathlib import Path
import json, shutil

MODELS = [('nuke','common_zm','zombie_bomb'), ('insta_kill','common_zm','zombie_skull'), ('double_points','common_zm','zombie_x2_icon'), ('full_ammo','zm_tomb','zombie_ammocan'), ('zombie_blood','zm_tomb','p6_zm_tm_blood_power_up')]

def prepare(source, pack):
    source, pack = Path(source).resolve(), Path(pack).resolve()
    content_path = pack / 'game.json'
    game = json.loads(content_path.read_text(encoding='utf-8-sig'))
    copied = set()
    for name, zone, model in MODELS:
        origin = source / zone
        relative = Path('powerups') / zone
        output = pack / relative
        output.mkdir(parents=True, exist_ok=True)
        scene = model + '_lod0.gltf'
        document = json.loads((origin / scene).read_text())
        if any(set(image) - {'uri', 'name'} or Path(image.get('uri', '')).suffix != '.ktx2'
               for image in document.get('images', [])):
            raise ValueError('Powerup preparation requires external KTX2 images from the prepared source library')
        def dependencies(value):
            if isinstance(value, dict):
                for key, item in value.items():
                    if key == 'uri' and not item.startswith('data:'):
                        yield item
                    else:
                        yield from dependencies(item)
            elif isinstance(value, list):
                for item in value:
                    yield from dependencies(item)
        for path in [scene, *set(dependencies(document))]:
            if Path(path).suffix not in {'.gltf', '.bin', '.ktx2'}:
                raise ValueError(f'Powerup dependency is outside the runtime KTX2 model profile: {path}')
            src, dst = (origin / path).resolve(), (output / path).resolve()
            if not src.is_relative_to(origin.resolve()) or not dst.is_relative_to(output.resolve()):
                raise ValueError(path)
            if not src.is_file() and path.startswith('textures/'):
                src = source.parents[2] / path
            if not src.is_file() or src.read_bytes().startswith(b'version https://git-lfs'):
                raise ValueError(f'Missing native asset {src}')
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dst)
            copied.add(dst.relative_to(pack).as_posix())
        for material in document.get('materials', []):
            for channel in material.get('extras', {}).get('surface', {}).get('channels', []):
                channel['uri'] = (relative / channel['uri']).as_posix()
        (output / scene).write_text(json.dumps(document, separators=(',', ':')))
        identity = 'powerup.' + name
        presentation = 'presentation.' + identity
        game['content']['powerups'][:] = [p for p in game['content']['powerups'] if p['id'] != identity]
        game['content']['powerups'].append({'id':identity,'duration':30 if name in ('insta_kill','double_points','zombie_blood') else 0,'presentation':presentation})
        game['content']['presentations'][:] = [p for p in game['content']['presentations'] if p['id'] != presentation]
        game['content']['presentations'].append({'id':presentation,'scene':(relative / scene).as_posix()+'#Scene0','animations':[],'sounds':[]})
    content_path.write_text(json.dumps(game, indent=2)+'\n')
    print(json.dumps({'powerups':len(MODELS),'verified_dependencies':len(copied)}))

if __name__ == '__main__':
    import argparse
    parser=argparse.ArgumentParser()
    parser.add_argument('source',type=Path)
    parser.add_argument('pack',type=Path)
    args=parser.parse_args()
    prepare(args.source,args.pack)
