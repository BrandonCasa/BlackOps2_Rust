//! Optional sandbox controls. Normal gameplay owns its inventory, damage, and progression.
mod menu;
pub(crate) mod physics;
mod tools;
mod visuals;

use crate::{GameState, gameplay::GameStep, player::Player};
use bevy::prelude::*;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gun {
    #[default]
    Weapon,
    Tool,
    Physics,
}

#[derive(Resource, Default)]
pub(crate) struct Cheats {
    pub menu_open: bool,
    return_to_pause: bool,
    pub god: bool,
    pub noclip: bool,
    pub ammo: bool,
    pub sprint: bool,
    pub one_hit: bool,
    pub freeze_enemies: bool,
    pub freeze_rounds: bool,
    tool_granted: bool,
    physics_granted: bool,
    gun: Gun,
    spawn: tools::Spawn,
    notice: String,
    revision: u64,
}

pub struct CheatPlugin;
#[derive(Component)]
struct Flight {
    origin: Transform,
    collider_disabled: bool,
}

impl Plugin for CheatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cheats>()
            .init_resource::<tools::GunInput>()
            .init_resource::<tools::SpawnHistory>()
            .init_resource::<physics::PhysicsGun>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                reset.before(crate::session::SessionSetup::Initialize),
            )
            .add_systems(OnEnter(GameState::MainMenu), reset)
            .add_systems(
                PreUpdate,
                tools::input
                    .after(crate::input::collect)
                    .before(crate::player::look),
            )
            .add_systems(
                FixedUpdate,
                fly.in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing))
                    .run_if(flying),
            )
            .add_systems(
                FixedUpdate,
                sustain
                    .after(GameStep::Movement)
                    .before(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                guard_damage
                    .in_set(GameStep::Damage)
                    .before(crate::damage::resolve)
                    .run_if(in_state(GameState::Playing)),
            );
        menu::install(app);
        tools::install(app);
        physics::install(app);
        visuals::install(app);
    }
}

fn reset(world: &mut World) {
    physics::restore_all(world);
    world.insert_resource(Cheats::default());
    world.insert_resource(tools::GunInput::default());
    world.insert_resource(tools::SpawnHistory::default());
    world.insert_resource(physics::PhysicsGun::default());
}

pub(crate) fn walking(cheats: Option<Res<Cheats>>) -> bool {
    !cheats.is_some_and(|c| c.noclip)
}
fn flying(cheats: Res<Cheats>) -> bool {
    cheats.noclip
}
pub(crate) fn enemies_active(cheats: Option<Res<Cheats>>) -> bool {
    !cheats.is_some_and(|c| c.freeze_enemies)
}
pub(crate) fn rounds_active(cheats: Option<Res<Cheats>>) -> bool {
    !cheats.is_some_and(|c| c.freeze_rounds)
}
pub(crate) fn weapon_equipped(cheats: Option<Res<Cheats>>) -> bool {
    cheats.is_none_or(|c| c.gun == Gun::Weapon)
}

fn fly(
    time: Res<Time<Fixed>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<crate::input::GameInput>,
    controls: Res<crate::input::Actions>,
    mut actions: ResMut<crate::player::TickActions>,
    mut players: Query<(&mut Player, &mut Transform, &crate::life::Vitality)>,
) {
    let Ok((mut player, mut pose, life)) = players.single_mut() else {
        return;
    };
    let (jump, fire, melee, reload) = input.take_edges();
    *actions = crate::player::TickActions {
        fire,
        melee,
        reload,
        fire_held: input.fire_held,
        ads: input.ads,
        cycle_weapon: std::mem::take(&mut input.cycle_weapon),
        weapon_slot: input.weapon_slot.take(),
        ..default()
    };
    if life.state() != crate::life::LifeState::Standing {
        return;
    }
    let rotation = Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch);
    let up = jump || controls.state(crate::input::Action::Jump).held;
    let down = input.crouch || keys.pressed(KeyCode::ControlLeft);
    let vertical = u8::from(up) as f32 - u8::from(down) as f32;
    let direction =
        rotation * Vec3::new(input.movement.x, 0., -input.movement.y) + Vec3::Y * vertical;
    let speed = if input.sprint { 24. } else { 8. };
    pose.translation += direction.normalize_or_zero() * speed * time.delta_secs();
    player.velocity = Vec3::ZERO;
    player.grounded = false;
    input.prone = false;
}

fn sustain(
    cheats: Res<Cheats>,
    game: Res<crate::assets::GameDefinition>,
    mut players: Query<(
        &mut crate::weapons::WeaponInventory,
        &mut crate::movement::SprintState,
    )>,
) {
    if !cheats.ammo && !cheats.sprint {
        return;
    }
    for (mut inventory, mut sprint) in &mut players {
        if cheats.ammo
            && let Some(weapon) = inventory.active().and_then(|held| {
                game.content
                    .weapons
                    .iter()
                    .find(|w| w.id == held.definition)
            })
        {
            inventory.fill_received(weapon);
        }
        if cheats.sprint {
            sprint.remaining = sprint.capacity;
            sprint.exhausted = false;
        }
    }
}

fn guard_damage(
    cheats: Res<Cheats>,
    mut queue: ResMut<crate::damage::DamageQueue>,
    players: Query<(), With<Player>>,
    actors: Query<(), With<crate::enemies::Zombie>>,
    held: Query<(), With<physics::LivingRagdoll>>,
) {
    queue.0.retain_mut(|impact| {
        if (cheats.god || cheats.noclip) && players.contains(impact.target)
            || held.contains(impact.target)
            || held.contains(impact.attacker)
        {
            return false;
        }
        if cheats.freeze_enemies && matches!(impact.source, crate::damage::ImpactSource::Enemy(_)) {
            return false;
        }
        if cheats.one_hit && players.contains(impact.attacker) && actors.contains(impact.target) {
            impact.amount = i32::MAX as u32;
        }
        true
    });
}
