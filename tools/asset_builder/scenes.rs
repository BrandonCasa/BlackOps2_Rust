use crate::{
    catalog::Catalog,
    core::*,
    entities::{self, Entity},
    fastfile::{Address, Streams},
    geometry,
    gltf::Scene,
    pack::Pack,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn scalar(streams: &Streams, address: Address) -> Result<f32> {
    let value = f32::from_bits(streams.u32(address)?);
    if !value.is_finite() {
        return Err("Non-finite native placement".into());
    }
    Ok(value)
}

fn vector(streams: &Streams, address: Address) -> Result<[f32; 3]> {
    Ok([
        scalar(streams, address)?,
        scalar(streams, address.add(4)?)?,
        scalar(streams, address.add(8)?)?,
    ])
}

fn matrix(origin: [f32; 3], axis: [[f32; 3]; 3], scale: f32) -> [f32; 16] {
    let [a, b, c] = axis;
    [
        a[0] * scale,
        a[2] * scale,
        -a[1] * scale,
        0.,
        c[0] * scale,
        c[2] * scale,
        -c[1] * scale,
        0.,
        -b[0] * scale,
        -b[2] * scale,
        b[1] * scale,
        0.,
        origin[0] * 0.0254,
        origin[2] * 0.0254,
        -origin[1] * 0.0254,
        1.,
    ]
}

pub fn entity_matrix(entity: &Entity) -> Result<[f32; 16]> {
    let origin = entities::vector(entity, "origin")?.unwrap_or([0.; 3]);
    let mut angles = entities::vector(entity, "angles")?.unwrap_or([0.; 3]);
    if !entity.contains_key("angles")
        && let Some(angle) = entity.get("angle")
    {
        angles[1] = angle.parse()?;
        if !angles[1].is_finite() {
            return Err("Non-finite native brush yaw".into());
        }
    }
    let [pitch, yaw, roll] = angles.map(f32::to_radians);
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();
    let (sr, cr) = roll.sin_cos();
    let axis = [
        [cp * cy, cp * sy, -sp],
        [sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, sr * cp],
        [cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp],
    ];
    Ok(matrix(origin, axis, 1.))
}

fn brush_ranges(
    catalog: &Catalog<'_>,
    zone: usize,
    root: Address,
) -> Result<BTreeMap<usize, usize>> {
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u32(root.add(536)?)? as usize;
    let models = Address::decode(streams.u32(root.add(540)?)?)?;
    let surface_count = streams.u32(root.add(16)?)? as usize;
    let mut brushes = BTreeMap::new();
    for index in 1..count {
        let brush = models.add(index * 64)?;
        let count = streams.u32(brush.add(56)?)? as usize;
        let first = streams.u32(brush.add(60)?)? as usize;
        if count == 0 {
            continue;
        }
        if first + count > surface_count {
            return Err(format!("Native brush model {index} of {} exceeds {surface_count} world surfaces: first {first}, count {count}", streams.u32(root.add(536)?)?).into());
        }
        for surface in first..first + count {
            if brushes.insert(surface, index).is_some() {
                return Err("Native brush surface ranges overlap".into());
            }
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
                if brushes.contains(&index) {
                    entity_matrix(entity)?;
                    placed.insert(index);
                }
            }
        }
        let missing: Vec<_> = brushes.difference(&placed).collect();
        if !missing.is_empty() {
            return Err(format!(
                "Native brush models have no recovered entity placement: {missing:?}"
            )
            .into());
        }
        println!(
            "Resolved {} rendered brush models from native entity placements",
            placed.len()
        );
    }
    Ok(())
}

pub fn brush_entities(entities: &[Entity]) -> Result<Vec<Value>> {
    entities.iter().enumerate().filter_map(|(entity_index, entity)| {
        entity.get("model").and_then(|model| model.strip_prefix('*')).map(|index| (entity_index,entity,index))
    }).map(|(entity_index,entity,index)| {
        let index = index.parse::<usize>()?;
        if index == 0 { return Err("Native brush entity references the world model".into()); }
        Ok(json!({"id":format!("brush.native.{entity_index}"),"native_entity_index":entity_index,
            "native_brush_model":index,"matrix":entity_matrix(entity)?,"source":entity,"visual_meshes":0}))
    }).collect()
}

pub fn required_models(catalog: &Catalog<'_>, game: &Value) -> Result<BTreeSet<String>> {
    let mut models = BTreeSet::new();
    for (_, zone, root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let instances = Address::decode(streams.u32(root.add(876)?)?)?;
        for index in 0..streams.u32(root.add(784)?)? as usize {
            models.insert(
                catalog
                    .pointer(zone, instances.add(index * 152 + 56)?, 5)?
                    .0
                    .to_ascii_lowercase(),
            );
        }
    }
    for presentation in game["content"]["presentations"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(name) = presentation["source_model"].as_str() {
            catalog.resolve(5, name)?;
            models.insert(name.to_ascii_lowercase());
        } else if !presentation["scene"].is_null() {
            return Err(format!(
                "Initial presentation {} has no native model dependency",
                presentation["id"]
            )
            .into());
        }
    }
    Ok(models)
}

pub fn bind_presentations(game: &mut Value, models: &[Value]) -> Result<()> {
    for presentation in game["content"]["presentations"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        let Some(name) = presentation["source_model"].as_str() else {
            continue;
        };
        let model = models
            .iter()
            .find(|model| {
                model["name"]
                    .as_str()
                    .is_some_and(|model| model.eq_ignore_ascii_case(name))
            })
            .ok_or_else(|| {
                format!(
                    "Initial presentation {} model {name} was not exported",
                    presentation["id"]
                )
            })?;
        let uri = model["uri"]
            .as_str()
            .ok_or("Prepared model URI is missing")?;
        presentation["scene"] = json!(format!("{uri}#Scene0"));
        presentation
            .as_object_mut()
            .ok_or("Initial presentation is not an object")?
            .remove("source_model");
    }
    let owners: BTreeMap<_, _> = game["content"]["presentations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|presentation| {
            Some((
                presentation["id"].as_str()?.to_owned(),
                presentation["scene"]
                    .as_str()?
                    .split('#')
                    .next()?
                    .to_owned(),
            ))
        })
        .collect();
    for animation in game["content"]["animations"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        let id = animation["id"]
            .as_str()
            .ok_or("Initial animation identity is missing")?;
        let clip = animation["source_clip"]
            .as_str()
            .or_else(|| id.rsplit('.').next())
            .ok_or("Initial native clip name is missing")?;
        let skeleton = animation["skeleton"]
            .as_str()
            .ok_or("Initial animation skeleton is missing")?;
        let uri = owners
            .get(skeleton)
            .ok_or_else(|| format!("Initial animation {id} has no selected skeleton {skeleton}"))?;
        let model = models
            .iter()
            .find(|model| model["uri"] == *uri)
            .ok_or("Initial animation model was not exported")?;
        let index = model["animations"]
            .as_object()
            .and_then(|clips| {
                clips
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(clip))
            })
            .and_then(|(_, index)| index.as_u64())
            .ok_or_else(|| format!("Initial animation {id} clip {clip} was not exported"))?;
        animation["path"] = json!(format!("{uri}#Animation{index}"));
        animation
            .as_object_mut()
            .ok_or("Initial animation is not an object")?
            .remove("source_clip");
    }
    Ok(())
}

pub(crate) fn prepared_surfaces(
    source: &[geometry::Surface],
    baked: &BTreeMap<String, Vec<geometry::Surface>>,
) -> Result<Vec<geometry::Surface>> {
    let surfaces: Vec<_> = source
        .iter()
        .flat_map(|surface| {
            baked
                .get(&surface.name)
                .cloned()
                .unwrap_or_else(|| vec![surface.clone()])
        })
        .collect();
    if source
        .iter()
        .map(|surface| surface.indices.len())
        .sum::<usize>()
        != surfaces
            .iter()
            .map(|surface| surface.indices.len())
            .sum::<usize>()
    {
        return Err("Prepared model changed its source triangle count".into());
    }
    Ok(surfaces)
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    #[test]
    fn initial_presentations_bind_only_exported_source_models_and_clips() {
        let mut game = json!({"content":{
            "presentations":[{"id":"presentation.initial","source_model":"Exact_Model","scene":null}],
            "animations":[{"id":"animation.initial.exact_fire","source_clip":"exact_fire","skeleton":"presentation.initial","path":null},
                {"id":"animation.initial.viewmodel_M4m203_knife_melee_1","source_clip":"viewmodel_M4m203_knife_melee_1","skeleton":"presentation.initial","path":null}]
        }});
        let models = vec![
            json!({"name":"exact_model","uri":"prepared/models/0/0.gltf","animations":{"exact_fire":3,"viewmodel_m4m203_knife_melee_1":8,"viewmodel_m4m203_knife_melee_10":9}}),
        ];
        bind_presentations(&mut game, &[]).unwrap_err();
        bind_presentations(&mut game, &models).unwrap();
        assert_eq!(
            game["content"]["presentations"][0]["scene"],
            "prepared/models/0/0.gltf#Scene0"
        );
        assert!(
            game["content"]["presentations"][0]
                .get("source_model")
                .is_none()
        );
        assert_eq!(
            game["content"]["animations"][0]["path"],
            "prepared/models/0/0.gltf#Animation3"
        );
        assert_eq!(
            game["content"]["animations"][1]["path"],
            "prepared/models/0/0.gltf#Animation8"
        );
        assert_eq!(
            game["content"]["animations"][1]["id"],
            "animation.initial.viewmodel_M4m203_knife_melee_1"
        );
        assert!(
            game["content"]["animations"][0]
                .get("source_clip")
                .is_none()
        );
        let mut missing = json!({"content":{
            "presentations":[{"id":"presentation.initial","source_model":"exact_model","scene":null}],
            "animations":[{"id":"animation.initial.missing_clip","skeleton":"presentation.initial"}]
        }});
        assert!(
            bind_presentations(&mut missing, &models)
                .unwrap_err()
                .to_string()
                .contains("missing_clip")
        );
        let mut similar_only = models.clone();
        similar_only[0]["animations"]
            .as_object_mut()
            .unwrap()
            .remove("viewmodel_m4m203_knife_melee_1");
        let mut missing_melee = json!({"content":{
            "presentations":[{"id":"presentation.initial","source_model":"exact_model","scene":null}],
            "animations":[{"id":"animation.initial.viewmodel_M4m203_knife_melee_1","source_clip":"viewmodel_M4m203_knife_melee_1","skeleton":"presentation.initial","path":null}]
        }});
        assert!(
            bind_presentations(&mut missing_melee, &similar_only)
                .unwrap_err()
                .to_string()
                .contains("viewmodel_M4m203_knife_melee_1")
        );
    }
}

pub fn convert(
    catalog: &Catalog<'_>,
    entities: &[Entity],
    required_models: Option<&BTreeSet<String>>,
    animation_bindings: &BTreeMap<String, Vec<&crate::animation::DecodedClip>>,
    materials: &BTreeMap<String, Value>,
    baked: &BTreeMap<String, Vec<geometry::Surface>>,
    pack: &mut Pack,
) -> Result<(Vec<Value>, Vec<Value>)> {
    let mut models = Vec::new();
    for (name, zone, root) in catalog.definitions(5) {
        if required_models.is_some_and(|models| !models.contains(&name.to_ascii_lowercase())) {
            continue;
        }
        let lods = geometry::model(catalog, name, zone, root)?;
        let joints = geometry::skeleton(catalog, name, zone, root)?;
        let mut rows = Vec::new();
        let mut animations = BTreeMap::new();
        for (lod, source) in lods.iter().enumerate() {
            let surfaces = prepared_surfaces(source, baked)?;
            if surfaces.is_empty() {
                return Err(format!("Model {name} has an empty native LOD {lod}").into());
            }
            let mut scene = Scene::new(name);
            let indices = scene.materials(&surfaces, materials)?;
            let mesh = scene.mesh(name, &surfaces, &indices, true)?;
            let owner = scene.node(json!({"name":name,"mesh":mesh,"extras":{"source_model":name}}));
            scene.skin(owner, &joints)?;
            for decoded in animation_bindings
                .get(&name.to_ascii_lowercase())
                .into_iter()
                .flatten()
            {
                let index = scene.animation(owner, &joints, &decoded.clip)?;
                if lod == 0 {
                    animations.insert(decoded.clip.name.clone(), index);
                }
            }
            let uri = format!("prepared/models/{}/{lod}.gltf", models.len());
            scene.write(&uri, pack)?;
            let distance =
                scalar(&catalog.zones[zone].1.streams, root.add(40 + lod * 28)?)? * 0.0254;
            rows.push(json!({"level":lod,"uri":uri,"distance":distance}));
        }
        let zone_name = &catalog.zones[zone].0;
        models.push(json!({"name":name,"zone":format!("all-{zone_name}"),"uri":rows[0]["uri"],"lods":rows,"animations":animations}));
    }
    let mut worlds = Vec::new();
    for (name, zone, root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let surfaces: Vec<_> = geometry::world(catalog, zone, root)?
            .into_iter()
            .flat_map(|surface| {
                baked
                    .get(&surface.name)
                    .cloned()
                    .unwrap_or_else(|| vec![surface])
            })
            .collect();
        let brushes = brush_ranges(catalog, zone, root)?;
        let mut groups = BTreeMap::<usize, Vec<&geometry::Surface>>::new();
        for surface in &surfaces {
            groups
                .entry(brushes.get(&surface.source_index).copied().unwrap_or(0))
                .or_default()
                .push(surface);
        }
        let mut scene = Scene::new(name);
        let mut brush_meshes = BTreeMap::new();
        for (&brush, group) in &groups {
            let indices = scene.materials(group.iter().copied(), materials)?;
            let mesh = scene.mesh(
                &format!("{name}/*{brush}"),
                group.iter().copied(),
                &indices,
                false,
            )?;
            brush_meshes.insert(brush, mesh);
        }
        if let Some(&mesh) = brush_meshes.get(&0) {
            scene.node(json!({"name":name,"mesh":mesh}));
        }
        let mut placed = BTreeSet::new();
        let mut brush_entities = brush_entities(entities)?;
        for metadata in &mut brush_entities {
            let entity_index = metadata["native_entity_index"]
                .as_u64()
                .ok_or("Native entity index is missing")?;
            let entity = &metadata["source"];
            let index = metadata["native_brush_model"]
                .as_u64()
                .ok_or("Native brush model index is missing")? as usize;
            let id = metadata["id"]
                .as_str()
                .ok_or("Native brush entity ID is missing")?;
            let transform = &metadata["matrix"];
            let mut node = json!({"name":entity.get("targetname").or_else(|| entity.get("model")),
                "matrix":transform,"extras":{"brush_entity":id,"native_brush_model":index,
                    "native_entity_index":entity_index,"native_entity":entity}});
            let mut visual_meshes = 0;
            if let Some(&mesh) = brush_meshes.get(&index) {
                node["mesh"] = json!(mesh);
                visual_meshes = scene.data["meshes"][mesh]["primitives"]
                    .as_array()
                    .ok_or("Native brush mesh has no primitives")?
                    .len();
                placed.insert(index);
            }
            scene.node(node);
            metadata["visual_meshes"] = json!(visual_meshes);
        }
        if brush_meshes
            .keys()
            .any(|index| *index != 0 && !placed.contains(index))
        {
            return Err("Unplaced rendered native brush model".into());
        }
        let count = streams.u32(root.add(784)?)? as usize;
        let instances = Address::decode(streams.u32(root.add(876)?)?)?;
        let mut cache = BTreeMap::<String, (Vec<usize>, Vec<f32>)>::new();
        for index in 0..count {
            let instance = instances.add(index * 152)?;
            let (model_name, owner, model_root) = catalog.pointer(zone, instance.add(56)?, 5)?;
            let key = model_name.to_ascii_lowercase();
            if !cache.contains_key(&key) {
                let lods = geometry::model(catalog, &model_name, owner, model_root)?;
                let mut meshes = Vec::new();
                let mut distances = Vec::new();
                for (level, source) in lods.iter().enumerate() {
                    let surfaces = prepared_surfaces(source, baked)?;
                    let indices = scene.materials(&surfaces, materials)?;
                    meshes.push(scene.mesh(
                        &format!("{model_name}/lod{level}"),
                        &surfaces,
                        &indices,
                        false,
                    )?);
                    distances.push(
                        scalar(
                            &catalog.zones[owner].1.streams,
                            model_root.add(40 + level * 28)?,
                        )? * 0.0254,
                    );
                }
                cache.insert(key.clone(), (meshes, distances));
            }
            let (meshes, distances) = &cache[&key];
            let origin = vector(streams, instance.add(4)?)?;
            let axis = [
                vector(streams, instance.add(16)?)?,
                vector(streams, instance.add(28)?)?,
                vector(streams, instance.add(40)?)?,
            ];
            let scale = scalar(streams, instance.add(52)?)?;
            if scale == 0. {
                return Err("Native static model has zero placement scale".into());
            }
            let parent = scene.node(json!({"name":format!("{model_name}_{index}"),"matrix":matrix(origin, axis, scale),
                "extras":{"static_instance":true,"source_model":model_name,"native_static_model":index,"native_flags":streams.u32(instance.add(60)?)?,
                    "native_lod":{"distances":distances,"scale":scale,"cull_distance":scalar(streams,instance)?*0.0254}}}));
            for (level, mesh) in meshes.iter().enumerate() {
                scene.child(parent,json!({"name":format!("{model_name}_{index}_lod{level}"),"mesh":mesh,"extras":{"native_lod_level":level}}));
            }
        }
        let uri = "prepared/worlds/zm_tomb.gltf";
        let mesh_count = scene.data["meshes"].as_array().unwrap().len();
        scene.write(uri, pack)?;
        worlds.push(json!({"map":"zm_tomb","name":name,"zone":format!("all-{}",catalog.zones[zone].0),"uri":uri,
            "source_surfaces":surfaces.len(),"render_meshes":mesh_count,"static_model_instances":count,"brush_entities":brush_entities}));
    }
    println!(
        "Packed {} native models and {} world scenes",
        models.len(),
        worlds.len()
    );
    Ok((models, worlds))
}
