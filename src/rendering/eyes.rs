use super::pbr::SurfaceMetadata;
use super::{GameCamera, NativeArtInputs, opaque::StandardReady, pbr::PbrReady};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct EyeLuminance {
    original: Handle<StandardMaterial>,
    target: AssetId<StandardMaterial>,
}

fn validate(source: &serde_json::Value) -> Result<(), String> {
    if source["code_constant"] != 171
        || source["component"] != 1
        || source["lod"] != 0.
        || source["base_state"] != serde_json::json!([0., 0., 0., 0.])
        || source["prepared_unit_luminance"] != true
        || source["runtime_operation"] != "square_scriptVector2_y"
    {
        return Err("Unsupported native eye luminance input".into());
    }
    Ok(())
}

pub(super) fn prepare(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            Ref<SurfaceMetadata>,
            Option<&EyeLuminance>,
        ),
        (
            With<PbrReady>,
            With<StandardReady>,
            Or<(
                Added<StandardReady>,
                Changed<SurfaceMetadata>,
                Changed<MeshMaterial3d<StandardMaterial>>,
            )>,
        ),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: ResMut<super::OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for (entity, current, extras, existing) in &surfaces {
        if existing.is_some_and(|eye| eye.target == current.0.id()) && !extras.is_changed() {
            continue;
        }
        let result = (|| -> Result<bool, String> {
            let source = &extras.0["surface"]["source_eye_luminance"];
            if source.is_null() {
                return Ok(false);
            }
            validate(source)?;
            Ok(true)
        })();
        match result {
            Ok(false) => {
                commands.entity(entity).remove::<EyeLuminance>();
            }
            Ok(true) => {
                let Some(source) = materials.get(&current.0).cloned() else {
                    status.pending += 1;
                    continue;
                };
                let target = materials.add(source);
                commands.entity(entity).insert((
                    EyeLuminance {
                        original: current.0.clone(),
                        target: target.id(),
                    },
                    MeshMaterial3d(target),
                ));
            }
            Err(error) => {
                failure.0 = error;
                next.set(crate::GameState::LoadError);
                return;
            }
        }
    }
}

pub(super) fn animate(
    surfaces: Query<(Entity, &EyeLuminance, &MeshMaterial3d<StandardMaterial>)>,
    parents: Query<&ChildOf>,
    rigs: Query<&crate::presentation::Rig>,
    inputs: Query<&NativeArtInputs>,
    normal_views: Query<(), (With<GameCamera>, With<Camera3d>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: ResMut<super::OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for (entity, eye, current) in &surfaces {
        if eye.target != current.0.id() {
            continue;
        }
        let mut ancestor = Some(entity);
        let mut luminance = None;
        while let Some(current) = ancestor {
            let active = |owner| {
                inputs
                    .get(owner)
                    .ok()
                    .filter(|input| input.script_available & (1 << 2) != 0)
                    .map(|input| input.script_vectors[2][1])
            };
            luminance = active(current)
                .or_else(|| rigs.get(current).ok().and_then(|rig| active(rig.owner)));
            if luminance.is_some() {
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        let result = luminance
            .or_else(|| (!normal_views.is_empty()).then_some(0.))
            .ok_or("Native eye ScriptVector2 is unavailable in this view")
            .and_then(|luminance| {
                let factor = luminance * luminance;
                if !factor.is_finite() {
                    return Err("Native eye luminance square is non-finite");
                }
                Ok(Color::linear_rgba(factor, factor, factor, 1.))
            });
        let color = match result {
            Ok(color) => color,
            Err(error) => {
                failure.0 = error.into();
                next.set(crate::GameState::LoadError);
                return;
            }
        };
        let Some(material) = materials.get(&current.0) else {
            status.pending += 1;
            continue;
        };
        if material.base_color != color
            && let Some(mut material) = materials.get_mut(&current.0)
        {
            material.base_color = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::gltf::GltfMaterialExtras;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<StandardMaterial>>()
            .init_resource::<super::super::OpaqueSurfaceStatus>()
            .init_resource::<crate::assets::LoadFailure>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .add_systems(Update, (super::super::pbr::parse_metadata, prepare, animate).chain());
        app
    }

    fn inputs(luminance: f32, active: bool) -> NativeArtInputs {
        NativeArtInputs {
            script_vectors: [[0.; 4], [0.; 4], [0., luminance, 0., 0.], [0.; 4]],
            script_available: if active { 1 << 2 } else { 0 },
            ..default()
        }
    }

    fn metadata() -> GltfMaterialExtras {
        GltfMaterialExtras {
            value: serde_json::json!({"surface":{"source_eye_luminance":{
                "code_constant":171,"component":1,"lod":0.,"base_state":[0.,0.,0.,0.],
                "prepared_unit_luminance":true,"runtime_operation":"square_scriptVector2_y"
            }}}).to_string(),
        }
    }

    fn surface(app: &mut App, owner: Entity, material: Handle<StandardMaterial>) -> Entity {
        app.world_mut()
            .spawn((
                ChildOf(owner),
                MeshMaterial3d(material),
                metadata(),
                PbrReady,
                StandardReady,
            ))
            .id()
    }

    fn color(app: &App, entity: Entity) -> Color {
        let material = app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).unwrap();
        app.world().resource::<Assets<StandardMaterial>>().get(&material.0).unwrap().base_color
    }

    #[test]
    fn eye_luminance_is_private_and_resets_when_actor_mapping_disappears() {
        let mut app = app();
        app.world_mut().spawn((GameCamera, Camera3d::default()));
        let texture = Handle::<Image>::default();
        let original = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
            base_color: Color::BLACK,
            base_color_texture: Some(texture.clone()),
            unlit: true,
            ..default()
        });
        let first = app.world_mut().spawn(inputs(3., true)).id();
        let second = app.world_mut().spawn(inputs(0.25, true)).id();
        let a = surface(&mut app, first, original.clone());
        let b = surface(&mut app, second, original.clone());
        app.update();
        let ah = &app.world().get::<MeshMaterial3d<StandardMaterial>>(a).unwrap().0;
        let bh = &app.world().get::<MeshMaterial3d<StandardMaterial>>(b).unwrap().0;
        assert_ne!(ah.id(), bh.id());
        assert_ne!(ah.id(), original.id());
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(materials.get(ah).unwrap().base_color_texture, Some(texture.clone()));
        assert_eq!(materials.get(bh).unwrap().base_color_texture, Some(texture));
        assert_eq!(materials.get(&original).unwrap().base_color, Color::BLACK);
        assert_eq!(color(&app, a), Color::linear_rgba(9., 9., 9., 1.));
        assert_eq!(color(&app, b), Color::linear_rgba(0.0625, 0.0625, 0.0625, 1.));
        app.world_mut().get_mut::<NativeArtInputs>(first).unwrap().script_available = 0;
        app.update();
        assert_eq!(color(&app, a), Color::linear_rgba(0., 0., 0., 1.));
        assert_eq!(color(&app, b), Color::linear_rgba(0.0625, 0.0625, 0.0625, 1.));
        app.world_mut().entity_mut(a).insert(ChildOf(second));
        app.update();
        assert_eq!(color(&app, a), color(&app, b));
        app.world_mut().entity_mut(second).remove::<NativeArtInputs>();
        app.update();
        assert_eq!(color(&app, a), Color::linear_rgba(0., 0., 0., 1.));
        assert_eq!(color(&app, b), Color::linear_rgba(0., 0., 0., 1.));
    }

    #[test]
    fn nearest_active_eye_override_wins_and_invalid_active_values_fail() {
        let mut app = app();
        let original = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let root = app.world_mut().spawn(inputs(3., true)).id();
        let nearest = app.world_mut().spawn((ChildOf(root), inputs(f32::NAN, false))).id();
        let mesh = surface(&mut app, nearest, original);
        app.update();
        assert_eq!(color(&app, mesh), Color::linear_rgba(9., 9., 9., 1.));
        app.world_mut().entity_mut(nearest).insert(inputs(-0.25, true));
        app.update();
        assert_eq!(color(&app, mesh), Color::linear_rgba(0.0625, 0.0625, 0.0625, 1.));
        app.world_mut().get_mut::<NativeArtInputs>(nearest).unwrap().script_available = 0;
        app.update();
        assert_eq!(color(&app, mesh), Color::linear_rgba(9., 9., 9., 1.));
        app.world_mut().entity_mut(nearest).insert(inputs(f32::INFINITY, true));
        app.update();
        assert!(app.world().resource::<crate::assets::LoadFailure>().0.contains("non-finite"));
        assert_eq!(color(&app, mesh), Color::linear_rgba(9., 9., 9., 1.));
        app.world_mut().resource_mut::<crate::assets::LoadFailure>().0.clear();
        app.world_mut().entity_mut(nearest).remove::<NativeArtInputs>();
        app.world_mut().entity_mut(root).remove::<NativeArtInputs>();
        app.update();
        assert!(app.world().resource::<crate::assets::LoadFailure>().0.contains("unavailable in this view"));
    }

    #[test]
    fn eye_material_rebinding_and_session_reset_release_private_handles() {
        let mut app = app();
        let original = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial::default());
        let owner = app.world_mut().spawn(inputs(3., true)).id();
        let mesh = surface(&mut app, owner, original.clone());
        app.update();
        let replacement = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
            base_color: Color::linear_rgba(0.1, 0.2, 0.3, 1.),
            ..default()
        });
        app.world_mut().entity_mut(mesh).insert((
            MeshMaterial3d(replacement.clone()),
            GltfMaterialExtras { value: "{}".into() },
        ));
        app.update();
        assert!(app.world().get::<EyeLuminance>(mesh).is_none());
        assert_eq!(color(&app, mesh), Color::linear_rgba(0.1, 0.2, 0.3, 1.));
        app.world_mut().entity_mut(mesh).insert((MeshMaterial3d(original.clone()), metadata()));
        app.update();
        assert!(app.world().get::<EyeLuminance>(mesh).is_some());
        app.add_systems(Last, reset);
        app.update();
        assert!(app.world().get::<EyeLuminance>(mesh).is_none());
        assert_eq!(app.world().get::<MeshMaterial3d<StandardMaterial>>(mesh).unwrap().0, original);
    }
}

pub(super) fn reset(
    mut commands: Commands,
    surfaces: Query<(Entity, &EyeLuminance, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, eye, current) in &surfaces {
        let mut entity = commands.entity(entity);
        if eye.target == current.0.id() {
            entity.insert(MeshMaterial3d(eye.original.clone()));
        }
        entity.remove::<EyeLuminance>();
    }
}
