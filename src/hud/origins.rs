use super::native::{self, Canvas, NativeText, SafeAnchor, rect};
use crate::{GameState, assets::GameDefinition, gameplay::Session, player::Player};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct HudRoot;
#[derive(Component)]
pub(super) struct ScoreboardRoot;
#[derive(Component, Clone, Copy)]
pub(super) enum Field {
    Round,
    Points,
    Character,
    Weapon,
    Prompt,
    Subtitle,
    Revive,
    Reload,
    Score,
    Kills,
    Downs,
    Headshots,
    GameOver,
    Notice,
}
#[derive(Component)]
struct Ammo;
#[derive(Component)]
struct RoundMarks;
#[derive(Component)]
struct RoundDigit;
#[derive(Component)]
struct RoundTitle;
#[derive(Component)]
struct AmmoDigit(bool);
#[derive(Component)]
struct PerkRow;
#[derive(Component)]
struct Powerup(usize);
#[derive(Component)]
struct Equipment(&'static str, u32);
#[derive(Component)]
struct Charge(u32);
#[derive(Component)]
pub(super) struct GeneratorSector(pub usize);
#[derive(Component)]
struct PromptProgress;
#[derive(Component)]
struct RobotFade;
#[derive(Component)]
struct CrosshairArm(usize);
#[derive(Component)]
struct Hitmarker;
#[derive(Component)]
struct PointChange {
    remaining: f32,
    motion: Vec2,
}
#[derive(Component)]
struct PowerupNotice(f32);
#[derive(Resource, Default)]
struct HudCache {
    player: Option<Entity>,
    round: u32,
    previous_round: u32,
    drawn_round: u32,
    ammo: String,
    perks: Vec<String>,
    points: Option<u32>,
    hit: f32,
    round_age: f32,
    round_first: bool,
    powerups: Vec<usize>,
    weapon: String,
    weapon_age: f32,
}

pub(super) fn text(
    parent: &mut ChildSpawnerCommands,
    field: Field,
    value: &str,
    size: f32,
    node: Node,
) {
    parent.spawn((field, NativeText::new(value, size), node, Pickable::IGNORE));
}

pub struct OriginsHudPlugin;
impl Plugin for OriginsHudPlugin {
    fn build(&self, app: &mut App) {
        native::install(app);
        app.init_resource::<HudCache>().add_systems(
            Update,
            (
                setup,
                update,
                icons,
                combat_feedback,
                powerup_notices,
                round_animation,
                visibility,
                robot_fade,
            )
                .chain(),
        );
        super::scoreboard::install(app);
        super::widgets::install(app);
    }
}

fn setup(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
    roots: Query<(), With<HudRoot>>,
) {
    let (Some(game), Ok(camera)) = (game, cameras.single()) else {
        return;
    };
    if !roots.is_empty() {
        return;
    }
    commands
        .spawn((
            HudRoot,
            Canvas,
            super::PlayerOverlay,
            UiTargetCamera(camera),
            GlobalZIndex(30),
            rect(0., 0., 1280., 720.),
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((RoundMarks, SafeAnchor::left(0.), rect(0., 624., 192., 96.)));
            parent.spawn((
                Field::Round,
                RoundDigit,
                SafeAnchor::left(0.),
                NativeText {
                    style: "extraBig",
                    ..NativeText::new("", 96.)
                },
                rect(0., 624., 192., 96.),
            ));
            parent.spawn((
                RoundTitle,
                NativeText {
                    style: "extraBig",
                    ..NativeText::new("Round", 80.).centered()
                },
                rect(400., 216., 480., 80.),
            ));
            parent.spawn((
                ImageNode {
                    color: Color::srgb(0.21, 0., 0.),
                    ..native::image(&game, &server, "scorebar_zom_1")
                },
                SafeAnchor::right(1092.),
                rect(1092., 550., 158., 40.),
            ));
            parent.spawn((
                Field::Character,
                SafeAnchor::right(950.),
                NativeText {
                    align: Justify::Right,
                    ..NativeText::new("", 25.)
                },
                rect(950., 557.5, 140., 25.),
            ));
            parent.spawn((
                Field::Points,
                SafeAnchor::right(1102.),
                NativeText::new("", 40.),
                rect(1102., 550., 140., 40.),
            ));
            parent.spawn((
                Field::Weapon,
                NativeText {
                    align: Justify::Right,
                    ..NativeText::new("", 25.)
                },
                SafeAnchor::right(768.),
                rect(768., 598.5, 399., 25.),
            ));
            parent.spawn((
                Ammo,
                SafeAnchor::right(1152.),
                rect(1152., 577., 128., 128.),
            ));
            parent.spawn((
                native::image(&game, &server, "hud_tomb_dpad_bar"),
                SafeAnchor::right(960.),
                rect(960., 673., 256., 16.),
            ));
            parent.spawn((
                native::image(&game, &server, "hud_tomb_dpad"),
                SafeAnchor::right(1152.),
                rect(1152., 577., 128., 128.),
            ));
            parent.spawn((
                PerkRow,
                SafeAnchor::left(10.),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(10.),
                    top: px(535.),
                    column_gap: px(8.),
                    height: px(36.),
                    ..default()
                },
            ));
            for (index, name) in [
                "specialty_2x_zombies",
                "specialty_instakill_zombies",
                "specialty_zomblood_zombies",
                "specialty_firesale_zombies",
            ]
            .into_iter()
            .enumerate()
            {
                parent.spawn((
                    Powerup(index),
                    native::image(&game, &server, name),
                    rect(542. + index as f32 * 56., 672., 48., 48.),
                    Visibility::Hidden,
                ));
            }
            for (key, name, x, y, width, height) in [
                ("lethal", "hud_us_grenade", 1207., 692., 24., 24.),
                ("tactical", "hud_cymbal_monkey", 1099., 692., 24., 24.),
                ("mine", "hud_claymore_32", 1174., 625., 32., 32.),
                (
                    "shield",
                    "zom_hud_craftable_zshield_tomb_body",
                    1237.,
                    625.,
                    32.,
                    32.,
                ),
                ("drone", "hud_quadrotor_tomb", 1200., 584., 32., 32.),
                (
                    "shovel",
                    "zom_hud_craftable_tank_shovel",
                    1220.,
                    555.,
                    30.,
                    30.,
                ),
                ("helmet", "zom_hud_helmet_gold", 1250., 555., 30., 30.),
                ("tablet", "zm_hud_icon_oneinch_dirty", 0., 574., 50., 50.),
            ] {
                let count = match key {
                    "lethal" => 4,
                    "tactical" => 3,
                    "mine" => 2,
                    _ => 1,
                };
                for index in 0..count {
                    parent.spawn((
                        Equipment(key, index),
                        SafeAnchor {
                            x: x - index as f32 * width,
                            side: if key == "tablet" { -1. } else { 1. },
                        },
                        native::image(&game, &server, name),
                        rect(x - index as f32 * width, y, width, height),
                        Visibility::Hidden,
                    ));
                }
            }
            for level in 1..=3 {
                parent.spawn((
                    Charge(level),
                    SafeAnchor::right(1206. + level as f32 * 7.),
                    rect(1206. + level as f32 * 7., 663., 4., 8.),
                    BackgroundColor(Color::NONE),
                ));
            }
            parent.spawn((
                Field::Prompt,
                NativeText::new("", 24.).centered(),
                rect(140., 493., 1000., 70.),
            ));
            parent.spawn((
                PromptProgress,
                rect(510., 560., 0., 3.),
                BackgroundColor(Color::WHITE),
            ));
            parent.spawn((
                Field::Subtitle,
                NativeText::new("", 22.).centered(),
                rect(160., 580., 960., 72.),
            ));
            parent.spawn((
                Field::Revive,
                NativeText::new("", 28.).centered(),
                rect(380., 452., 520., 38.),
            ));
            parent.spawn((
                Field::Reload,
                NativeText::new("", 22.).centered(),
                rect(480., 416., 320., 32.),
            ));
            parent.spawn((
                Field::Notice,
                NativeText::new("", 24.).centered(),
                rect(260., 116., 760., 62.),
            ));
            parent
                .spawn((super::Crosshair, rect(640., 360., 0., 0.)))
                .with_children(|cross| {
                    for index in 0..5 {
                        cross.spawn((
                            CrosshairArm(index),
                            rect(0., 0., 0., 0.),
                            ImageNode::default(),
                            UiTransform::default(),
                            Visibility::Hidden,
                        ));
                    }
                });
            parent.spawn((
                Hitmarker,
                native::image(&game, &server, "damage_feedback"),
                rect(625., 345., 30., 30.),
                Visibility::Hidden,
            ));
        });
    commands.spawn((
        RobotFade,
        super::PlayerOverlay,
        UiTargetCamera(camera),
        GlobalZIndex(150),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

fn update(
    state: Res<State<GameState>>,
    session: Res<Session>,
    game: Option<Res<GameDefinition>>,
    focus: Res<crate::interactions::InteractionFocus>,
    settings: Res<crate::input::ControlSettings>,
    (subtitle, audio, buildables): (
        Res<crate::audio::Subtitle>,
        Res<crate::audio::AudioSettings>,
        Res<crate::buildables::BuildableState>,
    ),
    raids: Res<crate::generators::GeneratorRaids>,
    players: Query<
        (
            &crate::weapons::WeaponInventory,
            &crate::equipment::GrenadeInventory,
            &crate::equipment::TacticalInventory,
            &crate::mines::MineInventory,
            &crate::crafting::BuildableInventory,
            &crate::environment::digging::Digger,
            &crate::tablets::Tablet,
            Option<&crate::solo_revive::SoloEpisode>,
        ),
        With<Player>,
    >,
    mut labels: Query<(&Field, &mut NativeText)>,
    mut equipment: Query<(&Equipment, &mut ImageNode, &mut Visibility)>,
    server: Res<AssetServer>,
    mut charges: Query<(&Charge, &mut BackgroundColor)>,
    mut progress: Query<&mut Node, With<PromptProgress>>,
) {
    let (Some(game), Ok((inventory, lethal, tactical, mines, buildable, dig, tablet, episode))) =
        (game, players.single())
    else {
        return;
    };
    let weapon = inventory.definition(&game.content.weapons);
    let held = inventory.active();
    let notice = raids
        .target
        .as_ref()
        .filter(|_| raids.under_attack)
        .and_then(|id| game.content.generators.iter().find(|g| g.id == *id))
        .map(|g| format!("Generator {} is under attack!", g.number))
        .unwrap_or_default();
    for (field, mut label) in &mut labels {
        let value = match field {
            Field::Round => {
                if session.round > 10 {
                    session.round.to_string()
                } else {
                    String::new()
                }
            }
            Field::Points => session.points.to_string(),
            Field::Character => {
                ["Dempsey", "Nikolai", "Richtofen", "Takeo"][session.character.min(3)].into()
            }
            Field::Weapon => weapon.map_or("", |w| w.name.as_str()).to_uppercase(),
            Field::Prompt => {
                let mut prompt = focus.prompt.clone();
                if let Some(reason) = focus.rejection {
                    let message = match reason {
                        crate::interactions::InteractionRejection::InsufficientPoints => {
                            "Not enough points"
                        }
                        crate::interactions::InteractionRejection::FullAmmo => "Ammo full",
                        crate::interactions::InteractionRejection::NoPower => {
                            "You must turn on the power first!"
                        }
                        crate::interactions::InteractionRejection::PerkLimit => {
                            "You can only carry four perks"
                        }
                        _ => "",
                    };
                    if !message.is_empty() {
                        prompt = message.into();
                    }
                }
                prompt
            }
            Field::Subtitle => {
                if audio.subtitles {
                    subtitle.text.clone()
                } else {
                    String::new()
                }
            }
            Field::Revive => episode.map_or(String::new(), |_| "Reviving".into()),
            Field::Reload => {
                if held.is_some_and(|w| w.magazine <= 2)
                    && inventory.action == crate::weapons::WeaponAction::Ready
                    && weapon.is_some_and(|w| inventory.reserve(w) > 0)
                {
                    format!("{} Reload", settings.label(crate::input::Action::Reload))
                } else {
                    String::new()
                }
            }
            Field::Notice => notice.clone(),
            _ => continue,
        };
        if label.value != value {
            label.value = value;
        }
        if matches!(field, Field::Points | Field::Character) {
            label.color = Color::WHITE;
        }
    }
    for (slot, mut icon, mut visible) in &mut equipment {
        let (show, name) = match slot.0 {
            "lethal" => (
                true,
                if lethal.definition.0.contains("sticky") {
                    "hud_sticky_grenade_32"
                } else {
                    "hud_us_grenade"
                },
            ),
            "tactical" => (
                tactical.0.is_some(),
                if tactical
                    .0
                    .as_ref()
                    .is_some_and(|t| t.definition.0.contains("beacon"))
                {
                    "hud_homing_beacon"
                } else {
                    "hud_cymbal_monkey"
                },
            ),
            "mine" => (mines.definition.is_some(), "hud_claymore_32"),
            "shield" => (
                buildable.owned.contains("tomb_shield_zm") && buildable.shield_health > 0,
                "zom_hud_craftable_zshield_tomb_body",
            ),
            "drone" => (
                buildable.owned.contains("equip_dieseldrone_zm"),
                "hud_quadrotor_tomb",
            ),
            "shovel" => (
                dig.shovel,
                if dig.golden {
                    "zom_hud_shovel_gold"
                } else {
                    "zom_hud_craftable_tank_shovel"
                },
            ),
            "helmet" => (dig.helmet, "zom_hud_helmet_gold"),
            "tablet" => (
                tablet.stage > 0 && tablet.stage < 4,
                if tablet.stage >= 2 && !tablet.reclean {
                    "zm_hud_icon_oneinch_clean"
                } else {
                    "zm_hud_icon_oneinch_dirty"
                },
            ),
            _ => (false, ""),
        };
        *visible = if show
            && (!matches!(slot.0, "lethal" | "tactical" | "mine")
                || slot.1
                    < match slot.0 {
                        "lethal" => lethal.count,
                        "tactical" => tactical.0.as_ref().map_or(0, |t| t.count),
                        _ => mines.count,
                    })
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let image = native::image(&game, &server, name).image;
        if icon.image != image {
            icon.image = image;
        }
        let count = match slot.0 {
            "lethal" => lethal.count,
            "tactical" => tactical.0.as_ref().map_or(0, |t| t.count),
            "mine" => mines.count,
            "drone" => u32::from(
                !buildables.drone_disabled
                    && buildables.drone.is_none()
                    && buildables.drone_cooldown <= 0.,
            ),
            _ => 1,
        };
        let alpha = if matches!(slot.0, "lethal" | "tactical") {
            0.75_f32.powi(count.saturating_sub(slot.1 + 1) as i32)
        } else if slot.1 < count {
            1.
        } else {
            0.6
        };
        icon.color = Color::srgba(1., 1., 1., alpha);
    }
    for (level, mut color) in &mut charges {
        color.0 = if inventory.charge_level >= level.0 {
            Color::srgb(0.3, 0.85, 1.)
        } else {
            Color::NONE
        };
    }
    for mut node in &mut progress {
        node.width = px(
            if **state == GameState::Playing && focus.hold_seconds > 0. {
                260. * (focus.progress / focus.hold_seconds).clamp(0., 1.)
            } else {
                0.
            },
        );
    }
}

fn icons(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    session: Res<Session>,
    state: Res<State<GameState>>,
    players: Query<
        (
            Entity,
            &crate::weapons::WeaponInventory,
            &crate::solo_revive::PerkInventory,
        ),
        With<Player>,
    >,
    roots: Query<
        (
            Entity,
            Option<&Children>,
            Option<&Ammo>,
            Option<&RoundMarks>,
            Option<&PerkRow>,
        ),
        Or<(With<Ammo>, With<RoundMarks>, With<PerkRow>)>,
    >,
    mut cache: ResMut<HudCache>,
    effects: Res<crate::powerups::PowerupEffects>,
    time: Res<Time>,
    mut powerups: Query<(&Powerup, &mut Visibility, &mut ImageNode, &mut Node)>,
    mut digits: Query<(&AmmoDigit, &mut ImageNode), Without<Powerup>>,
) {
    if **state != GameState::Playing {
        return;
    }
    let (Some(game), Ok((owner, inventory, perks))) = (game, players.single()) else {
        return;
    };
    if cache.player != Some(owner) {
        *cache = HudCache {
            player: Some(owner),
            ..default()
        };
    }
    let weapon = inventory.definition(&game.content.weapons);
    let ammo = inventory.active().map_or(String::new(), |held| {
        let magazine = held.left_magazine.map_or_else(
            || held.magazine.to_string(),
            |left| format!("{left}|{}", held.magazine),
        );
        format!("{magazine}/{}", weapon.map_or(0, |w| inventory.reserve(w)))
    });
    let new_round = session.round != cache.round;
    if new_round {
        cache.previous_round = cache.round;
        cache.round_first = cache.round == 0;
        cache.round_age = 0.;
    } else {
        cache.round_age += time.delta_secs();
    }
    let mut perk_ids = cache.perks.clone();
    let pulses = (2. + (1. - session.round.min(100) as f32 / 100.) * 5.).ceil();
    let display_round = if cache.round_first || cache.round_age >= pulses + 1. {
        session.round
    } else {
        cache.previous_round
    };
    perk_ids.retain(|id| perks.owned.iter().any(|perk| perk.0 == *id));
    for perk in &perks.owned {
        if !perk_ids.contains(&perk.0) {
            perk_ids.push(perk.0.clone());
        }
    }
    for (root, children, is_ammo, round, perk_row) in &roots {
        let changed = if is_ammo.is_some() {
            ammo != cache.ammo
        } else if round.is_some() {
            display_round != cache.drawn_round
        } else if perk_row.is_some() {
            perk_ids != cache.perks
        } else {
            false
        };
        if !changed {
            continue;
        }
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(root).with_children(|parent| {
            if is_ammo.is_some() {
                let mut x = 20.;
                let mut small = true;
                for c in ammo.chars().rev() {
                    let stock = small;
                    if c == '/' {
                        small = false;
                    }
                    let (name, width) = match c {
                        '/' => ("hud_tomb_zom_num_slash".into(), 24.),
                        '|' => ("hud_tomb_zom_num_line".into(), 32.),
                        _ => (
                            format!("hud_tomb_zom_num_{c}"),
                            if stock { 24. } else { 32. },
                        ),
                    };
                    parent.spawn((
                        AmmoDigit(!stock && c != '|'),
                        native::image(&game, &server, &name),
                        rect(
                            x - width,
                            if stock { 53. } else { 41. },
                            width,
                            if stock { 48. } else { 64. },
                        ),
                    ));
                    x -= if stock { 16.5 } else { 22. };
                }
            } else if round.is_some() && display_round <= 10 {
                for (index, count) in [display_round.min(5), display_round.saturating_sub(5)]
                    .into_iter()
                    .enumerate()
                    .filter(|(_, count)| *count > 0)
                {
                    parent.spawn((
                        native::image(&game, &server, &format!("chalkmarks_hellcatraz_{count}")),
                        rect(index as f32 * 96., 0., 96., 96.),
                    ));
                }
            } else if perk_row.is_some() {
                for id in &perk_ids {
                    if let Some(asset) = game
                        .content
                        .perks
                        .iter()
                        .find(|p| p.id.0 == *id)
                        .and_then(|p| p.icon.as_ref())
                        .and_then(|id| game.content.ui_assets.iter().find(|a| a.id == *id))
                    {
                        parent.spawn((
                            ImageNode::new(server.load(asset.path.clone())),
                            Node {
                                width: px(36.),
                                height: px(36.),
                                ..default()
                            },
                        ));
                    }
                }
            }
        });
    }
    cache.ammo = ammo;
    cache.round = session.round;
    cache.drawn_round = display_round;
    cache.perks = perk_ids;
    let durations = [
        effects.double_points,
        effects.insta_kill,
        effects.zombie_blood,
        effects.fire_sale,
    ];
    let active = durations.iter().filter(|s| **s > 0.).count();
    cache.powerups.retain(|index| durations[*index] > 0.);
    for (index, seconds) in durations.iter().enumerate() {
        if *seconds > 0. && !cache.powerups.contains(&index) {
            cache.powerups.push(index);
        }
    }
    for (index, mut visible, mut image, mut node) in &mut powerups {
        let seconds = durations[index.0];
        let preceding = cache
            .powerups
            .iter()
            .position(|i| *i == index.0)
            .unwrap_or(0);
        node.left = px(640. - (active as f32 * 56. - 8.) * 0.5 + preceding as f32 * 56.);
        *visible = if seconds > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        image.color = Color::srgba(
            1.,
            1.,
            1.,
            if seconds < 5. {
                (time.elapsed_secs() % 1. - 0.5).abs() * 2.
            } else {
                1.
            },
        );
    }
    let low = inventory.active().is_some_and(|w| {
        weapon.is_some_and(|d| {
            w.magazine as f32
                <= d.magazine as f32 * d.hud.as_ref().map_or(0.25, |h| h.low_ammo_fraction)
        })
    });
    for (digit, mut image) in &mut digits {
        let red = if digit.0 && low {
            1. - (time.elapsed_secs() % 1. - 0.5).abs() * 2.
        } else {
            0.
        };
        image.color = Color::srgb(1., 1. - red, 1. - red);
    }
}

fn round_animation(
    cache: Res<HudCache>,
    mut roots: Query<(&mut Node, &mut SafeAnchor), (With<RoundMarks>, Without<RoundDigit>)>,
    mut numbers: Query<
        (&mut Node, &mut SafeAnchor, &mut NativeText),
        (With<RoundDigit>, Without<RoundMarks>),
    >,
    mut title: Query<&mut NativeText, (With<RoundTitle>, Without<RoundDigit>)>,
    children: Query<&Children, With<RoundMarks>>,
    mut chalks: Query<&mut ImageNode, Without<AmmoDigit>>,
) {
    let age = cache.round_age;
    let fall = if cache.round_first {
        ((age - 4.) / 2.).clamp(0., 1.)
    } else {
        1.
    };
    let start_x = if cache.round == 1 { 625. } else { 592. };
    let x = start_x * (1. - fall);
    let y = 312. + (624. - 312.) * fall;
    let pulses = (2. + (1. - cache.round.min(100) as f32 / 100.) * 5.).ceil();
    let switching = !cache.round_first && age < pulses + 2.;
    let alpha = if switching && age < pulses {
        (age % 1. - 0.5).abs() * 2.
    } else if switching && age < pulses + 1. {
        1. - (age - pulses)
    } else if switching {
        age - pulses - 1.
    } else {
        1.
    };
    let rgb = if switching && age < pulses {
        [1.; 3]
    } else {
        [0.21, 0., 0.]
    };
    for (mut node, mut anchor) in &mut roots {
        node.top = px(y);
        anchor.x = x;
    }
    for (mut node, mut anchor, mut text) in &mut numbers {
        node.top = px(y);
        anchor.x = x;
        text.color = Color::srgba(rgb[0], rgb[1], rgb[2], alpha);
        text.value = if cache.drawn_round > 10 {
            cache.drawn_round.to_string()
        } else {
            String::new()
        };
    }
    for mut text in &mut title {
        text.color = Color::srgba(
            0.21,
            0.,
            0.,
            if cache.round_first {
                (5. - age).clamp(0., 1.)
            } else {
                0.
            },
        );
    }
    for children in &children {
        for child in children.iter() {
            if let Ok(mut image) = chalks.get_mut(child) {
                image.color = Color::srgba(rgb[0], rgb[1], rgb[2], alpha);
            }
        }
    }
}

fn combat_feedback(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<Session>,
    state: Res<State<GameState>>,
    mut cache: ResMut<HudCache>,
    mut hits: MessageReader<crate::damage::DamageApplied>,
    players: Query<(
        Entity,
        &Player,
        &crate::weapons::WeaponInventory,
        &crate::life::Vitality,
    )>,
    roots: Query<Entity, With<HudRoot>>,
    mut crosshair: Query<
        &mut Visibility,
        (
            With<super::Crosshair>,
            Without<Hitmarker>,
            Without<CrosshairArm>,
        ),
    >,
    mut markers: Query<
        &mut Visibility,
        (
            With<Hitmarker>,
            Without<super::Crosshair>,
            Without<CrosshairArm>,
        ),
    >,
    mut arms: Query<
        (
            &CrosshairArm,
            &mut Node,
            &mut ImageNode,
            &mut UiTransform,
            &mut Visibility,
        ),
        (Without<super::Crosshair>, Without<Hitmarker>),
    >,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    cameras: Query<&Projection, With<crate::rendering::GameCamera>>,
    mut weapon_labels: Query<(&Field, &mut NativeText), Without<PointChange>>,
    mut changes: Query<
        (Entity, &mut PointChange, &mut NativeText, &mut Node),
        Without<CrosshairArm>,
    >,
) {
    cache.hit = (cache.hit - time.delta_secs()).max(0.);
    if let Ok((owner, player, inventory, health)) = players.single() {
        for event in hits.read() {
            if event.impact.attacker == owner && event.impact.target != owner {
                cache.hit = 0.12;
            }
        }
        let weapon = game
            .as_ref()
            .and_then(|g| inventory.definition(&g.content.weapons));
        let hud = weapon.and_then(|w| w.hud.as_ref());
        let show = **state == GameState::Playing
            && (inventory.ads < 0.05 || hud.is_some_and(|h| h.keep_when_ads))
            && hud.is_some_and(|h| h.center.is_some() || h.side.is_some())
            && inventory.action == crate::weapons::WeaponAction::Ready
            && health.state() == crate::life::LifeState::Standing;
        for mut visible in &mut crosshair {
            *visible = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        let spread = weapon.map_or(0., |w| {
            let stance = if player.prone {
                2
            } else if player.crouched {
                1
            } else {
                0
            };
            let hip = w.mechanics.spread[stance]
                * inventory.firing.hip_spread_scale
                * (1. + player.velocity.xz().length().min(6.) / 6.);
            (hip + (w.mechanics.spread[3] - hip) * inventory.ads) * inventory.attachment_spread()
        });
        let fov = cameras
            .single()
            .ok()
            .and_then(|p| match p {
                Projection::Perspective(p) => Some(p.fov),
                _ => None,
            })
            .unwrap_or(55_f32.to_radians());
        for (arm, mut node, mut image, mut transform, mut visible) in &mut arms {
            let asset = hud.and_then(|h| {
                if arm.0 == 4 {
                    h.center.as_ref()
                } else {
                    h.side.as_ref()
                }
            });
            *visible = if asset.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let (Some(game), Some(h), Some(asset)) = (game.as_ref(), hud, asset) {
                image.image = native::image(game, &server, asset.0.trim_start_matches("ui.")).image;
                let size = if arm.0 == 4 {
                    h.center_size
                } else {
                    h.side_size
                } * 1.5;
                let gap = (360. * spread.to_radians().tan() / (fov * 0.5).tan())
                    .max(h.minimum_offset * 1.5)
                    - size * h.side_position;
                let half = size * 0.5;
                let extra = size * 0.125;
                let (x, y) = match arm.0 {
                    0 => (-half, -size - gap - extra),
                    1 => (gap + extra, -half + extra),
                    2 => (-half - extra, gap + extra),
                    3 => (-size - gap - extra, -half),
                    _ => (-half, -half),
                };
                *node = rect(x, y, size, size);
                transform.rotation = Rot2::radians(if arm.0 == 4 {
                    0.
                } else {
                    arm.0 as f32 * std::f32::consts::FRAC_PI_2
                });
            }
        }
        let selected = inventory.active().map_or("", |w| w.definition.0.as_str());
        if selected != cache.weapon {
            cache.weapon = selected.into();
            cache.weapon_age = 0.;
        } else if **state == GameState::Playing {
            cache.weapon_age += time.delta_secs();
        }
        for (field, mut label) in &mut weapon_labels {
            if matches!(field, Field::Weapon) {
                label
                    .color
                    .set_alpha((1. - cache.weapon_age / 2.).clamp(0., 1.));
            }
        }
    } else {
        hits.clear();
    }
    for mut visible in &mut markers {
        *visible = if cache.hit > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if let Some(previous) = cache.points
        && previous != session.points
        && **state == GameState::Playing
    {
        let delta = i64::from(session.points) - i64::from(previous);
        if let Ok(root) = roots.single() {
            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    PointChange {
                        remaining: 0.9,
                        motion: Vec2::new(-110., (session.points % 101) as f32),
                    },
                    NativeText::new(format!("{delta:+}"), 23.).color(if delta > 0 {
                        Color::srgb(0.9, 0.9, 0.)
                    } else {
                        Color::srgb(0.21, 0., 0.)
                    }),
                    rect(1102., 550., 120., 30.),
                ));
            });
        }
    }
    cache.points = Some(session.points);
    for (entity, mut change, mut text, mut node) in &mut changes {
        change.remaining -= time.delta_secs();
        node.left = px(1102. + change.motion.x * (1. - change.remaining / 0.9));
        node.top = px(550. + change.motion.y * (1. - change.remaining / 0.9));
        text.color.set_alpha((change.remaining / 0.3).clamp(0., 1.));
        if change.remaining <= 0. || **state != GameState::Playing {
            commands.entity(entity).despawn();
        }
    }
}

fn powerup_notices(
    mut commands: Commands,
    mut pickups: MessageReader<crate::powerups::PowerupCollected>,
    time: Res<Time>,
    state: Res<State<GameState>>,
    roots: Query<Entity, With<HudRoot>>,
    mut notices: Query<(Entity, &mut PowerupNotice, &mut NativeText)>,
) {
    for (entity, mut notice, mut text) in &mut notices {
        notice.0 -= time.delta_secs();
        if notice.0 <= 0. || **state != GameState::Playing {
            commands.entity(entity).despawn();
        } else {
            text.color.set_alpha((notice.0 / 0.35).clamp(0., 1.));
        }
    }
    for pickup in pickups.read() {
        let message = match pickup.0.0.as_str() {
            "powerup.full_ammo" => "Max Ammo!",
            "powerup.nuke" => "Kaboom!",
            "powerup.double_points" => "Double Points!",
            "powerup.insta_kill" => "Insta-Kill!",
            "powerup.zombie_blood" => "Zombie Blood!",
            "powerup.fire_sale" => "Fire Sale!",
            "powerup.random_perk" => "Random Perk Bottle!",
            "powerup.bonus_points_player" | "powerup.bonus_points_team" => "Blood Money!",
            _ => continue,
        };
        for (entity, _, _) in &notices {
            commands.entity(entity).despawn();
        }
        if let Ok(root) = roots.single() {
            commands.entity(root).with_children(|parent| {
                parent.spawn((
                    PowerupNotice(3.),
                    NativeText::new(message, 32.).centered(),
                    rect(400., 655., 480., 44.),
                ));
            });
        }
    }
}

fn visibility(
    state: Res<State<GameState>>,
    input: Res<crate::input::GameInput>,
    mut roots: Query<&mut Visibility, With<HudRoot>>,
) {
    for mut visible in &mut roots {
        *visible = if **state == GameState::Playing && !input.scoreboard {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn robot_fade(
    state: Res<State<GameState>>,
    players: Query<
        (
            Option<&crate::robots::RobotTransit>,
            Option<&crate::robots::StompReaction>,
            Option<&crate::tank::TankRescue>,
            Option<&crate::finale::FistPickupFlash>,
        ),
        With<Player>,
    >,
    mut overlay: Query<(&mut BackgroundColor, &mut Visibility), With<RobotFade>>,
) {
    let fade = players
        .single()
        .ok()
        .and_then(|(transit, stomp, tank, fist)| {
            [
                transit.map(|t| (t.fade, t.black)),
                stomp.map(|s| (s.fade, s.black)),
                tank.map(|t| (t.fade, true)),
                fist.map(|f| (f.fade, false)),
            ]
            .into_iter()
            .flatten()
            .max_by(|a, b| a.0.total_cmp(&b.0))
        });
    for (mut color, mut visible) in &mut overlay {
        let (opacity, black) = fade.unwrap_or((0., true));
        *visible = if **state == GameState::Playing && opacity > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let value = if black { 0. } else { 1. };
        color.0 = Color::srgba(value, value, value, opacity.clamp(0., 1.));
    }
}
