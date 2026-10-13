use crate::{convert::Catalog, core::*, entities::{self, Entity}, fastfile::{Address, Streams}, geometry, gltf::Scene, pack::Pack};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn scalar(streams: &Streams, address: Address) -> Result<f32> {
    let value = f32::from_bits(streams.u32(address)?);
    if !value.is_finite() { return Err("Non-finite native placement".into()); }
    Ok(value)
}

fn vector(streams: &Streams, address: Address) -> Result<[f32; 3]> {
    Ok([scalar(streams, address)?, scalar(streams, address.add(4)?)?, scalar(streams, address.add(8)?)?])
}

fn matrix(origin: [f32; 3], axis: [[f32; 3]; 3], scale: f32) -> [f32; 16] {
    let [a, b, c] = axis;
    [a[0]*scale, a[2]*scale, -a[1]*scale, 0.,
        c[0]*scale, c[2]*scale, -c[1]*scale, 0.,
        -b[0]*scale, -b[2]*scale, b[1]*scale, 0.,
        origin[0]*0.0254, origin[2]*0.0254, -origin[1]*0.0254, 1.]
}

pub fn entity_matrix(entity: &Entity) -> Result<[f32; 16]> {
    let origin = entities::vector(entity, "origin")?.unwrap_or([0.; 3]);
    let mut angles = entities::vector(entity, "angles")?.unwrap_or([0.; 3]);
    if !entity.contains_key("angles") && let Some(angle) = entity.get("angle") {
        angles[1] = angle.parse()?;
        if !angles[1].is_finite() { return Err("Non-finite native brush yaw".into()); }
    }
    let [pitch, yaw, roll] = angles.map(f32::to_radians);
    let (sp, cp) = pitch.sin_cos(); let (sy, cy) = yaw.sin_cos(); let (sr, cr) = roll.sin_cos();
    let axis = [[cp*cy, cp*sy, -sp], [sr*sp*cy-cr*sy, sr*sp*sy+cr*cy, sr*cp], [cr*sp*cy+sr*sy, cr*sp*sy-sr*cy, cr*cp]];
    Ok(matrix(origin, axis, 1.))
}

fn brush_ranges(catalog: &Catalog<'_>, zone: usize, root: Address) -> Result<BTreeMap<usize, usize>> {
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u32(root.add(536)?)? as usize;
    let models = Address::decode(streams.u32(root.add(540)?)?)?;
    let surface_count = streams.u32(root.add(16)?)? as usize;
    let mut brushes = BTreeMap::new();
    for index in 1..count {
        let brush = models.add(index * 64)?;
        let count = streams.u32(brush.add(56)?)? as usize;
        let first = streams.u32(brush.add(60)?)? as usize;
        if count == 0 { continue; }
        if first + count > surface_count { return Err(format!("Native brush model {index} of {} exceeds {surface_count} world surfaces: first {first}, count {count}", streams.u32(root.add(536)?)?).into()); }
        for surface in first..first + count {
            if brushes.insert(surface, index).is_some() { return Err("Native brush surface ranges overlap".into()); }
        }
    }
    Ok(brushes)
}

pub fn validate_placements(catalog: &Catalog<'_>, entities: &[Entity]) -> Result<()> {
    for (_, zone, root) in catalog.definitions(17) {
        let brushes: BTreeSet<_> = brush_ranges(catalog, zone, root)?.into_values().collect();
        let mut placed = BTreeSet::new();
        for entity in entities {
            if let Some(index) = entity.get("model").and_then(|name| name.strip_prefix('*')) {
                let index = index.parse::<usize>()?;
                if brushes.contains(&index) { entity_matrix(entity)?; placed.insert(index); }
            }
        }
        let missing: Vec<_> = brushes.difference(&placed).collect();
        if !missing.is_empty() { return Err(format!("Native brush models have no recovered entity placement: {missing:?}").into()); }
        println!("Resolved {} rendered brush models from native entity placements", placed.len());
    }
    Ok(())
}

pub fn convert(catalog: &Catalog<'_>, entities: &[Entity], materials: &BTreeMap<String, Value>, baked: &BTreeMap<String, Vec<geometry::Surface>>, pack: &mut Pack) -> Result<(Vec<Value>, Vec<Value>)> {
    let mut models = Vec::new();
    for (name, zone, root) in catalog.definitions(5) {
        let lods = geometry::model(catalog, name, zone, root)?;
        let joints = geometry::skeleton(catalog,name,zone,root)?;
        let mut rows = Vec::new();
        for (lod, surfaces) in lods.iter().enumerate() {
            if surfaces.is_empty() { return Err(format!("Model {name} has an empty native LOD {lod}").into()); }
            let mut scene = Scene::new(name);
            let indices = scene.materials(surfaces, materials)?;
            let mesh = scene.mesh(name, surfaces, &indices, true)?;
            let owner = scene.node(json!({"name":name,"mesh":mesh,"extras":{"source_model":name}}));
            scene.skin(owner,&joints)?;
            let uri = format!("prepared/models/{}/{lod}.gltf", models.len());
            scene.write(&uri, pack)?;
            let distance = scalar(&catalog.zones[zone].1.streams, root.add(40 + lod * 28)?)? * 0.0254;
            rows.push(json!({"level":lod,"uri":uri,"distance":distance}));
        }
        let zone_name = &catalog.zones[zone].0;
        models.push(json!({"name":name,"zone":format!("all-{zone_name}"),"uri":rows[0]["uri"],"lods":rows,"animations":{}}));
    }
    let mut worlds = Vec::new();
    for (name, zone, root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let surfaces: Vec<_> = geometry::world(catalog, zone, root)?.into_iter().flat_map(|surface| {
            baked.get(&surface.name).cloned().unwrap_or_else(|| vec![surface])
        }).collect();
        let brushes = brush_ranges(catalog, zone, root)?;
        let mut groups = BTreeMap::<usize, Vec<&geometry::Surface>>::new();
        for surface in &surfaces { groups.entry(brushes.get(&surface.source_index).copied().unwrap_or(0)).or_default().push(surface); }
        let mut scene = Scene::new(name);
        let mut brush_meshes = BTreeMap::new();
        for (&brush, group) in &groups {
            let indices = scene.materials(group.iter().copied(), materials)?;
            let mesh = scene.mesh(&format!("{name}/*{brush}"), group.iter().copied(), &indices, false)?;
            brush_meshes.insert(brush, mesh);
        }
        if let Some(&mesh) = brush_meshes.get(&0) { scene.node(json!({"name":name,"mesh":mesh})); }
        let mut placed = BTreeSet::new();
        let mut brush_entities = Vec::new();
        for (entity_index, entity) in entities.iter().enumerate() {
            if let Some(index) = entity.get("model").and_then(|name| name.strip_prefix('*')) {
                let index = index.parse::<usize>()?;
                if index == 0 { return Err("Native brush entity references the world model".into()); }
                let id = format!("brush.native.{entity_index}");
                let transform = entity_matrix(entity)?;
                let mut node = json!({"name":entity.get("targetname").or_else(|| entity.get("model")),
                    "matrix":transform,"extras":{"brush_entity":id,"native_brush_model":index,
                        "native_entity_index":entity_index,"native_entity":entity}});
                let mut visual_meshes = 0;
                if let Some(&mesh) = brush_meshes.get(&index) {
                    node["mesh"] = json!(mesh);
                    visual_meshes = scene.data["meshes"][mesh]["primitives"].as_array().ok_or("Native brush mesh has no primitives")?.len();
                    placed.insert(index);
                }
                scene.node(node);
                brush_entities.push(json!({"id":id,"native_entity_index":entity_index,"native_brush_model":index,
                    "matrix":transform,"source":entity,"visual_meshes":visual_meshes}));
            }
        }
        if brush_meshes.keys().any(|index| *index != 0 && !placed.contains(index)) { return Err("Unplaced rendered native brush model".into()); }
        let count = streams.u32(root.add(784)?)? as usize;
        let instances = Address::decode(streams.u32(root.add(876)?)?)?;
        let mut cache = BTreeMap::<String,(Vec<usize>,Vec<f32>)>::new();
        for index in 0..count {
            let instance = instances.add(index * 152)?;
            let (model_name, owner, model_root) = catalog.pointer(zone, instance.add(56)?, 5)?;
            let key = model_name.to_ascii_lowercase();
            if !cache.contains_key(&key) {
                let lods = geometry::model(catalog, &model_name, owner, model_root)?;
                let mut meshes = Vec::new();
                let mut distances = Vec::new();
                for (level, surfaces) in lods.iter().enumerate() {
                    let indices = scene.materials(surfaces, materials)?;
                    meshes.push(scene.mesh(&format!("{model_name}/lod{level}"), surfaces, &indices, false)?);
                    distances.push(scalar(&catalog.zones[owner].1.streams,model_root.add(40+level*28)?)? * 0.0254);
                }
                cache.insert(key.clone(),(meshes,distances));
            }
            let (meshes,distances) = &cache[&key];
            let origin = vector(streams, instance.add(4)?)?;
            let axis = [vector(streams, instance.add(16)?)?, vector(streams, instance.add(28)?)?, vector(streams, instance.add(40)?)?];
            let scale = scalar(streams, instance.add(52)?)?;
            if scale == 0. { return Err("Native static model has zero placement scale".into()); }
            let parent = scene.node(json!({"name":format!("{model_name}_{index}"),"matrix":matrix(origin, axis, scale),
                "extras":{"static_instance":true,"source_model":model_name,"native_static_model":index,"native_flags":streams.u32(instance.add(60)?)?,
                    "native_lod":{"distances":distances,"scale":scale,"cull_distance":scalar(streams,instance)?*0.0254}}}));
            for (level,mesh) in meshes.iter().enumerate() {
                scene.child(parent,json!({"name":format!("{model_name}_{index}_lod{level}"),"mesh":mesh,"extras":{"native_lod_level":level}}));
            }
        }
        let uri = "prepared/worlds/zm_tomb.gltf";
        let mesh_count = scene.data["meshes"].as_array().unwrap().len();
        scene.write(uri, pack)?;
        worlds.push(json!({"map":"zm_tomb","name":name,"zone":format!("all-{}",catalog.zones[zone].0),"uri":uri,
            "source_surfaces":surfaces.len(),"render_meshes":mesh_count,"static_model_instances":count,"brush_entities":brush_entities}));
    }
    println!("Packed {} native models and {} world scenes", models.len(), worlds.len());
    Ok((models, worlds))
}
