//! Original shadow-volume selection (00413040), shared by both sun views and
//! local-light shadows. Plane equations and eye positions stay in source inches.
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    prelude::*,
    render::extract_component::ExtractComponent,
};
use serde::Deserialize;
pub(crate) mod gpu;
pub(crate) mod sun;

pub(crate) struct ShadowPlugin;
impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "native_depth_entry.wesl");
        let shader = load_embedded_asset!(app, "native_depth_entry.wesl");
        gpu::install(app, shader);
    }
}

#[derive(Resource, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShadowWorld {
    bounds: Bounds,
    shadow_map_volumes: Vec<Volume>,
    shadow_map_volume_planes: Vec<[f32; 4]>,
}

#[derive(Deserialize)]
struct Point {
    x: f32,
    y: f32,
    z: f32,
}

impl Point {
    fn vector(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

#[derive(Deserialize)]
struct Bounds {
    mins: Point,
    maxs: Point,
}

#[derive(Deserialize)]
struct Volume {
    control: u32,
}

impl Volume {
    fn plane_range(&self) -> std::ops::Range<usize> {
        // Unlike fog: low16 is the first plane, next8 is count minus1.
        let first = (self.control & 0xffff) as usize;
        first..first + ((self.control >> 16) & 255) as usize + 1
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, ExtractComponent)]
#[extract_app(bevy::render::RenderApp)]
pub(crate) struct ShadowViewSettings {
    pub sun_enabled: bool,
    pub small_sun: bool,
    pub spot_enabled: bool,
}

impl ShadowWorld {
    pub(crate) fn validate(&self) {
        let min = self.bounds.mins.vector();
        let max = self.bounds.maxs.vector();
        assert!(
            min.is_finite() && max.is_finite() && min.cmple(max).all(),
            "Original world shadow bounds"
        );
        assert!(
            self.shadow_map_volume_planes
                .iter()
                .flatten()
                .all(|v| v.is_finite()),
            "Original shadow volume planes must be finite"
        );
        for volume in &self.shadow_map_volumes {
            assert!(
                self.shadow_map_volume_planes
                    .get(volume.plane_range())
                    .is_some(),
                "Original shadow volume plane range"
            );
        }
    }

    pub(crate) fn sun_views(
        &self,
        eye: Vec3,
        sun: Vec3,
        clip_from_eye: Mat4,
    ) -> sun::SunShadowViews {
        sun::SunShadowViews::new(
            eye,
            sun,
            clip_from_eye,
            [self.bounds.mins.vector(), self.bounds.maxs.vector()],
        )
    }

    pub(crate) fn settings(&self, eye: Vec3) -> ShadowViewSettings {
        // 00413040 resets these before inspecting volumes. The source default
        // sm_sunShadowSmall is false (0041f2b0); no runtime override exists here.
        let mut settings = ShadowViewSettings {
            sun_enabled: true,
            small_sun: false,
            spot_enabled: true,
        };
        for volume in &self.shadow_map_volumes {
            let flags = volume.control & 0x1f00_0000;
            // A volume with only bit27 is an inclusion constraint consumed by
            // 00413200, not a view-setting override.
            if flags == 0x0800_0000 {
                continue;
            }
            let inside = self.shadow_map_volume_planes[volume.plane_range()]
                .iter()
                .all(|p| p[0] * eye.x + p[1] * eye.y + p[2] * eye.z + p[3] <= 0.0);
            if inside {
                settings.sun_enabled = flags & 0x0100_0000 == 0;
                settings.small_sun = flags & 0x0200_0000 != 0 && flags & 0x1000_0000 == 0;
                settings.spot_enabled = flags & 0x0400_0000 == 0;
                // First containing volume wins, including volumes with no flags.
                break;
            }
        }
        settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volumes(controls: &[u32]) -> ShadowWorld {
        ShadowWorld {
            bounds: Bounds {
                mins: Point {
                    x: -1.,
                    y: -1.,
                    z: -1.,
                },
                maxs: Point {
                    x: 1.,
                    y: 1.,
                    z: 1.,
                },
            },
            shadow_map_volumes: controls.iter().map(|&control| Volume { control }).collect(),
            // First half-space x<=0; second y<=0. Control's low16 selects it.
            shadow_map_volume_planes: vec![[1., 0., 0., 0.], [0., 1., 0., 0.]],
        }
    }

    #[test]
    fn first_containing_volume_wins_including_boundary_and_zero_flags() {
        let world = volumes(&[0, 0x0700_0000]);
        assert_eq!(
            world.settings(Vec3::ZERO),
            ShadowViewSettings {
                sun_enabled: true,
                small_sun: false,
                spot_enabled: true,
            }
        );
        let world = volumes(&[0x0100_0000, 0x0600_0001]);
        assert_eq!(
            world.settings(Vec3::new(1., -1., 0.)),
            ShadowViewSettings {
                sun_enabled: true,
                small_sun: true,
                spot_enabled: false,
            }
        );
        assert!(!world.settings(Vec3::new(-1., -1., 0.)).sun_enabled);
        assert!(world.settings(Vec3::ONE).sun_enabled);
    }

    #[test]
    fn inclusion_only_volumes_do_not_override_and_full_resolution_wins() {
        let world = volumes(&[0x0800_0000, 0x1f00_0000]);
        assert_eq!(
            world.settings(Vec3::ZERO),
            ShadowViewSettings {
                sun_enabled: false,
                small_sun: false,
                spot_enabled: false,
            }
        );
    }

    #[test]
    #[should_panic(expected = "Original shadow volume plane range")]
    fn rejects_truncated_plane_ranges_before_camera_use() {
        volumes(&[0x0001_0001]).validate();
    }
}
