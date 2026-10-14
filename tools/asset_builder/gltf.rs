use crate::{core::*, geometry::Surface, pack::Pack};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

pub struct Scene {
    pub data: Value,
    bytes: Vec<u8>,
    material_indices: BTreeMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_rotation_avoids_single_key_cubic_loader_path() {
        for frames in [0,10] {
            let mut scene=Scene::new("native");
            let owner=scene.node(json!({"name":"native"}));
            let identity=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.];
            let joints=[crate::geometry::Joint {name:Some("root".into()),parent:None,bind:identity,
                bind_local:identity,inverse_bind:identity,default_rotation:[0.,0.,0.,1.],default_translation:[0.;3]}];
            scene.skin(owner,&joints).unwrap();
            let clip=crate::animation::Clip {name:"constant".into(),frame_rate:30.,frames,source_flags:[0;4],streamed_file_size:123456789,loop_entry_time:2.5,notifies:Vec::new(),
                bones:vec![crate::animation::Bone {name:Some("root".into()),rotation:Some(crate::animation::Track {
                    frames:vec![0],values:vec![[0.,0.,0.,0.99997]]}),translation:None}]};
            scene.animation(owner,&joints,&clip).unwrap();
            let sampler=&scene.data["animations"][0]["samplers"][0];
            let input=sampler["input"].as_u64().unwrap() as usize;
            let output=sampler["output"].as_u64().unwrap() as usize;
            assert_eq!(sampler["interpolation"],if frames == 0 {"LINEAR"} else {"CUBICSPLINE"});
            assert_eq!(scene.data["accessors"][input]["count"],if frames == 0 {1} else {2});
            assert_eq!(scene.data["accessors"][output]["count"],if frames == 0 {1} else {6});
            let mut jumpcut=clip;
            jumpcut.notifies.push((Some("jumpcut".into()),0.5));
            assert!(scene.animation(owner,&joints,&jumpcut).is_err());
            jumpcut.notifies.clear();
            jumpcut.source_flags[0]=1;
            assert!(scene.animation(owner,&joints,&jumpcut).is_err());
        }
    }

    #[test]
    fn joint_trs_preserves_quarter_turn_and_half_turns() {
        let matrix = [0.,0.,-1.,0.,0.,1.,0.,0.,1.,0.,0.,0.,3.,-2.,5.,1.];
        let (node,_) = joint_transform(matrix).unwrap();
        assert!(node.get("matrix").is_none());
        assert_eq!(node["translation"],json!([3.,-2.,5.]));
        let q: [f64;4] = std::array::from_fn(|i| node["rotation"][i].as_f64().unwrap());
        assert!(q[0].abs() < 0.000001 && q[2].abs() < 0.000001);
        assert!((q[1].abs()-0.5_f64.sqrt()).abs() < 0.000001);
        assert!((q[3].abs()-0.5_f64.sqrt()).abs() < 0.000001);
        assert!(q[1]*q[3] > 0.);
        for axis in 0..3 {
            let mut matrix = [0.;16];
            for i in 0..3 { matrix[i*5]=if i == axis {1.} else {-1.}; }
            matrix[15]=1.;
            let (node,_) = joint_transform(matrix).unwrap();
            let q: [f64;4] = std::array::from_fn(|i| node["rotation"][i].as_f64().unwrap());
            assert!((q[axis].abs()-1.).abs() < 0.000001);
            assert!((0..4).filter(|&i| i != axis).all(|i| q[i].abs() < 0.000001));
        }
        let mut scaled = [1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.];
        scaled[0]=2.;
        assert!(joint_transform(scaled).is_err());
        scaled[0]=-1.;
        assert!(joint_transform(scaled).is_err());
        scaled[0]=1.; scaled[4]=0.01;
        assert!(joint_transform(scaled).is_err());
        scaled[4]=0.; scaled[0]=f32::NAN;
        assert!(joint_transform(scaled).is_err());
    }
}

fn joint_transform(matrix: [f32;16]) -> Result<(Value,[f32;16])> {
    if matrix.iter().any(|v| !v.is_finite()) || [3,7,11].into_iter().any(|i| matrix[i] != 0.) || matrix[15] != 1. {
        return Err("Native joint local transform is not finite affine data".into());
    }
    let element = |row: usize,column: usize| matrix[column*4+row] as f64;
    let trace = element(0,0)+element(1,1)+element(2,2);
    let mut rotation = [0_f64;4];
    if trace > 0. {
        let divisor = (1.+trace).sqrt()*2.;
        rotation = [(element(2,1)-element(1,2))/divisor,(element(0,2)-element(2,0))/divisor,
            (element(1,0)-element(0,1))/divisor,divisor*0.25];
    } else {
        let i = (0..3).max_by(|&a,&b| element(a,a).total_cmp(&element(b,b))).unwrap();
        let (j,k) = ((i+1)%3,(i+2)%3);
        let divisor = (1.+element(i,i)-element(j,j)-element(k,k)).sqrt()*2.;
        rotation[i]=divisor*0.25;
        rotation[j]=(element(j,i)+element(i,j))/divisor;
        rotation[k]=(element(k,i)+element(i,k))/divisor;
        rotation[3]=(element(k,j)-element(j,k))/divisor;
    }
    let length = rotation.iter().map(|v| v*v).sum::<f64>().sqrt();
    if !length.is_finite() || length <= 0. { return Err("Native joint has no representable rotation".into()); }
    let rotation = rotation.map(|v| (v/length) as f32);
    let [x,y,z,w] = rotation;
    let translation = [matrix[12],matrix[13],matrix[14]];
    let reconstructed = [1.-2.*(y*y+z*z),2.*(x*y+z*w),2.*(x*z-y*w),0.,
        2.*(x*y-z*w),1.-2.*(x*x+z*z),2.*(y*z+x*w),0.,
        2.*(x*z+y*w),2.*(y*z-x*w),1.-2.*(x*x+y*y),0.,translation[0],translation[1],translation[2],1.];
    let error = matrix.into_iter().zip(reconstructed).map(|(a,b)| (a-b).abs()).fold(0_f32,f32::max);
    if error > 0.0001 { return Err(format!("Native joint transform is not rigid glTF TRS: maximum residual {error}").into()); }
    Ok((json!({"translation":translation,"rotation":rotation}),reconstructed))
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self { data: json!({"asset":{"version":"2.0","generator":"Origins Rust asset builder"},
            "scene":0, "scenes":[{"name":name,"nodes":[]}], "nodes":[], "meshes":[],
            "materials":[], "buffers":[], "bufferViews":[], "accessors":[]}), bytes:Vec::new(), material_indices:BTreeMap::new() }
    }

    fn accessor(&mut self, bytes: &[u8], count: usize, component: u32, shape: &str, target: u32) -> usize {
        while self.bytes.len() % 4 != 0 { self.bytes.push(0); }
        let views = self.data["bufferViews"].as_array_mut().unwrap();
        let view = views.len();
        let mut data = json!({"buffer":0,"byteOffset":self.bytes.len(),"byteLength":bytes.len()});
        if target != 0 { data["target"]=json!(target); }
        views.push(data);
        self.bytes.extend_from_slice(bytes);
        let accessors = self.data["accessors"].as_array_mut().unwrap();
        let index = accessors.len();
        accessors.push(json!({"bufferView":view,"componentType":component,"count":count,"type":shape}));
        index
    }

    fn floats<const N: usize>(&mut self, values: &[[f32; N]], shape: &str) -> usize {
        let bytes: Vec<_> = values.iter().flatten().flat_map(|value| value.to_le_bytes()).collect();
        self.accessor(&bytes, values.len(), 5126, shape, 34962)
    }

    pub fn mesh<'a>(&mut self, name: &str, surfaces: impl IntoIterator<Item = &'a Surface>, materials: &BTreeMap<String, usize>, skinned: bool) -> Result<usize> {
        let mut primitives = Vec::new();
        for surface in surfaces {
            if surface.positions.is_empty() || surface.indices.is_empty() { continue; }
            let positions = self.floats(&surface.positions, "VEC3");
            let minimum: [f32; 3] = std::array::from_fn(|axis| surface.positions.iter().map(|v| v[axis]).fold(f32::INFINITY, f32::min));
            let maximum: [f32; 3] = std::array::from_fn(|axis| surface.positions.iter().map(|v| v[axis]).fold(f32::NEG_INFINITY, f32::max));
            self.data["accessors"][positions]["min"] = json!(minimum);
            self.data["accessors"][positions]["max"] = json!(maximum);
            let normal = self.floats(&surface.normals, "VEC3");
            let tangent = self.floats(&surface.tangents, "VEC4");
            let uv = self.floats(&surface.uv, "VEC2");
            let index_bytes: Vec<_> = surface.indices.iter().flat_map(|index| index.to_le_bytes()).collect();
            let indices = self.accessor(&index_bytes, surface.indices.len(), 5123, "SCALAR", 34963);
            let material = *materials.get(&surface.material.to_ascii_lowercase())
                .ok_or_else(|| format!("Converted surface material is missing: {}", surface.material))?;
            let surface_material = &self.data["materials"][material]["extras"]["surface"];
            let flag_motion = !surface_material["source_flag_motion"].is_null();
            let transfer = surface_material["source_vertex_color_transfer"].as_str();
            let colors = if matches!(transfer,Some("gamma2" | "gamma2_alpha")) {
                let values: Vec<[f32; 4]> = surface.colors.iter().map(|color| std::array::from_fn(|c| {
                    let value = color[c] as f32 / 255.;
                    if c < 3 { value * value * if transfer == Some("gamma2_alpha") {color[3] as f32/255.} else {1.} }
                    else if surface_material["source_coverage"] == "unlit_add" {1.} else {value}
                })).collect();
                self.floats(&values, "VEC4")
            } else {
                let bytes: Vec<_> = surface.colors.iter().flatten().copied().collect();
                let index = self.accessor(&bytes, surface.colors.len(), 5121, "VEC4", 34962);
                self.data["accessors"][index]["normalized"] = json!(true);
                index
            };
            let mut attributes = json!({"POSITION":positions,"NORMAL":normal,"TANGENT":tangent,"TEXCOORD_0":uv,"COLOR_0":colors});
            if flag_motion {
                if surface.raw_normals.len() != surface.positions.len() || surface.raw_tangents.len() != surface.positions.len() || surface.packed_basis.len() != surface.positions.len() {
                    return Err("Source flag raw basis does not match its vertices".into());
                }
                attributes["_ORIGINS_NORMAL"] = json!(self.floats(&surface.raw_normals,"VEC3"));
                attributes["_ORIGINS_TANGENT"] = json!(self.floats(&surface.raw_tangents,"VEC3"));
                let packed: Vec<_> = surface.packed_basis.iter().flatten().flat_map(|word| word.to_le_bytes()).collect();
                attributes["_ORIGINS_PACKED_BASIS"] = json!(self.accessor(&packed,surface.packed_basis.len(),5125,"VEC2",34962));
            }
            if !surface.lightmap_uv.is_empty() {
                attributes["TEXCOORD_1"] = json!(self.floats(&surface.lightmap_uv, "VEC2"));
            }
            if skinned {
                let bindings = surface.bindings.as_ref().ok_or("Model mesh has no native vertex bindings")?;
                let source = bindings.selected();
                if source.len() != surface.positions.len() { return Err("Native vertex bindings do not match model mesh vertices".into()); }
                let influences = source.iter().copied().collect::<Option<Vec<_>>>().ok_or("Model mesh has an unbound native vertex")?;
                let joints: Vec<_> = influences.iter().flat_map(|value| value.joints).flat_map(u16::to_le_bytes).collect();
                let weights: Vec<_> = influences.iter().map(|value| value.weights).collect();
                attributes["JOINTS_0"] = json!(self.accessor(&joints,influences.len(),5123,"VEC4",34962));
                attributes["WEIGHTS_0"] = json!(self.floats(&weights,"VEC4"));
            }
            primitives.push(json!({"attributes":attributes,
                "indices":indices,"material":material,"extras":{"native_surface":surface.name,"native_material":self.data["materials"][material]["extras"]["surface"]["native"]["name"].as_str().unwrap_or(&surface.material)}}));
        }
        if primitives.is_empty() { return Err(format!("Native mesh has no triangles: {name}").into()); }
        let meshes = self.data["meshes"].as_array_mut().unwrap();
        let index = meshes.len();
        meshes.push(json!({"name":name,"primitives":primitives}));
        Ok(index)
    }

    pub fn materials<'a>(&mut self, surfaces: impl IntoIterator<Item = &'a Surface>, converted: &BTreeMap<String, Value>) -> Result<BTreeMap<String, usize>> {
        let mut indices = BTreeMap::new();
        for surface in surfaces {
            let key = surface.material.to_ascii_lowercase();
            if indices.contains_key(&key) { continue; }
            if let Some(&index) = self.material_indices.get(&key) {
                indices.insert(key, index);
                continue;
            }
            let material = converted.get(&key).ok_or_else(|| format!("Material has not been converted: {}", surface.material))?;
            let list = self.data["materials"].as_array_mut().unwrap();
            self.material_indices.insert(key.clone(), list.len());
            indices.insert(key, list.len());
            list.push(material.clone());
        }
        Ok(indices)
    }

    pub fn node(&mut self, node: Value) -> usize {
        let nodes = self.data["nodes"].as_array_mut().unwrap();
        let index = nodes.len();
        nodes.push(node);
        self.data["scenes"][0]["nodes"].as_array_mut().unwrap().push(json!(index));
        index
    }

    pub fn child(&mut self, parent: usize, node: Value) -> usize {
        let nodes = self.data["nodes"].as_array_mut().unwrap();
        let index = nodes.len();
        nodes.push(node);
        let children = nodes[parent].as_object_mut().unwrap().entry("children").or_insert_with(|| json!([]));
        children.as_array_mut().unwrap().push(json!(index));
        index
    }

    pub fn skin(&mut self, owner: usize, joints: &[crate::geometry::Joint]) -> Result<()> {
        if joints.is_empty() { return Err("Model skin has no native joints".into()); }
        let mut nodes = Vec::with_capacity(joints.len());
        let mut poses = Vec::with_capacity(joints.len());
        for joint in joints {
            let parent = match joint.parent {
                Some(parent) => *nodes.get(parent).ok_or("Model skin parent must precede its child")?,
                None => owner,
            };
            let (mut node,local) = joint_transform(joint.bind_local)?;
            let pose = match joint.parent {
                Some(parent) => crate::geometry::compose(*poses.get(parent).ok_or("Model skin pose parent is missing")?,local),
                None => local,
            };
            let error = pose.into_iter().zip(joint.bind).map(|(a,b)| (a-b).abs()).fold(0_f32,f32::max);
            if error > 0.0001 { return Err(format!("glTF joint TRS hierarchy differs from native bind pose: maximum residual {error}").into()); }
            poses.push(pose);
            if let Some(name) = &joint.name { node["name"]=json!(name); }
            node["extras"] = json!({"native_local_default":{"rotation":joint.default_rotation,"translation":joint.default_translation}});
            nodes.push(self.child(parent,node));
        }
        let bytes: Vec<_> = joints.iter().flat_map(|joint| joint.inverse_bind).flat_map(f32::to_le_bytes).collect();
        let accessor = self.accessor(&bytes,joints.len(),5126,"MAT4",0);
        let skins = self.data.as_object_mut().unwrap().entry("skins").or_insert_with(|| json!([])).as_array_mut().unwrap();
        let index = skins.len();
        skins.push(json!({"joints":nodes,"inverseBindMatrices":accessor}));
        self.data["nodes"][owner]["skin"]=json!(index);
        Ok(())
    }

    pub fn animation(&mut self, owner: usize, joints: &[crate::geometry::Joint], clip: &crate::animation::Clip) -> Result<usize> {
        if clip.source_flags.iter().any(|&flag| flag != 0) {
            return Err(format!("Animation {} requires native bLoop/bDelta/bDelta3D/bLeftHandGripIK interpretation before standalone export: {:?}; animation-tree composition must be selected separately",clip.name,clip.source_flags).into());
        }
        if !clip.loop_entry_time.is_finite() { return Err("Native animation loop-entry time is non-finite".into()); }
        if clip.notifies.iter().any(|(name,_)| name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case("jumpcut"))) {
            return Err("Native animation jumpcut intervals require source-faithful stepped sampling".into());
        }
        let skin = self.data["nodes"][owner]["skin"].as_u64().ok_or("Animation owner has no native skin")? as usize;
        let nodes = self.data["skins"][skin]["joints"].as_array().ok_or("Animation owner skin has no joints")?
            .iter().map(|value| value.as_u64().map(|value| value as usize).ok_or("Invalid animation joint node"))
            .collect::<std::result::Result<Vec<_>,_>>()?;
        if nodes.len() != joints.len() { return Err("Animation skin and model joint counts differ".into()); }
        let mut names=std::collections::BTreeSet::new();
        for name in joints.iter().filter_map(|joint| joint.name.as_ref()) {
            if !names.insert(name.to_ascii_lowercase()) { return Err("Native animation binding requires the DObj duplicate-bone consumer for repeated joint names".into()); }
        }
        let mut channels = Vec::new();
        let mut samplers = Vec::new();
        let mut matched = 0;
        for (joint,&node) in joints.iter().zip(&nodes) {
            let name = joint.name.as_deref();
            let mut candidates = clip.bones.iter().filter(|bone| match (name,bone.name.as_deref()) {
                (Some(a),Some(b)) => a.eq_ignore_ascii_case(b), _ => false,
            });
            let bone = candidates.next();
            if candidates.next().is_some() { return Err(format!("Animation {} has ambiguous native bone name {name:?}",clip.name).into()); }
            if bone.is_some() { matched+=1; }
            for rotation in [true,false] {
                let mut frames = vec![0,clip.frames];
                frames.dedup();
                let mut rotations = Vec::new();
                let mut translations = Vec::new();
                if rotation {
                    if let Some(track) = bone.and_then(|bone| bone.rotation.as_ref()) {
                        frames=track.frames.clone(); rotations=track.values.clone();
                    } else { rotations.resize(frames.len(),joint.default_rotation); }
                } else {
                    if let Some(track) = bone.and_then(|bone| bone.translation.as_ref()) {
                        frames=track.frames.clone();
                        translations=track.values.iter().map(|value| std::array::from_fn(|axis| value[axis]+joint.default_translation[axis])).collect();
                    } else { translations.resize(frames.len(),joint.default_translation); }
                }
                if frames.is_empty() { return Err("Native animation track has no keys".into()); }
                if frames.len() != if rotation { rotations.len() } else { translations.len() } {
                    return Err("Native animation key and value counts differ".into());
                }
                if frames.len() == 1 && frames[0] == 0 && clip.frames > 0 {
                    frames.push(clip.frames);
                    if rotation { rotations.push(rotations[0]); } else { translations.push(translations[0]); }
                }
                let time_bytes: Vec<_> = frames.iter().flat_map(|&frame| (frame as f32/clip.frame_rate).to_le_bytes()).collect();
                let input=self.accessor(&time_bytes,frames.len(),5126,"SCALAR",0);
                self.data["accessors"][input]["min"]=json!([frames[0] as f32/clip.frame_rate]);
                self.data["accessors"][input]["max"]=json!([frames[frames.len()-1] as f32/clip.frame_rate]);
                let (values,shape,count,interpolation) = if rotation {
                    let (samples,interpolation)=crate::animation::rotation_samples(&frames,&rotations,clip.frame_rate)?;
                    (samples.iter().flatten().flat_map(|value| value.to_le_bytes()).collect::<Vec<_>>(),"VEC4",samples.len(),interpolation)
                } else {
                    (translations.iter().flatten().flat_map(|value| value.to_le_bytes()).collect::<Vec<_>>(),"VEC3",translations.len(),"LINEAR")
                };
                let output=self.accessor(&values,count,5126,shape,0);
                channels.push(json!({"sampler":samplers.len(),"target":{"node":node,"path":if rotation {"rotation"} else {"translation"}}}));
                samplers.push(json!({"input":input,"output":output,"interpolation":interpolation}));
            }
        }
        if matched == 0 { return Err(format!("Animation {} has no named bones on its explicitly selected model",clip.name).into()); }
        let animations=self.data.as_object_mut().unwrap().entry("animations").or_insert_with(|| json!([])).as_array_mut().unwrap();
        let index=animations.len();
        animations.push(json!({"name":clip.name,"channels":channels,"samplers":samplers,
            "extras":{"native_frame_rate":clip.frame_rate,"native_frames":clip.frames,"native_source_flags":clip.source_flags,
                "native_streamed_file_size":clip.streamed_file_size,"native_loop_entry_time":clip.loop_entry_time,
                "native_notifies":clip.notifies.iter().map(|(name,time)| json!({"name":name,"fraction":time,"time":time*clip.frames as f32/clip.frame_rate})).collect::<Vec<_>>(),
                "native_bones_outside_selected_model":clip.bones.iter().filter(|bone| bone.name.as_ref().is_none_or(|name| !names.contains(&name.to_ascii_lowercase()))).map(|bone| &bone.name).collect::<Vec<_>>(),
                "native_rotation_interpolation":"component-linear quaternion interpolation preserved by normalized cubic output",
                "native_pose_path":"standalone local tracks plus authored local defaults; no DObj attachments or additive blending"}}));
        Ok(index)
    }

    pub fn collision(&mut self, name: &str, positions: &[[f32; 3]], flags: u32) -> Result<()> {
        if positions.is_empty() || positions.len() % 3 != 0 { return Err("Invalid collision triangle list".into()); }
        let accessor = self.floats(positions, "VEC3");
        self.data["accessors"][accessor]["min"] = json!(std::array::from_fn::<_, 3, _>(|axis| positions.iter().map(|v| v[axis]).fold(f32::INFINITY, f32::min)));
        self.data["accessors"][accessor]["max"] = json!(std::array::from_fn::<_, 3, _>(|axis| positions.iter().map(|v| v[axis]).fold(f32::NEG_INFINITY, f32::max)));
        let materials = self.data["materials"].as_array_mut().unwrap();
        let material = materials.len();
        materials.push(json!({"name":name,"extras":{"surface":{"flags":flags}}}));
        let meshes = self.data["meshes"].as_array_mut().unwrap();
        let mesh = meshes.len();
        meshes.push(json!({"name":name,"primitives":[{"attributes":{"POSITION":accessor},"material":material}]}));
        self.node(json!({"name":name,"mesh":mesh}));
        Ok(())
    }

    pub fn write(mut self, uri: &str, pack: &mut Pack) -> Result<()> {
        let buffer_uri = uri.strip_suffix(".gltf").ok_or("glTF URI must end in .gltf")?.to_owned() + ".bin";
        let filename = buffer_uri.rsplit('/').next().ok_or("Missing glTF buffer filename")?;
        self.data["buffers"] = json!([{"uri":filename,"byteLength":self.bytes.len()}]);
        pack.entries.insert(uri.into(), Arc::new(serde_json::to_vec(&self.data)?));
        pack.entries.insert(buffer_uri, Arc::new(self.bytes));
        Ok(())
    }
}
