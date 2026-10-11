use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::GameStep,
    life::*,
    origins::{PerkId, WeaponId},
    player::Player,
    player_health::{HealthUpdate, PlayerDeath, Recovery},
    weapons::WeaponInventory,
};
use bevy::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeSet, time::Duration};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoloReviveProfile {
    pub perk: PerkId,
    pub seconds: f32,
    pub bleedout_seconds: f32,
    pub bleedout_poll_seconds: f32,
    pub maximum_purchases: u32,
    pub downed_weapon: WeaponId,
    pub reserve_clips: u32,
    pub downed_height: f32,
    pub downed_speed: f32,
}
impl SoloReviveProfile {
    pub fn valid(&self, game: &GameDefinition) -> bool {
        [
            self.seconds,
            self.bleedout_seconds,
            self.bleedout_poll_seconds,
            self.downed_height,
            self.downed_speed,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.)
            && self.seconds < self.bleedout_seconds
            && self.bleedout_seconds.fract() == 0.
            && self.bleedout_seconds <= 3600.
            && self.bleedout_poll_seconds >= 0.001
            && self.bleedout_poll_seconds <= self.bleedout_seconds
            && self.maximum_purchases > 0
            && self.reserve_clips <= 100
            && self.downed_height >= game.tuning.radius * 2.
            && self.downed_height < game.tuning.standing_height
            && game.content.perks.iter().any(|p| p.id == self.perk)
            && game
                .content
                .weapons
                .iter()
                .any(|w| w.id == self.downed_weapon)
    }
}
#[derive(Component, Default)]
pub struct PerkInventory {
    pub owned: BTreeSet<PerkId>,
    pub solo_charges: u32,
    pub solo_purchases: u32,
    pub earned_slots: u32,
}
impl PerkInventory {
    pub fn purchase_limit(&self) -> usize {
        (4 + self.earned_slots.min(4)) as usize
    }
    pub fn can_buy_solo(
        &self,
        profile: &SoloReviveProfile,
    ) -> Result<(), crate::interactions::InteractionRejection> {
        use crate::interactions::InteractionRejection;
        if self.owned.contains(&profile.perk) || self.solo_charges > 0 {
            return Err(InteractionRejection::Duplicate);
        }
        if self.solo_purchases >= profile.maximum_purchases {
            return Err(InteractionRejection::Unavailable);
        }
        Ok(())
    }
    pub fn buy_solo(
        &mut self,
        profile: &SoloReviveProfile,
    ) -> Result<(), crate::interactions::InteractionRejection> {
        self.can_buy_solo(profile)?;
        self.owned.insert(profile.perk.clone());
        self.solo_charges = 1;
        self.solo_purchases += 1;
        Ok(())
    }
}
#[derive(Message, Clone)]
pub struct PlayerDowned {
    pub target: Entity,
    pub attacker: Entity,
    pub episode: DownedEpisode,
}
#[derive(Message)]
pub(crate) struct RemoteRevive {
    pub player: Entity,
}
#[derive(Component)]
pub struct SoloEpisode {
    pub remaining: f32,
    pub elapsed: f32,
    pub episode: DownedEpisode,
    attacker: Entity,
    token: Option<SelfReviveToken>,
    pending: Option<u64>,
    backup: WeaponInventory,
    bleedout: BleedoutClock,
    remote: bool,
}
fn submit(queue: &mut LifeCommandQueue, target: Entity, command: LifeCommand) -> Option<u64> {
    match queue.submit_next(target, command) {
        Ok(id) => Some(id),
        Err(error) => {
            error!("Solo life command rejected for {target:?}: {error}");
            None
        }
    }
}
#[derive(Resource, Default)]
pub struct SoloReviveEvidence {
    pub events: Vec<serde_json::Value>,
}
impl SoloReviveEvidence {
    fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event);
        }
    }
}
pub struct SoloRevivePlugin;
impl Plugin for SoloRevivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerDowned>()
            .add_message::<RemoteRevive>()
            .init_resource::<SoloReviveEvidence>()
            .configure_sets(
                FixedUpdate,
                ApplyLifeCommands
                    .in_set(GameStep::PlayerLife)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                eligibility
                    .in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (begin, tick)
                    .chain()
                    .in_set(GameStep::PlayerLife)
                    .before(ApplyLifeCommands)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (finish, retire)
                    .chain()
                    .in_set(GameStep::PlayerLife)
                    .after(ApplyLifeCommands)
                    .before(HealthUpdate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
pub(crate) fn eligibility(
    definition: Res<GameDefinition>,
    drone: Res<crate::buildables::BuildableState>,
    mut players: Query<(&PerkInventory, &mut Vitality), With<Player>>,
) {
    for (perks, mut health) in &mut players {
        if health.state() == LifeState::Standing {
            health.set_player_policy(
                if (perks.solo_charges > 0 && perks.owned.contains(&definition.solo_revive.perk))
                    || drone.drone.is_some()
                {
                    PlayerLethalPolicy::SoloSelfRevive
                } else {
                    PlayerLethalPolicy::SoloOrLastEligiblePlayer
                },
            );
        }
    }
}
fn begin(
    mut events: MessageReader<PlayerDowned>,
    mut commands: Commands,
    definition: Res<GameDefinition>,
    mut queue: ResMut<LifeCommandQueue>,
    mut players: Query<(&Vitality, &mut PerkInventory, &mut WeaponInventory), With<Player>>,
    mut evidence: ResMut<SoloReviveEvidence>,
) {
    for event in events.read() {
        let Ok((health, mut perks, mut inventory)) = players.get_mut(event.target) else {
            continue;
        };
        if health.downed_episode() != Some(event.episode) {
            continue;
        }
        let profile = &definition.solo_revive;
        let weapon = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == profile.downed_weapon)
            .expect("validated downed weapon");
        inventory.cancel_action();
        let lost_weapon = inventory.set_third_slot(false, &definition.content.weapons);
        let backup = std::mem::replace(
            &mut *inventory,
            WeaponInventory::temporary(weapon, profile.reserve_clips),
        );
        let automatic =
            perks.solo_charges > 0 && perks.owned.contains(&definition.solo_revive.perk);
        perks.owned.clear();
        let timing = BleedoutTiming::new(
            Duration::from_secs_f32(profile.bleedout_seconds),
            Duration::from_secs_f32(profile.bleedout_poll_seconds),
        )
        .expect("validated bleedout timing");
        let bleedout =
            BleedoutClock::new(event.target, *health, timing).expect("admitted solo episode");
        let pending = submit(
            &mut queue,
            event.target,
            LifeCommand::BeginSelfRevive {
                episode: event.episode,
            },
        );
        commands.entity(event.target).insert(SoloEpisode {
            remaining: if automatic {
                profile.seconds
            } else {
                f32::INFINITY
            },
            elapsed: 0.,
            episode: event.episode,
            attacker: event.attacker,
            token: None,
            pending,
            backup,
            bleedout,
            remote: false,
        });
        evidence.record(serde_json::json!({"event":"downed","target":event.target.to_bits(),"episode":event.episode.number(),"health":health.current(),"weapon":weapon.id.0,"magazine":weapon.magazine,"reserve":inventory.reserve(weapon),"perks_lost":true,"lost_third_weapon":lost_weapon.map(|w|w.definition.0)}));
    }
}
fn tick(
    time: Res<Time<Fixed>>,
    mut remote: MessageReader<RemoteRevive>,
    definition: Res<GameDefinition>,
    mut queue: ResMut<LifeCommandQueue>,
    mut players: Query<(Entity, &Vitality, &mut SoloEpisode), With<Player>>,
) {
    let requested: std::collections::HashSet<_> = remote.read().map(|event| event.player).collect();
    for (entity, health, mut episode) in &mut players {
        if requested.contains(&entity) {
            episode.remote = true;
            episode.remaining = 0.;
        }
        if health.downed_episode() != Some(episode.episode) {
            continue;
        }
        episode.elapsed += time.delta_secs();
        episode.remaining = (episode.remaining - time.delta_secs()).max(0.);
        let due = episode
            .bleedout
            .tick(entity, Some(*health), time.delta())
            .ok()
            .flatten();
        if episode.pending.is_some() {
            continue;
        }
        let command = if due.is_some() {
            due.map(LifeCommand::ExpireDowned)
        } else if episode.remaining == 0. {
            episode.token.map(|token| LifeCommand::CompleteSelfRevive {
                token,
                restore: ReviveRestoration::new(
                    definition.player_health.maximum,
                    definition.player_health.maximum,
                )
                .expect("validated maximum"),
            })
        } else {
            None
        };
        if let Some(command) = command {
            episode.pending = submit(&mut queue, entity, command);
        }
    }
}
pub(crate) fn finish(
    definition: Res<GameDefinition>,
    mut results: ResMut<LifeCommandResults>,
    mut commands: Commands,
    mut queue: ResMut<LifeCommandQueue>,
    mut players: Query<
        (
            &Vitality,
            &mut SoloEpisode,
            &mut PerkInventory,
            &mut WeaponInventory,
            &mut Recovery,
        ),
        With<Player>,
    >,
    mut deaths: MessageWriter<PlayerDeath>,
    mut evidence: ResMut<SoloReviveEvidence>,
) {
    for result in results.drain() {
        let Ok((health, mut episode, mut perks, mut inventory, mut recovery)) =
            players.get_mut(result.target)
        else {
            continue;
        };
        if episode.pending != Some(result.id) {
            continue;
        }
        episode.pending = None;
        match result.result {
            Ok(LifeCommandOutcome::SelfReviveBegan(token)) => {
                episode.token = Some(token);
            }
            Ok(LifeCommandOutcome::Revived { current, maximum }) => {
                *inventory = std::mem::take(&mut episode.backup);
                inventory.cancel_action();
                let _ = inventory.set_third_slot(false, &definition.content.weapons);
                if !episode.remote {
                    perks.solo_charges = perks.solo_charges.saturating_sub(1);
                }
                *recovery = Recovery::default();
                queue.retire_pending_target(result.target);
                evidence.record(serde_json::json!({"event":"revived","target":result.target.to_bits(),"episode":episode.episode.number(),"seconds":episode.elapsed,"health":current,"maximum":maximum,"charges":perks.solo_charges,"weapon":inventory.active().map(|w|w.definition.0.clone())}));
                commands.entity(result.target).remove::<SoloEpisode>();
            }
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated { .. })) => {
                queue.retire_pending_target(result.target);
                deaths.write(PlayerDeath {
                    target: result.target,
                    attacker: episode.attacker,
                });
                commands.entity(result.target).remove::<SoloEpisode>();
                evidence.record(
                    serde_json::json!({"event":"bled_out","target":result.target.to_bits(),"seconds":episode.elapsed,"episode":episode.episode.number()}),
                );
            }
            other => {
                evidence.record(serde_json::json!({"event":"command_rejected","target":result.target.to_bits(),"result":format!("{other:?}"),"state":format!("{:?}",health.state())}));
                episode.token = None;
            }
        }
    }
}

fn retire(
    mut commands: Commands,
    mut queue: ResMut<LifeCommandQueue>,
    players: Query<(Entity, &Vitality, &SoloEpisode), With<Player>>,
) {
    for (entity, health, episode) in &players {
        if health.downed_episode() != Some(episode.episode) {
            queue.retire_pending_target(entity);
            commands.entity(entity).remove::<SoloEpisode>();
        }
    }
}
