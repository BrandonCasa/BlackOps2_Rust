//! Fixed-step consumers of actual action, damage, movement, and round state.
use super::*;
use crate::{
    gameplay::{GameStep, Session},
    life::{LifeState, Vitality},
    origins::{EnemyAnimation, WeaponAnimation},
    player::Player,
    weapons::{WeaponInventory, WeaponSignal},
};
use avian3d::prelude::*;

#[derive(Default)]
struct ActionClock {
    key: String,
    sequence: u64,
    age: f32,
    sampled: f32,
}
#[derive(Default)]
struct ActorClock {
    position: Option<Vec3>,
    distance: f32,
    vocal: f32,
    action: Option<EnemyAnimation>,
    elapsed: f32,
}
#[derive(Resource, Default)]
struct AudioState {
    player: Option<Entity>,
    actions: BTreeMap<Entity, ActionClock>,
    actors: BTreeMap<Entity, ActorClock>,
    loops: BTreeMap<String, Entity>,
    perks: BTreeSet<crate::origins::PerkId>,
    round: u32,
    intermission: bool,
    critical: bool,
    downed: bool,
    position: Option<Vec3>,
    crawl_distance: f32,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<AudioState>().add_systems(
        FixedUpdate,
        (actions, actors, world, feedback)
            .chain()
            .after(GameStep::Rounds)
            .run_if(in_state(GameState::Playing)),
    );
}
pub(crate) fn voice(
    commands: &mut Commands,
    game: &GameDefinition,
    event: &str,
    position: Vec3,
    index: u32,
) {
    let prefix = format!("vox_plr_{}_{}", game.audio.character, event);
    let choices: Vec<_> = game
        .audio
        .aliases
        .keys()
        .filter(|n| n.starts_with(&prefix))
        .collect();
    if !choices.is_empty() {
        emit(
            commands,
            choices[index as usize % choices.len()].clone(),
            position,
            None,
        );
    }
}
fn actions(
    mut commands: Commands,
    game: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    mut state: ResMut<AudioState>,
    players: Query<
        (
            Entity,
            &Transform,
            &WeaponInventory,
            &crate::equipment::GrenadeInventory,
            &crate::equipment::TacticalInventory,
            &crate::mines::MineInventory,
            Option<&crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
) {
    state.actions.retain(|e, _| players.contains(*e));
    for (owner, pose, inventory, lethal, tactical, mines, drinking) in &players {
        let selected = drinking
            .map(|d| d.selection())
            .or_else(|| crate::presentation::offhand::selection(lethal, tactical, mines, &game));
        let Some(id) = selected
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition))
        else {
            continue;
        };
        let Some(weapon) = game.content.weapons.iter().find(|w| w.id == *id) else {
            continue;
        };
        let (action, shot_seek) = selected.as_ref().map_or_else(
            || crate::presentation::primary_action(inventory, weapon, &game),
            |s| (s.action, None),
        );
        let animation_weapon = if action == WeaponAnimation::Melee {
            game.content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.knife_zm")
                .unwrap_or(weapon)
        } else {
            weapon
        };
        let Some(clip) = animation_weapon.animation_actions.get(&action) else {
            continue;
        };
        let clock = state.actions.entry(owner).or_default();
        let sequence = if selected.is_some() {
            0
        } else {
            inventory.sequence
        };
        let timing = selected
            .as_ref()
            .and_then(|s| s.timing)
            .or_else(|| inventory.presentation_timing(weapon, animation_weapon));
        let backwards =
            timing.is_some_and(|(duration, remaining)| duration - remaining + 0.001 < clock.age);
        let finishing_shot =
            selected.is_none() && shot_seek.is_some() && inventory.signal == WeaponSignal::Idle;
        let changed =
            clock.key != clip.0 || (clock.sequence != sequence && !finishing_shot) || backwards;
        if changed {
            clock.key = clip.0.clone();
            clock.sequence = sequence;
            clock.age = 0.;
            clock.sampled = -0.001;
            let event = match action {
                WeaponAnimation::Equip => Some("equip"),
                WeaponAnimation::Unequip => Some("unequip"),
                WeaponAnimation::Prime => Some("prime"),
                WeaponAnimation::Melee => Some("melee"),
                WeaponAnimation::Fire if selected.is_some() => Some("throw"),
                _ => None,
            };
            if let Some(event) = event {
                binding(
                    &mut commands,
                    &game,
                    &format!("{}.{}", weapon.id.0, event),
                    pose.translation,
                    None,
                );
            }
            if inventory.signal == WeaponSignal::DryFire && selected.is_none() {
                binding(
                    &mut commands,
                    &game,
                    &format!("{}.dry_fire", weapon.id.0),
                    pose.translation,
                    None,
                );
                voice(
                    &mut commands,
                    &game,
                    "ammo_out",
                    pose.translation,
                    sequence as u32,
                );
            }
        }
        let native_duration = game
            .content
            .animations
            .iter()
            .find(|a| a.id == *clip)
            .and_then(|a| a.notifications.last())
            .map_or(1., |n| n.seconds);
        let sampled = if let Some((duration, remaining)) = timing {
            clock.age = (duration - remaining).max(0.);
            clock.age / duration * native_duration
        } else if selected.is_none() && shot_seek.is_some() {
            clock.age = shot_seek.unwrap_or(0.);
            clock.age.min(native_duration)
        } else {
            clock.age += time.delta_secs();
            clock.age
        };
        // One timeline per owner, independent of duplicated arms/model scene players.
        if let Some(notes) = game.audio.timelines.get(&clip.0) {
            for note in notes
                .iter()
                .filter(|n| n.seconds > clock.sampled && n.seconds <= sampled)
            {
                emit(&mut commands, note.alias.clone(), pose.translation, None);
            }
        }
        clock.sampled = sampled;
        clock.sequence = sequence;
    }
}
fn actors(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<AudioState>,
    zombies: Query<(
        Entity,
        &crate::enemies::Zombie,
        &Transform,
        &Vitality,
        Option<&crate::generators::GeneratorDefender>,
    )>,
    query: SpatialQuery,
    surfaces: Query<&SurfaceSound>,
) {
    state.actors.retain(|e, _| zombies.contains(*e));
    for (owner, zombie, pose, health, defender) in &zombies {
        let new_actor = !state.actors.contains_key(&owner);
        let clock = state.actors.entry(owner).or_default();
        if new_actor && defender.is_some() {
            emit(
                &mut commands,
                "zmb_vocals_capzomb_spawn",
                pose.translation,
                Some(owner),
            );
        }
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        let moving = matches!(
            zombie.action,
            EnemyAnimation::Walk | EnemyAnimation::Run | EnemyAnimation::Sprint
        );
        let distance = clock
            .position
            .map_or(0., |p| p.xz().distance(pose.translation.xz()));
        clock.position = Some(pose.translation);
        if moving && distance < 0.5 {
            clock.distance += distance;
            if clock.distance >= 0.75 {
                clock.distance %= 0.75;
                let surface = ground_surface(&query, &surfaces, pose.translation);
                let gait = match zombie.action {
                    EnemyAnimation::Run => "run",
                    EnemyAnimation::Sprint => "sprint",
                    _ => "walk",
                };
                emit(
                    &mut commands,
                    format!("fly_step_{gait}_npc_{surface}"),
                    pose.translation,
                    Some(owner),
                );
            }
        }
        if clock.action != Some(zombie.action) || zombie.elapsed < clock.elapsed {
            if zombie.action == EnemyAnimation::Attack {
                emit(
                    &mut commands,
                    "zmb_attack_whoosh",
                    pose.translation,
                    Some(owner),
                );
            }
            let alias = match zombie.action {
                EnemyAnimation::Attack => Some(if defender.is_some() {
                    "zmb_vocals_capzomb_attack"
                } else {
                    "zmb_vocals_zombie_attack"
                }),
                EnemyAnimation::Tear => Some("zmb_vocals_zombie_teardown"),
                _ => None,
            };
            if let Some(alias) = alias {
                emit(&mut commands, alias, pose.translation, Some(owner));
            }
        }
        clock.action = Some(zombie.action);
        clock.elapsed = zombie.elapsed;
        clock.vocal -= time.delta_secs();
        if clock.vocal <= 0. {
            emit(
                &mut commands,
                match (defender.is_some(), zombie.action == EnemyAnimation::Sprint) {
                    (true, true) => "zmb_vocals_capzomb_sprint",
                    (true, false) => "zmb_vocals_capzomb_ambience",
                    (false, true) => "zmb_vocals_zombie_sprint",
                    (false, false) => "zmb_vocals_zombie_ambience",
                },
                pose.translation,
                Some(owner),
            );
            clock.vocal = 4. + (owner.to_bits() % 37) as f32 * 0.1;
        }
    }
}
fn world(
    mut commands: Commands,
    game: Res<GameDefinition>,
    session: Res<Session>,
    mut state: ResMut<AudioState>,
    players: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            &crate::player_health::Recovery,
            &crate::solo_revive::PerkInventory,
            &Player,
        ),
        With<Player>,
    >,
    bombs: Query<(Entity, &crate::equipment::Grenade, &Transform)>,
    exists: Query<Entity>,
    query: SpatialQuery,
    surfaces: Query<&SurfaceSound>,
) {
    let Ok((player, pose, health, recovery, perks, body)) = players.single() else {
        return;
    };
    if state.player != Some(player) {
        for entity in state.loops.values() {
            if exists.contains(*entity) {
                commands.entity(*entity).despawn();
            }
        }
        *state = AudioState {
            player: Some(player),
            ..default()
        };
    }
    let position = pose.translation;
    let distance = state
        .position
        .map_or(0., |p| p.xz().distance(position.xz()));
    state.position = Some(position);
    if (body.prone || health.state() == LifeState::Downed) && body.grounded && distance < 0.5 {
        state.crawl_distance += distance;
        if state.crawl_distance >= 0.6 {
            state.crawl_distance %= 0.6;
            let surface = ground_surface(&query, &surfaces, position);
            let alias = if surface == "water" {
                "fly_prone_water".to_owned()
            } else {
                format!("fly_prone_{surface}_plr")
            };
            if game.audio.aliases.contains_key(&alias) {
                emit(&mut commands, alias, position, None);
            }
        }
    } else {
        state.crawl_distance = 0.;
    }
    let mut wanted = BTreeMap::<String, (String, Vec3, Option<Entity>)>::new();
    let critical = recovery.critical || health.state() == LifeState::Downed;
    if critical {
        wanted.insert(
            "player.critical".into(),
            ("chr_breathing_hurt".into(), position, Some(player)),
        );
        if !state.critical {
            emit(&mut commands, "chr_breathing_hurt_start", position, None);
        }
    } else if state.critical {
        emit(&mut commands, "chr_breathing_better", position, None);
    }
    state.critical = critical;
    let downed = health.state() == LifeState::Downed;
    if state.downed && !downed {
        emit(&mut commands, "chr_revive_gasp", position, None);
        voice(
            &mut commands,
            &game,
            "heal_revived",
            position,
            session.kills,
        );
    }
    state.downed = downed;
    if state.round != session.round {
        emit(&mut commands, "mus_zombie_round_start", position, None);
        state.round = session.round;
        if session.round == 1 {
            voice(&mut commands, &game, "during_round_1", position, 0);
        }
    }
    if session.intermission.is_some() && !state.intermission {
        emit(&mut commands, "mus_zombie_round_over", position, None);
        if session.round == 1 {
            voice(&mut commands, &game, "end_round_1", position, session.round);
        }
    }
    state.intermission = session.intermission.is_some();
    for perk in game
        .content
        .perks
        .iter()
        .filter(|p| perks.owned.contains(&p.id) && !state.perks.contains(&p.id))
    {
        let native = perk_music(&perk.id.0);
        if let Some(native) = native {
            emit(
                &mut commands,
                format!("mus_perks_{native}_sting"),
                position,
                None,
            );
            let event = match native {
                "jugganog" => Some("perk_jugga"),
                "mulekick" => Some("perk_mule"),
                "revive" => Some("perk_revive"),
                "speed" => Some("perk_speed"),
                "stamin" => Some("perk_stamine"),
                _ => None,
            };
            if let Some(event) = event {
                voice(&mut commands, &game, event, position, 0);
            }
        }
    }
    state.perks = perks.owned.clone();
    // Place loops only at native machine placements that are actually implemented.
    for placement in &game.content.placements {
        if placement.definition.0 == "perk.quick_revive"
            && perks.solo_purchases >= game.solo_revive.maximum_purchases
            && !perks.owned.contains(&game.solo_revive.perk)
        {
            continue;
        }
        if !game
            .content
            .perks
            .iter()
            .any(|p| p.id.0 == placement.definition.0)
        {
            continue;
        }
        let Some(native) = perk_music(&placement.definition.0) else {
            continue;
        };
        let p = Vec3::from_array(placement.position);
        if p.distance(position) < 10. {
            wanted.insert(
                placement.id.0.clone(),
                (format!("mus_perks_{native}_jingle"), p, None),
            );
        }
    }
    for (i, emitter) in game.audio.emitters.iter().enumerate() {
        let p = Vec3::from_array(emitter.position);
        let range = game
            .audio
            .aliases
            .get(&emitter.alias)
            .and_then(|a| a.sounds.first())
            .and_then(|id| game.content.sounds.iter().find(|s| s.id == *id))
            .map_or(20., |s| s.distance);
        if p.distance(position) < range {
            wanted.insert(format!("ambient.{i}"), (emitter.alias.clone(), p, None));
        }
    }
    // Current procedural sky is rainy throughout: source outdoor rain bed follows it.
    wanted.insert(
        "weather".into(),
        ("amb_weather_2d_rain_outdoor".into(), position, None),
    );
    for (e, bomb, p) in &bombs {
        if bomb.lure_active {
            wanted.insert(
                format!("monkey.{e:?}"),
                ("zmb_monkey_song".into(), p.translation, Some(e)),
            );
        } else if bomb.definition.0 == "weapon.sticky_grenade_zm" {
            wanted.insert(
                format!("semtex.{e:?}"),
                ("wpn_semtex_charge".into(), p.translation, Some(e)),
            );
        }
    }
    state.loops.retain(|key, e| {
        let keep = wanted.contains_key(key) && exists.contains(*e);
        if !keep && exists.contains(*e) {
            commands.entity(*e).despawn();
        }
        keep
    });
    for (key, (alias, p, owner)) in wanted {
        if !state.loops.contains_key(&key) && game.audio.aliases.contains_key(&alias) {
            let entity = emit_loop(&mut commands, alias, p, owner);
            state.loops.insert(key, entity);
        }
    }
}
fn perk_music(id: &str) -> Option<&'static str> {
    Some(match id {
        "perk.quick_revive" => "revive",
        "perk.juggernog" => "jugganog",
        "perk.speed_cola" => "speed",
        "perk.stamin_up" => "stamin",
        "perk.mule_kick" => "mulekick",
        "perk.double_tap" => "doubletap",
        "perk.deadshot" => "deadshot",
        "perk.electric_cherry" => "cherry",
        "perk.phd_flopper" => "phdflopper",
        _ => return None,
    })
}
fn feedback(
    mut commands: Commands,
    game: Res<GameDefinition>,
    session: Res<Session>,
    mut steps: MessageReader<crate::environment::Footstep>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    mut damages: MessageReader<crate::player_health::PlayerDamage>,
    mut applied: MessageReader<crate::damage::DamageApplied>,
    poses: Query<&Transform>,
    defenders: Query<(), With<crate::generators::GeneratorDefender>>,
    players: Query<(&crate::movement::SprintState, &Player), With<Player>>,
    query: SpatialQuery,
    surfaces: Query<&SurfaceSound>,
) {
    for damage in applied.read() {
        if matches!(
            damage.impact.category,
            crate::origins::DamageCategory::Melee
        ) && let Ok(pose) = poses.get(damage.impact.target)
        {
            binding(
                &mut commands,
                &game,
                "weapon.knife_zm.hit",
                pose.translation,
                None,
            );
        }
    }
    for step in steps.read() {
        let surface = match step.surface {
            crate::environment::FootstepSurface::Mud => "mud",
            _ => ground_surface(&query, &surfaces, step.position + Vec3::Y * 0.25),
        };
        let gait = if players.get(step.owner).is_ok_and(|(s, _)| s.sprinting) {
            "sprint"
        } else if players.get(step.owner).is_ok_and(|(_, p)| p.crouched) {
            "crouch_walk"
        } else {
            "walk"
        };
        emit(
            &mut commands,
            format!("fly_step_{gait}_plr_{surface}"),
            step.position,
            None,
        );
    }
    for crate::damage::EnemyDeath(receipt) in deaths.read() {
        let position = poses
            .get(receipt.impact.target)
            .map_or(Vec3::ZERO, |p| p.translation);
        emit(
            &mut commands,
            if defenders.contains(receipt.impact.target) {
                "zmb_vocals_capzomb_death"
            } else {
                "zmb_vocals_zombie_death"
            },
            position,
            None,
        );
        let event = if receipt.impact.region == crate::weapons::HitRegion::Head {
            Some("kill_headshot")
        } else if matches!(
            receipt.impact.source,
            crate::damage::ImpactSource::Projectile(_)
        ) {
            Some("kill_explo")
        } else if matches!(
            receipt.impact.category,
            crate::origins::DamageCategory::Melee
        ) {
            Some("kill_close")
        } else {
            None
        };
        if let Some(event) = event {
            voice(&mut commands, &game, event, position, session.kills);
        }
    }
    for damage in damages.read() {
        if let Ok(pose) = poses.get(damage.impact.target) {
            voice(
                &mut commands,
                &game,
                if damage.after < 25 {
                    "exert_pain_high"
                } else {
                    "exert_pain_low"
                },
                pose.translation,
                session.shots,
            );
        }
    }
}

fn ground_surface<'a>(
    query: &SpatialQuery,
    surfaces: &'a Query<&SurfaceSound>,
    position: Vec3,
) -> &'a str {
    query
        .cast_ray_predicate(
            position,
            Dir3::NEG_Y,
            2.,
            true,
            &crate::environment::solids([]),
            &|e| surfaces.contains(e),
        )
        .and_then(|hit| surfaces.get(hit.entity).ok())
        .map_or("default", |s| s.0.as_str())
}
