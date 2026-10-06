"""Compile the read-only callback against an existing external OAT build.

All intermediate files and extracted metadata are written to the supplied external
directory. Requires OAT's Release x86 libraries and Visual Studio C++ tools.
"""
import os
from pathlib import Path
import subprocess


def export(source, zone, output, image_output=None, include_actor_images=False, callback=None):
    output.mkdir(parents=True, exist_ok=True)
    callback = callback or Path(__file__).with_name("static_instances.cpp")
    installations = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Microsoft Visual Studio"
    developer = next(installations.glob("*/Community/Common7/Tools/VsDevCmd.bat"), None)
    if developer is None:
        raise RuntimeError("Visual Studio C++ tools unavailable")
    environment_script = output / "environment.cmd"
    environment_script.write_text(f'@echo off\ncall "{developer}" -arch=x86 -host_arch=x64 >nul\nset\n')
    result = subprocess.run(["cmd", "/d", "/c",
                             str(environment_script)],
                            check=True, capture_output=True)
    environment = dict(os.environ)
    for line in result.stdout.decode(errors="replace").splitlines():
        key, separator, value = line.partition("=")
        if separator and key:
            environment[key] = value
    includes = [source / "src" / folder for folder in
                ("Unlinking/ContentLister", "Common", "ZoneCommon", "Utils", "ObjCommon", "ObjWriting", "ObjImage", "ObjLoading")]
    includes.append(source / "build/src/ObjWriting")
    compiler = Path(environment["VCToolsInstallDir"]) / "bin/Hostx64/x86/cl.exe"
    args = [str(compiler), "/nologo", "/std:c++latest", "/W3", "/WX", "/EHsc", "/MD", "/O2",
            "/DNDEBUG", "/DARCH_x86", "/DWIN32", "/D_CRT_SECURE_NO_WARNINGS",
            *[f"/I{x}" for x in includes], "/c", str(callback), f"/Fo:{output / 'instances.obj'}"]
    compiled = subprocess.run(args, cwd=output, env=environment, capture_output=True)
    (output / "compile.log").write_bytes(compiled.stdout + compiled.stderr)
    if compiled.returncode:
        raise RuntimeError(compiled.stdout.decode(errors="replace") + compiled.stderr.decode(errors="replace"))
    args = [str(compiler.with_name("link.exe")), "/NOLOGO", "/MACHINE:X86", "/SUBSYSTEM:CONSOLE", "/LARGEADDRESSAWARE",
            f"/OUT:{output / 'Unlinker.exe'}", str(output / "instances.obj"),
            str(source / "build/obj/x86/Release/UnlinkerCli/main.obj"),
            *map(str, (source / "build/lib/Release_x86").glob("*.lib")),
            *[f"{name}.lib" for name in
              "kernel32 user32 gdi32 winspool comdlg32 advapi32 shell32 ole32 oleaut32 uuid odbc32 odbccp32".split()]]
    subprocess.run(args, cwd=output, env=environment, check=True, capture_output=True)
    with (output / "export.log").open("w") as log:
        if image_output is not None:
            environment["ORIGINS_DATA_ROOT"] = str(zone.parent)
            environment["ORIGINS_IMAGE_OUTPUT"] = str(image_output)
        if include_actor_images:
            environment["ORIGINS_INCLUDE_ACTOR_IMAGES"] = "1"
        subprocess.run([str(output / "Unlinker.exe"), "--no-color", "--list", str(zone)],
                       cwd=output, env=environment, check=True, stdout=log, stderr=log)
    return output / "static-instances.json"
