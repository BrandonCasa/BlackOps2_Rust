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
pub struct SoloReviveProfile {
    pub perk: PerkId,
    pub seconds: f32,
    pub bleedout_seconds: f32,
    pub bleedout_poll_seconds: f32,
    pub maximum_purchases: u32,
    pub downed_weapon: WeaponId,
    pub reserve_clips: u32,
    pub pistol_ranks: Vec<PistolRank>,
    pub pistol_replace_below: u32,
    pub downed_height: f32,
    pub downed_eye: f32,
    pub downed_speed: f32,
}
#[derive(Clone, Deserialize)]
pub struct PistolRank {
    pub weapon: WeaponId,
    pub rank: u32,
}
impl SoloReviveProfile {
    fn choose_weapon(&self, inventory: &WeaponInventory) -> &WeaponId {
        let mut best: Option<&PistolRank> = None;
        for (_, weapon) in inventory.owned() {
            if let Some(pistol) = self
                .pistol_ranks
                .iter()
                .find(|p| p.weapon == weapon.definition)
                && best.is_none_or(|previous| pistol.rank > previous.rank)
            {
                best = Some(pistol);
            }
        }
        best.filter(|p| p.rank > self.pistol_replace_below)
            .map_or(&self.downed_weapon, |p| &p.weapon)
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
    pub episode: DownedEpisode,
    attacker: Entity,
    token: SelfReviveToken,
    backup: WeaponInventory,
    downed_weapon: WeaponId,
    given_ammo: u32,
    bleedout: BleedoutClock,
    remote: bool,
}
pub(crate) fn reset(world: &mut World) {
    world.resource_mut::<Messages<RemoteRevive>>().clear();
    world.insert_resource(crate::player_health::HealthEvidence::default());
    world.insert_resource(crate::perks::PerkEvidence::default());
    world.insert_resource(crate::movement::MovementEvidence::default());
    world.insert_resource(crate::movement::physics::PhysicsEvidence::default());
}
pub struct SoloRevivePlugin;
impl Plugin for SoloRevivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerDowned>()
            .add_message::<RemoteRevive>()
            .add_systems(
                FixedUpdate,
                eligibility
                    .in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (begin, finish, retire)
                    .chain()
                    .in_set(GameStep::PlayerLife)
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
    mut players: Query<(&mut Vitality, &mut PerkInventory, &mut WeaponInventory), With<Player>>,
) {
    for event in events.read() {
        let Ok((mut health, mut perks, mut inventory)) = players.get_mut(event.target) else {
            continue;
        };
        if health.downed_episode() != Some(event.episode) {
            continue;
        }
        let profile = &definition.solo_revive;
        inventory.cancel_action();
        inventory.set_third_slot(false, &definition.content.weapons);
        let weapon_id = profile.choose_weapon(&inventory);
        let weapon = definition
            .content
            .weapons
            .iter()
            .find(|w| &w.id == weapon_id)
            .expect("validated downed weapon");
        let (downed, given_ammo) = inventory.last_stand(
            weapon,
            profile.reserve_clips,
            weapon.id == profile.downed_weapon,
        );
        let backup = std::mem::replace(&mut *inventory, downed);
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
        let token = health
            .begin_self_revive(event.target, event.episode)
            .expect("Downed solo player can begin their revival");
        commands.entity(event.target).insert(SoloEpisode {
            remaining: if automatic {
                profile.seconds
            } else {
                f32::INFINITY
            },
            episode: event.episode,
            attacker: event.attacker,
            token,
            backup,
            downed_weapon: weapon.id.clone(),
            given_ammo,
            bleedout,
            remote: false,
        });
    }
}
pub(crate) fn finish(
    time: Res<Time<Fixed>>,
    mut remote: MessageReader<RemoteRevive>,
    definition: Res<GameDefinition>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut Vitality,
            &mut SoloEpisode,
            &mut PerkInventory,
            &mut WeaponInventory,
            &mut Recovery,
        ),
        With<Player>,
    >,
    mut deaths: MessageWriter<PlayerDeath>,
) {
    for request in remote.read() {
        if let Ok((_, _, mut episode, _, _, _)) = players.get_mut(request.player) {
            episode.remote = true;
            episode.remaining = 0.;
        }
    }
    for (entity, mut health, mut episode, mut perks, mut inventory, mut recovery) in &mut players {
        if health.downed_episode() != Some(episode.episode) {
            continue;
        }
        episode.remaining = (episode.remaining - time.delta_secs()).max(0.);
        let due = episode
            .bleedout
            .tick(entity, Some(*health), time.delta())
            .expect("Bleedout belongs to the current downed episode");
        // Expiry wins when the bleedout and revival deadlines share a gameplay tick.
        if due.is_some() {
            if matches!(
                episode.bleedout.expire(entity, &mut health),
                ExpiryOutcome::Terminated { .. }
            ) {
                deaths.write(PlayerDeath {
                    target: entity,
                    attacker: episode.attacker,
                });
                commands.entity(entity).remove::<SoloEpisode>();
            }
            continue;
        }
        if episode.remaining != 0. {
            continue;
        }
        let maximum = definition.player_health.maximum;
        health
            .complete_self_revive(
                entity,
                episode.token,
                ReviveRestoration::new(maximum, maximum).expect("Source player maximum is valid"),
            )
            .expect("Revival belongs to the current downed episode");
        let weapon = definition
            .content
            .weapons
            .iter()
            .find(|weapon| weapon.id == episode.downed_weapon)
            .expect("Downed weapon remains in the source weapon catalog");
        let remaining = inventory.total_ammunition(weapon);
        let used = episode.given_ammo.saturating_sub(remaining);
        episode.backup.debit_ammunition(weapon, used);
        *inventory = std::mem::take(&mut episode.backup);
        inventory.cancel_action();
        inventory.set_third_slot(false, &definition.content.weapons);
        if !episode.remote {
            perks.solo_charges = perks.solo_charges.saturating_sub(1);
        }
        *recovery = Recovery::default();
        commands.entity(entity).remove::<SoloEpisode>();
    }
}

fn retire(mut commands: Commands, players: Query<(Entity, &Vitality, &SoloEpisode), With<Player>>) {
    for (entity, health, episode) in &players {
        if health.downed_episode() != Some(episode.episode) {
            commands.entity(entity).remove::<SoloEpisode>();
        }
    }
}
