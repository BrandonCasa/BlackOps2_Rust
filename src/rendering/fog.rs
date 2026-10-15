//! Origins world fog: native camera-volume selection and shader constants.
//!
//! Source: 00413340 selects/blends volumes, 004115e0 applies modifiers,
//! 00465d30 prepares fog constants, 00413bb0/004166b0 prepare exposure.
use super::native::NativeWorldScene;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorldFogDefinition {
    sun: Sun,
    fog_transition_time: f32,
    init_world_fog: WorldFog,
    world_fog_volumes: Vec<FogVolume>,
    world_fog_volume_planes: Vec<[f32; 4]>,
    world_fog_modifier_volumes: Vec<FogModifier>,
    world_fog_modifier_volume_planes: Vec<[f32; 4]>,
    exposure_volumes: Vec<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
struct Sun {
    exposure: f32,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldFog {
    base_dist: f32,
    half_dist: f32,
    base_height: f32,
    half_height: f32,
    sun_fog_pitch: f32,
    sun_fog_yaw: f32,
    sun_fog_inner: f32,
    sun_fog_outer: f32,
    fog_color: [f32; 3],
    fog_opacity: f32,
    sun_fog_color: [f32; 3],
    sun_fog_opacity: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FogVolume {
    control: u32,
    fog_transition_time: f32,
    volume_world_fog: WorldFog,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FogModifier {
    control: u32,
    transition_time: f32,
    depth_scale: f32,
    color_adjust: [f32; 4],
}

impl WorldFogDefinition {
    /// Check required source data once when loading the world.
    pub(crate) fn validate(&self) {
        assert!(
            !self.world_fog_volumes.is_empty(),
            "Origins requires its authored world fog volumes"
        );
        assert!(
            self.exposure_volumes.is_empty(),
            "Origins source exposure volumes changed; native exposure selection is required"
        );
        for volume in &self.world_fog_volumes {
            validate_planes(volume.control, &self.world_fog_volume_planes);
        }
        for volume in &self.world_fog_modifier_volumes {
            validate_planes(volume.control, &self.world_fog_modifier_volume_planes);
        }
    }

    fn fog(&self, index: Option<usize>) -> (WorldFog, f32) {
        index.map_or((self.init_world_fog, self.fog_transition_time), |index| {
            let volume = &self.world_fog_volumes[index];
            (volume.volume_world_fog, volume.fog_transition_time)
        })
    }

    fn modifier(&self, index: Option<usize>) -> ModifierFactors {
        index.map_or(ModifierFactors::IDENTITY, |index| {
            let modifier = &self.world_fog_modifier_volumes[index];
            ModifierFactors {
                depth_scale: modifier.depth_scale,
                color_adjust: Vec4::from_array(modifier.color_adjust),
            }
        })
    }
}

fn validate_planes(control: u32, planes: &[[f32; 4]]) {
    let first = (control >> 16) as usize;
    let count = ((control >> 8) & 255) as usize + 1;
    assert!(
        first + count <= planes.len(),
        "Invalid native fog plane slice"
    );
}

fn contains(control: u32, planes: &[[f32; 4]], eye: Vec3) -> bool {
    let first = (control >> 16) as usize;
    let count = ((control >> 8) & 255) as usize + 1;
    planes[first..first + count]
        .iter()
        .all(|p| p[0] * eye.x + p[1] * eye.y + p[2] * eye.z + p[3] <= 0.)
}

fn transition(now_ms: i32, start_ms: i32, rate: f32) -> f32 {
    // The authored field is a rate per millisecond; native only clamps above.
    let t = (now_ms.wrapping_sub(start_ms) as f32 * rate).min(1.);
    (3. - 2. * t) * t * t
}

impl WorldFog {
    fn blend(self, target: Self, weight: f32) -> Self {
        let blend = |a: f32, b: f32| a * (1. - weight) + b * weight;
        Self {
            base_dist: blend(self.base_dist, target.base_dist),
            half_dist: blend(self.half_dist, target.half_dist),
            base_height: blend(self.base_height, target.base_height),
            half_height: blend(self.half_height, target.half_height),
            sun_fog_pitch: blend(self.sun_fog_pitch, target.sun_fog_pitch),
            sun_fog_yaw: blend(self.sun_fog_yaw, target.sun_fog_yaw),
            sun_fog_inner: blend(self.sun_fog_inner, target.sun_fog_inner),
            sun_fog_outer: blend(self.sun_fog_outer, target.sun_fog_outer),
            fog_color: std::array::from_fn(|i| blend(self.fog_color[i], target.fog_color[i])),
            fog_opacity: blend(self.fog_opacity, target.fog_opacity),
            sun_fog_color: std::array::from_fn(|i| {
                blend(self.sun_fog_color[i], target.sun_fog_color[i])
            }),
            sun_fog_opacity: blend(self.sun_fog_opacity, target.sun_fog_opacity),
        }
    }

    fn write_scene(self, modifier: ModifierFactors, eye: Vec3, scene: &mut NativeWorldScene) {
        // Native does not use the modifier's heightScale, or premultiply RGB by opacity.
        let depth = (self.half_dist * modifier.depth_scale).max(1.).recip();
        let height = self.half_height.max(1.).recip();
        let density = 1.0_f32.min(depth * 100.).max(depth);
        let ln2 = std::f32::consts::LN_2;
        let at_eye = (depth / density).ln() - height * (eye.z - self.base_height) * ln2;
        scene.fog_consts = Vec4::new(
            at_eye,
            -density * ln2,
            self.base_dist * depth,
            -height * ln2,
        );
        scene.fog_consts2 = Vec4::new(
            if at_eye < 0. {
                at_eye.exp()
            } else {
                at_eye + 1.
            },
            0.,
            0.,
            0.,
        );
        scene.fog_color =
            Vec3::from_array(self.fog_color).extend(self.fog_opacity) * modifier.color_adjust;
        scene.sun_fog_color = Vec3::from_array(self.sun_fog_color).extend(self.sun_fog_opacity)
            * modifier.color_adjust;
        let pitch = self.sun_fog_pitch * 0.017453292;
        let yaw = self.sun_fog_yaw * 0.017453292;
        scene.sun_fog_dir = Vec4::new(
            pitch.cos() * yaw.cos(),
            pitch.cos() * yaw.sin(),
            -pitch.sin(),
            0.,
        );
        let inner = (self.sun_fog_inner * 0.017453292).cos();
        let outer = (self.sun_fog_outer * 0.017453292).cos();
        let scale = if inner == outer {
            1e7
        } else {
            (inner - outer).recip()
        };
        scene.sun_fog = Vec4::new(-outer * scale, scale, 0., 0.);
    }
}

#[derive(Clone, Copy)]
struct ModifierFactors {
    depth_scale: f32,
    color_adjust: Vec4,
}

impl ModifierFactors {
    const IDENTITY: Self = Self {
        depth_scale: 1.,
        color_adjust: Vec4::ONE,
    };

    fn blend(self, target: Self, weight: f32) -> Self {
        Self {
            depth_scale: self.depth_scale * (1. - weight) + target.depth_scale * weight,
            color_adjust: self.color_adjust * (1. - weight) + target.color_adjust * weight,
        }
    }
}

/// History belongs to the camera; replace with default when replacing its world.
#[derive(Component, Default)]
pub(crate) struct CameraFog(Option<FogHistory>);

struct FogHistory {
    previous_eye: Vec3,
    fog_target: Option<usize>,
    fog_start_ms: i32,
    fog_start: WorldFog,
    fog_current: WorldFog,
    modifier_target: Option<usize>,
    modifier_start_ms: i32,
    modifier_rate: f32,
    modifier_start: ModifierFactors,
}

impl CameraFog {
    /// `eye_native` is absolute native XYZ in inches. The shader separately uses
    /// eye-to-vertex vectors in those same axes/units. `now_ms` is the game view
    /// clock; request a reset on session start or when changing its clock source.
    /// Solo weather masks are clear=1, rain=2, snow=4 (not bit indices).
    pub(crate) fn write_scene(
        &mut self,
        world: &WorldFogDefinition,
        eye_native: Vec3,
        bank_mask: u16,
        now_ms: i32,
        reset: bool,
        scene: &mut NativeWorldScene,
    ) {
        let fog_target = world.world_fog_volumes.iter().position(|volume| {
            volume.control & u32::from(bank_mask) != 0
                && contains(volume.control, &world.world_fog_volume_planes, eye_native)
        });
        let modifier_target = world.world_fog_modifier_volumes.iter().position(|volume| {
            volume.control & u32::from(bank_mask & 15) != 0
                && contains(
                    volume.control,
                    &world.world_fog_modifier_volume_planes,
                    eye_native,
                )
        });
        let (fog, fog_rate) = world.fog(fog_target);
        let modifier = world.modifier(modifier_target);
        let snap = reset
            || self.0.as_ref().is_none_or(|history| {
                eye_native.distance_squared(history.previous_eye) > 1024. * 1024.
            });
        if snap {
            self.0 = Some(FogHistory {
                previous_eye: eye_native,
                fog_target,
                fog_start_ms: now_ms,
                fog_start: fog,
                fog_current: fog,
                modifier_target,
                modifier_start_ms: now_ms,
                modifier_rate: modifier_target.map_or(1., |index| {
                    world.world_fog_modifier_volumes[index].transition_time
                }),
                modifier_start: modifier,
            });
        }
        let history = self.0.as_mut().expect("Camera fog initialized above");
        history.previous_eye = eye_native;

        // Finish the old modifier step before capturing an interrupted transition.
        let modifier_current = history.modifier_start.blend(
            world.modifier(history.modifier_target),
            transition(now_ms, history.modifier_start_ms, history.modifier_rate),
        );
        if modifier_target != history.modifier_target {
            history.modifier_target = modifier_target;
            history.modifier_start_ms = now_ms;
            history.modifier_start = modifier_current;
            // On exit the previous volume's rate remains authoritative.
            if let Some(index) = modifier_target {
                history.modifier_rate = world.world_fog_modifier_volumes[index].transition_time;
            }
        }

        // Full fog captures the previous output before evaluating the new target.
        if fog_target != history.fog_target {
            history.fog_target = fog_target;
            history.fog_start_ms = now_ms;
            history.fog_start = history.fog_current;
        }
        history.fog_current = history
            .fog_start
            .blend(fog, transition(now_ms, history.fog_start_ms, fog_rate));
        history
            .fog_current
            .write_scene(modifier_current, eye_native, scene);

        // Installed Origins has no exposure volumes. Native initialization and
        // every subsequent update therefore retain the authored sun exposure.
        let exposure = 2.0_f64.powf(f64::from(world.sun.exposure + 2.)) as f32;
        scene.hdr_control0 = Vec4::new(exposure.recip(), 0., exposure, exposure);
    }
}
