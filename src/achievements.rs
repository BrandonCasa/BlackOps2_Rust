use crate::{GameState, gameplay::GameStep, player::Player};
use bevy::prelude::*;
use std::collections::BTreeSet;
#[derive(Resource, Default)]
pub struct Achievements {
    pub earned: BTreeSet<String>,
    pub notification: String,
    pub persistence_error: Option<String>,
    notification_seconds: f32,
    progress: MatchProgress,
    pending: std::collections::VecDeque<String>,
    ride_start: Option<usize>,
    ride_departed: bool,
    path: Option<std::path::PathBuf>,
}
#[derive(Default)]
struct MatchProgress {
    staffs: [bool; 4],
    claw_releases: [bool; 2],
    revives: [bool; 4],
    blood_revives: u32,
    blood_capture: bool,
}
impl MatchProgress {
    fn release_claw(&mut self, player: Entity, captive: Entity) -> bool {
        self.claw_releases[usize::from(player != captive)] = true;
        self.claw_releases.iter().all(|released| *released)
    }

    fn revive(&mut self, event: &crate::solo_revive::PlayerRevived) {
        // The source tracks reviving another player, never solo Quick Revive.
        if event.player == event.reviver {
            return;
        }
        let index = match event.method {
            crate::solo_revive::ReviveMethod::Normal => 0,
            crate::solo_revive::ReviveMethod::QuickRevive => 1,
            crate::solo_revive::ReviveMethod::Drone => 2,
            crate::solo_revive::ReviveMethod::Staff => 3,
        };
        self.revives[index] = true;
        self.blood_revives = self
            .blood_revives
            .saturating_add(u32::from(event.zombie_blood));
    }

    fn master_of_disguise(&self) -> bool {
        self.blood_capture && self.blood_revives >= 3
    }
}
pub struct AchievementPlugin;
impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Achievements>()
            .add_systems(Startup, load)
            .add_systems(OnEnter(GameState::StartingSession), reset)
            .add_systems(
                FixedUpdate,
                observe
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, notification);
    }
}
fn load(mut state: ResMut<Achievements>) {
    state.path = std::env::var_os("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("Origins/achievements.json"));
    if let Some(path) = &state.path
        && path.exists()
    {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
        {
            Ok(earned) => state.earned = earned,
            Err(e) => state.persistence_error = Some(e),
        }
    }
}
fn reset(mut state: ResMut<Achievements>) {
    state.progress = default();
    state.pending.clear();
    state.ride_start = None;
    state.ride_departed = false;
    state.notification.clear();
    state.notification_seconds = 0.;
}
fn earn(state: &mut Achievements, id: &str, name: &str) {
    if !state.earned.insert(id.into()) {
        return;
    }
    state
        .pending
        .push_back(format!("Achievement earned: {name}"));
    if let Some(path) = &state.path {
        let result = (|| {
            let parent = path
                .parent()
                .ok_or_else(|| "Missing achievement directory".to_owned())?;
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec_pretty(&state.earned).map_err(|e| e.to_string())?;
            let temp = path.with_extension("tmp");
            std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(temp, path).map_err(|e| e.to_string())
        })();
        if let Err(error) = result {
            warn!("Achievements: {error}");
            state.persistence_error = Some(error);
        }
    }
}
fn observe(
    time: Res<Time<Fixed>>,
    mut state: ResMut<Achievements>,
    quest: Res<crate::quests::MainQuest>,
    crafting: Res<crate::crafting::Crafting>,
    generators: Res<crate::generators::Generators>,
    raids: Res<crate::generators::GeneratorRaids>,
    rituals: Res<crate::rituals::Rituals>,
    session: Res<crate::gameplay::Session>,
    tank: Res<crate::tank::Tank>,
    mut releases: MessageReader<crate::panzer::PanzerClawReleased>,
    mut shared: MessageReader<crate::environment::digging::DigWeaponShared>,
    mut revives: MessageReader<crate::solo_revive::PlayerRevived>,
    mut captures: MessageReader<crate::generators::GeneratorCaptured>,
    players: Query<
        (
            Entity,
            &crate::weapons::WeaponInventory,
            &crate::movement::platforms::PlatformSupport,
            &crate::life::Vitality,
        ),
        With<Player>,
    >,
) {
    state.notification_seconds = (state.notification_seconds - time.delta_secs()).max(0.);
    if state.notification_seconds == 0. {
        state.notification = state.pending.pop_front().unwrap_or_default();
        if !state.notification.is_empty() {
            state.notification_seconds = 6.;
        }
    }
    let Ok((player, weapons, support, health)) = players.single() else {
        return;
    };
    for release in releases.read() {
        if release.releaser == player && state.progress.release_claw(player, release.captive) {
            earn(&mut state, "ZM_DLC4_KUNG_FU_GRIP", "Kung Fu Grip");
        }
    }
    for pickup in shared.read() {
        if pickup.digger == player && pickup.collector != player {
            earn(&mut state, "ZM_DLC4_NOT_A_GOLD_DIGGER", "Not a Gold Digger");
        }
    }
    for revive in revives.read() {
        if revive.reviver == player {
            state.progress.revive(revive);
        }
    }
    for capture in captures.read() {
        if capture.player == player && capture.zombie_blood {
            state.progress.blood_capture = true;
        }
    }
    if state.progress.revives.iter().all(|revived| *revived) {
        earn(
            &mut state,
            "ZM_DLC4_SAVING_THE_DAY_ALL_DAY",
            "Saving the Day... All Day",
        );
    }
    if state.progress.master_of_disguise() {
        earn(
            &mut state,
            "ZM_DLC4_MASTER_OF_DISGUISE",
            "Master of Disguise",
        );
    }
    if let Some(weapon) = weapons.active()
        && health.state() == crate::life::LifeState::Standing
        && let Some(index) = [
            "weapon.staff_water_upgraded_zm",
            "weapon.staff_lightning_upgraded_zm",
            "weapon.staff_fire_upgraded_zm",
            "weapon.staff_air_upgraded_zm",
        ]
        .iter()
        .position(|id| *id == weapon.definition.0)
    {
        state.progress.staffs[index] = true;
    }
    if quest.stage == crate::quests::QuestStage::Complete {
        earn(&mut state, "ZM_DLC4_TOMB_SIDEQUEST", "Little Lost Girl");
    }
    if generators.active_count() == 6 && raids.shutdowns == 0 {
        earn(&mut state, "ZM_DLC4_ALL_YOUR_BASE", "All Your Base");
    }
    if [
        "elemental_staff_fire",
        "elemental_staff_water",
        "elemental_staff_air",
        "elemental_staff_lightning",
    ]
    .iter()
    .all(|id| crafting.crafted.contains(*id))
    {
        earn(
            &mut state,
            "ZM_DLC4_PLAYING_WITH_POWER",
            "Playing with Power",
        );
    }
    if session.challenge_headshots >= 115
        && rituals.generators >= 6
        && session.spent >= 30000
        && rituals.chests >= 4
    {
        earn(&mut state, "ZM_DLC4_OVERACHIEVER", "Overachiever");
    }
    if state.progress.staffs.iter().all(|staff| *staff) {
        earn(&mut state, "ZM_DLC4_MASTER_WIZARD", "Master Wizard");
    }
    let moving = matches!(
        tank.phase,
        crate::tank::TankPhase::Moving | crate::tank::TankPhase::Departing
    );
    let aboard = tank.entity.is_some()
        && support.entity == tank.entity
        && health.state() == crate::life::LifeState::Standing;
    if !aboard {
        state.ride_start = None;
        state.ride_departed = false;
    } else if let Some(start) = state.ride_start {
        if moving {
            state.ride_departed = true;
        } else if state.ride_departed && tank.station == start {
            earn(&mut state, "ZM_DLC4_IM_ON_A_TANK", "I'm on a Tank!");
            state.ride_departed = false;
        }
    } else if !moving {
        state.ride_start = Some(tank.station);
        state.ride_departed = false;
    }
}
#[derive(Component)]
struct Toast;
fn notification(
    mut commands: Commands,
    game_state: Res<State<GameState>>,
    state: Res<Achievements>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    mut text: Query<(&mut crate::hud::native::NativeText, &mut Visibility), With<Toast>>,
) {
    if let Ok((mut text, mut visibility)) = text.single_mut() {
        *visibility = if **game_state == GameState::Playing {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if text.value != state.notification {
            text.value = state.notification.clone();
        }
        return;
    }
    if let Ok(camera) = cameras.single() {
        commands.spawn((
            Toast,
            Visibility::Hidden,
            crate::hud::PlayerOverlay,
            UiTargetCamera(camera),
            crate::hud::native::NativeText::new(&state.notification, 22.)
                .color(Color::srgb(1., 0.8, 0.25)),
            Node {
                position_type: PositionType::Absolute,
                left: percent(30.),
                top: percent(8.),
                width: px(700.),
                height: px(60.),
                ..default()
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solo_revive::{PlayerRevived, ReviveMethod};

    fn revive(
        reviver: Entity,
        player: Entity,
        method: ReviveMethod,
        zombie_blood: bool,
    ) -> PlayerRevived {
        PlayerRevived {
            player,
            reviver,
            method,
            zombie_blood,
        }
    }

    #[test]
    fn kung_fu_grip_requires_self_and_friendly_releases_in_either_order() {
        let mut world = World::new();
        let player = world.spawn_empty().id();
        let friend = world.spawn_empty().id();
        let mut progress = MatchProgress::default();
        assert!(!progress.release_claw(player, player));
        assert!(!progress.release_claw(player, player));
        assert!(progress.release_claw(player, friend));
        let mut progress = MatchProgress::default();
        assert!(!progress.release_claw(player, friend));
        assert!(progress.release_claw(player, player));
    }

    #[test]
    fn saving_the_day_requires_four_distinct_teammate_revive_methods() {
        let mut world = World::new();
        let player = world.spawn_empty().id();
        let friend = world.spawn_empty().id();
        let mut progress = MatchProgress::default();
        progress.revive(&revive(player, player, ReviveMethod::QuickRevive, false));
        assert_eq!(progress.revives, [false; 4]);
        for method in [
            ReviveMethod::Normal,
            ReviveMethod::Normal,
            ReviveMethod::Drone,
            ReviveMethod::Staff,
        ] {
            progress.revive(&revive(player, friend, method, false));
        }
        assert!(!progress.revives.iter().all(|revived| *revived));
        progress.revive(&revive(player, friend, ReviveMethod::QuickRevive, false));
        assert!(progress.revives.iter().all(|revived| *revived));
    }

    #[test]
    fn disguise_counts_only_completed_actions_during_blood_across_pickups() {
        let mut world = World::new();
        let player = world.spawn_empty().id();
        let friend = world.spawn_empty().id();
        let mut progress = MatchProgress::default();
        progress.blood_capture = true;
        for _ in 0..3 {
            progress.revive(&revive(player, friend, ReviveMethod::Normal, false));
        }
        assert!(!progress.master_of_disguise());
        for _ in 0..2 {
            progress.revive(&revive(player, friend, ReviveMethod::Drone, true));
        }
        assert!(!progress.master_of_disguise());
        progress.revive(&revive(player, friend, ReviveMethod::Staff, true));
        assert!(progress.master_of_disguise());
        progress.blood_capture = false;
        assert!(!progress.master_of_disguise());
    }
}
