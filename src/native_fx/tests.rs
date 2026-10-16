use super::*;
use std::{path::Path, sync::Arc};

#[test]
#[ignore = "requires the authorized local Origins export"]
fn exported_fx_use_current_poses_before_transform_propagation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let mut catalog: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
    let mut effect = catalog
        .effects
        .remove("explosions/fx_grenadeexp_default")
        .unwrap();
    effect.flags = 0;
    effect.looping_count = 0;
    effect.non_looping_life_ms = 1000.;
    effect.elements.truncate(1);
    let zero = Range {
        base: 0.,
        amplitude: 0.,
    };
    let element = &mut effect.elements[0];
    element.kind = 10;
    element.flags = 0;
    element.origin = [zero; 3];
    element.angles = [zero; 3];
    element.delay_ms = zero;
    element.spawn = Spawn::Once {
        count: Range {
            base: 1.,
            amplitude: 0.,
        },
    };
    element.spawn_sound = None;
    element.visuals = vec![Some(
        serde_json::from_value(serde_json::json!({
            "sound": "wpn_grenade_bounce_concrete"
        }))
        .unwrap(),
    )];
    catalog.effects.clear();
    catalog.effects.insert("pose_test".into(), effect);
    catalog.materials.clear();

    let world: serde_json::Value = crate::origins::read(&root.join("world.json"));
    let grid: crate::rendering::light_grid::GridDefinition =
        serde_json::from_value(world["lightGrid"].clone()).unwrap();
    let coefficients = crate::rendering::lighting::LightingCoefficients::from_bytes(
        &std::fs::read(root.join(&grid.coefficients_file)).unwrap(),
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin {
            file_path: root.to_string_lossy().into_owned(),
            ..default()
        },
    ))
    .insert_resource(GameDefinition::load_from_root(&root))
    .insert_resource(crate::rendering::model_lighting::ModelLightingSource {
        coefficients: Arc::new(coefficients),
        grid: Arc::new(grid.load(&root)),
    })
    .insert_resource(lighting::Source { primary: vec![] })
    .init_resource::<crate::environment::weather::Weather>()
    .insert_resource(cloud::Geometry(Handle::default()))
    .init_resource::<Assets<NativeEffectMaterial>>()
    .init_resource::<Assets<NativeEffectGeometry>>()
    .init_resource::<Assets<crate::rendering::native::NativeWorldMaterial>>()
    .init_resource::<Assets<crate::rendering::native::NativeWorldGeometry>>()
    .add_systems(Update, spawn);
    let mut materials = Assets::default();
    let bindings = material::Materials::load(
        &root,
        catalog.materials.keys(),
        app.world().resource::<AssetServer>(),
        &mut materials,
    );
    app.insert_resource(bindings)
        .insert_resource(decal::Materials::load(&root, &catalog))
        .insert_resource(catalog);

    let origin = Vec3::new(11., 3., -7.);
    let owner_pose = Transform::from_xyz(25., 4., 6.).with_rotation(Quat::from_rotation_y(0.5));
    let offset = Transform::from_xyz(2., 1., 0.);
    let owner = app.world_mut().spawn(owner_pose).id();
    {
        let mut commands = app.world_mut().commands();
        emit(
            &mut commands,
            "pose_test",
            Transform::from_translation(origin),
        );
        attach(&mut commands, "pose_test", owner, offset);
    }
    app.world_mut().flush();
    // Neither the newly created emitter nor its parent has been propagated.
    assert_eq!(
        app.world()
            .get::<GlobalTransform>(owner)
            .unwrap()
            .translation(),
        Vec3::ZERO
    );
    app.update();
    let positions: Vec<_> = crate::audio::requested_sounds(app.world_mut())
        .into_iter()
        .filter_map(|(_, source)| match source {
            crate::audio::SoundSource::World(point) => Some(point),
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 2);
    assert!(
        positions
            .iter()
            .any(|point| point.abs_diff_eq(origin, 0.0001))
    );
    let attached = owner_pose.transform_point(offset.translation);
    assert!(
        positions
            .iter()
            .any(|point| point.abs_diff_eq(attached, 0.0001))
    );
}
