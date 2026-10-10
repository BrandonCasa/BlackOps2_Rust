import os
import subprocess
import threading
import time

import psutil

ACTIVE_CAPTURE_PROCESSES = {"origins.exe", "origins_flythrough.exe", "capture.exe", "tracy-capture.exe", "tracy-capture-daemon.exe",
                            "tracy-csvexport.exe", "wpr.exe", "xperf.exe", "renderdoccmd.exe"}
_pressure_sample = threading.local()


def resource_pressure(now):
    if not hasattr(_pressure_sample, "last_check"):
        psutil.cpu_percent()
        _pressure_sample.last_check = now
        _pressure_sample.busy = psutil.virtual_memory().available < 4*1024**3
    elif now-_pressure_sample.last_check >= 1.:
        _pressure_sample.busy = psutil.cpu_percent() >= 85.
        _pressure_sample.busy |= psutil.virtual_memory().available < 4*1024**3
        _pressure_sample.last_check = now
    return _pressure_sample.busy


class PreparationResources:
    def __init__(self, pause_file=None, gpu_memory_mib=2048, exit_for_shared_slot=False, saturate=False):
        self.pause_file = pause_file
        self.last_check = 0.
        self.busy = False
        self.gpu_memory_mib = gpu_memory_mib
        self.exit_for_shared_slot = exit_for_shared_slot
        self.shared_slot_busy = False
        self.saturate = saturate
        if os.name == "nt" and not saturate:
            psutil.Process().nice(psutil.BELOW_NORMAL_PRIORITY_CLASS)
        resource_pressure(time.monotonic())

    def refresh(self, gpu_admission=False, force=False, gpu_owned=False):
        now = time.monotonic()
        if not force and now-self.last_check < .5:
            return self.busy
        self.last_check = now
        names = set()
        for process in psutil.process_iter(["name"]):
            try:
                if process.info["name"]:
                    names.add(process.info["name"].lower())
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        self.shared_slot_busy = bool(names & ACTIVE_CAPTURE_PROCESSES)
        self.shared_slot_busy |= self.pause_file is not None and self.pause_file.exists()
        self.busy = self.shared_slot_busy
        if not self.saturate:
            self.busy |= resource_pressure(now)
        if (gpu_owned or gpu_admission) and not self.saturate:
            self.busy |= bool(names & {"texconv.exe", "cargo.exe", "rustc.exe"})
        if gpu_admission and not self.busy and not self.saturate:
            result = subprocess.run(["nvidia-smi", "--query-gpu=utilization.gpu,memory.free",
                "--format=csv,noheader,nounits"], capture_output=True, text=True, check=True)
            for row in result.stdout.strip().splitlines():
                utilization, free = [float(x.strip()) for x in row.split(",")]
                self.busy |= utilization >= 30. or free < self.gpu_memory_mib
            self.busy |= "texconv.exe" in names
        return self.busy

    def wait(self, gpu_admission=False, on_pause=None, gpu_owned=False):
        announced = False
        while self.refresh(gpu_admission, gpu_owned=gpu_owned):
            if self.shared_slot_busy and self.exit_for_shared_slot:
                print("Preparation yielded the shared game/capture slot; resume from saved candidates", flush=True)
                raise SystemExit(75)
            if not announced:
                print("Preparation paused for game/capture or resource pressure", flush=True)
                if on_pause is not None:
                    on_pause()
                announced = True
            time.sleep(1)


def main():
    import argparse
    import json
    from pathlib import Path
    import shutil
    parser = argparse.ArgumentParser(description='Cooperative admission for offline preparation alongside the shared game task.')
    parser.add_argument("--pause-file", type=Path)
    parser.add_argument("--snapshot", type=Path, nargs=2, metavar=("SOURCE", "DESTINATION"))
    args = parser.parse_args()
    guard = PreparationResources(args.pause_file)
    if args.snapshot:
        source, destination = (path.resolve() for path in args.snapshot)
        workspace = Path(__file__).resolve().parents[1]
        if destination.is_relative_to(workspace) or destination.is_relative_to(source) or source.is_relative_to(destination):
            raise ValueError("Snapshot output must be external and separate from its source")
        if destination.exists():
            raise ValueError("Use a fresh snapshot destination")
        def copy_file(source_file, destination_file):
            guard.wait()
            return shutil.copy2(source_file, destination_file)
        shutil.copytree(source, destination, copy_function=copy_file)
        print(json.dumps({"snapshot": str(destination)}), flush=True)
        return
    print(json.dumps({"busy": guard.refresh(force=True),
                      "cpu_percent": psutil.cpu_percent(interval=.2),
                      "available_memory_bytes": psutil.virtual_memory().available}))


if __name__ == "__main__":
    main()
