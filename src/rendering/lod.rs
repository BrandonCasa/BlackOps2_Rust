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
        app.add_systems(Update, prepare).add_systems(
            PostUpdate,
            select
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
struct NativeLod {
    level: usize,
    source: NativeDistances,
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
                        return Ok(Some(NativeLod { level, source }));
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

fn select(
    cameras: Query<(Entity, &Projection, &GlobalTransform), With<super::GameCamera>>,
    mut meshes: Query<(&NativeLod, &GlobalTransform, &mut VisibilityRange)>,
    mut history: Local<Option<(Entity, f32)>>,
) {
    let Ok((camera, Projection::Perspective(projection), camera_transform)) = cameras.single()
    else {
        return;
    };
    let factor = (projection.fov * 0.5).tan() * f32::from_bits(0x40079857);
    if !factor.is_finite() || factor <= 0. {
        return;
    }
    let historical = match *history {
        Some((owner, previous))
            if owner == camera && previous > factor && previous / factor <= 1.25 =>
        {
            previous
        }
        _ => factor,
    };
    *history = Some((camera, historical));
    for (lod, transform, mut range) in &mut meshes {
        let raw = camera_transform
            .translation()
            .distance(transform.translation());
        let biased = raw * historical;
        let scale = lod.source.scale.max(1.);
        let selected = if biased < lod.source.cull_distance {
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
        };
        let end = if selected == Some(lod.level) {
            f32::MAX
        } else {
            0.
        };
        range.set_if_neq(VisibilityRange::abrupt(0., end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
