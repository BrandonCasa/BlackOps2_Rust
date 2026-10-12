use super::{
    native::{self, Canvas, NativeText, rect},
    origins::{Field, GeneratorSector, ScoreboardRoot, text},
};
use crate::{GameState, assets::GameDefinition, player::Player};
use bevy::prelude::*;

#[derive(Component)]
struct InventoryIcon {
    key: String,
}
#[derive(Component)]
struct QuestIcon(usize);
#[derive(Component)]
struct QuestElement(usize);
#[derive(Component)]
struct WheelEyes;
#[derive(Component)]
struct StaffPiece {
    element: usize,
    piece: usize,
}
#[derive(Component)]
struct StaffGlow(usize);
#[derive(Component)]
struct InventoryPreview(usize);
#[derive(Resource, Default)]
struct PreviewState {
    items: std::collections::BTreeSet<String>,
    remaining: [f32; 4],
    owner: Option<Entity>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<PreviewState>()
        .add_systems(Update, (setup, update, preview).chain());
}

fn label(parent: &mut ChildSpawnerCommands, value: &str, size: f32, node: Node, dim: bool) {
    parent.spawn((
        NativeText {
            style: "extraSmall",
            ..NativeText::new(value, size)
        }
        .color(if dim {
            Color::srgba(1., 1., 1., 0.5)
        } else {
            Color::WHITE
        }),
        node,
        Pickable::IGNORE,
    ));
}

fn slot(
    parent: &mut ChildSpawnerCommands,
    game: &GameDefinition,
    server: &AssetServer,
    key: &str,
    name: &str,
    x: f32,
    y: f32,
    width: f32,
) {
    parent
        .spawn((
            Node {
                border: UiRect::all(px(1.)),
                ..rect(x, y, width, 48.)
            },
            BorderColor::all(Color::srgba(1., 1., 1., 0.1)),
            BackgroundColor(Color::srgba(0., 0., 0., 0.3)),
        ))
        .with_children(|parent| {
            parent.spawn((
                InventoryIcon { key: key.into() },
                native::image(game, server, name),
                rect(0., 0., width, 48.),
            ));
        });
}

fn setup(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    roots: Query<(), With<ScoreboardRoot>>,
) {
    let (Some(game), Ok(camera)) = (game, cameras.single()) else {
        return;
    };
    if !roots.is_empty() {
        return;
    }
    commands
        .spawn((
            ScoreboardRoot,
            Canvas,
            super::PlayerOverlay,
            UiTargetCamera(camera),
            GlobalZIndex(80),
            rect(0., 0., 1280., 720.),
            UiTransform::default(),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .with_children(|canvas| {
            canvas.spawn((
                Field::GameOver,
                NativeText::new("", 44.).centered(),
                rect(320., 74., 640., 64.),
            ));
            canvas
                .spawn(rect(208., 163., 864., 310.))
                .with_children(|parent| {
                    for (x, width, category, title) in [
                        (0., 162.25, "Equipment", "Musical Parts"),
                        (177.25, 215.25, "Quest Items", "Staff Parts"),
                        (407.5, 334.25, "Recipes", ""),
                        (752., 103., "Captures", ""),
                    ] {
                        parent.spawn((
                            rect(x, 0., width, 32.),
                            BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
                        ));
                        parent.spawn((
                            rect(x, 38., width, 103.),
                            BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
                        ));
                        label(
                            parent,
                            category,
                            20.,
                            rect(x + 4., 6., width - 4., 20.),
                            true,
                        );
                        label(
                            parent,
                            title,
                            20.,
                            rect(x + 4., 42., width - 4., 20.),
                            false,
                        );
                    }
                    label(
                        parent,
                        "Maxis Drone",
                        20.,
                        rect(411.5, 42., 154., 20.),
                        false,
                    );
                    label(
                        parent,
                        "Zombie Shield",
                        20.,
                        rect(583.75, 42., 154., 20.),
                        false,
                    );
                    slot(
                        parent,
                        &game,
                        &server,
                        "gramophone",
                        "zom_hud_craftable_gramophone",
                        4.,
                        89.,
                        48.,
                    );
                    slot(
                        parent,
                        &game,
                        &server,
                        "master",
                        "zom_hud_craftable_record",
                        57.,
                        89.,
                        48.,
                    );
                    slot(
                        parent,
                        &game,
                        &server,
                        "gem",
                        "zom_hud_craftable_element_fire",
                        110.,
                        89.,
                        48.,
                    );
                    for (index, _element) in ["fire", "air", "lightning", "water"]
                        .into_iter()
                        .enumerate()
                    {
                        let x = 181.25 + index as f32 * 53.;
                        parent.spawn((
                            rect(x, 89., 48., 48.),
                            BackgroundColor(Color::srgba(0., 0., 0., 0.3)),
                        ));
                        parent.spawn((
                            QuestIcon(index),
                            native::image(&game, &server, "zom_hud_craftable_record_blank"),
                            rect(x + 1., 96., 34., 34.),
                        ));
                        parent.spawn((
                            QuestElement(index),
                            native::image(
                                &game,
                                &server,
                                &format!(
                                    "zom_hud_craftable_element_{}",
                                    ["fire", "wind", "lightning", "water"][index]
                                ),
                            ),
                            rect(x + 23., 91., 20., 20.),
                        ));
                        for piece in 0..3 {
                            parent.spawn((
                                StaffPiece {
                                    element: index,
                                    piece,
                                },
                                rect(x + 34., 93. + piece as f32 * 14.1, 11., 12.1),
                                native::image(&game, &server, "zm_hud_icon_staff_pieces"),
                            ));
                        }
                        parent.spawn((
                            StaffGlow(index),
                            native::image(&game, &server, "zom_hud_craftable_staff_glow"),
                            rect(x + 4., 96., 34., 34.),
                            Visibility::Hidden,
                        ));
                    }
                    for (piece, name, x) in [
                        (
                            "part.equip_dieseldrone_zm.body",
                            "zom_hud_craftable_zquad_body",
                            411.,
                        ),
                        (
                            "part.equip_dieseldrone_zm.brain",
                            "zom_hud_craftable_zquad_brain",
                            464.,
                        ),
                        (
                            "part.equip_dieseldrone_zm.engine",
                            "zom_hud_craftable_zquad_engine",
                            517.,
                        ),
                        (
                            "part.tomb_shield_zm.top",
                            "zom_hud_craftable_zshield_tomb_vizor",
                            583.,
                        ),
                        (
                            "part.tomb_shield_zm.door",
                            "zom_hud_craftable_zshield_tomb_body",
                            636.,
                        ),
                        (
                            "part.tomb_shield_zm.bracket",
                            "zom_hud_craftable_zshield_tomb_feet",
                            689.,
                        ),
                    ] {
                        slot(parent, &game, &server, piece, name, x, 89., 48.);
                    }
                    parent
                        .spawn(rect(756., 42., 95., 95.))
                        .with_children(|meter| {
                            meter.spawn((
                                native::image(&game, &server, "zm_capture_meter_back"),
                                rect(0., 0., 94., 94.),
                            ));
                            for index in 0..6 {
                                meter.spawn((
                                    GeneratorSector(index),
                                    native::image(&game, &server, "zm_capture_meter_wedge"),
                                    rect(0., 0., 94., 94.),
                                    UiTransform::from_rotation(Rot2::degrees(
                                        index as f32 * 60. - 120.,
                                    )),
                                ));
                            }
                            meter.spawn((
                                native::image(&game, &server, "zm_capture_meter_dial"),
                                rect(0., 0., 94., 94.),
                            ));
                            meter.spawn((
                                WheelEyes,
                                native::image(&game, &server, "zm_capture_meter_eyes"),
                                rect(35.625, 35.625, 23.75, 23.75),
                            ));
                        });
                    parent.spawn((
                        rect(0., 150., 864., 32.),
                        BackgroundColor(Color::srgba(0., 0., 0., 0.75)),
                    ));
                    label(
                        parent,
                        "Origins - Excavation Site 64",
                        25.,
                        rect(80., 153.5, 420., 25.),
                        false,
                    );
                    for (x, header) in [
                        (500., "Score"),
                        (566., "Kills"),
                        (632., "Downs"),
                        (698., "Revives"),
                        (764., "Headshots"),
                    ] {
                        label(parent, header, 20., rect(x, 156., 66., 20.), true);
                    }
                    for row in 0..4 {
                        parent.spawn((
                            rect(0., 188. + row as f32 * 29., 864., 27.),
                            BackgroundColor(Color::srgba(
                                0.,
                                0.,
                                0.,
                                if row == 0 { 0.8 } else { 0.55 },
                            )),
                        ));
                    }
                    for x in [500., 632., 764.] {
                        parent.spawn((
                            rect(x, 188., 66., 114.),
                            BackgroundColor(Color::srgba(0., 0.5, 1., 0.2)),
                        ));
                    }
                    parent.spawn((
                        Node {
                            border: UiRect::all(px(2.)),
                            ..rect(188., 188., 643., 27.)
                        },
                        BorderColor::all(Color::srgb(0.96, 0.58, 0.11)),
                    ));
                    text(
                        parent,
                        Field::Character,
                        "",
                        20.,
                        rect(221., 191.5, 232., 20.),
                    );
                    for (x, field) in [
                        (500., Field::Score),
                        (566., Field::Kills),
                        (632., Field::Downs),
                        (764., Field::Headshots),
                    ] {
                        parent.spawn((
                            field,
                            NativeText {
                                style: "extraSmall",
                                ..NativeText::new("0", 20.).centered()
                            },
                            rect(x, 191.5, 66., 20.),
                        ));
                    }
                    label(parent, "0", 20., rect(698., 191.5, 66., 20.), false);
                    parent.spawn((
                        native::image(&game, &server, "zm_hud_icon_evil_crusade"),
                        rect(31., 180., 128., 128.),
                    ));
                });
        });
}

fn update(
    state: Res<State<GameState>>,
    input: Res<crate::input::GameInput>,
    session: Res<crate::gameplay::Session>,
    history: Res<crate::damage::CombatHistory>,
    (crafting, parts): (
        Res<crate::crafting::Crafting>,
        Res<crate::environment::parts::Parts>,
    ),
    (upgrades, generators): (
        Res<crate::staff_upgrades::StaffUpgrades>,
        Res<crate::generators::Generators>,
    ),
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    mut roots: Query<&mut Visibility, (With<ScoreboardRoot>, Without<StaffGlow>)>,
    mut labels: Query<(&Field, &mut NativeText)>,
    mut images: Query<(
        &mut ImageNode,
        Option<&InventoryIcon>,
        Option<&GeneratorSector>,
        Option<&StaffPiece>,
        Option<&QuestIcon>,
        Option<&QuestElement>,
        Option<&WheelEyes>,
    )>,
    mut glows: Query<(&StaffGlow, &mut Visibility), Without<ScoreboardRoot>>,
) {
    let show = **state == GameState::GameOver || **state == GameState::Playing && input.scoreboard;
    for mut visible in &mut roots {
        *visible = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (field, mut text) in &mut labels {
        let value = match field {
            Field::Score => if **state == GameState::GameOver {
                session.score
            } else {
                session.points
            }
            .to_string(),
            Field::Kills => session.kills.to_string(),
            Field::Downs => session.downs.to_string(),
            Field::Headshots => history.headshot_kills.to_string(),
            Field::GameOver => {
                if **state == GameState::GameOver {
                    format!("GAME OVER\nYou Survived {} Rounds", session.round)
                } else {
                    String::new()
                }
            }
            _ => continue,
        };
        if text.value != value {
            text.value = value;
        }
    }
    let Some(game) = game else { return };
    let colors = [
        [1., 0.24, 0.],
        [1., 0.94, 0.],
        [0.4, 0.18, 0.57],
        [0., 0.71, 1.],
    ];
    for (mut image, icon, sector, piece, quest, element, eyes) in &mut images {
        if let Some(icon) = icon {
            let acquired = match icon.key.as_str() {
                "gramophone" => crafting.gramophone,
                "master" => crafting.records.contains("master"),
                key if key.starts_with("record.") => crafting.records.contains(match &key[7..] {
                    "water" => "ice",
                    "lightning" => "elec",
                    element => element,
                }),
                key => {
                    crafting.consumed.contains(key)
                        || matches!(
                            parts.pieces.get(key),
                            Some(
                                crate::environment::parts::PartState::Owned { .. }
                                    | crate::environment::parts::PartState::Consumed
                            )
                        )
                }
            };
            let rgb = match icon.key.as_str() {
                "record.fire" => [1., 0.25, 0.15],
                "record.water" => [0.25, 0.65, 1.],
                "record.air" => [1., 0.8, 0.2],
                "record.lightning" => [0.75, 0.3, 1.],
                _ => [1.; 3],
            };
            image.color = Color::srgba(rgb[0], rgb[1], rgb[2], if acquired { 1. } else { 0.2 });
            if icon.key == "gem" {
                image.color.set_alpha(0.);
                for element in ["fire", "air", "lightning", "water"] {
                    if matches!(
                        parts
                            .pieces
                            .get(&format!("part.elemental_staff_{element}.gem")),
                        Some(crate::environment::parts::PartState::Owned { .. })
                    ) {
                        let name = if element == "air" { "wind" } else { element };
                        image.image = native::image(
                            &game,
                            &server,
                            &format!("zom_hud_craftable_element_{name}"),
                        )
                        .image;
                        image.color = Color::WHITE;
                        break;
                    }
                }
            }
        } else if let Some(piece) = piece {
            let element = ["fire", "air", "lightning", "water"][piece.element];
            let part = ["upper_staff", "middle_staff", "lower_staff"][piece.piece];
            let key = format!("part.elemental_staff_{element}.{part}");
            let collected = crafting.consumed.contains(&key)
                || matches!(
                    parts.pieces.get(&key),
                    Some(
                        crate::environment::parts::PartState::Owned { .. }
                            | crate::environment::parts::PartState::Consumed
                    )
                );
            let rgb = colors[piece.element];
            image.color = Color::srgba(
                rgb[0],
                rgb[1],
                rgb[2],
                if crafting
                    .crafted
                    .contains(&format!("elemental_staff_{element}"))
                {
                    0.
                } else if collected {
                    1.
                } else {
                    0.2
                },
            );
        } else if let Some(quest) = quest {
            let element = ["fire", "air", "lightning", "water"][quest.0];
            let record = crafting.records.contains(match element {
                "water" => "ice",
                "lightning" => "elec",
                other => other,
            });
            let gem = format!("part.elemental_staff_{element}.gem");
            let owned = crafting.consumed.contains(&gem)
                || matches!(
                    parts.pieces.get(&gem),
                    Some(
                        crate::environment::parts::PartState::Owned { .. }
                            | crate::environment::parts::PartState::Consumed
                    )
                );
            let built = crafting
                .crafted
                .contains(&format!("elemental_staff_{element}"));
            let name = if built {
                "zom_hud_craftable_staff".into()
            } else if owned {
                format!(
                    "zom_hud_craftable_element_{}",
                    if element == "air" { "wind" } else { element }
                )
            } else {
                "zom_hud_craftable_record_blank".into()
            };
            image.image = native::image(&game, &server, &name).image;
            let rgb = if built || owned {
                [1.; 3]
            } else {
                colors[quest.0]
            };
            image.color = Color::srgba(
                rgb[0],
                rgb[1],
                rgb[2],
                if record || owned || built { 1. } else { 0.2 },
            );
        } else if let Some(element) = element {
            image.color.set_alpha(
                if crafting.crafted.contains(&format!(
                    "elemental_staff_{}",
                    ["fire", "air", "lightning", "water"][element.0]
                )) {
                    1.
                } else {
                    0.
                },
            );
        } else if let Some(sector) = sector {
            let active = game
                .content
                .generators
                .iter()
                .find(|g| g.number as usize == sector.0 + 1)
                .is_some_and(|g| generators.powered(&g.id));
            image.color = if active {
                Color::srgba(0., 0.5, 1., 0.3)
            } else {
                Color::srgba(0., 0., 0., 0.25)
            };
        } else if eyes.is_some() {
            image.color.set_alpha(if generators.active_count() == 6 {
                1.
            } else {
                0.
            });
        }
    }
    for (glow, mut visible) in &mut glows {
        *visible = if upgrades.charged[[0, 2, 3, 1][glow.0]] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn preview(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<GameState>>,
    input: Res<crate::input::GameInput>,
    crafting: Res<crate::crafting::Crafting>,
    parts: Res<crate::environment::parts::Parts>,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    players: Query<Entity, With<Player>>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    mut roots: Query<(
        Entity,
        &InventoryPreview,
        &mut Visibility,
        &mut native::Opacity,
    )>,
    mut cache: ResMut<PreviewState>,
) {
    let (Some(game), Ok(owner), Ok(camera)) = (game, players.single(), cameras.single()) else {
        return;
    };
    let mut items = crafting.records.clone();
    if crafting.gramophone {
        items.insert("gramophone".into());
    }
    items.extend(
        parts
            .pieces
            .iter()
            .filter(|(_, part)| matches!(part, crate::environment::parts::PartState::Owned { .. }))
            .map(|(id, _)| id.clone()),
    );
    items.extend(crafting.crafted.iter().cloned());
    if cache.owner != Some(owner) {
        *cache = PreviewState {
            owner: Some(owner),
            items: items.clone(),
            ..default()
        };
    }
    let added: Vec<_> = items.difference(&cache.items).cloned().collect();
    cache.items = items;
    for left in &mut cache.remaining {
        *left = (*left - time.delta_secs()).max(0.);
    }
    let groups: std::collections::BTreeSet<_> = added
        .iter()
        .map(|id| {
            if id == "gramophone" || id == "master" {
                0
            } else if id.contains("dieseldrone") {
                2
            } else if id.contains("shield") {
                3
            } else {
                1
            }
        })
        .collect();
    for group in groups {
        for (root, preview, _, _) in &roots {
            if preview.0 == group {
                commands.entity(root).despawn();
            }
        }
        cache.remaining[group] = 6.;
        let musical = group == 0;
        let drone = group == 2;
        let shield = group == 3;
        commands
            .spawn((
                InventoryPreview(group),
                native::Opacity(0.),
                Canvas,
                super::PlayerOverlay,
                UiTargetCamera(camera),
                GlobalZIndex(45),
                rect(0., 0., 1280., 720.),
                UiTransform::default(),
                Pickable::IGNORE,
            ))
            .with_children(|canvas| {
                canvas
                    .spawn((
                        native::SafeAnchor::left(0.),
                        rect(
                            0.,
                            if musical {
                                0.
                            } else if group == 1 {
                                90.
                            } else {
                                190.
                            },
                            320.,
                            78.,
                        ),
                    ))
                    .with_children(|parent| {
                        label(
                            parent,
                            if musical {
                                "Musical Parts"
                            } else if drone {
                                "Maxis Drone"
                            } else if shield {
                                "Zombie Shield"
                            } else {
                                "Staff Parts"
                            },
                            20.,
                            rect(0., 0., 300., 20.),
                            false,
                        );
                        if musical {
                            slot(
                                parent,
                                &game,
                                &server,
                                "gramophone",
                                "zom_hud_craftable_gramophone",
                                0.,
                                25.,
                                48.,
                            );
                            slot(
                                parent,
                                &game,
                                &server,
                                "master",
                                "zom_hud_craftable_record",
                                53.,
                                25.,
                                48.,
                            );
                            slot(
                                parent,
                                &game,
                                &server,
                                "gem",
                                "zom_hud_craftable_element_fire",
                                106.,
                                25.,
                                48.,
                            );
                        } else if drone || shield {
                            let (recipe, names, pieces) = if drone {
                                (
                                    "equip_dieseldrone_zm",
                                    ["zquad_body", "zquad_brain", "zquad_engine"],
                                    ["body", "brain", "engine"],
                                )
                            } else {
                                (
                                    "tomb_shield_zm",
                                    [
                                        "zshield_tomb_vizor",
                                        "zshield_tomb_body",
                                        "zshield_tomb_feet",
                                    ],
                                    ["top", "door", "bracket"],
                                )
                            };
                            for index in 0..3 {
                                slot(
                                    parent,
                                    &game,
                                    &server,
                                    &format!("part.{recipe}.{}", pieces[index]),
                                    &format!("zom_hud_craftable_{}", names[index]),
                                    index as f32 * 53.,
                                    25.,
                                    48.,
                                );
                            }
                        } else {
                            for (element, _name) in ["fire", "air", "lightning", "water"]
                                .into_iter()
                                .enumerate()
                            {
                                let x = element as f32 * 53.;
                                parent.spawn((
                                    QuestIcon(element),
                                    native::image(&game, &server, "zom_hud_craftable_record_blank"),
                                    rect(x + 1., 32., 34., 34.),
                                ));
                                parent.spawn((
                                    QuestElement(element),
                                    native::image(
                                        &game,
                                        &server,
                                        &format!(
                                            "zom_hud_craftable_element_{}",
                                            ["fire", "wind", "lightning", "water"][element]
                                        ),
                                    ),
                                    rect(x + 23., 27., 20., 20.),
                                ));
                                for piece in 0..3 {
                                    parent.spawn((
                                        StaffPiece { element, piece },
                                        rect(x + 34., 29. + piece as f32 * 14.1, 11., 12.1),
                                        native::image(&game, &server, "zm_hud_icon_staff_pieces"),
                                    ));
                                }
                            }
                        }
                    });
            });
    }
    for (_, group, mut visible, mut opacity) in &mut roots {
        let left = cache.remaining[group.0];
        opacity.0 = (left / 0.5)
            .clamp(0., 1.)
            .min(((6. - left) / 0.5).clamp(0., 1.));
        *visible = if left > 0. && **state == GameState::Playing && !input.scoreboard {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
