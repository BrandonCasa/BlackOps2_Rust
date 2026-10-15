use super::opaque::{AnimatedMaterial, StandardReady, SurfaceLibrary};
use bevy::{gltf::GltfMaterialName, prelude::*, render::render_resource::AsBindGroup};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BakedKey {
    mode: u32,
    falloff: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(BakedKey)]
pub(super) struct BakedExtension {
    key: BakedKey,
}

impl From<&BakedExtension> for BakedKey {
    fn from(extension: &BakedExtension) -> Self {
        extension.key
    }
}

impl bevy::pbr::MaterialExtension for BakedExtension {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "embedded://origins/rendering/baked.wesl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn enable_oit() -> bool {
        false
    }
    fn specialize(
        _: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader_defs.push(bevy::shader::ShaderDefVal::UInt(
            "BAKED_MODE".into(),
            key.bind_group_data.mode,
        ));
        fragment.shader_defs.push(bevy::shader::ShaderDefVal::UInt(
            "BAKED_FALLOFF_BITS".into(),
            key.bind_group_data.falloff,
        ));
        Ok(())
    }
}

type BakedMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, BakedExtension>;

#[derive(Component)]
pub(super) struct Base {
    pub material: Handle<StandardMaterial>,
    identity: String,
    key: BakedKey,
}

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "baked.wesl");
    app.add_plugins(MaterialPlugin::<BakedMaterial>::default())
        .add_systems(
            PostUpdate,
            prepare
                .after(super::MaterialAnimation)
                .run_if(super::opaque::material_scene_active),
        );
}

fn prepare(
    mut commands: Commands,
    new: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            Option<&AnimatedMaterial>,
        ),
        (With<StandardReady>, Without<Base>),
    >,
    existing: Query<(
        Entity,
        &Base,
        Option<&AnimatedMaterial>,
        &MeshMaterial3d<BakedMaterial>,
    )>,
    library: Option<Res<SurfaceLibrary>>,
    sources: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<BakedMaterial>>,
    mut cache: Local<HashMap<(String, usize), Handle<BakedMaterial>>>,
) {
    let Some(library) = library else { return };
    let mut used = HashSet::new();
    let mut prepared =
        |identity: &str, frame: usize, key: &BakedKey, source: &Handle<StandardMaterial>| {
            let identity = (identity.to_owned(), frame.min(usize::MAX - 1));
            used.insert(identity.clone());
            if let Some(material) = cache.get(&identity) {
                return Some(material.clone());
            }
            let mut base = sources.get(source)?.clone();
            base.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
            let material = materials.add(bevy::pbr::ExtendedMaterial {
                base,
                extension: BakedExtension { key: *key },
            });
            cache.insert(identity, material.clone());
            Some(material)
        };
    for (entity, source, name, animation) in &new {
        let Some(definition) = library.definitions.get(&name.0) else {
            continue;
        };
        let effect = &definition.metadata["baked_effect"];
        let Some(mode) = effect["mode"].as_u64() else {
            continue;
        };
        let key = BakedKey {
            mode: mode as u32,
            falloff: (effect["falloff_power"].as_f64().unwrap() as f32).to_bits(),
        };
        let frame = animation.map_or(0, |animation| animation.selected.min(usize::MAX - 1));
        if let Some(material) = prepared(&name.0, frame, &key, &source.0) {
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert((
                    Base {
                        material: source.0.clone(),
                        identity: name.0.clone(),
                        key,
                    },
                    MeshMaterial3d(material),
                ));
        }
    }
    for (entity, base, animation, current) in &existing {
        let frame = animation.map_or(0, |animation| animation.selected.min(usize::MAX - 1));
        if let Some(material) = prepared(&base.identity, frame, &base.key, &base.material) {
            if material != current.0 {
                commands.entity(entity).insert(MeshMaterial3d(material));
            }
        }
    }
    cache.retain(|key, _| used.contains(key));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_objects_share_immutable_render_material_and_keep_their_animation_source() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_asset::<BakedMaterial>()
            .init_resource::<SurfaceLibrary>()
            .add_systems(Update, prepare);
        super::super::opaque::register_prepared(
            &mut app.world_mut().resource_mut::<SurfaceLibrary>(),
            &serde_json::json!({"identity":"prepared", "channels":[],
                "baked_effect":{"mode":1,"falloff_power":0.}}),
            &[],
        )
        .unwrap();
        let mut objects = Vec::new();
        for _ in 0..1024 {
            let source = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    emissive: LinearRgba::rgb(4., 2., 1.),
                    ..default()
                });
            let entity = app
                .world_mut()
                .spawn((
                    StandardReady,
                    GltfMaterialName("prepared".into()),
                    MeshMaterial3d(source.clone()),
                ))
                .id();
            objects.push((entity, source));
        }
        app.update();
        let shared = app
            .world()
            .get::<MeshMaterial3d<BakedMaterial>>(objects[0].0)
            .unwrap()
            .0
            .clone();
        assert_eq!(app.world().resource::<Assets<BakedMaterial>>().len(), 1);
        for (entity, source) in &objects {
            assert!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(*entity)
                    .is_none()
            );
            assert_eq!(app.world().get::<Base>(*entity).unwrap().material, *source);
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<BakedMaterial>>(*entity)
                    .unwrap()
                    .0,
                shared
            );
        }
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .get_mut(&objects[0].1)
            .unwrap()
            .emissive = LinearRgba::BLACK;
        app.update();
        let materials = app.world().resource::<Assets<BakedMaterial>>();
        assert_eq!(materials.len(), 1);
        assert_eq!(materials.get(&shared).unwrap().base.emissive.red, 4.);
    }
}
