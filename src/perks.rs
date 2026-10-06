//! Purchased bottle actions keep primary ownership separate from visible presentation.
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::GameStep,
    interactions::{InteractionRejection, Usable},
    life::{LifeState, Vitality},
    origins::{PerkDefinition, PerkId, WeaponAnimation, WeaponId},
    player::Player,
    solo_revive::PerkInventory,
    weapons::WeaponInventory,
};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrinkPhase {
    Drinking,
    Lowering,
    Returning,
}
#[derive(Component)]
pub struct PerkDrinking {
    pub perk: PerkId,
    pub bottle: WeaponId,
    pub phase: DrinkPhase,
    pub remaining: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub activated: bool,
    activation_at: f32,
    lower_seconds: f32,
    return_weapon: WeaponId,
    return_seconds: f32,
}
impl PerkDrinking {
    pub fn new(
        perk: &PerkDefinition,
        inventory: &WeaponInventory,
        game: &GameDefinition,
    ) -> Result<Self, InteractionRejection> {
        let drink = perk
            .drink
            .as_ref()
            .ok_or(InteractionRejection::Unavailable)?;
        let primary = inventory
            .definition(&game.content.weapons)
            .ok_or(InteractionRejection::Unavailable)?;
        Ok(Self {
            perk: perk.id.clone(),
            bottle: drink.bottle.clone(),
            phase: DrinkPhase::Drinking,
            remaining: drink.seconds,
            duration: drink.seconds,
            elapsed: 0.,
            activated: false,
            activation_at: drink.seconds + drink.activation_delay,
            lower_seconds: drink.lower_seconds,
            return_weapon: primary.id.clone(),
            return_seconds: primary.mechanics.equip_seconds,
        })
    }
    pub(crate) fn selection(&self) -> crate::presentation::offhand::Selection {
        crate::presentation::offhand::Selection {
            weapon: if self.phase == DrinkPhase::Returning {
                self.return_weapon.clone()
            } else {
                self.bottle.clone()
            },
            action: if self.phase == DrinkPhase::Lowering {
                WeaponAnimation::Unequip
            } else {
                WeaponAnimation::Equip
            },
            // The retained lower asset aliases the entire drink animation. Hold
            // its completed pose instead of replaying opening/swallowing twice.
            timing: Some((
                self.duration,
                if self.phase == DrinkPhase::Lowering {
                    0.
                } else {
                    self.remaining
                },
            )),
        }
    }
}
#[derive(Resource, Default)]
pub struct PerkEvidence {
    pub events: Vec<serde_json::Value>,
}
impl PerkEvidence {
    fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event);
        }
    }
}
pub struct PerkPlugin;
impl Plugin for PerkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PerkEvidence>()
            .add_systems(
                FixedUpdate,
                tick.in_set(GameStep::Interactions)
                    .before(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                update_maximum
                    .in_set(GameStep::Interactions)
                    .after(tick)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                update_maximum
                    .in_set(GameStep::PlayerLife)
                    .after(crate::solo_revive::finish)
                    .before(crate::player_health::HealthUpdate)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                cancel
                    .in_set(GameStep::PlayerLife)
                    .before(crate::life::ApplyLifeCommands)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut players: Query<
        (
            Entity,
            &Vitality,
            &mut PerkDrinking,
            &mut PerkInventory,
            &mut WeaponInventory,
        ),
        With<Player>,
    >,
    machines: Query<(Entity, &Usable)>,
    mut evidence: ResMut<PerkEvidence>,
) {
    for (owner, health, mut drink, mut perks, mut inventory) in &mut players {
        if health.state() != LifeState::Standing {
            continue;
        }
        if drink.elapsed == 0. {
            evidence.record(serde_json::json!({"event":"drink_started","owner":owner.to_bits(),"perk":drink.perk.0,"seconds":drink.duration,"activation_at":drink.activation_at}));
        }
        drink.elapsed += time.delta_secs();
        drink.remaining = (drink.remaining - time.delta_secs()).max(0.);
        if !drink.activated && drink.elapsed + 0.00001 >= drink.activation_at {
            let accepted = if drink.perk == game.solo_revive.perk {
                perks.buy_solo(&game.solo_revive).is_ok()
            } else {
                game.content
                    .perks
                    .iter()
                    .any(|p| p.id == drink.perk && p.maximum_health.is_some())
                    && perks.owned.insert(drink.perk.clone())
            };
            if accepted {
                drink.activated = true;
                evidence.record(serde_json::json!({"event":"perk_activated","owner":owner.to_bits(),"perk":drink.perk.0,"seconds":drink.elapsed,"purchases":perks.solo_purchases}));
                if drink.perk == game.solo_revive.perk
                    && perks.solo_purchases >= game.solo_revive.maximum_purchases
                {
                    for (entity, usable) in &machines {
                        if matches!(&usable.source.action, crate::origins::InteractionAction::Perk { perk } if *perk == drink.perk)
                        {
                            commands.entity(entity).despawn();
                        }
                    }
                }
            } else {
                evidence.record(serde_json::json!({"event":"activation_rejected","owner":owner.to_bits(),"perk":drink.perk.0}));
                commands.entity(owner).remove::<PerkDrinking>();
                continue;
            }
        }
        if drink.remaining > 0. {
            continue;
        }
        match drink.phase {
            DrinkPhase::Drinking => {
                drink.phase = DrinkPhase::Lowering;
                drink.duration = drink.lower_seconds;
                drink.remaining = drink.duration;
            }
            DrinkPhase::Lowering => {
                drink.phase = DrinkPhase::Returning;
                drink.duration = drink.return_seconds;
                drink.remaining = drink.duration;
            }
            DrinkPhase::Returning => {
                inventory.cancel_action();
                evidence.record(serde_json::json!({"event":"drink_finished","owner":owner.to_bits(),"perk":drink.perk.0,"seconds":drink.elapsed}));
                commands.entity(owner).remove::<PerkDrinking>();
            }
        }
    }
}
fn update_maximum(
    game: Res<GameDefinition>,
    mut players: Query<(Entity, &PerkInventory, &mut Vitality), With<Player>>,
    mut evidence: ResMut<PerkEvidence>,
) {
    for (owner, perks, mut health) in &mut players {
        let maximum = game
            .content
            .perks
            .iter()
            .filter(|p| perks.owned.contains(&p.id))
            .filter_map(|p| p.maximum_health)
            .max()
            .unwrap_or(game.player_health.maximum);
        if health.maximum() != maximum {
            let before = health.maximum();
            match health.set_human_maximum(maximum) {
                Ok(()) => evidence.record(serde_json::json!({"event":"maximum_changed","owner":owner.to_bits(),"before":before,"maximum":maximum,"current":health.current(),"state":format!("{:?}",health.state())})),
                Err(error) => error!("Perk maximum rejected for {owner:?}: {error}"),
            }
        }
    }
}
fn cancel(
    mut commands: Commands,
    players: Query<(Entity, &Vitality, &PerkDrinking), With<Player>>,
    mut evidence: ResMut<PerkEvidence>,
) {
    for (owner, health, drink) in &players {
        if health.state() != LifeState::Standing {
            evidence.record(serde_json::json!({"event":"drink_cancelled","owner":owner.to_bits(),"perk":drink.perk.0,"activated":drink.activated,"seconds":drink.elapsed}));
            commands.entity(owner).remove::<PerkDrinking>();
        }
    }
}
