use super::{Cheats, Gun, physics, tools};
use crate::{GameState, player::Player};
use avian3d::prelude::*;
use bevy::{
    picking::{events::PointerPress, pointer::PointerButton},
    prelude::*,
    ui_widgets::Button,
};

const PAGE_SIZE: usize = 12;
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
    Page(bool),
    Close,
}
#[derive(Resource, Default)]
struct Menu {
    category: Category,
    page: usize,
    selected: usize,
    pending: Option<Click>,
}
#[derive(Component)]
struct MenuRoot;

pub(super) fn install(app: &mut App) {
    app.init_resource::<Menu>()
        .add_observer(click)
        .add_systems(
            Update,
            (keyboard, apply, rebuild)
                .chain()
                .before(crate::input::menu::navigate)
                .before(crate::input::pause_and_restart),
        )
        .add_systems(OnExit(GameState::Paused), close);
}

fn choices(
    cheats: &Cheats,
    game: &crate::assets::GameDefinition,
    category: Category,
) -> Vec<(String, Action)> {
    let toggle = |label: &str, value| format!("{label}   [{}]", if value { "ON" } else { "OFF" });
    match category {
        Category::Cheats => vec![
            (toggle("God mode", cheats.god), Action::God),
            (toggle("Noclip / fly", cheats.noclip), Action::Noclip),
            (toggle("Unlimited ammo", cheats.ammo), Action::Ammo),
            (toggle("Unlimited sprint", cheats.sprint), Action::Sprint),
            (toggle("One hit kills", cheats.one_hit), Action::OneHit),
            (
                toggle("Freeze enemy AI", cheats.freeze_enemies),
                Action::FreezeEnemies,
            ),
            (
                toggle("Freeze round spawning", cheats.freeze_rounds),
                Action::FreezeRounds,
            ),
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
            .map(|weapon| {
                (
                    weapon.id.0.trim_start_matches("weapon.").to_owned(),
                    Action::Weapon(weapon.id.0.clone()),
                )
            })
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
                        p.id.0.clone(),
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
                        enemy.id.0.clone(),
                        Action::Spawn(tools::Spawn::Enemy(enemy.id.0.clone())),
                    )
                }),
        )
        .collect(),
    }
}

fn keyboard(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut cheats: ResMut<Cheats>,
    mut menu: ResMut<Menu>,
    game: Option<Res<crate::assets::GameDefinition>>,
    mut input: ResMut<crate::input::GameInput>,
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
            cheats.revision += 1;
            *menu = default();
            input.clear();
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
    }
    if keys.just_pressed(KeyCode::Tab) {
        menu.pending = Some(Click::Category(match menu.category {
            Category::Cheats => Category::Weapons,
            Category::Weapons => Category::Spawns,
            Category::Spawns => Category::Cheats,
        }));
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        menu.pending = Some(Click::Page(false));
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        menu.pending = Some(Click::Page(true));
    }
    if keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::Enter)
    {
        let Some(game) = game else { return };
        let rows = choices(&cheats, &game, menu.category);
        let count = rows
            .len()
            .saturating_sub(menu.page * PAGE_SIZE)
            .min(PAGE_SIZE);
        if count == 0 {
            return;
        }
        if keys.just_pressed(KeyCode::ArrowUp) {
            menu.selected = (menu.selected + count - 1) % count;
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            menu.selected = (menu.selected + 1) % count;
        }
        if keys.just_pressed(KeyCode::Enter) {
            menu.pending = Some(Click::Action(
                rows[menu.page * PAGE_SIZE + menu.selected].1.clone(),
            ));
        }
        cheats.revision += 1;
    }
}

fn click(event: On<PointerPress>, choices: Query<&Choice>, mut menu: ResMut<Menu>) {
    if event.button == PointerButton::Primary
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
    world.resource_mut::<Cheats>().revision += 1;
    match pending {
        Click::Close => {
            let return_to_pause = world.resource::<Cheats>().return_to_pause;
            world.resource_mut::<Cheats>().menu_open = false;
            crate::input::reset(world);
            if !return_to_pause {
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
            }
        }
        Click::Category(category) => {
            let mut menu = world.resource_mut::<Menu>();
            menu.category = category;
            menu.page = 0;
            menu.selected = 0;
        }
        Click::Page(forward) => {
            let menu = world.resource::<Menu>();
            let pages = choices(
                world.resource::<Cheats>(),
                world.resource::<crate::assets::GameDefinition>(),
                menu.category,
            )
            .len()
            .div_ceil(PAGE_SIZE)
            .max(1);
            let mut menu = world.resource_mut::<Menu>();
            menu.page = (menu.page + if forward { 1 } else { pages - 1 }) % pages;
            menu.selected = 0;
        }
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
    mut revision: Local<Option<u64>>,
) {
    if !cheats.menu_open || **state != GameState::Paused {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }
    if *revision == Some(cheats.revision) && !roots.is_empty() {
        return;
    }
    let (Some(game), Ok(camera)) = (game, cameras.single()) else {
        return;
    };
    let rows = choices(&cheats, &game, menu.category);
    let pages = rows.len().div_ceil(PAGE_SIZE).max(1);
    for root in &roots {
        commands.entity(root).despawn();
    }
    *revision = Some(cheats.revision);
    commands.spawn((MenuRoot, UiTargetCamera(camera), GlobalZIndex(200),
        Node { position_type: PositionType::Absolute, width: percent(100.), height: percent(100.),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.01, 0.015, 0.02, 0.88)),
    )).with_children(|root| {
        root.spawn((Node { width: percent(90.), max_width: px(1100.), padding: UiRect::all(px(18.)),
            flex_direction: FlexDirection::Column, row_gap: px(10.), ..default() },
            BackgroundColor(Color::srgb(0.045, 0.065, 0.085)),
        )).with_children(|panel| {
            panel.spawn(text("ORIGINS  /  CHEATS", 30.));
            panel.spawn(text("GAME PAUSED    F1 / Esc: close    Tab: category    Arrows / Enter: select", 16.));
            panel.spawn(Node { column_gap: px(12.), ..default() }).with_children(|tabs| {
                for (label, category) in [("CHEATS", Category::Cheats), ("WEAPONS", Category::Weapons), ("TOOL GUN SPAWNS", Category::Spawns)] {
                    tabs.spawn(button(label, Click::Category(category), category == menu.category));
                }
            });
            panel.spawn(Node { flex_wrap: FlexWrap::Wrap, column_gap: px(12.), row_gap: px(8.), ..default() })
                .with_children(|list| {
                    for (index, (label, action)) in rows.into_iter().skip(menu.page * PAGE_SIZE).take(PAGE_SIZE).enumerate() {
                        list.spawn(button(&label, Click::Action(action), index == menu.selected));
                    }
                });
            panel.spawn(Node { column_gap: px(12.), align_items: AlignItems::Center, ..default() }).with_children(|footer| {
                footer.spawn(button("< PREVIOUS", Click::Page(false), false));
                footer.spawn(text(format!("{} / {}", menu.page + 1, pages), 18.));
                footer.spawn(button("NEXT >", Click::Page(true), false));
                footer.spawn(button("CLOSE", Click::Close, false));
            });
            panel.spawn(text(format!("Selected spawn: {}", cheats.spawn.label()), 17.));
            if !cheats.notice.is_empty() { panel.spawn(text(&cheats.notice, 17.)); }
            panel.spawn(text("Noclip: jump / crouch to rise / descend, sprint to boost.  F2: Tool Gun  F3: Physics Gun  F4: weapon", 15.));
        });
    });
}

fn text(label: impl Into<String>, size: f32) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font_size: size.into(),
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.94, 0.97)),
    )
}
fn button(label: &str, click: Click, selected: bool) -> impl Bundle {
    let row = matches!(click, Click::Action(_));
    (
        Button,
        Choice(click),
        Node {
            padding: UiRect::all(px(10.)),
            width: if row { percent(48.) } else { Val::Auto },
            min_height: px(42.),
            ..default()
        },
        BackgroundColor(if selected {
            Color::srgb(0.08, 0.35, 0.43)
        } else {
            Color::srgb(0.09, 0.13, 0.17)
        }),
        children![(text(label, 17.), Pickable::IGNORE)],
    )
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
}
