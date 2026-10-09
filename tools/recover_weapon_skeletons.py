import re
import subprocess
from pathlib import Path

from prepare_gameplay_content import external


def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    output = external(settings.get("weapon_assembly_output", inspection / "weapon-assembly")) / "native-models"
    unlinker = external(settings["native_unlinker"])
    zones = Path(config["assets"]["installation_root"]) / "zone/all"
    output.mkdir(parents=True, exist_ok=True)
    for mode, loads, target in (
        ("base", ["common_zm.ff"], "zm_tomb.ff"),
        ("shared", ["common_zm.ff"], "patch_zm.ff"),
        ("patch", ["common_zm.ff", "patch_zm.ff", "zm_tomb.ff"], "zm_tomb_patch.ff"),
    ):
        command = [str(unlinker), "--no-color", "--include-assets", "xmodel", "--model-format", "GLTF", "-o", str(output / mode)]
        for load in loads:
            command += ["--load", str(zones / load)]
        command.append(str(zones / target))
        log_path = output / (mode + ".log")
        with log_path.open("wb") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
        log = log_path.read_text(encoding="utf-8", errors="replace")
        if "ERROR:" in log or not re.search(r"Finished with \d+ warnings, 0 errors", log):
            raise ValueError("native skeleton export reported errors: " + mode)
