use super::*;
use std::{path::Path, sync::Arc};

#[test]
fn source_spawn_range_includes_both_boundaries_and_zero_amplitude_is_unlimited() {
    let range = Range {
        base: 100.,
        amplitude: 200.,
    };
    assert!(!in_spawn_range(range, 99. * 99.));
    assert!(in_spawn_range(range, 100. * 100.));
    assert!(in_spawn_range(range, 300. * 300.));
    assert!(!in_spawn_range(range, 301. * 301.));
    assert!(in_spawn_range(
        Range {
            base: 100.,
            amplitude: 0.
        },
        0.
    ));
    assert!(in_spawn_range(
        Range {
            base: 100.,
            amplitude: 0.
        },
        1e20
    ));
}

#[test]
#[ignore = "requires the authorized local FX export"]
fn warm_ambient_loop_discards_old_births_and_preserves_emission_sequence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let catalog: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
    let effect = &catalog.effects["maps/zombie_tomb/fx_tomb_light_md"];
    let element = &effect.elements[3];
    assert!(matches!(element.spawn, Spawn::Loop { interval_ms: 1, .. }));
    let mut world = World::new();
    let id = emit(
        &mut world.commands(),
        "maps/zombie_tomb/fx_tomb_light_md",
        Transform::IDENTITY,
    );
    world.flush();
    let mut emitter = world.get_mut::<Emitter>(id).unwrap();
    emitter.elapsed = 60.;
    emitter.next = vec![0.; effect.elements.len()];
    emitter.counts = vec![0; effect.elements.len()];
    emitter.limits = vec![i32::MAX as u32; effect.elements.len()];
    let births = schedule_element(
        &mut emitter,
        effect,
        element,
        3,
        Transform::IDENTITY,
        0.016,
        &catalog,
    );
    assert!(
        births.len() <= 12,
        "Expired one-millisecond births must not be replayed: {}",
        births.len()
    );
    assert!(!births.is_empty());
    assert!(births.iter().all(|birth| *birth >= 59.988));
    assert!(emitter.counts[3] >= 60000);
    assert!(emitter.next[3] > 60.);
}

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
