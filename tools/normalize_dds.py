"""Use DirectXTex to preserve opaque legacy DDS texels in Bevy's supported format."""
import json
import struct
import subprocess
import sys
from pathlib import Path

images, texconv, report = map(Path, sys.argv[1:])
converted = []
for path in sorted(images.glob("*.dds")):
    with path.open("rb") as stream:
        header = stream.read(128)
    # DDS_PIXELFORMAT: DDPF_RGB, no alpha, 32 bits, original BGRX masks.
    if len(header) != 128 or header[:4] != b"DDS " or struct.unpack_from("<8I", header, 76) != (
        32, 64, 0, 32, 0xFF0000, 0xFF00, 0xFF, 0
    ):
        continue
    original = path.read_bytes()
    mip_count = max(1, struct.unpack_from("<I", header, 28)[0])
    result = subprocess.run([
        str(texconv.resolve()), "-nologo", "-ft", "dds", "-f", "R8G8B8A8_UNORM",
        "-m", str(mip_count), "-y", "-o", str(images.resolve()), str(path.resolve()),
    ], capture_output=True, text=True,
        creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0)
    try:
        if result.returncode:
            raise ValueError(result.stdout + result.stderr)
        normalized = path.read_bytes()
        assert len(normalized) == len(original), "DDS texel count changed"
        assert normalized[12:20] == original[12:20], "DDS dimensions changed"
        assert max(1, struct.unpack_from("<I", normalized, 28)[0]) == mip_count
        assert struct.unpack_from("<4I", normalized, 92) == (255, 65280, 16711680, 4278190080)
        assert normalized[128::4] == original[130::4], "Red texels changed"
        assert normalized[129::4] == original[129::4], "Green texels changed"
        assert normalized[130::4] == original[128::4], "Blue texels changed"
        assert all(alpha == 255 for alpha in normalized[131::4]), "Non-opaque alpha"
    except Exception:
        path.write_bytes(original)
        raise
    converted.append({"image": path.name, "mips": mip_count, "verified_texels": (len(original) - 128) // 4})
report.write_text(json.dumps(converted, indent=2) + "\n", encoding="utf-8")
print(f"Normalized and verified {len(converted)} opaque DDS images")
