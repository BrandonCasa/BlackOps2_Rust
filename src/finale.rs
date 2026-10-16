use crate::{
    GameState,
    gameplay::{GameStep, SessionEntity},
    life::{LifeState, Vitality},
    quests::{MainQuest, QuestStage},
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Resource, Default)]
pub struct Finale {
    pub souls: u32,
    pub portal_open: bool,
    pub drone_entered: bool,
    pub portal_active: bool,
    pub successful: bool,
    drone_ascending: bool,
    drone_pause: f32,
    drone_voice: Option<Entity>,
    portal_delay: f32,
    entry_delay: f32,
}
impl Finale {
    pub(crate) fn collect_soul(
        &mut self,
        commands: &mut Commands,
        quest: &MainQuest,
        upgrades: &crate::staff_upgrades::StaffUpgrades,
        position: Vec3,
    ) -> bool {
        if !quest.enabled
            || quest.stage != QuestStage::Finale
            || !upgrades.inserted.into_iter().all(|inserted| inserted)
            || self.souls >= 100
        {
            return false;
        }
        if (position - Vec3::new(262.636, -9.2456, 200.8124))
            .xz()
            .length_squared()
            < 45.72_f32.powi(2)
        {
            crate::quests::soul_effect(
                commands,
                position,
                Vec3::new(262.636, -8.4074, 200.8124),
                0.,
                1.,
            );
            self.souls += 1;
            let line = match self.souls {
                1 => Some(3),
                33 => Some(4),
                66 => Some(5),
                100 => Some(0),
                _ => None,
            };
            if let Some(line) = line {
                crate::audio::samantha(
                    commands,
                    format!("vox_sam_generic_encourage_{line}"),
                    crate::audio::SoundSource::World(Vec3::new(262.636, -8.5344, 200.8124)),
                );
            }
        }
        // The source portal-active override returns before chest/staff dispatch,
        // including kills outside the Crazy Place that supply no portal soul.
        true
    }
}
#[derive(Component)]
struct FistSoul {
    spent: bool,
    lights: Vec<Entity>,
}
impl FistSoul {
    fn punch(&mut self, fists: &mut crate::rituals::IronFists) -> bool {
        if self.spent || fists.upgraded || fists.souls >= 20 {
            return false;
        }
        self.spent = true;
        fists.souls += 1;
        true
    }
}
#[derive(Component)]
struct FistReward {
    owner: Entity,
}
#[derive(Component)]
pub(crate) struct FistCandidate(pub f32);
impl Default for FistCandidate {
    fn default() -> Self {
        Self(0.1)
    }
}
#[derive(Component)]
struct FistUpgrade {
    remaining: f32,
    element: Option<crate::quests::Element>,
}
#[derive(Component)]
pub(crate) struct FistPickupFlash {
    remaining: f32,
    pub fade: f32,
}
#[derive(Component)]
struct FinalPortal {
    phase: u8,
    fx: Option<Entity>,
    sound: Option<Entity>,
    beam: bool,
}
pub struct FinalePlugin;
impl Plugin for FinalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Finale>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                fist_flash
                    .before(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                fists
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                fist_fx
                    .in_set(GameStep::Rewards)
                    .after(fists)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                portal
                    .in_set(GameStep::Rewards)
                    .after(crate::quests::souls)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                enter
                    .in_set(GameStep::Interactions)
                    .after(crate::quest_recovery::recover)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(mut commands: Commands, mut finale: ResMut<Finale>) {
    *finale = default();
    commands.spawn((
        SessionEntity,
        FinalPortal {
            phase: 0,
            fx: None,
            sound: None,
            beam: false,
        },
        Transform::from_translation(Vec3::new(262.636, -8.5344, 200.8124)),
        Visibility::Hidden,
    ));
}
fn fists(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    spatial: SpatialQuery,
    mut quest: ResMut<MainQuest>,
    mut applied: MessageReader<crate::damage::DamageApplied>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    mut candidates: Query<
        (
            Entity,
            &mut FistCandidate,
            &crate::rounds::SpawnZone,
            &Vitality,
        ),
        Without<FistSoul>,
    >,
    mut actors: ParamSet<(
        Query<(&mut FistSoul, &Transform, &crate::enemies::Zombie)>,
        Query<
            (
                &Transform,
                &Vitality,
                &mut crate::enemies::Zombie,
                &mut crate::combat_effects::Wounds,
            ),
            (
                Without<crate::player::Player>,
                Without<crate::panzer::Panzer>,
            ),
        >,
    )>,
    mut players: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            &crate::weapons::WeaponInventory,
            &mut crate::rituals::IronFists,
            Option<&mut FistUpgrade>,
        ),
        With<crate::player::Player>,
    >,
    rewards: Query<(Entity, &Transform, &FistReward)>,
) {
    if !quest.enabled || quest.stage != QuestStage::UpgradeFists {
        applied.clear();
        return;
    }
    for (entity, mut candidate, zone, health) in &mut candidates {
        candidate.0 -= time.delta_secs();
        if candidate.0 > 0. {
            continue;
        }
        commands.entity(entity).remove::<FistCandidate>();
        if zone.0.0 != "zone.ug_bottom_zone" || health.state() != LifeState::ActorAlive {
            continue;
        }
        commands.entity(entity).insert(FistSoul {
            spent: false,
            lights: Vec::new(),
        });
    }
    for hit in applied.read() {
        if hit.impact.source.id() != "weapon.one_inch_punch_zm" {
            continue;
        }
        let Ok((_, _, _, _, mut fists, _)) = players.get_mut(hit.impact.attacker) else {
            continue;
        };
        let mut souls = actors.p0();
        let Ok((mut soul, pose, zombie)) = souls.get_mut(hit.impact.target) else {
            continue;
        };
        if !zombie.completed_emerging() || !soul.punch(&mut fists) {
            continue;
        }
        for light in soul.lights.drain(..) {
            commands.entity(light).try_despawn();
        }
        crate::audio::emit(
            &mut commands,
            "zmb_squest_punchtime_punched",
            crate::audio::SoundSource::World(pose.translation),
        );
        if fists.souls == 20 {
            let reward = commands
                .spawn((
                    SessionEntity,
                    FistReward {
                        owner: hit.impact.attacker,
                    },
                    Transform::from_translation(pose.translation + Vec3::Y * 1.27),
                    Visibility::Inherited,
                ))
                .id();
            if let Some(scene) = game
                .content
                .presentations
                .iter()
                .filter_map(|p| p.scene.as_ref())
                .find(|s| s.contains("p6_zm_tm_tablet_lod0"))
            {
                commands
                    .entity(reward)
                    .insert(WorldAssetRoot(server.load(scene.clone())));
            }
            crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_punchtime_tablet_loop",
                crate::audio::SoundSource::attached(reward),
            );
        }
    }
    let mut player_upgraded = false;
    for (owner, player, health, inventory, mut fists, upgrade) in &mut players {
        if let Some(mut upgrade) = upgrade {
            upgrade.remaining -= time.delta_secs();
            if upgrade.remaining <= 0. {
                fists.unlocked = true;
                fists.upgraded = true;
                fists.element = upgrade.element;
                commands.entity(owner).remove::<FistUpgrade>();
                player_upgraded = true;
            }
            continue;
        }
        if health.state() != LifeState::Standing {
            continue;
        }
        for (entity, pose, reward) in &rewards {
            if reward.owner != owner
                || !spatial
                    .shape_intersections(
                        &Collider::cuboid(0.508, 0.508, 0.127),
                        pose.translation,
                        pose.rotation,
                        &SpatialQueryFilter::default(),
                    )
                    .contains(&owner)
            {
                continue;
            }
            let element = [
                (crate::quests::Element::Fire, "fire"),
                (crate::quests::Element::Wind, "air"),
                (crate::quests::Element::Lightning, "lightning"),
                (crate::quests::Element::Ice, "water"),
            ]
            .into_iter()
            .find(|(_, family)| {
                inventory
                    .owned()
                    .any(|(_, w)| w.definition.0 == format!("weapon.staff_{family}_upgraded_zm"))
            })
            .map(|(e, _)| e);
            commands.entity(entity).despawn();
            commands.entity(owner).insert((
                FistUpgrade {
                    remaining: 1.,
                    element,
                },
                FistPickupFlash {
                    remaining: 1.3,
                    fade: 0.,
                },
                crate::actions::HandReservation::timed(crate::actions::HandAction::Station, 1.),
            ));
            crate::audio::emit(
                &mut commands,
                "zmb_squest_punchtime_tablet_pickup",
                crate::audio::SoundSource::attached(owner),
            );
            for (pose, health, mut zombie, mut wounds) in &mut actors.p1() {
                if health.state() == LifeState::ActorAlive
                    && !wounds.crawler
                    && zombie.completed_emerging()
                    && (pose.translation - player.translation)
                        .xz()
                        .length_squared()
                        < 6.5024_f32.powi(2)
                {
                    crate::combat_effects::knockdown(
                        &mut wounds,
                        &mut zombie,
                        player.translation,
                        pose.translation,
                        1.,
                    );
                }
            }
            break;
        }
    }
    if player_upgraded && players.iter().all(|(_, _, _, _, fists, _)| fists.upgraded) {
        quest.transition(QuestStage::UpgradeFists, QuestStage::Finale);
    }
}
fn fist_flash(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut flashes: Query<(Entity, &Vitality, &mut FistPickupFlash)>,
) {
    for (entity, health, mut flash) in &mut flashes {
        flash.remaining = (flash.remaining - time.delta_secs()).max(0.);
        let elapsed = 1.3 - flash.remaining;
        flash.fade = if elapsed < 0.5 {
            elapsed / 0.5
        } else if elapsed <= 0.8 {
            1.
        } else {
            flash.remaining / 0.5
        };
        if flash.remaining == 0. || health.state() == LifeState::HumanTerminal {
            commands.entity(entity).remove::<FistPickupFlash>();
        }
    }
}
fn fist_fx(
    mut commands: Commands,
    mut souls: Query<(Entity, &mut FistSoul)>,
    children: Query<&Children>,
    tags: Query<&Name>,
) {
    for (entity, mut soul) in &mut souls {
        if soul.spent || !soul.lights.is_empty() {
            continue;
        }
        let wrists: Vec<_> = children
            .iter_descendants(entity)
            .filter(|e| {
                tags.get(*e).is_ok_and(|n| {
                    n.as_str().eq_ignore_ascii_case("J_Wrist_RI")
                        || n.as_str().eq_ignore_ascii_case("J_Wrist_LE")
                })
            })
            .collect();
        if wrists.len() == 2 {
            for wrist in wrists {
                soul.lights.push(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_ee_fists",
                    wrist,
                    Transform::IDENTITY,
                ));
            }
        }
    }
}
fn portal(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut finale: ResMut<Finale>,
    mut quest: ResMut<MainQuest>,
    upgrades: Res<crate::staff_upgrades::StaffUpgrades>,
    mut drones: Query<(Entity, &mut crate::buildables::Drone, &Transform)>,
    mut buildables: ResMut<crate::buildables::BuildableState>,
    mut portals: Query<(Entity, &Transform, &mut Visibility, &mut FinalPortal)>,
    mut playback: MessageReader<crate::audio::PlaybackEvent>,
) {
    for event in playback.read() {
        if finale.drone_voice == Some(event.entity)
            && matches!(
                event.status,
                crate::audio::PlaybackStatus::Completed
                    | crate::audio::PlaybackStatus::Rejected
                    | crate::audio::PlaybackStatus::Cancelled
            )
        {
            finale.drone_voice = None;
            finale.drone_pause = 1.;
        }
    }
    if !quest.enabled {
        return;
    }
    finale.portal_active =
        quest.stage == QuestStage::Finale && upgrades.inserted.into_iter().all(|inserted| inserted);
    for (entity, pose, mut visible, mut portal) in &mut portals {
        *visible = if finale.portal_active || finale.portal_open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let phase = if finale.portal_open {
            2
        } else if finale.portal_active {
            1
        } else {
            0
        };
        if phase != portal.phase {
            if let Some(fx) = portal.fx.take() {
                commands.entity(fx).try_despawn();
            }
            if let Some(sound) = portal.sound.take() {
                commands.entity(sound).try_despawn();
            }
            if phase > 0 {
                portal.fx = Some(crate::native_fx::attach(
                    &mut commands,
                    if phase == 1 {
                        "maps/zombie_tomb/fx_tomb_challenge_fire"
                    } else {
                        "maps/zombie_tomb/fx_tomb_ee_vortex"
                    },
                    entity,
                    Transform::IDENTITY,
                ));
                portal.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    if phase == 1 {
                        "zmb_squest_sam_portal_closed_loop"
                    } else {
                        "zmb_squest_sam_portal_open_loop"
                    },
                    crate::audio::SoundSource::attached(entity),
                ));
                if phase == 2 {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_squest_sam_portal_open",
                        crate::audio::SoundSource::attached(entity),
                    );
                }
            }
            portal.phase = phase;
        }
        if finale.drone_entered && !portal.beam {
            portal.beam = true;
            crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_ee_beam",
                entity,
                Transform::from_xyz(0., -7.62, 0.)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            );
            crate::audio::emit(
                &mut commands,
                "zmb_squest_crystal_sky_pillar_start",
                crate::audio::SoundSource::World(pose.translation),
            );
            crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_crystal_sky_pillar_loop",
                crate::audio::SoundSource::attached(entity),
            );
        }
    }
    if finale.souls >= 100 && !finale.portal_open && finale.portal_delay == 0. {
        finale.portal_delay = 0.5;
    }
    if finale.portal_delay > 0. {
        finale.portal_delay = (finale.portal_delay - time.delta_secs()).max(0.);
        if finale.portal_delay == 0. {
            finale.portal_open = true;
            quest.transition(QuestStage::Finale, QuestStage::Complete);
        }
    }
    if finale.portal_open && !finale.drone_entered {
        for (entity, mut drone, pose) in &mut drones {
            let entrance = Vec3::new(262.636, -6.2992, 200.8124);
            if !finale.drone_ascending
                && (pose.translation - entrance).xz().length_squared() >= 12.7_f32.powi(2)
            {
                continue;
            }
            drone.returning = false;
            if finale.drone_voice.is_some() {
                continue;
            }
            if finale.drone_pause > 0. {
                finale.drone_pause = (finale.drone_pause - time.delta_secs()).max(0.);
                if finale.drone_pause > 0. {
                    continue;
                }
                crate::audio::emit(
                    &mut commands,
                    "vox_maxi_drone_upgraded_2",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            drone.quest_goal = Some(if finale.drone_ascending {
                Vec3::new(262.636, 7.9248, 200.8124)
            } else {
                entrance
            });
            if pose.translation.distance(drone.quest_goal.unwrap()) < 0.5 {
                if finale.drone_ascending {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_qrdrone_leave",
                        crate::audio::SoundSource::World(pose.translation),
                    );
                    commands.entity(entity).despawn();
                    buildables.drone = None;
                    buildables.drone_disabled = true;
                    finale.drone_entered = true;
                    finale.entry_delay = 1.;
                } else {
                    finale.drone_voice = Some(crate::audio::emit(
                        &mut commands,
                        "vox_maxi_drone_upgraded_1",
                        crate::audio::SoundSource::attached(entity),
                    ));
                    finale.drone_ascending = true;
                }
            }
        }
    }
    if finale.entry_delay > 0. {
        finale.entry_delay = (finale.entry_delay - time.delta_secs()).max(0.);
        if finale.entry_delay == 0. {
            crate::audio::samantha(
                &mut commands,
                "vox_sam_all_staff_freedom_0",
                crate::audio::SoundSource::World(Vec3::new(262.636, -8.5344, 200.8124)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_finale_claims_souls_before_other_collectors_until_one_hundred() {
        let mut quest = MainQuest::reset(crate::frontend::Difficulty::Original);
        let mut upgrades = crate::staff_upgrades::StaffUpgrades::default();
        upgrades.inserted = [true; 4];
        let mut finale = Finale::default();
        let mut world = World::new();
        let mut commands = world.commands();
        let center = Vec3::new(262.636, -9.2456, 200.8124);
        assert!(!finale.collect_soul(&mut commands, &quest, &upgrades, center));
        quest.stage = QuestStage::Finale;
        upgrades.inserted[0] = false;
        assert!(!finale.collect_soul(&mut commands, &quest, &upgrades, center));
        upgrades.inserted[0] = true;
        assert!(finale.collect_soul(&mut commands, &quest, &upgrades, Vec3::ZERO));
        assert_eq!(finale.souls, 0);
        assert!(finale.collect_soul(&mut commands, &quest, &upgrades, center));
        assert_eq!(finale.souls, 1);
        finale.souls = 99;
        assert!(finale.collect_soul(&mut commands, &quest, &upgrades, center));
        assert_eq!(finale.souls, 100);
        assert!(!finale.collect_soul(&mut commands, &quest, &upgrades, center));
        assert_eq!(finale.souls, 100);
    }

    fn soul() -> FistSoul {
        FistSoul {
            spent: false,
            lights: Vec::new(),
        }
    }

    #[test]
    fn each_player_needs_twenty_separate_fist_souls() {
        let mut first = crate::rituals::IronFists::default();
        let mut second = crate::rituals::IronFists::default();
        for _ in 0..19 {
            assert!(soul().punch(&mut first));
        }
        assert_eq!(first.souls, 19);
        assert_eq!(second.souls, 0);
        let mut shared = soul();
        assert!(shared.punch(&mut second));
        assert!(!shared.punch(&mut first));
        assert_eq!(first.souls, 19);
        assert_eq!(second.souls, 1);
        assert!(soul().punch(&mut first));
        assert_eq!(first.souls, 20);
        assert!(!soul().punch(&mut first));
        assert!(soul().punch(&mut second));
        assert_eq!(second.souls, 2);
    }
}
fn enter(
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut finale: ResMut<Finale>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    mut next: ResMut<NextState<GameState>>,
    targets: Query<(Entity, &Transform), With<FinalPortal>>,
    players: Query<(&crate::player::Player, &Transform, &Vitality), Without<FinalPortal>>,
) {
    if !finale.drone_entered || finale.entry_delay > 0. || focus.target.is_some() {
        return;
    }
    let Ok((player, pose, health)) = players.single() else {
        return;
    };
    if health.state() != LifeState::Standing {
        return;
    }
    for (entity, target) in &targets {
        let delta = target.translation - pose.translation;
        let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
        if delta.length() < 2.54 && forward.dot(delta.normalize_or_zero()) > 0.5 {
            focus.target = Some(entity);
            focus.hold_seconds = 0.;
            focus.prompt = format!(
                "Press {} to enter the final portal",
                settings.label(crate::input::Action::Interact)
            );
            if input.use_held {
                finale.successful = true;
                next.set(GameState::Ending);
                focus.accepted += 1;
            }
        }
    }
}
