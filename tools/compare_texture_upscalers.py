"""Make registered whole-texture and native-detail qualification contact sheets."""
import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw


def compare(inventory, remacri, deepbump, destination):
    entries = json.loads(inventory.read_text())["textures"]
    cell, label = 384, 36
    canvas = Image.new("RGB", (cell*3, (cell+label)*2*len(entries)), "#202020")
    drawing = ImageDraw.Draw(canvas)
    qualified = []
    for index, entry in enumerate(entries):
        relative = Path(entry["uri"])
        records = [json.loads((root/relative).with_suffix(".json").read_text())
                   for root in (remacri, deepbump)]
        if records[0]["source_sha256"] != records[1]["source_sha256"]:
            raise ValueError("Upscaler comparison requires identical canonical sources")
        images = [Image.open((remacri/relative).with_suffix(".source.png")).convert("RGB"),
                  Image.open((remacri/relative).with_suffix(".candidate.png")).convert("RGB"),
                  Image.open((deepbump/relative).with_suffix(".candidate.png")).convert("RGB")]
        if images[1].size != images[2].size or images[1].size != tuple(x*4 for x in images[0].size):
            raise ValueError("Comparison must share exact dimensions and texel layout")
        images[0] = images[0].resize(images[1].size, Image.Resampling.NEAREST)
        for column, (name, image) in enumerate(zip(("Original cells", "Remacri", "DeepBump"), images)):
            y = index*2*(cell+label)
            drawing.text((column*cell+8, y+8), name+" | "+entry["materials"][0][:37], fill="white")
            preview = image.copy()
            preview.thumbnail((cell, cell), Image.Resampling.LANCZOS)
            canvas.paste(preview, (column*cell+(cell-preview.width)//2, y+label))
            # Registered 1:1 output pixels, with exactly the same source region.
            left, top = (image.width-cell)//2, (image.height-cell)//2
            canvas.paste(image.crop((left, top, left+cell, top+cell)),
                         (column*cell, y+cell+2*label))
        qualified.append(dict(uri=entry["uri"], source_sha256=records[0]["source_sha256"],
                              candidate_sha256={name: record["output_sha256"]
                                  for name, record in zip(("remacri", "deepbump"), records)},
                              model_sha256={name: record["model_sha256"]
                                  for name, record in zip(("remacri", "deepbump"), records)}))
    destination.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(destination)
    destination.with_suffix(".json").write_text(json.dumps(dict(textures=qualified), indent=2)+"\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("inventory", "remacri", "deepbump", "destination"):
        parser.add_argument("--"+name, type=Path, required=True)
    args = parser.parse_args()
    compare(args.inventory, args.remacri, args.deepbump, args.destination)


if __name__ == "__main__":
    main()
