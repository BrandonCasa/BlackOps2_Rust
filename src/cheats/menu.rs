use super::{Cheats, Gun, physics, tools};
use crate::{GameState, player::Player};
use avian3d::prelude::*;
use bevy::{
    input_focus::{
        AutoFocus, FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    },
    picking::hover::Hovered,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::Pressed,
    ui_widgets::{
        Activate, Button, ControlOrientation, ScrollArea, ScrollIntoView, Scrollbar,
        ScrollbarThumb, TextInput,
    },
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Category {
    #[default]
    Cheats,
    Weapons,
    Spawns,
}
#[derive(Clone)]
enum Action {
    God,
    Noclip,
    Ammo,
    Sprint,
    OneHit,
    FreezeEnemies,
    FreezeRounds,
    Points,
    Heal,
    Refill,
    Perks,
    Power,
    Zones,
    Kill,
    NextRound,
    SpawnPoint,
    ToolGun,
    PhysicsGun,
    NormalGun,
    Undo,
    Weapon(String),
    Spawn(tools::Spawn),
}
#[derive(Component, Clone)]
struct Choice(Click);
#[derive(Clone)]
enum Click {
    Action(Action),
    Category(Category),
    Close,
}
#[derive(Resource, Default)]
struct Menu {
    category: Category,
    pending: Option<Click>,
}
#[derive(Component)]
struct MenuRoot;
#[derive(Component)]
struct Row {
    index: usize,
    search: String,
}
#[derive(Component)]
struct Search;
#[derive(Component)]
struct Badge(Click);
#[derive(Component)]
struct Notice;
#[derive(Component)]
struct ResultCount;
#[derive(Component)]
struct NoResults;

impl Action {
    fn enabled(&self, cheats: &Cheats) -> Option<bool> {
        Some(match self {
            Self::God => cheats.god,
            Self::Noclip => cheats.noclip,
            Self::Ammo => cheats.ammo,
            Self::Sprint => cheats.sprint,
            Self::OneHit => cheats.one_hit,
            Self::FreezeEnemies => cheats.freeze_enemies,
            Self::FreezeRounds => cheats.freeze_rounds,
            _ => return None,
        })
    }
}
impl Category {
    fn title(self) -> &'static str {
        match self {
            Self::Cheats => "Player & world",
            Self::Weapons => "Weapons",
            Self::Spawns => "Tool Gun spawns",
        }
    }
}

pub(super) fn install(app: &mut App) {
    if !app.is_plugin_added::<TabNavigationPlugin>() {
        app.add_plugins(TabNavigationPlugin);
    }
    app.init_resource::<Menu>()
        .init_resource::<InputFocus>()
        .add_observer(click)
        .add_systems(
            Update,
            (
                keyboard,
                apply,
                rebuild,
                filter,
                refresh,
                styles,
                reveal_focus,
            )
                .chain()
                .before(crate::input::menu::navigate)
                .before(crate::input::pause_and_restart),
        )
        .add_systems(OnExit(GameState::Paused), close);
}

fn choices(game: &crate::assets::GameDefinition, category: Category) -> Vec<(String, Action)> {
    match category {
        Category::Cheats => vec![
            ("God mode".into(), Action::God),
            ("Noclip / fly".into(), Action::Noclip),
            ("Unlimited ammo".into(), Action::Ammo),
            ("Unlimited sprint".into(), Action::Sprint),
            ("One hit kills".into(), Action::OneHit),
            ("Freeze enemy AI".into(), Action::FreezeEnemies),
            ("Freeze round spawning".into(), Action::FreezeRounds),
            ("Give 10,000 points".into(), Action::Points),
            ("Heal player".into(), Action::Heal),
            ("Refill ammunition".into(), Action::Refill),
            ("Give all perks".into(), Action::Perks),
            ("Power all generators".into(), Action::Power),
            ("Unlock all zones / doors".into(), Action::Zones),
            ("Kill all enemies".into(), Action::Kill),
            ("Finish current round".into(), Action::NextRound),
            ("Teleport to player spawn".into(), Action::SpawnPoint),
            ("Give / equip Tool Gun (F2)".into(), Action::ToolGun),
            ("Give / equip Physics Gun (F3)".into(), Action::PhysicsGun),
            ("Equip normal weapon (F4)".into(), Action::NormalGun),
            ("Undo last Tool Gun spawn".into(), Action::Undo),
        ],
        Category::Weapons => game
            .content
            .weapons
            .iter()
            .filter(|weapon| {
                weapon.presentation.is_some()
                    && weapon.throwable.is_none()
                    && weapon.mine.is_none()
                    && weapon.magazine > 0
            })
            .map(|weapon| (weapon.name.clone(), Action::Weapon(weapon.id.0.clone())))
            .collect(),
        Category::Spawns => std::iter::once((
            "Mystery Box".into(),
            Action::Spawn(tools::Spawn::MysteryBox),
        ))
        .chain(
            game.content
                .powerups
                .iter()
                .filter(|p| p.presentation.is_some())
                .filter(|p| {
                    matches!(
                        p.id.0.as_str(),
                        "powerup.nuke"
                            | "powerup.full_ammo"
                            | "powerup.insta_kill"
                            | "powerup.double_points"
                            | "powerup.fire_sale"
                            | "powerup.zombie_blood"
                    )
                })
                .map(|p| {
                    (
                        p.id.0.trim_start_matches("powerup.").replace('_', " "),
                        Action::Spawn(tools::Spawn::Powerup(p.id.0.clone())),
                    )
                }),
        )
        .chain(
            game.content
                .enemies
                .iter()
                .filter(|enemy| enemy.presentation.is_some())
                .map(|enemy| {
                    (
                        enemy.id.0.trim_start_matches("enemy.").replace('_', " "),
                        Action::Spawn(tools::Spawn::Enemy(enemy.id.0.clone())),
                    )
                }),
        )
        .collect(),
    }
}

fn keyboard(
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut cheats: ResMut<Cheats>,
    mut menu: ResMut<Menu>,
    mut input: ResMut<crate::input::GameInput>,
    mut focus: ResMut<InputFocus>,
    rows: Query<(Entity, &Row, &Node)>,
    search: Query<Entity, With<Search>>,
    #[cfg(feature = "dev-tools")] validator: Option<Res<crate::validation::Controller>>,
) {
    #[cfg(feature = "dev-tools")]
    if validator.is_some_and(|controller| controller.owns_input()) {
        return;
    }
    if !matches!(**state, GameState::Playing | GameState::Paused) {
        return;
    }
    if keys.just_pressed(KeyCode::F1) {
        if cheats.menu_open {
            menu.pending = Some(Click::Close);
        } else {
            cheats.return_to_pause = **state == GameState::Paused;
            cheats.menu_open = true;
            *menu = default();
            input.clear();
            focus.clear();
            if **state == GameState::Playing {
                next.set(GameState::Paused);
            }
        }
    }
    if !cheats.menu_open {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        keys.clear_just_pressed(KeyCode::Escape);
        menu.pending = Some(Click::Close);
        return;
    }
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if control {
        for (key, category) in [
            (KeyCode::Digit1, Category::Cheats),
            (KeyCode::Digit2, Category::Weapons),
            (KeyCode::Digit3, Category::Spawns),
        ] {
            if keys.just_pressed(key) {
                menu.pending = Some(Click::Category(category));
            }
        }
        if keys.just_pressed(KeyCode::KeyF)
            && let Ok(entity) = search.single()
        {
            focus.set(entity, FocusCause::Navigated);
        }
    }
    // Native buttons own Enter/Space and Tab/Shift+Tab. List navigation also
    // works from the search field, without rebuilding or losing pointer targets.
    if !keys.any_just_pressed([
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Enter,
    ]) {
        return;
    }
    let mut visible: Vec<_> = rows
        .iter()
        .filter(|(_, _, node)| node.display != Display::None)
        .map(|(entity, row, _)| (row.index, entity))
        .collect();
    if visible.is_empty() {
        return;
    }
    visible.sort_unstable_by_key(|row| row.0);
    let current = visible
        .iter()
        .position(|(_, entity)| Some(*entity) == focus.get());
    let searching = focus.get().is_some_and(|entity| search.contains(entity));
    if searching && keys.just_pressed(KeyCode::Enter) {
        commands.trigger(Activate {
            entity: visible[0].1,
        });
        return;
    }
    if searching && !keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::ArrowDown]) {
        return;
    }
    let index = if keys.just_pressed(KeyCode::Home) {
        0
    } else if keys.just_pressed(KeyCode::End) {
        visible.len() - 1
    } else if keys.just_pressed(KeyCode::ArrowUp) {
        current.map_or(visible.len() - 1, |i| i.saturating_sub(1))
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        current.map_or(0, |i| (i + 1).min(visible.len() - 1))
    } else if keys.just_pressed(KeyCode::PageUp) {
        current.unwrap_or(0).saturating_sub(8)
    } else if keys.just_pressed(KeyCode::PageDown) {
        (current.unwrap_or(0) + 8).min(visible.len() - 1)
    } else {
        return;
    };
    focus.set(visible[index].1, FocusCause::Navigated);
}

fn click(
    event: On<Activate>,
    choices: Query<&Choice>,
    cheats: Res<Cheats>,
    mut menu: ResMut<Menu>,
) {
    if cheats.menu_open
        && let Ok(choice) = choices.get(event.entity)
    {
        menu.pending = Some(choice.0.clone());
    }
}

fn apply(world: &mut World) {
    let pending = world.resource_mut::<Menu>().pending.take();
    let Some(pending) = pending else { return };
    if !world.resource::<Cheats>().menu_open {
        return;
    }
    match pending {
        Click::Close => {
            let return_to_pause = world.resource::<Cheats>().return_to_pause;
            world.resource_mut::<Cheats>().menu_open = false;
            world.resource_mut::<InputFocus>().clear();
            crate::input::reset(world);
            if !return_to_pause {
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
            }
        }
        Click::Category(category) => world.resource_mut::<Menu>().category = category,
        Click::Action(action) => execute(world, action),
    }
}

fn execute(world: &mut World, action: Action) {
    let mut players = world.query_filtered::<Entity, With<Player>>();
    let Ok(player) = players.single(world) else {
        return;
    };
    match action {
        Action::God => {
            let mut c = world.resource_mut::<Cheats>();
            c.god = !c.god;
        }
        Action::Noclip => {
            let enabled = !world.resource::<Cheats>().noclip;
            world.resource_mut::<Cheats>().noclip = enabled;
            if enabled {
                let origin = *world.get::<Transform>(player).unwrap();
                let collider_disabled = world.get::<ColliderDisabled>(player).is_some();
                world.entity_mut(player).insert((
                    ColliderDisabled,
                    super::Flight {
                        origin,
                        collider_disabled,
                    },
                ));
            } else {
                if let Some(flight) = world.entity_mut(player).take::<super::Flight>() {
                    let mut spatial = bevy::ecs::system::SystemState::<SpatialQuery>::new(world);
                    let query = spatial.get(world).expect("game physics is installed");
                    let pose = world.get::<Transform>(player).unwrap();
                    let collider = world.get::<Collider>(player).unwrap();
                    if !query
                        .shape_intersections(
                            collider,
                            pose.translation,
                            pose.rotation,
                            &crate::environment::player_solids([player]),
                        )
                        .is_empty()
                    {
                        world.entity_mut(player).insert(flight.origin);
                        if let Some(mut pose) = world.get_mut::<Position>(player) {
                            pose.0 = flight.origin.translation;
                        }
                    }
                    if !flight.collider_disabled {
                        world.entity_mut(player).remove::<ColliderDisabled>();
                    }
                }
            }
            if let Some(mut p) = world.get_mut::<Player>(player) {
                p.velocity = Vec3::ZERO;
            }
        }
        Action::Ammo => {
            let mut c = world.resource_mut::<Cheats>();
            c.ammo = !c.ammo;
        }
        Action::Sprint => {
            let mut c = world.resource_mut::<Cheats>();
            c.sprint = !c.sprint;
            if c.sprint
                && let Some(mut sprint) = world.get_mut::<crate::movement::SprintState>(player)
            {
                *sprint = crate::movement::SprintState::new(sprint.capacity);
            }
        }
        Action::OneHit => {
            let mut c = world.resource_mut::<Cheats>();
            c.one_hit = !c.one_hit;
        }
        Action::FreezeEnemies => {
            let mut c = world.resource_mut::<Cheats>();
            c.freeze_enemies = !c.freeze_enemies;
        }
        Action::FreezeRounds => {
            let mut c = world.resource_mut::<Cheats>();
            c.freeze_rounds = !c.freeze_rounds;
        }
        Action::Points => world
            .resource_mut::<crate::gameplay::Session>()
            .award(10_000),
        Action::Heal => {
            if let Some(mut life) = world.get_mut::<crate::life::Vitality>(player) {
                let maximum = life.maximum();
                let _ = life.regenerate(maximum);
            }
        }
        Action::Refill => {
            world.resource_scope(|world, game: Mut<crate::assets::GameDefinition>| {
                if let Some(mut inventory) =
                    world.get_mut::<crate::weapons::WeaponInventory>(player)
                {
                    inventory.refill(&game.content.weapons);
                    for weapon in &game.content.weapons {
                        if inventory
                            .owned()
                            .any(|(_, held)| held.definition == weapon.id)
                        {
                            inventory.fill_received(weapon);
                        }
                    }
                }
            });
        }
        Action::Perks => {
            let perks: Vec<_> = world
                .resource::<crate::assets::GameDefinition>()
                .content
                .perks
                .iter()
                .map(|perk| perk.id.clone())
                .collect();
            if let Some(mut inventory) = world.get_mut::<crate::solo_revive::PerkInventory>(player)
            {
                inventory.owned.extend(perks);
                inventory.solo_charges = inventory.solo_charges.max(1);
            }
        }
        Action::Power => {
            for site in world
                .resource_mut::<crate::generators::Generators>()
                .sites
                .values_mut()
            {
                site.state = crate::generators::GeneratorState::Active;
                site.progress = 100.;
                site.age = 0.;
            }
        }
        Action::Zones => {
            world.resource_scope(|world, game: Mut<crate::assets::GameDefinition>| {
                let mut access = world.resource_mut::<crate::access::ZoneAccess>();
                access
                    .open
                    .extend(game.content.zone_connections.iter().map(|c| c.id.clone()));
                access
                    .open
                    .extend(game.access.doors.iter().map(|d| d.connection.clone()));
                access
                    .activated
                    .extend(game.content.zones.iter().map(|z| z.id.clone()));
                access.refresh(&game.content);
            });
        }
        Action::Kill | Action::NextRound => {
            if matches!(action, Action::NextRound) {
                world.resource_mut::<Cheats>().freeze_rounds = false;
                let mut session = world.resource_mut::<crate::gameplay::Session>();
                session.pending = 0;
                session.intermission = None;
                world.resource_mut::<crate::panzer::PanzerWaves>().pending = 0;
            }
            physics::restore_all(world);
            let mut actors = world
                .query_filtered::<(Entity, &crate::life::Vitality), With<crate::enemies::Zombie>>();
            let targets: Vec<_> = actors
                .iter(world)
                .filter(|(_, life)| life.state() == crate::life::LifeState::ActorAlive)
                .map(|(entity, _)| entity)
                .collect();
            for target in targets {
                world
                    .resource_mut::<crate::damage::DamageQueue>()
                    .0
                    .push(crate::damage::Impact {
                        point: None,
                        attacker: player,
                        target,
                        source: crate::damage::ImpactSource::Powerup,
                        category: crate::origins::content::DamageCategory::Explosive,
                        region: crate::weapons::HitRegion::Torso,
                        amount: i32::MAX as u32,
                    });
            }
        }
        Action::SpawnPoint => {
            let spawn = Vec3::from_array(world.resource::<crate::assets::GameDefinition>().spawn);
            let height = world.get::<Player>(player).unwrap().height;
            let center = spawn + Vec3::Y * height * 0.5;
            world.get_mut::<Transform>(player).unwrap().translation = center;
            if let Some(mut p) = world.get_mut::<Position>(player) {
                p.0 = center;
            }
            world.get_mut::<Player>(player).unwrap().velocity = Vec3::ZERO;
        }
        Action::ToolGun => {
            let mut c = world.resource_mut::<Cheats>();
            c.tool_granted = true;
            c.gun = Gun::Tool;
        }
        Action::PhysicsGun => {
            let mut c = world.resource_mut::<Cheats>();
            c.physics_granted = true;
            c.gun = Gun::Physics;
        }
        Action::NormalGun => world.resource_mut::<Cheats>().gun = Gun::Weapon,
        Action::Undo => tools::undo(world),
        Action::Spawn(selection) => {
            let mut c = world.resource_mut::<Cheats>();
            c.notice = format!("Tool Gun: {}", selection.label());
            c.spawn = selection;
            c.tool_granted = true;
            c.gun = Gun::Tool;
        }
        Action::Weapon(id) => {
            world.resource_scope(|world, game: Mut<crate::assets::GameDefinition>| {
                if let Some(weapon) = game.content.weapons.iter().find(|weapon| weapon.id.0 == id)
                    && let Some(mut inventory) =
                        world.get_mut::<crate::weapons::WeaponInventory>(player)
                {
                    // The player may have paused during a reload or weapon raise.
                    inventory.action = crate::weapons::WeaponAction::Ready;
                    if inventory.receive(weapon).is_ok() {
                        inventory.fill_received(weapon);
                    }
                }
            });
            world.resource_mut::<Cheats>().gun = Gun::Weapon;
        }
    }
    if world.resource::<Cheats>().gun != Gun::Physics {
        physics::release(world);
    }
}

fn close(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>, mut cheats: ResMut<Cheats>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    cheats.menu_open = false;
}

fn rebuild(
    mut commands: Commands,
    cheats: Res<Cheats>,
    menu: Res<Menu>,
    state: Res<State<GameState>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    cameras: Query<Entity, With<crate::rendering::display::GameUiCamera>>,
    roots: Query<Entity, With<MenuRoot>>,
    mut built: Local<Option<Category>>,
) {
    if !cheats.menu_open || **state != GameState::Paused {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }
    // Actions and hover/focus changes update existing controls in place.
    if *built == Some(menu.category) && !roots.is_empty() {
        return;
    }
    let (Some(game), Ok(camera)) = (game, cameras.single()) else {
        return;
    };
    let rows = choices(&game, menu.category);
    for root in &roots {
        commands.entity(root).despawn();
    }
    *built = Some(menu.category);
    commands.spawn((MenuRoot, UiTargetCamera(camera), GlobalZIndex(200), TabGroup::modal(),
        Node { position_type: PositionType::Absolute, width: percent(100.), height: percent(100.),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.005, 0.008, 0.012, 0.85)),
    )).with_children(|root| {
        root.spawn((Node { width: vw(92.), max_width: units(1100.), height: vh(90.),
            padding: UiRect::all(units(20.)), flex_direction: FlexDirection::Column,
            row_gap: units(16.), border: UiRect::all(px(1.)), border_radius: BorderRadius::all(units(8.)), ..default() },
            BackgroundColor(Color::srgb(0.025, 0.035, 0.045)),
            BorderColor::all(Color::srgb(0.23, 0.29, 0.32)),
        )).with_children(|panel| {
            panel.spawn(Node { align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, flex_shrink: 0., ..default() })
                .with_children(|header| {
                    header.spawn(text("ORIGINS  /  SANDBOX", 27.));
                    header.spawn(button("Close", Click::Close, 0)).insert(Node { width: units(125.), height: units(40.), ..button_node() });
                });
            panel.spawn(Node { flex_grow: 1., min_height: px(0.), column_gap: units(18.), ..default() })
                .with_children(|body| {
                    body.spawn(Node { width: units(205.), flex_shrink: 0., flex_direction: FlexDirection::Column,
                        row_gap: units(8.), ..default() }).with_children(|sidebar| {
                        for (index, category) in [Category::Cheats, Category::Weapons, Category::Spawns].into_iter().enumerate() {
                            sidebar.spawn(button(category.title(), Click::Category(category), index as i32 + 1));
                        }
                    });
                    body.spawn(Node { flex_grow: 1., min_width: px(0.), flex_direction: FlexDirection::Column,
                        row_gap: units(8.), ..default() }).with_children(|content| {
                        content.spawn(text(menu.category.title(), 22.));
                        content.spawn(text("Search", 12.));
                        content.spawn((Search, TextInput, EditableText { allow_newlines: false, ..default() },
                            TextLayout::no_wrap(), TextCursorStyle::default(), TabIndex(4),
                            TextFont::from_font_size(FontSize::Vh(16. / 7.2)), TextColor(Color::srgb(0.92, 0.95, 0.96)),
                            Node { height: units(40.), flex_shrink: 0., padding: UiRect::all(units(9.)),
                                border: UiRect::all(px(1.)), border_radius: BorderRadius::all(units(4.)), ..default() },
                            BorderColor::all(Color::srgb(0.27, 0.34, 0.38)),
                            BackgroundColor(Color::srgb(0.045, 0.06, 0.075)),
                        ));
                        content.spawn((ResultCount, text(format!("{} items", rows.len()), 12.)));
                        content.spawn((NoResults, text("No matches", 16.),
                            Node { display: Display::None, padding: UiRect::all(units(16.)), ..default() }));
                        content.spawn(Node { display: Display::Grid, flex_grow: 1., min_height: px(0.),
                            grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.), RepeatedGridTrack::auto(1)],
                            grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.)], column_gap: units(8.), ..default() })
                            .with_children(|frame| {
                                let scroll = frame.spawn((ScrollArea, Node { flex_direction: FlexDirection::Column,
                                    min_height: px(0.), overflow: Overflow::scroll_y(), row_gap: units(5.),
                                    padding: UiRect::right(units(3.)), ..default() },
                                )).with_children(|list| {
                                    for (index, (label, action)) in rows.into_iter().enumerate() {
                                        let id = match &action { Action::Weapon(id) => id.as_str(), Action::Spawn(spawn) => spawn.label(), _ => "" };
                                        let mut row = list.spawn((button(&label, Click::Action(action.clone()), 10 + index as i32),
                                            Row { index, search: format!("{label} {id}").to_lowercase() }));
                                        if index == 0 { row.insert(AutoFocus); }
                                    }
                                }).id();
                                frame.spawn((Node { width: units(12.), ..default() },
                                    Scrollbar::new(scroll, ControlOrientation::Vertical, 24.),
                                    BackgroundColor(Color::srgb(0.04, 0.06, 0.07)),
                                )).with_child((ScrollbarThumb { border_radius: BorderRadius::all(units(6.)), ..default() },
                                    Hovered::default(), BackgroundColor(Color::srgb(0.28, 0.40, 0.44))));
                            });
                    });
                });
            panel.spawn((Notice, text("", 13.), Node { display: Display::None, flex_shrink: 0., ..default() }));
        });
    });
}

fn units(value: f32) -> Val {
    vh(value / 7.2)
}
fn text(label: impl Into<String>, size: f32) -> impl Bundle {
    (
        Text::new(label),
        TextFont::from_font_size(FontSize::Vh(size / 7.2)),
        TextColor(Color::srgb(0.86, 0.91, 0.93)),
        Pickable::IGNORE,
    )
}
fn button_node() -> Node {
    Node {
        width: percent(100.),
        min_height: units(44.),
        flex_shrink: 0.,
        padding: UiRect::axes(units(12.), units(8.)),
        border: UiRect::all(px(1.)),
        border_radius: BorderRadius::all(units(4.)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: units(10.),
        ..default()
    }
}
fn button(label: &str, click: Click, tab: i32) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Choice(click.clone()),
        TabIndex(tab),
        button_node(),
        BackgroundColor(Color::srgb(0.065, 0.085, 0.10)),
        BorderColor::all(Color::srgb(0.18, 0.24, 0.27)),
        children![
            (
                text(label, 16.),
                Node {
                    flex_grow: 1.,
                    ..default()
                }
            ),
            (Badge(click), text("", 12.))
        ],
    )
}

fn filter(
    search: Query<Ref<EditableText>, With<Search>>,
    mut rows: Query<(&Row, &mut Node, &mut TabIndex)>,
    mut counts: Query<&mut Text, With<ResultCount>>,
    mut empty: Query<&mut Node, (With<NoResults>, Without<Row>)>,
) {
    let Ok(search) = search.single() else { return };
    if !search.is_changed() {
        return;
    }
    let query = search.value().to_string().trim().to_lowercase();
    let mut visible = 0;
    for (row, mut node, mut tab) in &mut rows {
        let matches = row.search.contains(&query);
        visible += usize::from(matches);
        let display = if matches {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        tab.0 = if matches { 10 + row.index as i32 } else { -1 };
    }
    for mut text in &mut counts {
        text.0 = format!("{visible} items");
    }
    for mut node in &mut empty {
        node.display = if visible == 0 {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn refresh(
    cheats: Res<Cheats>,
    mut badges: Query<(Ref<Badge>, &mut Text, &mut TextColor)>,
    mut notices: Query<(Ref<Notice>, &mut Text, &mut Node), Without<Badge>>,
) {
    for (badge, mut text, mut color) in &mut badges {
        if !cheats.is_changed() && !badge.is_added() {
            continue;
        }
        let enabled = match &badge.0 {
            Click::Action(action) => action.enabled(&cheats),
            _ => None,
        };
        text.0 = match (&badge.0, enabled) {
            (_, Some(true)) => "ON",
            (_, Some(false)) => "OFF",
            (Click::Action(Action::Weapon(_)), _) => "EQUIP",
            (Click::Action(Action::Spawn(_)), _) => "SELECT",
            (Click::Action(_), _) => "RUN",
            (Click::Close, _) => "ESC",
            (Click::Category(Category::Cheats), _) => "1",
            (Click::Category(Category::Weapons), _) => "2",
            (Click::Category(Category::Spawns), _) => "3",
        }
        .into();
        color.0 = if enabled == Some(true) {
            Color::srgb(0.42, 0.90, 0.71)
        } else {
            Color::srgb(0.55, 0.69, 0.72)
        };
    }
    for (notice, mut text, mut node) in &mut notices {
        if !cheats.is_changed() && !notice.is_added() {
            continue;
        }
        node.display = if cheats.notice.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if text.0 != cheats.notice {
            text.0.clone_from(&cheats.notice);
        }
    }
}

fn styles(
    menu: Res<Menu>,
    focus: Res<InputFocus>,
    mut buttons: Query<
        (
            Entity,
            &Choice,
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<Search>,
    >,
    mut searches: Query<(Entity, &mut BorderColor), With<Search>>,
) {
    for (entity, choice, hovered, pressed, mut background, mut border) in &mut buttons {
        let active = match &choice.0 {
            Click::Category(category) => category.eq(&menu.category),
            _ => false,
        };
        let color = if pressed {
            Color::srgb(0.16, 0.37, 0.39)
        } else if hovered.0 {
            Color::srgb(0.15, 0.22, 0.26)
        } else if active {
            Color::srgb(0.08, 0.24, 0.27)
        } else {
            Color::srgb(0.065, 0.085, 0.10)
        };
        let edge = if focus.get() == Some(entity) {
            Color::srgb(0.50, 0.90, 0.91)
        } else if hovered.0 || active {
            Color::srgb(0.30, 0.55, 0.59)
        } else {
            Color::srgb(0.18, 0.24, 0.27)
        };
        if background.0 != color {
            background.0 = color;
        }
        let edge = BorderColor::all(edge);
        if *border != edge {
            *border = edge;
        }
    }
    for (entity, mut border) in &mut searches {
        let color = if focus.get() == Some(entity) {
            Color::srgb(0.50, 0.90, 0.91)
        } else {
            Color::srgb(0.27, 0.34, 0.38)
        };
        let wanted = BorderColor::all(color);
        if *border != wanted {
            *border = wanted;
        }
    }
}
fn reveal_focus(mut commands: Commands, focus: Res<InputFocus>, rows: Query<(), With<Row>>) {
    if focus.is_changed()
        && let Some(entity) = focus.get()
        && rows.contains(entity)
    {
        commands.trigger(ScrollIntoView { entity });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    };

    fn app(state: GameState) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_state::<GameState>()
            .insert_state(state)
            .init_resource::<Cheats>()
            .init_resource::<crate::input::Actions>()
            .init_resource::<crate::input::GameInput>()
            .init_resource::<crate::input::ControlSettings>()
            .init_resource::<crate::input::menu::ControlMenu>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<KeyboardInput>()
            .add_message::<MouseButtonInput>()
            .add_message::<MouseMotion>()
            .add_message::<MouseWheel>()
            .add_message::<crate::input::RestartGame>()
            .add_systems(
                OnEnter(GameState::Paused),
                |mut time: ResMut<Time<Virtual>>| time.pause(),
            )
            .add_systems(
                OnExit(GameState::Paused),
                |mut time: ResMut<Time<Virtual>>| time.unpause(),
            )
            .add_systems(Update, crate::input::pause_and_restart);
        install(&mut app);
        app.update();
        app
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
        app.update();
    }

    #[test]
    fn cheat_menu_pauses_and_consumes_close_input_before_resuming() {
        let mut app = app(GameState::Playing);
        tap(&mut app, KeyCode::F1);
        assert!(app.world().resource::<Cheats>().menu_open);
        assert_eq!(
            **app.world().resource::<State<GameState>>(),
            GameState::Paused
        );
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        {
            let mut input = app.world_mut().resource_mut::<crate::input::GameInput>();
            input.fire = true;
            input.fire_held = true;
        }
        tap(&mut app, KeyCode::Escape);
        assert!(!app.world().resource::<Cheats>().menu_open);
        assert_eq!(
            **app.world().resource::<State<GameState>>(),
            GameState::Playing
        );
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        let input = app.world().resource::<crate::input::GameInput>();
        assert!(
            !input.fire && !input.fire_held,
            "Closing the UI must not fire the player's weapon"
        );
    }

    #[test]
    fn opening_cheats_from_pause_returns_to_pause() {
        let mut app = app(GameState::Paused);
        tap(&mut app, KeyCode::F1);
        assert!(app.world().resource::<Cheats>().menu_open);
        tap(&mut app, KeyCode::Escape);
        assert!(!app.world().resource::<Cheats>().menu_open);
        assert_eq!(
            **app.world().resource::<State<GameState>>(),
            GameState::Paused
        );
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
    }
    fn ui() -> App {
        let mut app = app(GameState::Paused);
        app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        app.init_resource::<physics::PhysicsGun>();
        app.insert_resource(
            serde_json::from_str::<crate::assets::GameDefinition>(include_str!(
                "../../gameplay/origins/game.json"
            ))
            .unwrap(),
        );
        app.world_mut()
            .spawn(crate::rendering::display::GameUiCamera);
        app.world_mut().spawn(Player {
            yaw: 0.,
            pitch: 0.,
            velocity: Vec3::ZERO,
            height: 1.8,
            grounded: true,
            crouched: false,
            prone: false,
            eye_height: 1.62,
        });
        app.world_mut().resource_mut::<Cheats>().menu_open = true;
        app.update();
        app
    }

    #[test]
    fn mouse_activation_preserves_button_identity_hover_and_scroll() {
        use bevy::{
            camera::RenderTarget,
            picking::{
                backend::HitData,
                events::{Pointer, PointerClick, PointerPress, PointerRelease},
                pointer::{Location, PointerButton, PointerId},
            },
            window::WindowRef,
        };
        let mut app = ui();
        let world = app.world_mut();
        let button = world
            .query::<(Entity, &Choice)>()
            .iter(world)
            .find(|(_, c)| matches!(c.0, Click::Action(Action::God)))
            .unwrap()
            .0;
        let scroll = world
            .query_filtered::<Entity, With<ScrollArea>>()
            .single(world)
            .unwrap();
        world.get_mut::<ScrollPosition>(scroll).unwrap().y = 91.;
        let window = world.spawn(Window::default()).id();
        let pointer = Pointer::new(
            PointerId::Mouse,
            Location {
                target: RenderTarget::Window(WindowRef::Entity(window))
                    .normalize(Some(window))
                    .unwrap(),
                position: Vec2::ZERO,
            },
        );
        let hit = HitData::new(window, 0., None, None);
        world.trigger(PointerPress {
            entity: button,
            pointer: pointer.clone(),
            button: PointerButton::Primary,
            hit: hit.clone(),
            count: 1,
        });
        app.update();
        assert!(
            !app.world().resource::<Cheats>().god,
            "Pressing does not execute before the click completes"
        );
        app.world_mut().trigger(PointerClick {
            entity: button,
            pointer: pointer.clone(),
            button: PointerButton::Primary,
            hit: hit.clone(),
            count: 1,
            duration: std::time::Duration::from_millis(80),
        });
        app.world_mut().trigger(PointerRelease {
            entity: button,
            pointer,
            button: PointerButton::Primary,
            hit,
        });
        app.update();
        assert!(app.world().resource::<Cheats>().god);
        assert!(app.world().get_entity(button).is_ok());
        assert_eq!(app.world().get::<ScrollPosition>(scroll).unwrap().y, 91.);
        let idle = app.world().get::<BackgroundColor>(button).unwrap().0;
        app.world_mut().entity_mut(button).insert(Hovered(true));
        app.update();
        assert_ne!(app.world().get::<BackgroundColor>(button).unwrap().0, idle);
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert!(
            !app.world().resource::<Cheats>().god,
            "Keyboard activation uses the same stable control"
        );
    }

    #[test]
    fn search_filters_tab_order_and_keyboard_navigation_to_visible_rows() {
        let mut app = ui();
        let search = app
            .world_mut()
            .query_filtered::<Entity, With<Search>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(search)
            .insert(EditableText::new("FREEZE"));
        app.update();
        let visible: Vec<_> = app
            .world_mut()
            .query::<(Entity, &Row, &Node, &TabIndex)>()
            .iter(app.world())
            .filter(|(_, _, n, _)| n.display != Display::None)
            .map(|(e, r, _, t)| {
                assert!(t.0 >= 0);
                (r.index, e)
            })
            .collect();
        assert_eq!(visible.len(), 2);
        assert!(
            app.world_mut()
                .query::<(&Node, &TabIndex, &Row)>()
                .iter(app.world())
                .all(|(n, t, _)| n.display != Display::None || t.0 == -1)
        );
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(search, FocusCause::Navigated);
        tap(&mut app, KeyCode::ArrowDown);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(visible.iter().min_by_key(|r| r.0).unwrap().1)
        );
        tap(&mut app, KeyCode::End);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(visible.iter().max_by_key(|r| r.0).unwrap().1)
        );
        app.world_mut()
            .entity_mut(search)
            .insert(EditableText::new("no such cheat"));
        app.update();
        assert!(
            app.world_mut()
                .query::<(&Node, &Row)>()
                .iter(app.world())
                .all(|(n, _)| n.display == Display::None)
        );
        tap(&mut app, KeyCode::ArrowDown);
    }

    #[test]
    fn opening_menu_releases_cursor_before_the_pause_transition() {
        use bevy::window::{CursorGrabMode, CursorOptions};
        let mut app = app(GameState::Playing);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: true,
                    ..default()
                },
                CursorOptions {
                    visible: false,
                    grab_mode: CursorGrabMode::Locked,
                    ..default()
                },
            ))
            .id();
        tap(&mut app, KeyCode::F1);
        let cursor = app.world().get::<CursorOptions>(window).unwrap();
        assert!(cursor.visible);
        assert_eq!(cursor.grab_mode, CursorGrabMode::None);
    }

    #[test]
    #[ignore = "requires installed Origins assets and a windowed GPU session"]
    fn cheat_menu_native_pointer_and_keyboard_preview() {
        use bevy::{
            camera::RenderTarget,
            input::{
                ButtonState,
                keyboard::{Key, KeyboardInput},
                mouse::MouseScrollUnit,
                touch::TouchPhase,
            },
            picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
            render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
            window::{PrimaryWindow, WindowRef},
        };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let output = root.join("target/reports/cheat-menu-preview.png");
        let mut app = App::new();
        app.insert_resource(bevy::winit::WinitSettings::game())
            .insert_resource(bevy::anti_alias::dlss::DlssProjectId(
                bevy::asset::uuid::Uuid::from_u128(0x8e091984_d832_4d03_9f7c_38a3fca674dc),
            ))
            .add_plugins(
                DefaultPlugins
                    .build()
                    .disable::<bevy::audio::AudioPlugin>()
                    .set(bevy::winit::WinitPlugin {
                        run_on_any_thread: true,
                        ..default()
                    })
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "Origins cheat menu preview".into(),
                            resolution: (1600, 1000).into(),
                            ..default()
                        }),
                        ..default()
                    }),
            )
            .init_state::<GameState>()
            .insert_state(GameState::Paused)
            .init_resource::<crate::input::GameInput>()
            .insert_resource(crate::assets::GameDefinition::load_from_root(
                &root.join("target/assets"),
            ))
            .insert_resource(Cheats {
                menu_open: true,
                return_to_pause: true,
                ..default()
            });
        install(&mut app);
        app.init_resource::<physics::PhysicsGun>();
        app.world_mut()
            .spawn((Camera2d, crate::rendering::display::GameUiCamera));
        app.world_mut().spawn(Player {
            yaw: 0.,
            pitch: 0.,
            velocity: Vec3::ZERO,
            height: 1.8,
            grounded: true,
            crouched: false,
            prone: false,
            eye_height: 1.62,
        });
        app.add_systems(
            Update,
            (move |mut commands: Commands,
                   mut frame: Local<u32>,
                   mut target: Local<Option<(Entity, Location)>>,
                   rows: Query<(Entity, &Row, &UiGlobalTransform, &ComputedNode)>,
                   focus: Res<InputFocus>,
                   cheats: Res<Cheats>,
                   choices: Query<&Choice>,
                   windows: Query<Entity, With<PrimaryWindow>>,
                   scroll: Query<&ScrollPosition, With<ScrollArea>>,
                   mut pointer: MessageWriter<PointerInput>,
                   mut keyboard: MessageWriter<KeyboardInput>| {
                *frame += 1;
                let window = windows.single().unwrap();
                if *frame == 20 {
                    let (entity, _, pose, node) =
                        rows.iter().find(|(_, row, _, _)| row.index == 0).unwrap();
                    assert!(node.size().x > 0. && node.size().y > 0.);
                    let location = Location {
                        target: RenderTarget::Window(WindowRef::Entity(window))
                            .normalize(Some(window))
                            .unwrap(),
                        position: pose.translation * node.inverse_scale_factor,
                    };
                    pointer.write(PointerInput::new(
                        PointerId::Mouse,
                        location.clone(),
                        PointerAction::Move { delta: Vec2::ZERO },
                    ));
                    *target = Some((entity, location));
                }
                if let Some((_, location)) = target.as_ref() {
                    let action = match *frame {
                        22 => Some(PointerAction::Press(PointerButton::Primary)),
                        24 => Some(PointerAction::Release(PointerButton::Primary)),
                        30 => Some(PointerAction::Scroll {
                            unit: MouseScrollUnit::Line,
                            x: 0.,
                            y: -3.,
                            phase: TouchPhase::Moved,
                        }),
                        _ => None,
                    };
                    if let Some(action) = action {
                        pointer.write(PointerInput::new(
                            PointerId::Mouse,
                            location.clone(),
                            action,
                        ));
                    }
                }
                if *frame == 28 {
                    assert!(
                        cheats.god,
                        "A native pointer click must hit and activate the visible button"
                    );
                }
                if *frame == 34 {
                    assert!(
                        scroll.single().unwrap().y > 0.,
                        "Mouse wheel must scroll over the button"
                    );
                }
                if *frame == 35 || *frame == 40 {
                    let (key_code, logical_key) = if *frame == 35 {
                        (KeyCode::End, Key::End)
                    } else {
                        (KeyCode::Tab, Key::Tab)
                    };
                    keyboard.write(KeyboardInput {
                        key_code,
                        logical_key,
                        state: ButtonState::Pressed,
                        text: None,
                        repeat: false,
                        window,
                    });
                }
                if *frame == 38 {
                    let entity = focus.get().unwrap();
                    assert_eq!(
                        rows.get(entity).unwrap().1.index,
                        19,
                        "End focuses and reveals the final list item"
                    );
                }
                if *frame == 43 {
                    assert!(
                        matches!(choices.get(focus.get().unwrap()).unwrap().0, Click::Close),
                        "Tab must wrap inside the modal"
                    );
                }
                let key = match *frame {
                    44 => Some((KeyCode::ShiftLeft, Key::Shift, ButtonState::Pressed)),
                    45 => Some((KeyCode::Tab, Key::Tab, ButtonState::Released)),
                    46 => Some((KeyCode::Tab, Key::Tab, ButtonState::Pressed)),
                    49 => Some((KeyCode::ShiftLeft, Key::Shift, ButtonState::Released)),
                    50 => Some((KeyCode::Home, Key::Home, ButtonState::Pressed)),
                    53 => Some((KeyCode::Enter, Key::Enter, ButtonState::Pressed)),
                    _ => None,
                };
                if let Some((key_code, logical_key, state)) = key {
                    keyboard.write(KeyboardInput {
                        key_code,
                        logical_key,
                        state,
                        text: None,
                        repeat: false,
                        window,
                    });
                }
                if *frame == 48 {
                    assert_eq!(
                        rows.get(focus.get().unwrap()).unwrap().1.index,
                        19,
                        "Shift+Tab must return to the last list item"
                    );
                }
                if *frame == 56 {
                    assert!(!cheats.god, "Enter must activate the focused button once");
                }
                if *frame == 58 {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(output.clone()))
                        .observe(
                            |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                                exit.write(AppExit::Success);
                            },
                        );
                }
                assert!(*frame < 240, "Preview capture timed out");
            })
            .after(reveal_focus),
        );
        app.run();
    }
}
