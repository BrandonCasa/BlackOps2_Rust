"""Invoke unchanged external alias dump APIs; native fields remain source records."""
import argparse
import sys
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--tool", type=Path, required=True)
parser.add_argument("--game", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
args = parser.parse_args()
sys.path.insert(0, str(args.tool.resolve()))
from scripts.cod import backends

for name in ("common_zm", "patch_zm", "zm_tomb", "zm_tomb_patch", "en_zm_tomb"):
    language = "english" if name.startswith("en_") else "all"
    zone = args.game / "zone" / language / (name + ".ff")
    output = args.out / "audio" / "source" / name
    backends.for_fastfile(str(zone)).dump(str(zone), str(output), "sound")
print("Raw external audio alias fields exported; existing AudioDefinition semantic mapping remains required.")
