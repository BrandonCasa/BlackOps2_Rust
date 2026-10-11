use crate::{assets::GameDefinition, economy::{PointLedger, TransactionId}, gameplay::{GameStep, Session, SessionEntity}, input::GameInput, interactions::{InteractionFocus, InteractionRejection}, life::{LifeState, Vitality}, origins::PerkId, player::Player, solo_revive::PerkInventory, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;

const MACHINES: [[f32; 3]; 6] = [[-6.8072,-7.5184,-94.2848],[23.5712,7.6708,87.1728],[67.564,3.0988,-1.940306],[-59.944,5.842,-4.4704],[19.6088,-3.2512,-64.6176],[75.3872,-9.3472,-136.3472]];
#[derive(Component)]
pub(crate) struct Machine(usize);
#[derive(Clone, Debug)]
pub enum BottlePhase {
    Idle,
    Rolling { owner: Entity, perk: PerkId, remaining: f32, bear: bool, purchase: TransactionId },
    Available { owner: Entity, perk: PerkId, remaining: f32, purchase: TransactionId },
    Moving(f32),
}
#[derive(Resource)]
pub struct Wunderfizz {
    pub active: usize,
    pub phase: BottlePhase,
    pub rolls: u32,
    pub collected: u32,
    pub expired: u32,
    pub moves: u32,
    uses: u32,
    move_after: u32,
    latched: bool,
}
impl Default for Wunderfizz {
    fn default() -> Self { Self { active: 2, phase: BottlePhase::Idle, rolls: 0, collected: 0, expired: 0, moves: 0, uses: 0, move_after: 4, latched: false } }
}
#[derive(Component)]
pub struct PerkBottle { pub perk: PerkId, pub remaining: f32 }
pub struct VendorPlugin;
impl Plugin for VendorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Wunderfizz>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, (tick, interact, ground_bottles).chain().in_set(GameStep::Interactions).after(crate::tank::interact).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut state: ResMut<Wunderfizz>, mut session: ResMut<Session>) {
    *state = default(); state.active = 2 + session.random() as usize % 2; state.move_after = 4 + session.random() % 3;
    for (i, position) in MACHINES.iter().enumerate() {
        commands.spawn((Machine(i), SessionEntity, Transform::from_translation(Vec3::from_array(*position)), Visibility::Inherited));
    }
}
fn powered(state: &Wunderfizz, game: &GameDefinition, generators: &crate::generators::Generators) -> bool {
    let point = Vec3::from_array(MACHINES[state.active]);
    game.content.generators.iter().filter_map(|g| game.content.placements.iter().find(|p| p.id == g.placement).map(|p| (&g.id, Vec3::from_array(p.position).distance_squared(point))))
        .min_by(|a,b| a.1.total_cmp(&b.1)).is_some_and(|(id, _)| generators.powered(id))
}
fn roll(game: &GameDefinition, perks: &PerkInventory, session: &mut Session) -> Option<PerkId> {
    let mut candidates = Vec::new();
    let mut fallback = Vec::new();
    for perk in &game.content.perks {
        if perks.owned.contains(&perk.id) || !perk.has_supported_effect() || perk.drink.is_none() { continue; }
        if perk.id == game.solo_revive.perk && perks.can_buy_solo(&game.solo_revive).is_err() { continue; }
        fallback.push(perk.id.clone());
        let rare = matches!(perk.id.0.as_str(), "perk.double_tap" | "perk.deadshot" | "perk.mule_kick" | "perk.phd_flopper" | "perk.electric_cherry");
        if !rare || session.random() % 4 == 0 { candidates.push(perk.id.clone()); }
    }
    if candidates.is_empty() { candidates = fallback; }
    if candidates.is_empty() { None } else { Some(candidates[session.random() as usize % candidates.len()].clone()) }
}
fn tick(mut commands: Commands, time: Res<Time<Fixed>>, mut state: ResMut<Wunderfizz>, mut session: ResMut<Session>, mut ledger: ResMut<PointLedger>, players: Query<&PerkInventory, With<Player>>) {
    let dt = time.delta_secs();
    state.phase = match state.phase.clone() {
        BottlePhase::Rolling { owner, perk, mut remaining, bear, purchase } => {
            remaining -= dt;
            if remaining > 0. { BottlePhase::Rolling { owner, perk, remaining, bear, purchase } }
            else if bear {
                ledger.refund(&mut session.points, purchase);
                crate::audio::emit(&mut commands, "zmb_perk_bear", crate::audio::SoundSource::World(Vec3::from_array(MACHINES[state.active])));
                BottlePhase::Moving(10.)
            } else { BottlePhase::Available { owner, perk, remaining: 10., purchase } }
        }
        BottlePhase::Available { owner, perk, mut remaining, purchase } => {
            remaining -= dt;
            if remaining <= 0. || players.get(owner).is_ok_and(|p| p.owned.len() >= p.purchase_limit()) {
                if players.get(owner).is_ok_and(|p| p.owned.len() >= p.purchase_limit()) { ledger.refund(&mut session.points, purchase); }
                state.expired += 1; BottlePhase::Idle
            } else { BottlePhase::Available { owner, perk, remaining, purchase } }
        }
        BottlePhase::Moving(mut remaining) => {
            remaining -= dt;
            if remaining <= 0. {
                let previous = state.active;
                state.active = (previous + 1 + session.random() as usize % (MACHINES.len() - 1)) % MACHINES.len();
                state.uses = 0; state.move_after = 4 + session.random() % 3; state.moves += 1;
                BottlePhase::Idle
            } else { BottlePhase::Moving(remaining) }
        }
        BottlePhase::Idle => BottlePhase::Idle,
    };
}
pub(crate) fn interact(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, game: Res<GameDefinition>, generators: Res<crate::generators::Generators>, query: SpatialQuery,
    mut state: ResMut<Wunderfizz>, mut focus: ResMut<InteractionFocus>, mut session: ResMut<Session>, mut ledger: ResMut<PointLedger>,
    machines: Query<(Entity, &Machine, &Transform), Without<Player>>,
    players: Query<(Entity, &Player, &Transform, &Vitality, &PerkInventory, &WeaponInventory, Has<super::PerkDrinking>, &crate::equipment::TacticalInventory, &crate::equipment::GrenadeInventory, &crate::mines::MineInventory), With<Player>>) {
    if !input.use_held { state.latched = false; }
    let Ok((owner, player, pose, health, perks, inventory, drinking, tactical, grenades, mines)) = players.single() else { return; };
    if health.state() != LifeState::Standing || focus.target.is_some() { return; }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let candidate = machines.iter().filter_map(|(entity, machine, t)| {
        let delta = t.translation + Vec3::Y * 1.2 - eye; let distance = delta.length(); let direction = Dir3::new(delta).ok()?;
        (distance < 2.4 && forward.dot(*direction) > 0.8 && query.cast_ray(eye, direction, (distance - 0.1).max(0.), true, &crate::environment::solids([owner])).is_none()).then_some((entity, machine.0, distance))
    }).min_by(|a,b| a.2.total_cmp(&b.2));
    let Some((entity, index, _)) = candidate else { return; };
    focus.target = Some(entity); focus.progress = 0.; focus.hold_seconds = 0.;
    if index != state.active { focus.prompt = "Wunderfizz is at another location".into(); return; }
    if !powered(&state, &game, &generators) { focus.prompt = "Wunderfizz requires generator power".into(); focus.rejection = Some(InteractionRejection::NoPower); return; }
    let key = settings.label(crate::input::Action::Interact);
    let pressed = input.use_held && !state.latched;
    if pressed { state.latched = true; }
    let busy = drinking || inventory.action != WeaponAction::Ready || tactical.busy() || grenades.busy() || mines.busy();
    match state.phase.clone() {
        BottlePhase::Idle => {
            focus.prompt = format!("Press {key} for Wunderfizz [Cost: 1500]");
            if !pressed { return; }
            if busy { focus.rejection = Some(InteractionRejection::Busy); return; }
            if perks.owned.len() >= perks.purchase_limit() { focus.rejection = Some(InteractionRejection::PerkLimit); return; }
            let Some(perk) = roll(&game, perks, &mut session) else { focus.rejection = Some(InteractionRejection::Unavailable); return; };
            let purchase = ledger.command(1500);
            if let Err(error) = ledger.accept(&mut session.points, purchase, || Ok::<_, InteractionRejection>(())) { focus.rejection = Some(error.into()); return; }
            state.uses += 1; state.rolls += 1;
            let bear = state.uses >= state.move_after;
            state.phase = BottlePhase::Rolling { owner, perk, remaining: if bear { 7. } else { 4. }, bear, purchase: purchase.id };
            focus.accepted += 1;
        }
        BottlePhase::Available { owner: buyer, perk, remaining, .. } => {
            let Some(definition) = game.content.perks.iter().find(|p| p.id == perk) else { return; };
            focus.prompt = format!("Press {key} to take {} [{remaining:.0}s]", definition.name);
            if !pressed || buyer != owner || busy { return; }
            if perks.owned.len() >= perks.purchase_limit() { focus.rejection = Some(InteractionRejection::PerkLimit); return; }
            match super::PerkDrinking::new(definition, inventory, &game) {
                Ok(drink) => { commands.entity(owner).insert(drink); state.phase = BottlePhase::Idle; state.collected += 1; focus.accepted += 1; }
                Err(error) => focus.rejection = Some(error),
            }
        }
        BottlePhase::Rolling { .. } => focus.prompt = "Wunderfizz rolling".into(),
        BottlePhase::Moving(_) => focus.prompt = "Wunderfizz relocating".into(),
    }
}
fn ground_bottles(mut commands: Commands, time: Res<Time<Fixed>>, game: Res<GameDefinition>, mut bottles: Query<(Entity, &mut PerkBottle, &Transform), Without<Player>>,
    players: Query<(Entity, &Vitality, &Transform, &PerkInventory, &WeaponInventory, Has<super::PerkDrinking>), With<Player>>) {
    for (entity, mut bottle, pose) in &mut bottles {
        bottle.remaining -= time.delta_secs();
        if bottle.remaining <= 0. { commands.entity(entity).despawn(); continue; }
        for (owner, health, body, perks, inventory, drinking) in &players {
            if health.state() != LifeState::Standing || drinking || perks.owned.contains(&bottle.perk) || body.translation.distance_squared(pose.translation) > 2.25 { continue; }
            if let Some(perk) = game.content.perks.iter().find(|p| p.id == bottle.perk) && let Ok(drink) = super::PerkDrinking::new(perk, inventory, &game) {
                commands.entity(owner).insert(drink); commands.entity(entity).despawn(); break;
            }
        }
    }
}
