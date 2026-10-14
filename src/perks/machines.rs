use crate::{
    assets::GameDefinition, gameplay::SessionEntity, interactions::Usable, origins::PerkId,
    player::Player, solo_revive::PerkInventory,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

#[derive(Component)]
pub(crate) struct PerkMachine {
    perk: PerkId,
    powered: Option<bool>,
    visual: Option<Entity>,
    origin_tag: Option<Entity>,
    sound: Option<Entity>,
    glow: Option<Entity>,
    power_age: f32,
    expiry: Option<f32>,
    bump_inside: bool,
    bump_clock: f32,
    vibration: Option<Vibration>,
    paths_disconnected: bool,
}
impl PerkMachine {
    pub(crate) fn new(perk: PerkId) -> Self {
        Self {
            perk,
            powered: None,
            visual: None,
            origin_tag: None,
            sound: None,
            glow: None,
            power_age: 0.,
            expiry: None,
            bump_inside: false,
            bump_clock: 0.,
            vibration: None,
            paths_disconnected: false,
        }
    }
}
#[derive(Component)]
pub(crate) struct PerkMachineClip;

struct Vibration {
    age: f32,
    seconds: f32,
    period: f32,
    delta: Vec2,
}
impl Vibration {
    fn new(direction: Vec3, amplitude: f32, period: f32, seconds: f32, yaw: Quat) -> Self {
        let direction = direction.normalize_or_zero() * amplitude;
        Self {
            age: 0.,
            seconds,
            period,
            delta: Vec2::new(
                direction.dot(yaw * Vec3::X),
                -direction.dot(yaw * Vec3::NEG_Z),
            ),
        }
    }
    fn rotation(&self) -> Quat {
        if self.age >= self.seconds + self.period {
            return Quat::IDENTITY;
        }
        let phase = (std::f32::consts::TAU * (self.age - self.seconds) / self.period).sin();
        Quat::from_rotation_z((-self.delta.x * phase).to_radians())
            * Quat::from_rotation_x((self.delta.y * phase).to_radians())
    }
}

pub(super) fn ready(
    event: On<WorldInstanceReady>,
    roots: Query<&ChildOf>,
    mut machines: Query<&mut PerkMachine>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(parent) = roots.get(event.entity) else {
        return;
    };
    let Ok(mut machine) = machines.get_mut(parent.parent()) else {
        return;
    };
    if machine.visual != Some(event.entity) {
        return;
    }
    machine.origin_tag = children.iter_descendants(event.entity).find(|entity| {
        names
            .get(*entity)
            .is_ok_and(|name| name.as_str() == "tag_origin")
    });
}

pub(super) fn update(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameDefinition>,
    generators: Res<crate::generators::Generators>,
    server: Res<AssetServer>,
    spatial: SpatialQuery,
    mut navigation: ResMut<crate::navigation::Navigation>,
    mut machines: Query<(Entity, &Usable, &Transform, &Children, &mut PerkMachine)>,
    players: Query<
        (
            Entity,
            &Transform,
            &Collider,
            &crate::life::Vitality,
            &PerkInventory,
            &crate::access::TriggerClientMode,
        ),
        With<Player>,
    >,
    mut triggers: Query<
        (
            Entity,
            &crate::environment::triggers::MapTrigger,
            &Collider,
            &Transform,
            &mut crate::environment::triggers::TriggerState,
        ),
        Without<Player>,
    >,
    clips: Query<&ColliderAabb, With<PerkMachineClip>>,
) {
    let Some((owner, player_pose, player_shape, health, inventory, client_mode)) =
        players.iter().next()
    else {
        return;
    };
    for (entity, usable, pose, children, mut machine) in &mut machines {
        let Some(perk) = game
            .content
            .perks
            .iter()
            .find(|perk| perk.id == machine.perk)
        else {
            continue;
        };
        if !machine.paths_disconnected
            && machine.expiry.is_none_or(|elapsed| elapsed < 5.)
            && let Some(radius) = perk.path_bounds_radius
        {
            let clip_entities = children
                .iter()
                .filter(|child| {
                    clips
                        .get(*child)
                        .is_ok_and(|bounds| bounds.min.is_finite() && bounds.max.is_finite())
                })
                .collect();
            machine.paths_disconnected = navigation.disconnect_perk_paths(
                entity,
                pose.translation,
                radius,
                &spatial,
                &clip_entities,
            );
        }
        let powered = perk
            .generator
            .as_ref()
            .is_none_or(|generator| generators.powered(generator));
        let exhausted = machine.perk == game.solo_revive.perk
            && inventory.solo_purchases >= game.solo_revive.maximum_purchases;
        if exhausted && machine.expiry.is_none() {
            machine.expiry = Some(0.);
            if let Some(id) = &usable.source.source_trigger {
                for (_, native, _, _, mut state) in &mut triggers {
                    if native.0 == *id {
                        state.enabled = false;
                    }
                }
            }
        }
        if machine.powered != Some(powered) && machine.expiry.is_none_or(|elapsed| elapsed < 5.) {
            if let Some(visual) = machine.visual.take() {
                commands.entity(visual).try_despawn();
            }
            if let Some(sound) = machine.sound.take() {
                commands.entity(sound).try_despawn();
            }
            if let Some(glow) = machine.glow.take() {
                commands.entity(glow).try_despawn();
            }
            machine.origin_tag = None;
            let presentation = if powered {
                perk.presentation.as_ref()
            } else {
                perk.off_presentation.as_ref()
            };
            if let Some(scene) = presentation
                .and_then(|id| {
                    game.content
                        .presentations
                        .iter()
                        .find(|presentation| presentation.id == *id)
                })
                .and_then(|presentation| presentation.scene.as_ref())
            {
                machine.visual = Some(
                    commands
                        .spawn((
                            SessionEntity,
                            ChildOf(entity),
                            WorldAssetRoot(server.load(scene.clone())),
                            Transform::default(),
                            Visibility::Inherited,
                        ))
                        .id(),
                );
            }
            if powered {
                machine.vibration = Some(Vibration::new(Vec3::Z, 0.3, 0.4, 3., pose.rotation));
                crate::audio::emit(
                    &mut commands,
                    "zmb_perks_power_on",
                    crate::audio::SoundSource::attached(machine.visual.unwrap_or(entity)),
                );
                machine.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_perks_machine_loop",
                    crate::audio::SoundSource::attached(machine.visual.unwrap_or(entity)),
                ));
            }
            machine.powered = Some(powered);
            machine.power_age = 0.;
        }
        machine.power_age += time.delta_secs();
        if powered
            && machine.power_age >= 3.
            && machine.glow.is_none()
            && machine.expiry.is_none_or(|elapsed| elapsed < 5.)
            && let Some(origin) = machine.origin_tag
        {
            machine.glow = Some(crate::native_fx::attach(
                &mut commands,
                "misc/fx_zombie_cola_revive_on",
                origin,
                Transform::default(),
            ));
        }
        if let Some(previous) = machine.expiry {
            let elapsed = previous + time.delta_secs();
            machine.expiry = Some(elapsed);
            if previous < 2. && elapsed >= 2. {
                let mut direction = Vec2::new(-pose.translation.z, pose.translation.x);
                if direction.y < 0. || direction.x > 0. && direction.y > 0. {
                    direction.y = -direction.y;
                } else if direction.x < 0. {
                    direction.x = -direction.x;
                }
                machine.vibration = Some(Vibration::new(
                    Vec3::new(direction.x, 0., -direction.y),
                    10.,
                    0.5,
                    4.,
                    pose.rotation,
                ));
                crate::audio::emit(
                    &mut commands,
                    "zmb_box_move",
                    crate::audio::SoundSource::attached(machine.visual.unwrap_or(entity)),
                );
                crate::audio::emit(
                    &mut commands,
                    "zmb_whoosh",
                    crate::audio::SoundSource::World(pose.translation),
                );
            }
            if previous < 5. && elapsed >= 5. {
                navigation.connect_perk_paths(entity);
                machine.paths_disconnected = false;
                crate::native_fx::emit(
                    &mut commands,
                    "misc/fx_zombie_couch_effect",
                    Transform::from_translation(pose.translation + Vec3::Y * (40. * 0.0254)),
                );
                crate::audio::emit(
                    &mut commands,
                    "zmb_box_poof",
                    crate::audio::SoundSource::World(pose.translation + Vec3::Y * (40. * 0.0254)),
                );
                if let Some(visual) = machine.visual.take() {
                    commands.entity(visual).try_despawn();
                }
                if let Some(sound) = machine.sound.take() {
                    commands.entity(sound).try_despawn();
                }
                if let Some(glow) = machine.glow.take() {
                    commands.entity(glow).try_despawn();
                }
                for child in children.iter() {
                    if clips.contains(child) {
                        commands.entity(child).insert(ColliderDisabled);
                    }
                }
            }
        }
        if let Some(vibration) = machine.vibration.as_mut() {
            vibration.age += time.delta_secs();
        }
        if let Some(visual) = machine.visual {
            commands.entity(visual).insert(
                Transform::from_xyz(
                    0.,
                    machine.expiry.map_or(0., |elapsed| {
                        40. * 0.0254 * ((elapsed - 2.) / 3.).clamp(0., 1.)
                    }),
                    0.,
                )
                .with_rotation(
                    machine
                        .vibration
                        .as_ref()
                        .map_or(Quat::IDENTITY, Vibration::rotation),
                ),
            );
        }
        machine.bump_clock -= time.delta_secs();
        if machine.bump_clock <= 0.
            && let Some(bump) = &perk.bump_trigger
        {
            let inside = triggers
                .iter()
                .find(|(_, native, _, _, _)| native.0 == *bump)
                .is_some_and(|(trigger, _, _, _, state)| {
                    let actor = crate::environment::triggers::TriggerActor {
                        contact_mask: 0x200000,
                        ..default()
                    };
                    state.accepts_actor(&actor, Some(client_mode.0))
                        && spatial
                            .shape_intersections(
                                player_shape,
                                player_pose.translation,
                                player_pose.rotation,
                                &SpatialQueryFilter::from_mask(
                                    crate::environment::triggers::TRIGGER_LAYER,
                                ),
                            )
                            .contains(&trigger)
                });
            if inside && !machine.bump_inside {
                crate::audio::emit(
                    &mut commands,
                    "zmb_perks_bump_bottle",
                    crate::audio::SoundSource::attached(owner),
                );
            }
            machine.bump_inside = inside && health.state() == crate::life::LifeState::Standing;
            machine.bump_clock = if machine.bump_inside { 0.5 } else { 0. };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_vibration_projects_direction_and_uses_future_sine_phase() {
        let mut vibration = Vibration::new(
            Vec3::Z,
            0.3,
            0.4,
            3.,
            Quat::from_rotation_y(270_f32.to_radians()),
        );
        assert!((vibration.delta.x - 0.3).abs() < 0.000001);
        assert!(vibration.delta.y.abs() < 0.000001);
        vibration.age = 3.1;
        assert!(
            vibration
                .rotation()
                .angle_between(Quat::from_rotation_z(-0.3_f32.to_radians()))
                < 0.000001
        );
        vibration.age = 3.4;
        assert_eq!(vibration.rotation(), Quat::IDENTITY);
    }
}
