import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT).decode().strip()


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def sources():
    candidates = [ROOT / name for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "build.rs")]
    candidates += list((ROOT / "src").rglob("*"))
    candidates += list((ROOT / ".cargo").rglob("*"))
    candidates += list((ROOT / "config").rglob("*"))
    candidates += list((ROOT / "vendor").rglob("*"))
    return sorted(path for path in candidates if path.is_file())


def external(path):
    path = Path(path).resolve()
    if path.is_relative_to(ROOT):
        raise ValueError("Build evidence and pinned binaries must be outside the repository")
    return path


def snapshot(output):
    output = external(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    files = {}
    with zipfile.ZipFile(output.with_suffix(".zip"), "x", zipfile.ZIP_DEFLATED) as archive:
        for path in sources():
            name = path.relative_to(ROOT).as_posix()
            data = path.read_bytes()
            files[name] = hashlib.sha256(data).hexdigest()
            archive.writestr(name, data)
    result = {"revision": git("rev-parse", "HEAD"), "features": ["dev-dynamic"], "profile": "dev", "files": files}
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result


def pin(snapshot_path, output):
    import pefile

    expected = json.loads(Path(snapshot_path).read_text(encoding="utf-8"))
    current = {path.relative_to(ROOT).as_posix(): digest(path) for path in sources()}
    if current != expected["files"]:
        raise RuntimeError("Compiled inputs changed after the source snapshot; capture and build again")
    output = external(output)
    output.mkdir(parents=True, exist_ok=False)
    executable = ROOT / "target/debug/origins.exe"
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], cwd=ROOT).decode().strip())
    directories = [executable.parent, executable.parent / "deps", sysroot / "bin", sysroot / "lib/rustlib/x86_64-pc-windows-msvc/lib"]
    pending = [executable]
    binaries = {}
    while pending:
        path = pending.pop()
        if path.name in binaries:
            continue
        destination = output / path.name
        shutil.copy2(path, destination)
        binaries[path.name] = digest(destination)
        with pefile.PE(str(destination), fast_load=True) as executable_file:
            executable_file.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"]])
            for entry in getattr(executable_file, "DIRECTORY_ENTRY_IMPORT", []):
                name = entry.dll.decode()
                if not name.startswith(("bevy_dylib-", "std-")):
                    continue
                dependency = next((folder / name for folder in directories if (folder / name).is_file()), None)
                if dependency is None:
                    raise FileNotFoundError(name)
                pending.append(dependency)
    asset_root = ROOT / "assets/origins"
    paths = sorted(path for path in asset_root.rglob("*") if path.is_file())

    def asset(path):
        before = path.stat()
        sha256 = digest(path)
        after = path.stat()
        if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
            raise RuntimeError(f"Asset changed during capture: {path}")
        return path.relative_to(asset_root).as_posix(), {"sha256": sha256, "size": after.st_size, "mtime_ns": after.st_mtime_ns}

    with ThreadPoolExecutor(max_workers=4) as pool:
        assets = dict(pool.map(asset, paths))
    identity = {"source": expected, "source_archive": str(Path(snapshot_path).with_suffix(".zip").resolve()), "binaries": binaries, "asset_root": str(asset_root), "assets": assets}
    (output / "identity.json").write_text(json.dumps(identity, indent=2) + "\n", encoding="utf-8")
    return {"output": str(output), "revision": expected["revision"], "binaries": binaries, "asset_files": len(assets)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("snapshot", "pin"))
    parser.add_argument("--output", required=True)
    parser.add_argument("--snapshot")
    args = parser.parse_args()
    if args.command == "snapshot":
        result = snapshot(args.output)
        print(json.dumps({"revision": result["revision"], "source_files": len(result["files"])}))
    else:
        if not args.snapshot:
            parser.error("pin requires --snapshot")
        print(json.dumps(pin(args.snapshot, args.output)))


if __name__ == "__main__":
    main()
