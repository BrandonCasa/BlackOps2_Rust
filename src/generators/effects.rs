use super::{GeneratorState, GeneratorVisual, Generators};
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{Session, SessionEntity},
    life::{LifeState, Vitality},
};
use bevy::{prelude::*, world_serialization::WorldInstanceReady};

#[derive(Component, Default)]
struct GeneratorFx {
    phase: Option<usize>,
    state: Option<GeneratorState>,
    pole: Option<Entity>,
    exhaust: Option<Entity>,
    top: Option<Entity>,
    vats: [Option<Entity>; 2],
    active: Vec<Entity>,
    light: Option<Entity>,
    light_clock: f32,
}
#[derive(Component)]
struct Hole {
    generator: crate::origins::ContentId,
    index: usize,
    origin: Option<Entity>,
    effects: Vec<Entity>,
    sound: Option<Entity>,
    closing: Option<f32>,
    delay: f32,
}
#[derive(Component, Default)]
struct CaptureActorFx {
    spawned: bool,
    risen: bool,
    torso: Option<Entity>,
    eyes: Option<Entity>,
    sound: Option<Entity>,
    age: f32,
    billow: bool,
}
pub(super) struct CaptureEffectsPlugin;
impl Plugin for CaptureEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(ready).add_systems(
            Update,
            (generators, holes, actors)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    visuals: Query<&GeneratorVisual>,
    mut holes: Query<&mut Hole>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let joint = |name: &str| {
        children.iter_descendants(event.entity).find(|entity| {
            names
                .get(*entity)
                .is_ok_and(|value| value.as_str().eq_ignore_ascii_case(name))
        })
    };
    if visuals.contains(event.entity) {
        commands.entity(event.entity).insert(GeneratorFx {
            pole: joint("j_generator_pole"),
            exhaust: joint("fx_rear_exhaust"),
            top: joint("tag_pole_top"),
            vats: [joint("vat_01_sound"), joint("vat_02_sound")],
            ..default()
        });
    }
    if let Ok(mut hole) = holes.get_mut(event.entity) {
        hole.origin = joint("tag_origin");
    }
}
fn generators(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameDefinition>,
    sites: Res<Generators>,
    server: Res<AssetServer>,
    mut visuals: Query<(&GeneratorVisual, &mut GeneratorFx)>,
    existing: Query<&Hole>,
) {
    for (visual, mut fx) in &mut visuals {
        let Some(site) = sites.sites.get(&visual.id) else {
            continue;
        };
        let Some(profile) = game
            .content
            .generators
            .iter()
            .find(|profile| profile.id == visual.id)
        else {
            continue;
        };
        if fx.state != Some(site.state) && site.state == GeneratorState::Capturing {
            if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_tm_zone_capture_hole") {
                for (index, spot) in profile.emergence.iter().enumerate() {
                    if existing.iter().any(|hole| {
                        hole.generator == visual.id && hole.index == index && hole.closing.is_none()
                    }) {
                        continue;
                    }
                    commands.spawn((
                        SessionEntity,
                        Hole {
                            generator: visual.id.clone(),
                            index,
                            origin: None,
                            effects: Vec::new(),
                            sound: None,
                            closing: None,
                            delay: 0.,
                        },
                        WorldAssetRoot(server.load(scene.clone())),
                        Transform::from_translation(Vec3::from_array(spot.position))
                            .with_rotation(Quat::from_rotation_y(spot.yaw)),
                        Visibility::Inherited,
                    ));
                }
            }
        }
        let active = matches!(
            site.state,
            GeneratorState::Capturing | GeneratorState::Active
        );
        if active
            && (fx.state != Some(site.state)
                || fx.phase != site.pump_clip && matches!(site.pump_clip, Some(0 | 1)))
        {
            for effect in fx.active.drain(..) {
                commands.entity(effect).try_despawn();
            }
            if let Some(light) = fx.light.take() {
                commands.entity(light).try_despawn();
            }
            if let Some(pole) = fx.pole {
                fx.active.push(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_capture_progression",
                    pole,
                    Transform::default(),
                ));
            }
            if let Some(exhaust) = fx.exhaust {
                fx.active.push(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_capture_exhaust_back",
                    exhaust,
                    Transform::default(),
                ));
            }
            for vat in fx.vats.into_iter().flatten() {
                fx.active.push(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_capturezone_vat_loop",
                    crate::audio::SoundSource::attached(vat),
                ));
            }
        } else if !active {
            for effect in fx.active.drain(..) {
                commands.entity(effect).try_despawn();
            }
            if fx.state != Some(site.state) {
                fx.light_clock = 0.;
            }
            fx.light_clock -= time.delta_secs();
            if fx.light_clock <= 0.
                && let Some(top) = fx.top
            {
                if let Some(light) = fx.light.take() {
                    commands.entity(light).try_despawn();
                }
                fx.light = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_capture_light_red",
                    top,
                    Transform::default(),
                ));
                fx.light_clock = 9.;
            }
        }
        if fx.state != Some(site.state) && site.state == GeneratorState::Active {
            if let Some(placement) = game
                .content
                .placements
                .iter()
                .find(|placement| placement.id == profile.placement)
            {
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_capture_complete",
                    Transform::from_translation(Vec3::from_array(placement.position))
                        .with_rotation(Quat::from_rotation_y(placement.yaw)),
                );
            }
        }
        fx.phase = site.pump_clip;
        fx.state = Some(site.state);
    }
}
fn holes(
    mut commands: Commands,
    time: Res<Time>,
    sites: Res<Generators>,
    mut session: ResMut<Session>,
    mut holes: Query<(Entity, &GlobalTransform, &mut Hole)>,
) {
    for (entity, pose, mut hole) in &mut holes {
        let active = sites
            .sites
            .get(&hole.generator)
            .is_some_and(|site| site.state == GeneratorState::Capturing);
        if !active && hole.closing.is_none() {
            hole.closing = Some(0.);
            hole.delay = 0.5 + session.random() as f32 / u32::MAX as f32 * 1.5;
        }
        if let Some(previous) = hole.closing {
            let elapsed = previous + time.delta_secs();
            hole.closing = Some(elapsed);
            let start = hole.index as f32 * 0.05;
            if previous <= start && elapsed > start {
                for effect in hole.effects.drain(..) {
                    commands.entity(effect).try_despawn();
                }
                if let Some(sound) = hole.sound.take() {
                    commands.entity(sound).try_despawn();
                }
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_alcatraz/fx_alcatraz_afterlife_zmb_tport",
                    pose.compute_transform(),
                );
                crate::audio::emit(
                    &mut commands,
                    "zmb_capturezone_portal_stop",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            if elapsed >= start + hole.delay {
                commands.entity(entity).try_despawn();
            }
            continue;
        }
        if hole.effects.is_empty()
            && let Some(origin) = hole.origin
        {
            hole.effects.push(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_alcatraz/fx_alcatraz_afterlife_zmb_tport",
                origin,
                Transform::default(),
            ));
            hole.effects.push(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_screecher_vortex",
                origin,
                Transform::default(),
            ));
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_capturezone_portal_loop",
                crate::audio::SoundSource::attached(entity),
            );
            commands.entity(sound).insert(crate::audio::SoundControl {
                volume: 2.,
                pitch: 1.,
            });
            hole.sound = Some(sound);
            crate::audio::emit(
                &mut commands,
                "zmb_capturezone_portal_start",
                crate::audio::SoundSource::attached(entity),
            );
        }
    }
}
fn actors(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameDefinition>,
    actors: Query<
        (
            Entity,
            &crate::enemies::Zombie,
            &Transform,
            &Vitality,
            &crate::combat_effects::Wounds,
            Has<CaptureActorFx>,
        ),
        With<super::GeneratorDefender>,
    >,
    mut effects: Query<&mut CaptureActorFx>,
    rigs: Query<&crate::enemies::ActorRig>,
) {
    for (entity, zombie, pose, health, wounds, existing) in &actors {
        if !existing {
            commands.entity(entity).insert(CaptureActorFx::default());
            continue;
        }
        let Ok(mut fx) = effects.get_mut(entity) else {
            continue;
        };
        let Some(profile) = game
            .content
            .enemies
            .iter()
            .find(|profile| profile.id == zombie.definition)
        else {
            continue;
        };
        let random = |index: u64, count: usize| {
            (crate::weapons::sample(entity.to_bits().wrapping_add(index)) * count as f32) as usize
        };
        if !fx.spawned {
            let height = (5 + random(0, 5)) as f32 * 0.0254;
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_emergence_spawn",
                Transform::from_translation(
                    pose.translation - crate::enemies::body_center(profile, false)
                        + Vec3::Y * height,
                ),
            );
            crate::audio::emit(
                &mut commands,
                "zmb_vocals_capzomb_spawn",
                crate::audio::SoundSource::attached(entity),
            );
            fx.spawned = true;
        }
        fx.age += time.delta_secs();
        if fx.age >= 0.25 && !fx.billow {
            let x = (random(1, 20) as f32 - 10.) * 0.0254;
            let z = (random(2, 20) as f32 - 10.) * 0.0254;
            let height = (5 + random(3, 5)) as f32 * 0.0254;
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_emergence_spawn",
                Transform::from_translation(
                    pose.translation - crate::enemies::body_center(profile, false)
                        + Vec3::new(x, height, -z),
                ),
            );
            fx.billow = true;
        }
        if health.state() != LifeState::ActorAlive {
            if let Some(sound) = fx.sound.take() {
                commands.entity(sound).try_despawn();
            }
            if let Some(eyes) = fx.eyes.take() {
                commands.entity(eyes).try_despawn();
            }
            continue;
        }
        if zombie.completed_emerging() {
            if !fx.risen {
                crate::audio::emit(
                    &mut commands,
                    "zmb_vocals_capzomb_spawn",
                    crate::audio::SoundSource::attached(entity),
                );
                fx.risen = true;
            }
            if fx.torso.is_none()
                && let Some(torso) = rigs.iter().find_map(|rig| {
                    rig.joint(entity, "J_Spine4")
                        .or_else(|| rig.joint(entity, "j_spine4"))
                })
            {
                fx.torso = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_crusader_torso_loop",
                    torso,
                    Transform::default(),
                ));
                fx.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_capturezone_zombie_loop",
                    crate::audio::SoundSource::attached(entity),
                ));
            }
            if wounds.headless {
                if let Some(eyes) = fx.eyes.take() {
                    commands.entity(eyes).try_despawn();
                }
            } else if fx.eyes.is_none()
                && let Some(eye) = rigs.iter().find_map(|rig| {
                    rig.joint(entity, "J_Eyeball_LE")
                        .or_else(|| rig.joint(entity, "j_eyeball_le"))
                })
            {
                fx.eyes = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie/fx_zombie_crusader_eyes",
                    eye,
                    Transform::default(),
                ));
            }
        }
    }
}
