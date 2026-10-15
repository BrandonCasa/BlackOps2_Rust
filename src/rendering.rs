use crate::{GameState, assets::GameDefinition, player::Player};
use bevy::prelude::*;
pub(crate) const BASE_FOV_DEGREES: f32 = 55.;
#[derive(Component)]
pub struct GameCamera;
#[cfg(feature = "dev-tools")]
#[derive(Resource)]
pub(crate) struct ValidationView(pub Transform);
pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, camera_follow.run_if(crate::flythrough::normal_play));
    }
}
pub(crate) fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        bevy::pbr::DistanceFog::default(),
        GameCamera,
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        Projection::Perspective(PerspectiveProjection {
            fov: BASE_FOV_DEGREES.to_radians(),
            near: 0.01,
            ..default()
        }),
        Transform::from_xyz(0., 2., 0.),
        bevy::audio::SpatialListener::new(0.2),
        Name::new("Origins camera"),
    ));
    commands.spawn((
        DirectionalLight::default(),
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.)),
        Name::new("Sunlight"),
    ));
}
fn camera_follow(
    #[cfg(feature = "dev-tools")] validation_view: Option<Res<ValidationView>>,
    players: Query<
        (
            &Player,
            &Transform,
            &crate::player::PreviousTranslation,
            &crate::weapons::WeaponInventory,
            &crate::movement::Mobility,
            &crate::movement::MotionFeedback,
        ),
        Without<GameCamera>,
    >,
    time: Res<Time<Fixed>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
    definition: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
) {
    #[cfg(feature = "dev-tools")]
    if let Some(view) = validation_view {
        for (mut transform, _) in &mut camera {
            *transform = view.0;
        }
        return;
    }
    if !matches!(
        **state,
        GameState::StartingSession | GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let (
        Ok((player, body, previous, inventory, mobility, feedback)),
        Some(definition),
        Ok((mut camera, mut projection)),
    ) = (players.single(), definition, camera.single_mut())
    else {
        return;
    };
    let eye = if **state == GameState::Playing {
        mobility
            .previous_eye
            .lerp(player.eye_height, time.overstep_fraction())
    } else {
        player.eye_height
    };
    let bob = feedback
        .previous_bob
        .lerp(feedback.bob, time.overstep_fraction());
    let target = Transform {
        translation: (if **state == GameState::Playing {
            previous.0.lerp(body.translation, time.overstep_fraction())
        } else {
            body.translation
        }) + Vec3::Y * (eye - player.height * 0.5)
            + Quat::from_rotation_y(player.yaw) * Vec3::new(bob.x, bob.y, 0.),
        rotation: Quat::from_rotation_y(player.yaw + inventory.recoil.x + feedback.rumble.x)
            * Quat::from_rotation_x(
                (player.pitch + inventory.recoil.y + feedback.rumble.y).clamp(-1.5, 1.5),
            ),
        ..*camera
    };
    camera.set_if_neq(target);
    if **state == GameState::Playing
        && let Projection::Perspective(perspective) = &mut *projection
    {
        perspective.fov = inventory
            .definition(&definition.content.weapons)
            .map_or(BASE_FOV_DEGREES.to_radians(), |w| {
                inventory.ads_fov(BASE_FOV_DEGREES, w)
            });
    }
}
