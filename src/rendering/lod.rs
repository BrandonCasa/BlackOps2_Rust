use bevy::{
    camera::visibility::{VisibilityRange, VisibilitySystems},
    gltf::GltfExtras,
    prelude::*,
    transform::TransformSystems,
};
use serde::Deserialize;

pub(super) struct PropLodPlugin;

impl Plugin for PropLodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (prepare, select)
                .chain()
                .after(bevy::camera::CameraUpdateSystems)
                .after(TransformSystems::Propagate)
                .before(VisibilitySystems::CheckVisibility),
        );
    }
}

#[derive(Component)]
struct LodReady;

#[derive(Deserialize)]
struct NativeDistances {
    distances: Vec<f32>,
    scale: f32,
    cull_distance: f32,
}

#[derive(Component)]
pub(super) struct NativeLod {
    level: usize,
    source: NativeDistances,
    selected: bool,
}

impl NativeLod {
    pub(super) fn is_selected(&self) -> bool {
        self.selected
    }
}

fn prepare(
    mut commands: Commands,
    meshes: Query<Entity, (With<Mesh3d>, Without<LodReady>)>,
    extras: Query<&GltfExtras>,
    parents: Query<&ChildOf>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for entity in &meshes {
        let result = (|| -> Result<Option<NativeLod>, String> {
            let mut ancestor = Some(entity);
            let mut level = None;
            while let Some(current) = ancestor {
                if let Ok(extras) = extras.get(current) {
                    let value: serde_json::Value = serde_json::from_str(&extras.value)
                        .map_err(|error| format!("Invalid native LOD extras: {error}"))?;
                    if let Some(value) = value.get("native_lod_level") {
                        level = Some(
                            value
                                .as_u64()
                                .and_then(|v| usize::try_from(v).ok())
                                .ok_or("Invalid native LOD level")?,
                        );
                    }
                    if let Some(value) = value.get("native_lod") {
                        let source: NativeDistances = serde_json::from_value(value.clone())
                            .map_err(|error| format!("Invalid native LOD distances: {error}"))?;
                        let level = level.ok_or("Native LOD instance has no mesh level")?;
                        if level >= source.distances.len()
                            || !source.distances.iter().all(|d| d.is_finite())
                            || !source.scale.is_finite()
                            || source.scale == 0.
                            || !source.cull_distance.is_finite()
                        {
                            return Err("Invalid native LOD distance, scale or culling data".into());
                        }
                        return Ok(Some(NativeLod {
                            level,
                            source,
                            selected: false,
                        }));
                    }
                }
                ancestor = parents.get(current).ok().map(ChildOf::parent);
            }
            if level.is_some() {
                return Err("Native LOD mesh has no instance distances".into());
            }
            Ok(None)
        })();
        match result {
            Ok(Some(lod)) => {
                commands
                    .entity(entity)
                    .try_insert((lod, VisibilityRange::abrupt(0., 0.)));
            }
            Ok(None) => {}
            Err(error) => {
                error!("{error}");
                failure.0 = error;
                next.set(crate::GameState::LoadError);
            }
        }
        commands.entity(entity).try_insert(LodReady);
    }
}

pub(super) fn select(
    cameras: Query<(Entity, &Projection, &GlobalTransform), With<super::GameCamera>>,
    mut meshes: Query<(&mut NativeLod, &GlobalTransform, &mut VisibilityRange)>,
    mut history: Local<Option<(Entity, f32)>>,
) {
    let view = cameras
        .single()
        .ok()
        .and_then(|(camera, projection, transform)| {
            let Projection::Perspective(projection) = projection else {
                return None;
            };
            let factor = (projection.fov * 0.5).tan() * f32::from_bits(0x40079857);
            if !factor.is_finite() || factor <= 0. {
                return None;
            }
            let historical = match *history {
                Some((owner, previous))
                    if owner == camera && previous > factor && previous / factor <= 1.25 =>
                {
                    previous
                }
                _ => factor,
            };
            Some((camera, transform.translation(), historical))
        });
    *history = view.map(|(camera, _, factor)| (camera, factor));
    for (mut lod, transform, mut range) in &mut meshes {
        let selected = view.and_then(|(_, origin, historical)| {
            let raw = origin.distance(transform.translation());
            let biased = raw * historical;
            let scale = lod.source.scale.max(1.);
            if biased < lod.source.cull_distance {
                lod.source
                    .distances
                    .iter()
                    .position(|threshold| biased / scale < *threshold)
                    .or_else(|| {
                        (biased != raw)
                            .then(|| {
                                lod.source
                                    .distances
                                    .iter()
                                    .position(|threshold| raw / scale < *threshold)
                            })
                            .flatten()
                    })
            } else {
                None
            }
        });
        let active = selected == Some(lod.level);
        if lod.selected != active {
            lod.selected = active;
        }
        let end = if active { f32::MAX } else { 0. };
        range.set_if_neq(VisibilityRange::abrupt(0., end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera(app: &mut App, tangent: f32) -> Entity {
        app.world_mut()
            .spawn((
                super::super::GameCamera,
                Projection::Perspective(PerspectiveProjection {
                    fov: 2. * tangent.atan(),
                    ..default()
                }),
                GlobalTransform::IDENTITY,
            ))
            .id()
    }

    fn levels(app: &mut App, distances: &[f32], position: Vec3) -> Vec<Entity> {
        (0..distances.len())
            .map(|level| {
                app.world_mut()
                    .spawn((
                        NativeLod {
                            level,
                            source: NativeDistances {
                                distances: distances.to_vec(),
                                scale: 1.,
                                cull_distance: 100.,
                            },
                            selected: false,
                        },
                        GlobalTransform::from_translation(position),
                        VisibilityRange::abrupt(0., 0.),
                    ))
                    .id()
            })
            .collect()
    }

    fn assert_selection(app: &App, levels: &[Entity], expected: &[bool]) {
        let selected: Vec<_> = levels
            .iter()
            .map(|entity| app.world().get::<NativeLod>(*entity).unwrap().is_selected())
            .collect();
        let raster: Vec<_> = levels
            .iter()
            .map(|entity| {
                app.world()
                    .get::<VisibilityRange>(*entity)
                    .unwrap()
                    .is_visible_at_all(0.)
            })
            .collect();
        assert_eq!(selected, expected);
        assert_eq!(raster, expected);
    }

    #[test]
    fn spawned_scene_lods_are_prepared_and_selected_in_the_first_post_update() {
        let mut app = App::new();
        app.init_resource::<crate::assets::LoadFailure>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .add_plugins(PropLodPlugin)
            .add_systems(
                SpawnScene,
                |mut commands: Commands, mut spawned: Local<bool>| {
                    if *spawned {
                        return;
                    }
                    *spawned = true;
                    let parent = commands
                        .spawn(GltfExtras {
                            value: serde_json::json!({"native_lod": {
                                "distances": [20., 40.],
                                "scale": 1.,
                                "cull_distance": 100.
                            }})
                            .to_string(),
                        })
                        .id();
                    for level in 0..2 {
                        commands.spawn((
                            Mesh3d(Handle::default()),
                            GlobalTransform::from_translation(Vec3::new(10., 0., 0.)),
                            ChildOf(parent),
                            GltfExtras {
                                value: serde_json::json!({"native_lod_level": level}).to_string(),
                            },
                        ));
                    }
                },
            );
        camera(&mut app, 1.);
        app.update();
        let mut query = app.world_mut().query::<(Entity, &NativeLod)>();
        let mut levels: Vec<_> = query
            .iter(app.world())
            .map(|(entity, lod)| (lod.level, entity))
            .collect();
        levels.sort_by_key(|(level, _)| *level);
        assert_eq!(levels.len(), 2);
        assert_selection(
            &app,
            &levels.iter().map(|(_, entity)| *entity).collect::<Vec<_>>(),
            &[false, true],
        );
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .is_empty()
        );
    }

    #[test]
    fn source_thresholds_fallback_scale_and_cull_drive_the_same_selection() {
        let mut app = App::new();
        app.add_systems(Update, select);
        camera(&mut app, 1.);
        let factor = (std::f32::consts::FRAC_PI_2 * 0.5).tan() * f32::from_bits(0x40079857);
        let levels = levels(&mut app, &[10. * factor, 20. * factor], Vec3::X * 10.);
        app.update();
        assert_selection(&app, &levels, &[false, true]);

        for entity in &levels {
            *app.world_mut().get_mut::<GlobalTransform>(*entity).unwrap() =
                GlobalTransform::from_translation(Vec3::X * 20.);
        }
        app.update();
        assert_selection(&app, &levels, &[true, false]);

        for (scale, expected) in [(2., [true, false]), (0.5, [false, true])] {
            for entity in &levels {
                *app.world_mut().get_mut::<GlobalTransform>(*entity).unwrap() =
                    GlobalTransform::from_translation(Vec3::X * 10.);
                app.world_mut()
                    .get_mut::<NativeLod>(*entity)
                    .unwrap()
                    .source
                    .scale = scale;
            }
            app.update();
            assert_selection(&app, &levels, &expected);
        }

        for entity in &levels {
            app.world_mut()
                .get_mut::<NativeLod>(*entity)
                .unwrap()
                .source
                .cull_distance = 10. * factor;
        }
        app.update();
        assert_selection(&app, &levels, &[false, false]);
        for entity in &levels {
            *app.world_mut().get_mut::<GlobalTransform>(*entity).unwrap() =
                GlobalTransform::from_translation(Vec3::X);
        }
        app.update();
        assert_selection(&app, &levels, &[true, false]);
    }

    #[test]
    fn invalid_authoritative_views_clear_selection_and_reset_zoom_history() {
        let mut app = App::new();
        app.add_systems(Update, select);
        let mut owner = camera(&mut app, 1.);
        let levels = levels(&mut app, &[20., 40.], Vec3::X * 10.);
        app.world_mut().spawn((
            Projection::Perspective(PerspectiveProjection::default()),
            GlobalTransform::from_translation(Vec3::X * 1000.),
        ));
        app.update();
        assert_selection(&app, &levels, &[false, true]);
        let zoomed = Projection::Perspective(PerspectiveProjection {
            fov: 2. * 0.9_f32.atan(),
            ..default()
        });
        app.world_mut().entity_mut(owner).insert(zoomed.clone());
        app.update();
        assert_selection(&app, &levels, &[false, true]);
        app.world_mut()
            .entity_mut(owner)
            .remove::<super::super::GameCamera>();
        app.update();
        assert_selection(&app, &levels, &[false, false]);
        app.world_mut()
            .entity_mut(owner)
            .insert(super::super::GameCamera);
        app.update();
        assert_selection(&app, &levels, &[true, false]);

        let second = camera(&mut app, 1.);
        app.update();
        assert_selection(&app, &levels, &[false, false]);
        app.world_mut().despawn(second);
        app.update();
        assert_selection(&app, &levels, &[true, false]);

        let invalid = [
            Projection::Orthographic(OrthographicProjection::default_3d()),
            Projection::Perspective(PerspectiveProjection {
                fov: f32::NAN,
                ..default()
            }),
            Projection::Perspective(PerspectiveProjection {
                fov: f32::INFINITY,
                ..default()
            }),
            Projection::Perspective(PerspectiveProjection {
                fov: 0.,
                ..default()
            }),
            Projection::Perspective(PerspectiveProjection {
                fov: -0.5,
                ..default()
            }),
        ];
        for projection in invalid {
            app.world_mut().entity_mut(owner).insert(projection);
            app.update();
            assert_selection(&app, &levels, &[false, false]);
            app.world_mut().entity_mut(owner).insert(zoomed.clone());
            app.update();
            assert_selection(&app, &levels, &[true, false]);
        }

        app.world_mut()
            .entity_mut(owner)
            .insert(Projection::Perspective(PerspectiveProjection {
                fov: std::f32::consts::FRAC_PI_2,
                ..default()
            }));
        app.update();
        assert_selection(&app, &levels, &[false, true]);
        app.world_mut().despawn(owner);
        owner = camera(&mut app, 0.9);
        app.update();
        assert_selection(&app, &levels, &[true, false]);
        assert!(app.world().get::<super::super::GameCamera>(owner).is_some());
    }

    #[test]
    fn unchanged_selection_does_not_dirty_lod_or_raster_range() {
        #[derive(Resource, Default)]
        struct Changes(usize);

        let mut app = App::new();
        app.init_resource::<Changes>().add_systems(
            Update,
            (
                select,
                |changed: Query<Entity, Or<(Changed<NativeLod>, Changed<VisibilityRange>)>>,
                 mut count: ResMut<Changes>| {
                    count.0 = changed.iter().count();
                },
            )
                .chain(),
        );
        camera(&mut app, 1.);
        let levels = levels(&mut app, &[20., 40.], Vec3::X * 10.);
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, 2);
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, 0);
        for entity in &levels {
            *app.world_mut().get_mut::<GlobalTransform>(*entity).unwrap() =
                GlobalTransform::from_translation(Vec3::X);
        }
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, 2);
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, 0);
    }

    #[test]
    fn rigid_lod_uses_vertical_tangent_across_aspects_and_retains_zoom_hysteresis() {
        let mut app = App::new();
        app.add_systems(Update, select);
        let camera = app
            .world_mut()
            .spawn((
                super::super::GameCamera,
                Projection::Perspective(PerspectiveProjection {
                    fov: std::f32::consts::FRAC_PI_2,
                    aspect_ratio: 1.,
                    ..default()
                }),
                GlobalTransform::default(),
            ))
            .id();
        let levels: Vec<_> = (0..2)
            .map(|level| {
                app.world_mut()
                    .spawn((
                        NativeLod {
                            level,
                            source: NativeDistances {
                                distances: vec![20., 40.],
                                scale: 1.,
                                cull_distance: 100.,
                            },
                            selected: false,
                        },
                        GlobalTransform::from_translation(Vec3::new(10., 0., 0.)),
                        VisibilityRange::abrupt(0., 0.),
                    ))
                    .id()
            })
            .collect();
        let visible = |app: &App| {
            levels
                .iter()
                .map(|entity| {
                    app.world()
                        .get::<VisibilityRange>(*entity)
                        .unwrap()
                        .is_visible_at_all(10.)
                })
                .collect::<Vec<_>>()
        };
        app.update();
        assert_eq!(visible(&app), [false, true]);
        for aspect in [0.5, 16. / 9., 3.] {
            let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
            let Projection::Perspective(projection) = &mut *projection else {
                unreachable!()
            };
            projection.aspect_ratio = aspect;
            app.update();
            assert_eq!(visible(&app), [false, true]);
        }
        for (tangent, expected) in [(0.9, [false, true]), (0.79, [true, false])] {
            let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
            let Projection::Perspective(projection) = &mut *projection else {
                unreachable!()
            };
            projection.fov = 2. * f32::atan(tangent);
            app.update();
            assert_eq!(visible(&app), expected);
        }
    }
}
