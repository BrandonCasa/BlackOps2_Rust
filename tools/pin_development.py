# /// script
# requires-python = ">=3.11"
# dependencies = ["pefile==2024.8.26"]
# ///

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
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


def toolchain():
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], cwd=ROOT).decode().strip())
    compiler = sysroot / "bin/rustc.exe"
    version = subprocess.check_output([str(compiler), "-vV"], cwd=ROOT).decode().strip()
    host = next(line.removeprefix("host: ") for line in version.splitlines() if line.startswith("host: "))
    return {"sysroot": str(sysroot), "compiler": str(compiler), "version": version, "host": host}


def snapshot(output, profiling=None):
    output = external(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    files = {}
    with zipfile.ZipFile(output.with_suffix(".zip"), "x", zipfile.ZIP_DEFLATED) as archive:
        for path in sources():
            name = path.relative_to(ROOT).as_posix()
            data = path.read_bytes()
            files[name] = hashlib.sha256(data).hexdigest()
            archive.writestr(name, data)
    features = ["dev-dynamic"] + ([profiling] if profiling else [])
    result = {"revision": git("rev-parse", "HEAD"), "features": features, "profile": "dev", "toolchain": toolchain(), "files": files}
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result


def copy_binaries(executable, output, compiler):
    import pefile

    sysroot = Path(compiler["sysroot"])
    directories = [executable.parent, executable.parent / "deps", sysroot / "bin", sysroot / "lib/rustlib" / compiler["host"] / "lib"]
    available = {}
    for directory in directories:
        for path in directory.glob("*.dll"):
            available.setdefault(path.name.casefold(), path)
    pending = [executable]
    binaries = {}
    seen = set()
    while pending:
        path = pending.pop()
        if path.name.casefold() in seen:
            continue
        seen.add(path.name.casefold())
        destination = output / path.name
        original = digest(path)
        shutil.copy2(path, destination)
        binaries[path.name] = digest(destination)
        if original != binaries[path.name] or digest(path) != original:
            raise RuntimeError(f"Binary changed while pinning: {path}")
        with pefile.PE(str(destination), fast_load=True) as executable_file:
            executable_file.parse_data_directories(directories=[
                pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"],
                pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT"]])
            imports = list(getattr(executable_file, "DIRECTORY_ENTRY_IMPORT", []))
            imports += list(getattr(executable_file, "DIRECTORY_ENTRY_DELAY_IMPORT", []))
            for entry in imports:
                name = entry.dll.decode()
                dependency = available.get(name.casefold())
                if dependency is None:
                    if name.casefold().startswith(("bevy_dylib", "std-")):
                        raise FileNotFoundError(f"Missing runtime dependency {name} imported by {path}")
                    continue
                pending.append(dependency)
    return binaries


def link_assets(output, asset_root):
    if not (asset_root / "game.json").is_file():
        raise FileNotFoundError(f"Runtime asset pack is missing game.json: {asset_root}")
    (output / "assets").mkdir()
    subprocess.run([
        "powershell", "-NoProfile", "-NonInteractive", "-Command",
        "New-Item -ItemType Junction -Path $env:ORIGINS_BUNDLE_LINK "
        "-Target $env:ORIGINS_BUNDLE_ASSETS -ErrorAction Stop | Out-Null",
    ], check=True, env={**os.environ, "ORIGINS_BUNDLE_LINK": str(output / "assets/origins"),
                       "ORIGINS_BUNDLE_ASSETS": str(asset_root)})


def pin(snapshot_path, output, target_dir=None, asset_root=None, executable=None):
    expected = json.loads(Path(snapshot_path).read_text(encoding="utf-8"))
    current = {path.relative_to(ROOT).as_posix(): digest(path) for path in sources()}
    if current != expected["files"]:
        raise RuntimeError("Compiled inputs changed after the source snapshot; capture and build again")
    output = external(output)
    output.mkdir(parents=True, exist_ok=False)
    executable = executable or Path(target_dir or ROOT / "target").resolve() / "debug/origins.exe"
    binaries = copy_binaries(executable, output, expected.get("toolchain") or toolchain())
    asset_root = Path(asset_root or ROOT / "assets/origins").resolve()
    link_assets(output, asset_root)
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


def build(output, target_dir=None, asset_root=None, profiling=None):
    output = external(output)
    output.mkdir(parents=True, exist_ok=False)
    snapshot_path = output / "source.json"
    source = snapshot(snapshot_path, profiling)
    compiler = source["toolchain"]
    environment = {**os.environ, "RUSTC": compiler["compiler"],
                   "RUSTDOC": str(Path(compiler["sysroot"]) / "bin/rustdoc.exe")}
    command = ["cargo", "build", "--locked", "--features", ",".join(source["features"]), "--bin", "origins",
               "--message-format=json-render-diagnostics"]
    if target_dir:
        command += ["--target-dir", str(target_dir.resolve())]
    executable = None
    with (output / "cargo.jsonl").open("w", encoding="utf-8") as log:
        with subprocess.Popen(command, cwd=ROOT, env=environment, stdout=subprocess.PIPE,
                              text=True, encoding="utf-8") as process:
            for line in process.stdout:
                log.write(line)
                message = json.loads(line)
                if message.get("reason") == "compiler-message":
                    print(message["message"].get("rendered", ""), end="", file=sys.stderr)
                if message.get("reason") == "compiler-artifact" and message.get("executable"):
                    if message["target"]["name"] == "origins":
                        executable = Path(message["executable"])
            if process.wait() != 0:
                raise RuntimeError(f"Development build failed; see {output / 'cargo.jsonl'}")
    if executable is None:
        raise RuntimeError("Cargo did not report the origins executable")
    return pin(snapshot_path, output / "bin", target_dir, asset_root, executable)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("build", "snapshot", "pin"))
    parser.add_argument("--output", required=True)
    parser.add_argument("--snapshot")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--asset-root", type=Path)
    parser.add_argument("--profiling", choices=("profile-chrome", "profile-tracy", "profile-tracy-memory"))
    args = parser.parse_args()
    if args.command == "build":
        print(json.dumps(build(args.output, args.target_dir, args.asset_root, args.profiling)))
    elif args.command == "snapshot":
        result = snapshot(args.output, args.profiling)
        print(json.dumps({"revision": result["revision"], "source_files": len(result["files"])}))
    else:
        if not args.snapshot:
            parser.error("pin requires --snapshot")
        print(json.dumps(pin(args.snapshot, args.output, args.target_dir, args.asset_root)))


if __name__ == "__main__":
    main()
