use super::*;
use std::{path::Path, sync::Arc};

#[test]
fn velocity_samples_integrate_acceleration_instead_of_lerping_distance() {
    let catalog: Catalog = serde_json::from_value(serde_json::json!({
        "effects": {}, "materials": {}, "random_table": vec![0.5; 32],
        "map_effects": [], "impact_rows": [], "impact_directions": []
    }))
    .unwrap();
    let frame = |distance, speed, variance| VelocityFrame {
        total_delta: VectorRange {
            base: [distance, 0., 0.],
            amplitude: [0.; 3],
        },
        velocity: VectorRange {
            base: [speed, 0., 0.],
            amplitude: [variance, 0., 0.],
        },
    };
    for (start_speed, end_speed, variance) in [(0., 2., 0.), (2., 0., 0.), (1., 1., 2.)] {
        let start = frame(10., start_speed, variance);
        let end = frame(
            10. + (start_speed + end_speed + variance) * 0.5,
            end_speed,
            variance,
        );
        for t in [0., 0.25, 0.5, 0.75, 1.] {
            let (delta, velocity) = start.motion(&end, t, 0, &catalog);
            let speed = start_speed + variance * 0.5;
            let acceleration = end_speed - start_speed;
            assert!(delta.abs_diff_eq(
                native([10. + speed * t + 0.5 * acceleration * t * t, 0., 0.]),
                1e-6
            ));
            assert!(velocity.abs_diff_eq(native([speed + acceleration * t, 0., 0.]), 1e-6));
        }
    }
}

#[test]
#[ignore = "requires the authorized local Origins export"]
fn exported_particle_timelines_have_valid_sampling_and_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let catalog: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
    assert!(catalog.random_table.len() >= 508);
    for (name, effect) in &catalog.effects {
        assert!(effect.looping_count <= effect.elements.len(), "{name}");
        for (index, element) in effect.elements.iter().enumerate() {
            assert!(matches!(element.kind, 0..=8 | 10..=12), "{name}:{index}");
            assert!(element.velocity_samples.len() >= 2, "{name}:{index}");
            for child in [
                &element.effect_attached,
                &element.effect_emitted,
                &element.effect_on_death,
                &element.effect_on_impact,
            ]
            .into_iter()
            .flatten()
            .chain(
                element
                    .visuals
                    .iter()
                    .flatten()
                    .filter_map(|v| v.effect.as_ref()),
            ) {
                assert!(
                    catalog.effects.contains_key(child.trim_start_matches(',')),
                    "{name}:{index}: {child}"
                );
            }
            for model in element
                .visuals
                .iter()
                .flatten()
                .filter_map(|v| v.model.as_ref())
            {
                assert!(
                    root.join(format!(
                        "model_export/{}_lod0.glb",
                        model.trim_start_matches(',')
                    ))
                    .is_file(),
                    "{name}:{index}: {model}"
                );
            }
            for seed in 0..479 {
                let life = integer_range(element.life_ms, &catalog, seed, 11) * 0.001;
                assert!(life.is_finite() && life >= 0., "{name}:{index}");
                if element.effect_emitted.is_some() {
                    let distance = element.emit_distance.sample(channel(&catalog, seed, 14))
                        + element
                            .emit_distance_variance
                            .sample(channel(&catalog, seed, 14));
                    assert!(distance.is_finite() && distance > 0., "{name}:{index}");
                }
                if element.kind == 6 {
                    let (first, count) = cloud_range(element, &catalog, seed);
                    assert!(
                        count >= 6 && count <= 1024 * 6 && first + count <= 2048 * 6,
                        "{name}:{index}"
                    );
                }
            }
            if element.kind < 6 && !element.visuals.is_empty() {
                let entries = element.atlas.entry_count_and_range & 0x1ff;
                assert!(entries.is_power_of_two(), "{name}:{index}");
                for start in [0, element.atlas.index, entries - 1] {
                    for age in [0., 0.25, 0.5, 0.9999] {
                        let uv = atlas_transform(&element.atlas, start, age, 1.).unwrap();
                        assert!(
                            uv.matrix2.is_finite() && uv.translation.is_finite(),
                            "{name}:{index}"
                        );
                    }
                }
            }
        }
    }
}

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

    let mut app = source_fx_app(&root, catalog);

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

fn source_fx_app(root: &Path, catalog: Catalog) -> App {
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

    app
}

#[test]
#[ignore = "requires the authorized local Origins export"]
fn shell_impacts_ignore_actor_colliders_and_obey_source_collision_flags() {
    use avian3d::{
        collider_tree::{ColliderTreeProxy, ColliderTrees},
        prelude::*,
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let original: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
    // Rifle blur trails have an impact dependency but no collision flag.
    // Pistol blur dies on contact; distant pistol brass bounces instead.
    for (name, index, world_wall, expected_impact, survives) in [
        ("weapon/shellejects/fx_rifle", 2, true, false, true),
        ("weapon/shellejects/fx_pistol", 2, false, false, true),
        ("weapon/shellejects/fx_pistol", 2, true, true, false),
        ("weapon/shellejects/fx_pistol", 0, true, true, true),
    ] {
        let mut catalog: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
        let mut effect = catalog.effects.remove(name).unwrap();
        let mut element = effect.elements.remove(index);
        let flags = original.effects[name].elements[index].flags;
        assert_eq!(element.flags, flags);
        let zero = Range {
            base: 0.,
            amplitude: 0.,
        };
        element.kind = 0;
        element.visuals.clear();
        element.visual_samples.clear();
        element.spawn_range = zero;
        element.delay_ms = zero;
        element.life_ms = Range {
            base: 1000.,
            amplitude: 0.,
        };
        element.origin = [zero; 3];
        element.angles = [zero; 3];
        element.gravity = zero;
        element.spawn_sound = None;
        element.effect_on_impact = Some("shell_impact_test".into());
        let velocity = VectorRange {
            base: [0.1, 0., 0.],
            amplitude: [0.; 3],
        };
        let vector_zero = VectorRange {
            base: [0.; 3],
            amplitude: [0.; 3],
        };
        let frame = VelocitySample {
            local: VelocityFrame {
                total_delta: vector_zero.clone(),
                velocity,
            },
            world: VelocityFrame {
                total_delta: vector_zero.clone(),
                velocity: vector_zero,
            },
        };
        element.velocity_samples = vec![frame.clone(), frame];
        effect.flags = 0;
        effect.looping_count = 0;
        effect.elements = vec![element];
        let mut impact = catalog
            .effects
            .remove("weapon/shellejects/fx_pistol_resting")
            .unwrap();
        impact.elements.clear();
        catalog.effects.clear();
        catalog.effects.insert("shell_test".into(), effect);
        catalog.effects.insert("shell_impact_test".into(), impact);
        catalog.materials.clear();
        let mut app = source_fx_app(&root, catalog);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(0.05),
        ))
        .add_systems(Update, animate.after(spawn));
        let mut trees = ColliderTrees::default();
        // A player/trigger overlaps the ejection point. It must never create
        // resting shells, even before the particle reaches the actual wall.
        for (position, contents) in std::iter::once((Vec3::ZERO, None))
            .chain(world_wall.then_some((Vec3::X * 0.05, Some(1))))
        {
            let collider = Collider::cuboid(0.02, 1., 1.);
            let bounds = collider.aabb(position, Quat::IDENTITY, 0.);
            let mut entity =
                app.world_mut()
                    .spawn((Position(position), Rotation::IDENTITY, collider, bounds));
            if let Some(contents) = contents {
                entity.insert(crate::assets::WorldContents(contents));
            }
            let entity = entity.id();
            trees.static_tree.add_proxy(
                bounds.into(),
                ColliderTreeProxy {
                    collider: entity,
                    body: None,
                    layers: CollisionLayers::default(),
                    flags: default(),
                },
            );
        }
        app.insert_resource(trees);
        emit(
            &mut app.world_mut().commands(),
            "shell_test",
            Transform::IDENTITY,
        );
        app.world_mut().flush();
        app.update();
        app.update();
        let has_impact = app
            .world_mut()
            .query::<&Emitter>()
            .iter(app.world())
            .any(|emitter| emitter.name == "shell_impact_test");
        assert_eq!(
            has_impact, expected_impact,
            "{name}:{index}, wall={world_wall}"
        );
        let particles = app
            .world_mut()
            .query::<&Particle>()
            .iter(app.world())
            .count();
        assert_eq!(
            particles,
            usize::from(survives),
            "{name}:{index}, wall={world_wall}"
        );
    }
}

#[test]
#[ignore = "requires the authorized local Origins export"]
fn accelerated_particles_preserve_source_lifetime_scale_and_local_world_axes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let mut catalog: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
    let mut effect = catalog
        .effects
        .remove("impacts/fx_flesh_hit_knife_mp")
        .unwrap();
    let mut element = effect.elements.remove(4);
    element.effect_emitted = None;
    element.flags = 0x3000000;
    element.life_ms = Range {
        base: 1000.,
        amplitude: 0.,
    };
    element.gravity = Range {
        base: 0.,
        amplitude: 0.,
    };
    element.origin = [Range {
        base: 0.,
        amplitude: 0.,
    }; 3];
    element.angles = element.origin;
    let vector = |base| VectorRange {
        base,
        amplitude: [0.; 3],
    };
    element.velocity_samples = (0..3)
        .map(|i| {
            let i = i as f32;
            VelocitySample {
                local: VelocityFrame {
                    total_delta: vector([0.0005 * i * i, 0., 0.]),
                    velocity: vector([0.001 * i, 0., 0.]),
                },
                world: VelocityFrame {
                    total_delta: vector([0., 0.001 * i, 0.]),
                    velocity: vector([0., 0.001, 0.]),
                },
            }
        })
        .collect();
    effect.flags = 0;
    effect.elements = vec![element];
    effect.looping_count = 0;
    catalog.effects.clear();
    catalog.effects.insert("motion_test".into(), effect);
    catalog.materials.clear();
    let mut app = source_fx_app(&root, catalog);
    app.init_resource::<avian3d::collider_tree::ColliderTrees>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(0.25),
        ))
        .add_systems(Update, animate.after(spawn));
    let initial = Transform::from_xyz(2., 3., 4.).with_rotation(Quat::from_rotation_y(0.7));
    emit(&mut app.world_mut().commands(), "motion_test", initial);
    app.world_mut().flush();
    app.update();
    for age in [0.25, 0.5, 0.75] {
        app.update();
        let pose = app
            .world_mut()
            .query_filtered::<&Transform, With<Particle>>()
            .single(app.world())
            .unwrap();
        // Source normalized velocity rises from 0 to 4 inches/second locally;
        // world motion stays at 2 inches/second across both sample intervals.
        let expected = initial.translation
            + initial.rotation * native([2. * age * age, 0., 0.])
            + native([0., 2. * age, 0.]);
        assert!(
            pose.translation.abs_diff_eq(expected, 1e-5),
            "age={age}: {:?} != {expected:?}",
            pose.translation
        );
    }
}

#[test]
#[ignore = "requires the authorized local Origins export"]
fn visual_less_source_particles_move_emit_children_and_expire() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
    let mut catalog: Catalog = crate::origins::read(&root.join("fx_catalog.json"));
    let name = "impacts/fx_flesh_hit_knife_mp";
    let mut effect = catalog.effects.remove(name).unwrap();
    let element = effect.elements.remove(4);
    assert_eq!(element.kind, 0);
    assert!(element.visuals.is_empty());
    let child_name = element.effect_emitted.clone().unwrap();
    effect.elements = vec![element];
    effect.looping_count = 0;
    // Exercise distance emission within this short-lived source particle's
    // gravity-driven motion without needing rendered blood/decal dependencies.
    effect.elements[0].emit_distance = Range {
        base: 0.01,
        amplitude: 0.,
    };
    effect.elements[0].emit_distance_variance = Range {
        base: 0.,
        amplitude: 0.,
    };
    let mut child = catalog.effects.remove(&child_name).unwrap();
    child.elements.clear();
    catalog.effects.clear();
    catalog.effects.insert(name.into(), effect);
    catalog.effects.insert(child_name.clone(), child);
    catalog.materials.clear();
    let mut app = source_fx_app(&root, catalog);
    app.init_resource::<avian3d::collider_tree::ColliderTrees>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1. / 60.),
        ))
        .add_systems(Update, animate.after(spawn));
    emit(&mut app.world_mut().commands(), name, Transform::IDENTITY);
    app.world_mut().flush();
    app.update();
    let particle = app
        .world_mut()
        .query_filtered::<Entity, With<Particle>>()
        .single(app.world())
        .unwrap();
    assert!(app.world().get::<NativeEffectDraw>(particle).is_none());
    assert!(
        app.world()
            .resource::<Assets<NativeEffectGeometry>>()
            .is_empty()
    );
    let mut emitted = false;
    let mut moved = false;
    for _ in 0..40 {
        app.update();
        if let Some(pose) = app.world().get::<Transform>(particle) {
            moved |= pose.translation.y < 0.;
        }
        emitted |= app
            .world_mut()
            .query::<&Emitter>()
            .iter(app.world())
            .any(|emitter| emitter.name == child_name);
    }
    assert!(moved, "Invisible source particles must retain their motion");
    assert!(
        emitted,
        "Invisible source particles must emit their child effects"
    );
    assert!(
        app.world().get_entity(particle).is_err(),
        "Source lifetime must expire"
    );
}
