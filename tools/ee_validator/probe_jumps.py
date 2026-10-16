"""Inspect thousands of jump candidates through read-only native BRP collision queries."""
import argparse
import json
import math
from pathlib import Path
from client import Client
from routes import tank_point


def roof_triangles(path):
    mesh = json.loads(path.read_text())
    vertices = [[-y * .0254, z * .0254, -x * .0254] for x,y,z in mesh["vertices"]]
    return [[vertices[i] for i in face] for face in mesh["triangles"]]


def roof(triangles, x, z):
    heights = []
    for a,b,c in triangles:
        denominator = (b[2]-c[2])*(a[0]-c[0]) + (c[0]-b[0])*(a[2]-c[2])
        if abs(denominator) < 1e-8:
            continue
        u = ((b[2]-c[2])*(x-c[0]) + (c[0]-b[0])*(z-c[2])) / denominator
        v = ((c[2]-a[2])*(x-c[0]) + (a[0]-c[0])*(z-c[2])) / denominator
        if min(u,v,1-u-v) >= -1e-6:
            heights.append(u*a[1] + v*b[1] + (1-u-v)*c[1])
    return max(heights) if heights else None


def path_pose(z):
    a,b = ([34.036,-.2921,-45.4914],[33.9852,-2.54,-73.9902]) if z >= -73.9902 else (
        [33.9852,-2.54,-73.9902],[32.209994,-3.048,-83.769708])
    fraction = (z-a[2]) / (b[2]-a[2])
    yaw = math.atan2(a[0]-b[0], a[2]-b[2])
    return {"position":[a[i]+fraction*(b[i]-a[i]) for i in range(3)],
            "rotation":[0,math.sin(yaw/2),0,math.cos(yaw/2)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port",type=int,default=15702)
    parser.add_argument("--assets",type=Path,default=Path("target/assets"))
    parser.add_argument("--output",type=Path,required=True)
    args = parser.parse_args()
    client = Client(args.port)
    before = client.state()
    if not before["control"]["paused"]:
        raise RuntimeError("Pause the game before inspecting jump candidates")
    tuning = before["movement"]
    triangles = roof_triangles(args.assets / "model_export/veh_t6_dlc_mkiv_tank.collision.json")
    candidates = []
    for body_z in [-70-i*.25 for i in range(37)]:
        pose = path_pose(body_z)
        for x in [i*.2 for i in range(12)]:
            for z in [-4.5,-4.,-3.5,-3.]:
                y = roof(triangles,x,z)
                if y is None:
                    continue
                feet = tank_point(pose,[x,y+.012,z])
                for speed in [tuning["walk_speed"],tuning["sprint_speed"]*.9,tuning["sprint_speed"]]:
                    for aim in [[41.8,-3.76,-82.2],[42.5,-3.76,-83.],[43.5,-3.76,-83.9]]:
                        dx,dz = aim[0]-feet[0],aim[2]-feet[2]
                        distance = math.hypot(dx,dz)
                        candidates.append({"request":{"feet":feet,"velocity":[dx/distance*speed,tuning["jump_speed"],dz/distance*speed],"aim":aim},
                                           "tank_z":body_z,"local":[x,y,z],"speed":speed})
    viable = []
    for start in range(0,len(candidates),64):
        batch = candidates[start:start+64]
        response = client.rpc("origins/validator/jump_probes",[case["request"] for case in batch],timeout=30)
        for case,result in zip(batch,response["results"]):
            x,y,z = result["feet"]
            if result["landed"] is not None and abs(y+3.76)<.3 and 41.55<x<46 and -85<z<-82:
                margin = min(x-41.55,46-x,z+85,-82-z)
                viable.append({**case,"result":result,"margin":margin})
        if start % 512 == 0:
            print(f"Inspected {min(start+64,len(candidates))}/{len(candidates)}; catwalk landings: {len(viable)}",flush=True)
    after = client.state()
    if before["control"]["tick"] != after["control"]["tick"] or before["player"] != after["player"]:
        raise RuntimeError("Read-only inspection changed the paused player or simulation tick")
    viable.sort(key=lambda case:(case["result"]["blocked"],-case["margin"]))
    report = {"inspected":len(candidates),"catwalk_landings":len(viable),"best":viable[:30],
              "scope":response["scope"],"paused_state_preserved":True}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2))
    print(json.dumps({**report,"best":viable[:3]}),flush=True)


if __name__ == "__main__":
    main()
