"""Extract the shaders actually used at development d000103 and fixed swarm state."""
import ast
import math
from pathlib import Path
import numpy as np

directory = Path("assets/shaders")
directory.mkdir(parents=True,exist_ok=True)
sources = [(Path("omamusi/gpu.py"),None),(Path("omamusi/visual/horizon.py"),"REFERENCE_EVENT_HORIZON_FRAGMENT")]
for path,only in sources:
    for node in ast.parse(path.read_text()).body:
        if not isinstance(node,ast.Assign) or not isinstance(node.value,ast.Constant) or not isinstance(node.value.value,str):
            continue
        name=node.targets[0].id
        if only is not None and name!=only:
            continue
        if only is None and (not name.endswith(("_VERTEX","_FRAGMENT")) or name=="EVENT_HORIZON_FRAGMENT"):
            continue
        filename=name.lower().replace("_vertex",".vert").replace("_fragment",".frag").replace("reference_","")
        (directory/filename).write_text(node.value.value)

count=25840
rng=np.random.default_rng(2584)
state=np.zeros((count,6),dtype=np.float32)
group=np.arange(count,dtype=np.float32)%13.0
angle=group*(math.tau/13.0)
radius=rng.uniform(.25,2.1,count).astype(np.float32)
state[:,0]=np.cos(angle)*radius+rng.normal(0,.16,count)
state[:,1]=np.sin(angle)*radius+rng.normal(0,.16,count)
state[:,2]=rng.uniform(-2.2,2.2,count)
speed=rng.uniform(.08,.24,count)
state[:,3]=-np.sin(angle)*speed
state[:,4]=np.cos(angle)*speed
state[:,5]=rng.normal(0,.025,count)
Path("assets/swarm-seeds.bin").write_bytes(state.astype("<f4").tobytes())
print("Extracted shader sources and 25,840 original swarm seeds")
