use crate::{GameState, gameplay::GameStep, player::Player};
use bevy::prelude::*;
use std::collections::BTreeSet;
#[derive(Resource, Default)]
pub struct Achievements {
    pub earned: BTreeSet<String>,
    pub notification: String,
    pub persistence_error: Option<String>,
    notification_seconds: f32,
    staffs: BTreeSet<String>,
    pending: std::collections::VecDeque<String>,
    ride_start: Option<usize>,
    ride_departed: bool,
    path: Option<std::path::PathBuf>,
}
pub struct AchievementPlugin;
impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Achievements>()
            .add_systems(Startup, load)
            .add_systems(OnEnter(GameState::PreparingShaders), reset)
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
    state.staffs.clear();
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
    tank: Res<crate::tank::Tank>,
    mut releases: MessageReader<crate::panzer::PanzerClawReleased>,
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
        if release.releaser == player {
            earn(&mut state, "ZM_DLC4_KUNG_FU_GRIP", "Kung Fu Grip");
        }
    }
    if let Some(weapon) = weapons.active()
        && weapon.definition.0.starts_with("weapon.staff_")
        && weapon.definition.0.ends_with("_upgraded_zm")
    {
        state.staffs.insert(weapon.definition.0.clone());
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
    if rituals.headshots >= 115
        && rituals.generators >= 6
        && rituals.spent >= 30000
        && rituals.chests >= 4
    {
        earn(&mut state, "ZM_DLC4_OVERACHIEVER", "Overachiever");
    }
    if state.staffs.len() >= 4 {
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
