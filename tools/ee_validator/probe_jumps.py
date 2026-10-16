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
    parser.add_argument("--region",choices=("catwalk","approaches","first-bridge","tank-side"),default="catwalk")
    args = parser.parse_args()
    client = Client(args.port)
    before = client.state()
    if not before["control"]["paused"]:
        raise RuntimeError("Pause the game before inspecting jump candidates")
    tuning = before["movement"]
    triangles = roof_triangles(args.assets / "model_export/veh_t6_dlc_mkiv_tank.collision.json")
    candidates = []
    wide = args.region == "approaches"
    bridge = args.region == "first-bridge"
    side = args.region == "tank-side"
    body_positions = [-67-i*.25 for i in range(27)] if bridge else ([-69-i*1.5 for i in range(9)] if wide else [-70-i*.25 for i in range(37)])
    sides = [-.4,.2,.8,1.4,2.] if bridge else ([.6,1.4,2.2] if wide else [i*.2 for i in range(12)])
    decks = [-4.5,-3.5] if wide or bridge else [-4.5,-4.,-3.5,-3.]
    speeds = [tuning["walk_speed"]*.5,tuning["walk_speed"],tuning["sprint_speed"]*.9,tuning["sprint_speed"]] if bridge else (
        [tuning["sprint_speed"]] if wide else [tuning["walk_speed"],tuning["sprint_speed"]*.9,tuning["sprint_speed"]])
    aims = [[x,-2.578,-73.15] for x in [35.5,36.3,37.]] if bridge else (
        [[x,-3.76,z] for x in [35.5,38,40.5,43] for z in [-69,-72,-75,-78,-81,-84,-87,-90]] if wide else [
        [41.8,-3.76,-82.2],[42.5,-3.76,-83.],[43.5,-3.76,-83.9]])
    if side:
        body_positions = [-69-i*.5 for i in range(29)]
        sides = [1.8+i*.2 for i in range(8)]
        decks = [-6.,-5.,-4.,-3.,-2.,-1.,0.,1.,2.,3.]
        speeds = [tuning["sprint_speed"]*.9,tuning["sprint_speed"]]
        aims = [[41.7,-3.76,-82.1],[42.,-3.76,-82.5],[43.5,-3.76,-83.9]]
    local_points = []
    radius = tuning["radius"]
    offsets = [(0,0),(-radius,0),(radius,0),(0,-radius),(0,radius)] if side else [(0,0)]
    for x in sides:
        for z in decks:
            heights = [roof(triangles,x+dx,z+dz) for dx,dz in offsets]
            heights = [y for y in heights if y is not None]
            if heights:
                local_points.append((x,max(heights),z))
    for body_z in body_positions:
        pose = path_pose(body_z)
        for x,y,z in local_points:
                feet = tank_point(pose,[x,y+.012,z])
                for speed in speeds:
                    for aim in aims:
                        dx,dz = aim[0]-feet[0],aim[2]-feet[2]
                        distance = math.hypot(dx,dz)
                        candidates.append({"request":{"feet":feet,"velocity":[dx/distance*speed,tuning["jump_speed"],dz/distance*speed],"aim":aim},
                                           "tank_z":body_z,"local":[x,y,z],"speed":speed})
    viable = []
    approaches = []
    for start in range(0,len(candidates),64):
        batch = candidates[start:start+64]
        response = client.rpc("origins/validator/jump_probes",[case["request"] for case in batch],timeout=30)
        for case,result in zip(batch,response["results"]):
            x,y,z = result["feet"]
            if result["landed"] is not None and not result["blocked"]:
                approaches.append({**case,"result":result})
            in_region = abs(y+2.578)<.2 and 34.5<x<37.4 and -74.1<z<-72.2 if bridge else (
                abs(y+3.76)<.3 and 41.2<x<46.35 and -85.35<z<-81.65)
            if side:
                in_region = -4.2<y<-2.8 and 36.3<x<46 and -88<z<-81
            if result["landed"] is not None and in_region:
                margin = min(x-34.5,37.4-x,z+74.1,-72.2-z) if bridge else min(x-41.55,46-x,z+85,-82-z)
                if side:
                    margin = min(x-36.3,46-x,z+88,-81-z)
                viable.append({**case,"result":result,"margin":margin})
        if start % 512 == 0:
            print(f"Inspected {min(start+64,len(candidates))}/{len(candidates)}; target landings: {len(viable)}",flush=True)
    after = client.state()
    if before["control"]["tick"] != after["control"]["tick"] or before["player"] != after["player"]:
        raise RuntimeError("Read-only inspection changed the paused player or simulation tick")
    viable.sort(key=lambda case:(case["result"]["blocked"],-case["margin"]))
    approaches.sort(key=lambda case:math.hypot(case["result"]["feet"][0]-43.5,case["result"]["feet"][2]+83.9))
    report = {"region":args.region,"inspected":len(candidates),"target_landings":len(viable),"best":viable[:30],
              "approaches":approaches[:60],
              "farthest_east":sorted(approaches,key=lambda case:-case["result"]["feet"][0])[:5],
              "scope":response["scope"],"paused_state_preserved":True}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2))
    print(json.dumps({**report,"best":viable[:3],"approaches":approaches[:3]}),flush=True)


if __name__ == "__main__":
    main()
