use crate::{assets::GameDefinition, economy::{PointLedger, TransactionId}, gameplay::{GameStep, Session, SessionEntity}, input::GameInput, interactions::{InteractionFocus, InteractionRejection}, life::{LifeState, Vitality}, origins::content::WeaponId, player::Player, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;

const BOX_POSITIONS: [[f32; 3]; 6] = [
    [9.144, -3.2512, -64.6176], [73.66, -9.3472, -140.208],
    [-13.4112, -8.9408, -97.3328], [52.2224, 2.9972, -11.5824],
    [-72.5424, 5.8928, -4.4704], [28.448, 7.6708, 69.088],
];
const BOX_WEAPONS: &[&str] = &[
    "hamr_zm",
    "mg08_zm",
    "type95_zm",
    "galil_zm",
    "fnfal_zm",
    "scar_zm",
    "ksg_zm",
    "srm1216_zm",
    "ak74u_extclip_zm",
    "pdw57_zm",
    "thompson_zm",
    "qcw05_zm",
    "mp40_stalker_zm",
    "evoskorpion_zm",
    "dsr50_zm",
    "beretta93r_extclip_zm",
    "kard_zm",
    "python_zm",
    "fivesevendw_zm",
    "m32_zm",
    "cymbal_monkey_zm",
    "ray_gun_zm",
    "raygun_mark2_zm",
];
#[derive(Component, Clone, Copy)]
enum Station { Box(usize), PackAPunch }
#[derive(Clone)]
enum BoxPhase {
    Idle,
    Rolling { owner: Entity, weapon: WeaponId, remaining: f32, teddy: bool, purchase: TransactionId },
    Reward { owner: Entity, weapon: WeaponId, remaining: f32 },
    Relocating { remaining: f32 },
}
#[derive(Clone)]
enum UpgradePhase {
    Idle,
    Processing { owner: Entity, weapon: WeaponId, remaining: f32 },
    Ready { owner: Entity, weapon: WeaponId, remaining: f32 },
}
#[derive(Resource)]
pub struct Stations {
    pub normal_box: usize,
    pub moves: u32,
    pub uses: u32,
    pub purchases: u32,
    pub collections: u32,
    pub refunds: u32,
    boxes: [BoxPhase; 6],
    upgrade: UpgradePhase,
    latched: bool,
}
impl Default for Stations {
    fn default() -> Self {
        Self { normal_box: 0, moves: 0, uses: 0, purchases: 0, collections: 0, refunds: 0,
            boxes: std::array::from_fn(|_| BoxPhase::Idle), upgrade: UpgradePhase::Idle, latched: false }
    }
}
pub struct StationPlugin;
impl Plugin for StationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stations>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, (tick, interact).chain().in_set(GameStep::Interactions)
                .after(crate::interactions::interact).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut stations: ResMut<Stations>, mut session: ResMut<Session>) {
    *stations = Stations::default();
    stations.normal_box = [0, 2][session.random() as usize % 2];
    for (index, position) in BOX_POSITIONS.iter().enumerate() {
        commands.spawn((Station::Box(index), SessionEntity, Transform::from_translation(Vec3::from_array(*position))));
    }
    commands.spawn((Station::PackAPunch, SessionEntity, Transform::from_xyz(-0.1397, 8.5217, 0.2159)));
}
fn tick(time: Res<Time<Fixed>>, mut stations: ResMut<Stations>, mut session: ResMut<Session>,
    mut ledger: ResMut<PointLedger>, mut digging: ResMut<crate::environment::digging::Digging>) {
    let dt = time.delta_secs();
    for index in 0..6 {
        stations.boxes[index] = match stations.boxes[index].clone() {
            BoxPhase::Rolling { owner, weapon, mut remaining, teddy, purchase } => {
                remaining -= dt;
                if remaining > 0. { BoxPhase::Rolling { owner, weapon, remaining, teddy, purchase } }
                else if teddy {
                    if ledger.refund(&mut session.points, purchase) { stations.refunds += 1; }
                    BoxPhase::Relocating { remaining: 7. }
                } else { BoxPhase::Reward { owner, weapon, remaining: 12. } }
            }
            BoxPhase::Reward { owner, weapon, remaining } if remaining > dt => BoxPhase::Reward { owner, weapon, remaining: remaining - dt },
            BoxPhase::Relocating { remaining } if remaining > dt => BoxPhase::Relocating { remaining: remaining - dt },
            BoxPhase::Relocating { .. } => {
                stations.normal_box = (index + 1 + session.random() as usize % 5) % 6;
                stations.moves += 1;
                stations.uses = 0;
                digging.box_moved = true;
                BoxPhase::Idle
            }
            _ => BoxPhase::Idle,
        };
    }
    stations.upgrade = match stations.upgrade.clone() {
        UpgradePhase::Processing { owner, weapon, remaining } if remaining > dt => UpgradePhase::Processing { owner, weapon, remaining: remaining - dt },
        UpgradePhase::Processing { owner, weapon, .. } => UpgradePhase::Ready { owner, weapon, remaining: 15. },
        UpgradePhase::Ready { owner, weapon, remaining } if remaining > dt => UpgradePhase::Ready { owner, weapon, remaining: remaining - dt },
        _ => UpgradePhase::Idle,
    };
}
fn roll(definition: &GameDefinition, inventory: &WeaponInventory, tactical: &crate::equipment::TacticalInventory, session: &mut Session) -> Option<WeaponId> {
    let candidates: Vec<_> = BOX_WEAPONS.iter().filter_map(|id| {
        let weapon = definition.content.weapons.iter().find(|w| w.id.0.strip_prefix("weapon.").unwrap_or(&w.id.0) == *id)?;
        if inventory.owned().any(|(_, owned)| owned.definition == weapon.id || weapon.upgraded.as_ref() == Some(&owned.definition)) { return None; }
        let weight = if *id == "cymbal_monkey_zm" {
            if tactical.0.as_ref().is_some_and(|t| t.definition == weapon.id) { 1 } else if session.round < 10 { 3 } else { 5 }
        } else { 1 };
        Some((weapon.id.clone(), weight))
    }).collect();
    let total: u32 = candidates.iter().map(|(_, weight)| weight).sum();
    if total == 0 { return None; }
    let mut pick = session.random() % total;
    for (weapon, weight) in candidates {
        if pick < weight { return Some(weapon); }
        pick -= weight;
    }
    None
}
pub(crate) fn interact(
    input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, definition: Res<GameDefinition>,
    effects: Res<crate::powerups::PowerupEffects>, generators: Res<crate::generators::Generators>,
    query: SpatialQuery, targets: Query<(Entity, &Station, &Transform), Without<Player>>,
    mut players: Query<(Entity, &Player, &Transform, &Vitality, &mut WeaponInventory, &mut crate::equipment::TacticalInventory, &crate::equipment::GrenadeInventory, &crate::mines::MineInventory, Has<crate::perks::PerkDrinking>), With<Player>>,
    mut stations: ResMut<Stations>, mut focus: ResMut<InteractionFocus>, mut ledger: ResMut<PointLedger>, mut session: ResMut<Session>,
) {
    if !input.use_held { stations.latched = false; }
    let Ok((player_entity, player, body, vitality, mut inventory, mut tactical, grenades, mines, drinking)) = players.single_mut() else { return; };
    if vitality.state() != LifeState::Standing { stations.latched = input.use_held; return; }
    if focus.target.is_some() { return; }
    let origin = body.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw + inventory.recoil.x) * Quat::from_rotation_x(player.pitch + inventory.recoil.y) * Vec3::NEG_Z;
    let sale = effects.fire_sale > 0.;
    let candidate = targets.iter().filter_map(|(entity, station, transform)| {
        if let Station::Box(index) = station {
            if *index != stations.normal_box && !sale && matches!(stations.boxes[*index], BoxPhase::Idle) { return None; }
        }
        let delta = transform.translation + Vec3::Y * 0.8 - origin;
        let distance = delta.length();
        let direction = Dir3::new(delta).ok()?;
        if distance > 2.2 || forward.dot(*direction) < 0.85 || query.cast_ray(origin, direction, (distance - 0.12).max(0.), true, &crate::environment::solids([player_entity])).is_some() { return None; }
        Some((entity, *station, distance))
    }).min_by(|a,b| a.2.total_cmp(&b.2));
    let Some((entity, station, _)) = candidate else { return; };
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    let key = settings.label(crate::input::Action::Interact);
    let busy = inventory.action != WeaponAction::Ready || tactical.busy() || grenades.busy() || mines.busy() || drinking;
    let pressed = input.use_held && !stations.latched;
    if pressed { stations.latched = true; }
    match station {
        Station::Box(index) => match stations.boxes[index].clone() {
            BoxPhase::Idle => {
                let price = if sale { 10 } else { 950 };
                focus.prompt = format!("Press {key} for Mystery Box [Cost: {price}]");
                if !pressed { return; }
                if busy { focus.rejection = Some(InteractionRejection::Busy); return; }
                let Some(weapon) = roll(&definition, &inventory, &tactical, &mut session) else { focus.rejection = Some(InteractionRejection::Unavailable); return; };
                let purchase = ledger.command(price);
                if let Err(error) = ledger.accept(&mut session.points, purchase, || Ok::<_, InteractionRejection>(())) { focus.rejection = Some(error.into()); return; }
                stations.uses += 1;
                let uses = stations.uses;
                let chance = if uses < 4 { 0 } else if stations.moves == 0 && uses >= 8 { 100 } else if uses < 8 { 15 } else if uses < 13 { 30 } else { 50 };
                let teddy = !sale && session.random() % 100 < chance;
                stations.boxes[index] = BoxPhase::Rolling { owner: player_entity, weapon, remaining: 4., teddy, purchase: purchase.id };
                stations.purchases += 1;
                focus.accepted += 1;
            }
            BoxPhase::Reward { owner, weapon, .. } => {
                let Some(d) = definition.content.weapons.iter().find(|d| d.id == weapon) else { return; };
                focus.prompt = format!("Press {key} to take {}", d.name);
                if pressed && !busy && owner == player_entity {
                    let accepted = if d.throwable.is_some() { tactical.receive(d).is_ok() } else { inventory.receive(d).is_ok() };
                    if accepted { stations.boxes[index] = BoxPhase::Idle; stations.collections += 1; focus.accepted += 1; }
                    else { focus.rejection = Some(InteractionRejection::Unavailable); }
                }
            }
            BoxPhase::Rolling { .. } => focus.prompt = "Mystery Box rolling".into(),
            BoxPhase::Relocating { .. } => focus.prompt = "Mystery Box moving".into(),
        },
        Station::PackAPunch => {
            if generators.active_count() != 6 { focus.prompt = "Pack-a-Punch requires all six generators".into(); focus.rejection = Some(InteractionRejection::NoPower); return; }
            match stations.upgrade.clone() {
                UpgradePhase::Idle => {
                    focus.prompt = format!("Press {key} to Pack-a-Punch [Cost: 5000]");
                    if !pressed { return; }
                    if busy { focus.rejection = Some(InteractionRejection::Busy); return; }
                    let upgraded = inventory.definition(&definition.content.weapons).and_then(|d| d.upgraded.as_ref()).and_then(|id| definition.content.weapons.iter().find(|d| d.id == *id)).map(|d| d.id.clone());
                    let Some(weapon) = upgraded else { focus.rejection = Some(InteractionRejection::Unavailable); return; };
                    let purchase = ledger.command(5000);
                    let result = ledger.accept(&mut session.points, purchase, || inventory.deposit_active().map_err(|_| InteractionRejection::Busy));
                    match result {
                        Ok(_) => {
                            if inventory.active().is_none() {
                                if let Some(fists) = definition.content.weapons.iter().find(|d| d.id.0 == "weapon.zombie_fists_zm") { let _ = inventory.grant(fists); }
                            }
                            stations.upgrade = UpgradePhase::Processing { owner: player_entity, weapon, remaining: 3.35 }; stations.purchases += 1; focus.accepted += 1; }
                        Err(error) => focus.rejection = Some(error.into()),
                    }
                }
                UpgradePhase::Processing { .. } => focus.prompt = "Pack-a-Punch processing".into(),
                UpgradePhase::Ready { owner, weapon, remaining } => {
                    let Some(d) = definition.content.weapons.iter().find(|d| d.id == weapon) else { return; };
                    focus.prompt = format!("Press {key} to collect {} [{remaining:.0}s]", d.name);
                    if pressed && !busy && owner == player_entity && inventory.receive(d).is_ok() {
                        inventory.fill_received(d);
                        stations.upgrade = UpgradePhase::Idle;
                        stations.collections += 1;
                        focus.accepted += 1;
                    }
                }
            }
        }
    }
}
