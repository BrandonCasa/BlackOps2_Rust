use super::native::{self, Canvas, Opacity, SafeAnchor, rect};
use crate::{GameState, assets::GameDefinition, generators::GeneratorState, player::Player};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Resource)]
struct CaptureMask(Vec<(f32, u8)>);
impl Default for CaptureMask {
    fn default() -> Self {
        Self(
            (0..128 * 128)
                .map(|index| {
                    let delta = Vec2::new((index % 128) as f32 + 0.5, (index / 128) as f32 + 0.5)
                        / 128.
                        - Vec2::splat(0.5);
                    let radius = delta.length() * 2.;
                    let outer = ((radius - (1. - 1. / 12.)) * 12.).clamp(0., 1.);
                    let inner = ((1. - 1. / 12. - 0.06 - radius) * 12.).clamp(0., 1.);
                    (
                        delta.x.atan2(delta.y),
                        ((1. - outer.max(inner)) * 255.).round() as u8,
                    )
                })
                .collect(),
        )
    }
}
#[derive(Component)]
struct WidgetRoot;
#[derive(Component)]
struct Medal {
    index: usize,
    glow: bool,
}
#[derive(Component)]
struct CaptureWheel;
#[derive(Component)]
struct WheelImage(Option<usize>);
#[derive(Component)]
struct Marker {
    number: u8,
    position: Vec2,
    size: f32,
}
#[derive(Component)]
struct MarkerLayer {
    number: u8,
    layer: u8,
}
#[derive(Component)]
struct Progress {
    number: u8,
    value: f32,
}
#[derive(Component)]
struct RaiderMarker(usize);
#[derive(Resource, Default)]
struct WidgetState {
    owner: Option<Entity>,
    medals: Vec<usize>,
    ages: [f32; 4],
    powered: [bool; 6],
    wheel: f32,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<CaptureMask>()
        .init_resource::<WidgetState>()
        .add_systems(Update, (setup, update).chain());
}
fn full() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: percent(100.),
        height: percent(100.),
        ..default()
    }
}

fn setup(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    roots: Query<(), With<WidgetRoot>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(game), Ok(camera)) = (game, cameras.single()) else {
        return;
    };
    if !roots.is_empty() {
        return;
    }
    commands
        .spawn((
            WidgetRoot,
            Canvas,
            super::PlayerOverlay,
            UiTargetCamera(camera),
            GlobalZIndex(35),
            rect(0., 0., 1280., 720.),
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            for (index, name) in ["kill", "level", "economy", "team"].into_iter().enumerate() {
                if game.delivery_scope == crate::origins::DeliveryScope::FirstPlayableSolo {
                    break;
                }
                let width = if index == 3 { 72.5 } else { 50. };
                for glow in [false, true] {
                    let name = if glow {
                        format!(
                            "tomb_medal_glow_{}",
                            if index == 3 { "big" } else { "small" }
                        )
                    } else {
                        format!("tomb_medal_{name}")
                    };
                    parent.spawn((
                        Medal { index, glow },
                        SafeAnchor::left(5.),
                        native::image(&game, &server, &name),
                        rect(5., 480., width, 50.),
                        Visibility::Hidden,
                    ));
                }
            }
            parent
                .spawn((
                    CaptureWheel,
                    Opacity(0.),
                    SafeAnchor::right(1150.),
                    rect(1150., 10., 120., 120.),
                ))
                .with_children(|p| {
                    for name in ["back", "dial", "eyes"] {
                        p.spawn((
                            WheelImage(if name == "eyes" { Some(6) } else { None }),
                            native::image(&game, &server, &format!("zm_capture_meter_{name}")),
                            if name == "eyes" {
                                rect(45., 45., 30., 30.)
                            } else {
                                rect(0., 0., 120., 120.)
                            },
                            ZIndex(if name == "back" { -1 } else { 1 }),
                        ));
                    }
                    for index in 0..6 {
                        p.spawn((
                            WheelImage(Some(index)),
                            native::image(&game, &server, "zm_capture_meter_wedge"),
                            rect(0., 0., 120., 120.),
                            UiTransform::from_rotation(Rot2::degrees(index as f32 * 60. - 120.)),
                        ));
                    }
                });
            for number in 1..=6 {
                if !game
                    .content
                    .generators
                    .iter()
                    .any(|generator| generator.number == number)
                {
                    continue;
                }
                let image = images.add(Image::new_fill(
                    Extent3d {
                        width: 128,
                        height: 128,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &[255, 255, 255, 0],
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                ));
                parent
                    .spawn((
                        Marker {
                            number,
                            position: Vec2::new(612., 482.),
                            size: 56.,
                        },
                        rect(612., 482., 56., 56.),
                        Visibility::Hidden,
                    ))
                    .with_children(|p| {
                        for (layer, name) in [
                            (0, "hud_zm_tomb_capture_spin".into()),
                            (1, "hud_zm_tomb_capture_spin_glow".into()),
                            (2, format!("tomb_spinner_{number}_shadow")),
                            (3, format!("tomb_spinner_{number}_glow")),
                            (4, "waypoint_arrow_tomb".into()),
                        ] {
                            p.spawn((
                                MarkerLayer { number, layer },
                                native::image(&game, &server, &name),
                                if layer >= 2 && layer <= 3 {
                                    rect(20., 20., 16., 16.)
                                } else if layer == 4 {
                                    rect(12., 46., 32., 32.)
                                } else {
                                    full()
                                },
                                UiTransform::default(),
                            ));
                        }
                        p.spawn((
                            Progress { number, value: 0. },
                            ImageNode::new(image),
                            rect(-6., -6., 68., 68.),
                        ));
                    });
            }
            for slot in 0..4 {
                if game.delivery_scope == crate::origins::DeliveryScope::FirstPlayableSolo {
                    break;
                }
                parent.spawn((
                    RaiderMarker(slot),
                    native::image(&game, &server, "zm_hud_icon_evil_crusade"),
                    rect(0., 0., 32., 32.),
                    Visibility::Hidden,
                ));
            }
        });
}

fn project(camera: &Camera, transform: &GlobalTransform, position: Vec3) -> Option<Vec2> {
    let size = camera.logical_viewport_size()?;
    let ndc = camera.world_to_ndc(transform, position)?;
    let mut point = Vec2::new(ndc.x, -ndc.y) * Vec2::new(size.x / size.y * 360., 360.);
    if ndc.z < 0. {
        point = -point;
    }
    let bounds = Vec2::new(size.x / size.y * 360. - 32., 328.);
    let ratio = (point.abs() / bounds).max_element().max(1.);
    Some(point / ratio + Vec2::new(640., 360.))
}

fn update(
    time: Res<Time>,
    state: Res<State<GameState>>,
    input: Res<crate::input::GameInput>,
    game: Option<Res<GameDefinition>>,
    (rituals, generators, raids): (
        Res<crate::rituals::Rituals>,
        Res<crate::generators::Generators>,
        Res<crate::generators::GeneratorRaids>,
    ),
    (players, cameras, attackers): (
        Query<(Entity, &Transform, &crate::life::Vitality), With<Player>>,
        Query<(&Camera, &GlobalTransform), With<crate::rendering::GameCamera>>,
        Query<
            (
                &Transform,
                &crate::rounds::PopulationKind,
                &crate::life::Vitality,
            ),
            (With<crate::enemies::Zombie>, Without<Player>),
        >,
    ),
    mut cache: ResMut<WidgetState>,
    mut root: Query<&mut Visibility, With<WidgetRoot>>,
    mut medals: Query<
        (&Medal, &mut ImageNode, &mut Visibility, &mut SafeAnchor),
        (Without<WidgetRoot>, Without<Marker>, Without<RaiderMarker>),
    >,
    mut wheel: Query<&mut Opacity, With<CaptureWheel>>,
    mut wheel_images: Query<(&WheelImage, &mut ImageNode), (Without<Medal>, Without<MarkerLayer>)>,
    mut markers: Query<
        (&mut Marker, &mut Node, &mut Visibility),
        (
            Without<Medal>,
            Without<WidgetRoot>,
            Without<RaiderMarker>,
            Without<Progress>,
            Without<MarkerLayer>,
        ),
    >,
    mut layers: Query<
        (&MarkerLayer, &mut Node, &mut ImageNode, &mut UiTransform),
        (
            Without<Medal>,
            Without<WheelImage>,
            Without<Marker>,
            Without<RaiderMarker>,
            Without<Progress>,
        ),
    >,
    mut progress: Query<
        (&mut Progress, &mut ImageNode, &mut Node),
        (
            Without<Marker>,
            Without<MarkerLayer>,
            Without<RaiderMarker>,
            Without<Medal>,
            Without<WheelImage>,
        ),
    >,
    (mask, mut images): (Res<CaptureMask>, ResMut<Assets<Image>>),
    mut raiders: Query<
        (&RaiderMarker, &mut Node, &mut Visibility),
        (
            Without<Marker>,
            Without<Medal>,
            Without<WidgetRoot>,
            Without<Progress>,
            Without<MarkerLayer>,
        ),
    >,
) {
    let (Some(game), Ok((owner, player, health)), Ok((camera, camera_transform))) =
        (game, players.single(), cameras.single())
    else {
        return;
    };
    let powered = std::array::from_fn(|index| {
        game.content
            .generators
            .iter()
            .find(|g| g.number as usize == index + 1)
            .is_some_and(|g| generators.powered(&g.id))
    });
    if cache.owner != Some(owner) {
        *cache = WidgetState {
            owner: Some(owner),
            powered,
            ..default()
        };
    }
    let show = **state == GameState::Playing
        && !input.scoreboard
        && health.state() != crate::life::LifeState::HumanTerminal;
    for mut visible in &mut root {
        *visible = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for age in &mut cache.ages {
        *age += time.delta_secs();
    }
    let dt = time.delta_secs();
    cache.wheel = (cache.wheel - dt).max(0.);
    if powered != cache.powered {
        cache.wheel = 6.;
        cache.powered = powered;
    }
    for mut opacity in &mut wheel {
        opacity.0 = (cache.wheel / 0.5)
            .clamp(0., 1.)
            .min(((6. - cache.wheel) / 0.5).clamp(0., 1.));
    }
    for (layer, mut image) in &mut wheel_images {
        if let Some(index) = layer.0 {
            image.color = if index == 6 {
                Color::srgba(1., 1., 1., if powered.iter().all(|b| *b) { 1. } else { 0. })
            } else if powered[index] {
                Color::srgba(0., 0.5, 1., 0.3)
            } else {
                Color::srgba(0., 0., 0., 0.25)
            };
        }
    }
    cache.medals.retain(|index| rituals.eligible(*index));
    for index in 0..4 {
        if rituals.eligible(index) && !cache.medals.contains(&index) {
            cache.medals.push(index);
            cache.ages[index] = 0.;
        }
    }
    for (medal, mut image, mut visible, mut anchor) in &mut medals {
        let rank = cache.medals.iter().position(|index| *index == medal.index);
        *visible = if rank.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        anchor.x = 5.
            + cache
                .medals
                .iter()
                .take(rank.unwrap_or(0))
                .map(|index| if *index == 3 { 72.5 } else { 50. })
                .sum::<f32>();
        image.color.set_alpha(if medal.glow {
            if cache.ages[medal.index] < 3. {
                (cache.ages[medal.index] % 1. - 0.5).abs() * 2.
            } else {
                0.
            }
        } else {
            1.
        });
    }
    let pulse = 0.35 + 0.65 * (time.elapsed_secs() % 1.4 - 0.7).abs() / 0.7;
    let mut data = [(false, 56., 0., false); 6];
    for (mut marker, mut node, mut visible) in &mut markers {
        let Some(definition) = game
            .content
            .generators
            .iter()
            .find(|g| g.number == marker.number)
        else {
            continue;
        };
        let Some(site) = generators.sites.get(&definition.id) else {
            continue;
        };
        let Some(placement) = game
            .content
            .placements
            .iter()
            .find(|p| p.id == definition.placement)
        else {
            continue;
        };
        let origin = Vec3::from_array(placement.position);
        let capturing = site.state == GeneratorState::Capturing;
        let attacked = raids.under_attack && raids.target.as_ref() == Some(&definition.id);
        *visible = if capturing || attacked {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let occupied = (player.translation - origin).xz().length_squared()
            < definition.radius.powi(2)
            && health.state() == crate::life::LifeState::Standing;
        let large = capturing && occupied;
        let size = if large { 128. } else { 56. };
        let target = if large {
            Vec2::new(576., 482.)
        } else {
            project(camera, camera_transform, origin + Vec3::Y * 0.889)
                .unwrap_or(Vec2::splat(-1000.))
                - Vec2::new(28., 56.)
        };
        let factor = (dt / 0.25).clamp(0., 1.);
        marker.position = marker.position.lerp(target, factor);
        marker.size = marker.size + (size - marker.size) * factor;
        *node = rect(
            marker.position.x,
            marker.position.y,
            marker.size,
            marker.size,
        );
        data[marker.number as usize - 1] =
            (large, marker.size, site.progress, capturing && occupied);
    }
    for (layer, mut node, mut image, mut transform) in &mut layers {
        let (large, size, _, occupied) = data[layer.number as usize - 1];
        let rgb = if occupied {
            [0.68, 0.86, 1.]
        } else {
            [1., 0., 0.]
        };
        match layer.layer {
            1 => image.color = Color::srgba(rgb[0], rgb[1], rgb[2], pulse),
            2 | 3 => {
                let inset = if large { 42. } else { 20. };
                *node = rect(inset, inset, size - inset * 2., size - inset * 2.);
                image
                    .color
                    .set_alpha(if layer.layer == 3 { pulse } else { 1. });
            }
            4 => {
                *node = rect(size * 0.5 - 16., size - 10., 32., 32.);
                image.color.set_alpha(if large { 0. } else { 1. });
                transform.rotation = Rot2::IDENTITY;
            }
            _ => {}
        }
    }
    for (mut progress, mut image, mut node) in &mut progress {
        let (large, size, value, occupied) = data[progress.number as usize - 1];
        let width = if large && value > 0. { 68. } else { 28. };
        *node = rect((size - width) * 0.5, (size - width) * 0.5, width, width);
        image.color = if occupied {
            Color::linear_rgb(0.42, 0.71, 1.)
        } else {
            Color::linear_rgb(1., 0., 0.)
        };
        if progress.value != value
            && let Some(mut texture) = images.get_mut(&image.image)
            && let Some(pixels) = texture.data.as_mut()
        {
            let threshold = std::f32::consts::PI - value * std::f32::consts::TAU;
            for (pixel, &(angle, alpha)) in pixels.chunks_exact_mut(4).zip(&mask.0) {
                pixel[3] = if angle > threshold { alpha } else { 0 };
            }
            progress.value = value;
        }
    }
    let targets: Vec<_> = attackers
        .iter()
        .filter(|(_, kind, health)| {
            **kind == crate::rounds::PopulationKind::GeneratorDefender
                && health.state() != crate::life::LifeState::HumanTerminal
        })
        .map(|(transform, _, _)| transform.translation + Vec3::Y * 1.905)
        .collect();
    for (marker, mut node, mut visible) in &mut raiders {
        *visible = Visibility::Hidden;
        if raids.under_attack
            && let Some(position) = targets
                .get(marker.0)
                .and_then(|position| project(camera, camera_transform, *position))
        {
            *node = rect(position.x - 16., position.y - 32., 32., 32.);
            *visible = Visibility::Inherited;
        }
    }
}
