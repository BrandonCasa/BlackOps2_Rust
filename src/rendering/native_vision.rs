//! Ordinary Origins vision owner. Weather owns selection; this owner preserves
//! the native interrupted parameter blend before the film LUT is generated.
use super::native_film::{NativeFilmControls, NativeFilmSettings};
use crate::{
    assets::LaunchOptions,
    environment::weather::{Weather, WeatherKind},
    session::{SessionClock, SessionId},
};
use bevy::{image::ImageLoaderSettings, prelude::*};
use std::path::Path;

const SOURCE_FILES: [&str; 3] = [
    "vision/zm_tomb.vision",
    "vision/zm_tomb_rain.vision",
    "vision/zm_tomb_snow.vision",
];
const TRANSITION_MS: i32 = 1000;

#[derive(Resource)]
struct VisionSources([NativeFilmControls; 3]);
impl VisionSources {
    fn load(root: &Path) -> Self {
        Self(SOURCE_FILES.map(|name| {
            let path = root.join(name);
            let source = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("Required original vision {}: {error}", path.display())
            });
            NativeFilmControls::parse_source(&source)
                .unwrap_or_else(|error| panic!("Original vision {}: {error}", path.display()))
        }))
    }
    fn controls(&self, kind: WeatherKind) -> NativeFilmControls {
        self.0[match kind {
            WeatherKind::Clear => 0,
            WeatherKind::Rain => 1,
            WeatherKind::Snow => 2,
        }]
    }
}

#[derive(Clone, Copy)]
struct Transition {
    from: NativeFilmControls,
    began: i32,
    sampled: i32,
}

/// Only `settings` and the actual authored image cross into the render world.
/// This is the ordinary owner. The source priority manager's Zombie Blood,
/// burn, downed/death and explicit lerp overrides need their own gameplay owner.
#[derive(Resource)]
pub(crate) struct NativeVision {
    pub settings: NativeFilmSettings,
    pub lut: Handle<Image>,
    selected: WeatherKind,
    session: SessionId,
    transition: Option<Transition>,
}
impl NativeVision {
    fn new(
        sources: &VisionSources,
        selected: WeatherKind,
        session: SessionId,
        lut: Handle<Image>,
    ) -> Self {
        Self {
            settings: NativeFilmSettings::ordinary(sources.controls(selected)),
            lut,
            selected,
            session,
            transition: None,
        }
    }
    fn needs_update(&self, selected: WeatherKind, session: SessionId, now: i32) -> bool {
        self.session != session
            || self.selected != selected
            || self
                .transition
                .is_some_and(|transition| transition.sampled != now)
    }
    fn advance(
        &mut self,
        sources: &VisionSources,
        selected: WeatherKind,
        session: SessionId,
        now: i32,
    ) {
        if self.session != session
            || self
                .transition
                .is_some_and(|transition| now < transition.sampled)
        {
            // Initial snapshots/session resets/demo rewinds use duration0.
            self.settings = NativeFilmSettings::ordinary(sources.controls(selected));
            self.selected = selected;
            self.session = session;
            self.transition = None;
            return;
        }
        if let Some(transition) = &mut self.transition {
            let elapsed = now - transition.began;
            if elapsed >= TRANSITION_MS {
                self.settings.controls = sources.controls(self.selected);
                self.transition = None;
            } else {
                // visionsetnaked mode3: 005264f0 captures current mixed fields;
                // 00527090 applies sin(t*pi/2) to the original 23 parameters.
                let fraction = elapsed as f32 / TRANSITION_MS as f32;
                let weight = (fraction * std::f32::consts::PI * 0.5).sin();
                self.settings.controls = transition
                    .from
                    .interpolate(sources.controls(self.selected), weight);
                transition.sampled = now;
            }
        }
        if self.selected != selected {
            self.transition = Some(Transition {
                from: self.settings.controls,
                began: now,
                sampled: now,
            });
            self.selected = selected;
        }
    }
}

pub(crate) struct NativeVisionPlugin;
impl Plugin for NativeVisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_vision)
            .add_systems(PostUpdate, update_vision);
    }
}
fn load_vision(
    mut commands: Commands,
    options: Res<LaunchOptions>,
    server: Res<AssetServer>,
    weather: Res<Weather>,
    session: Res<SessionId>,
) {
    let sources = VisionSources::load(&options.asset_root);
    // GfxWorld.lutMaterial colorMap is the actual 1024x64 two-bank numeric atlas.
    // Ordinary weather uses bank0. Source cg_useColorControl=false supplies the
    // ordinary identity color matrix through NativeFilmSettings::ordinary.
    let lut = server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
        })
        .load("images/zm_tomb_lut_win.ktx2");
    commands.insert_resource(NativeVision::new(&sources, weather.vision, *session, lut));
    commands.insert_resource(sources);
}
fn update_vision(
    sources: Res<VisionSources>,
    mut vision: ResMut<NativeVision>,
    weather: Res<Weather>,
    clock: Res<SessionClock>,
    session: Res<SessionId>,
    time: Res<Time<Virtual>>,
) {
    let now = clock.milliseconds(&time);
    // Avoid marking stable/paused settings changed: LUT generation and constant
    // uploads are driven only by actual control changes in NativeFilmView.
    if vision.needs_update(weather.vision, *session, now) {
        vision.advance(&sources, weather.vision, *session, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires exported original clear/rain/snow vision files"]
    fn authored_vision_blends_parameters_and_interrupts_from_current_state() {
        let sources =
            VisionSources::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets"));
        let mut vision = NativeVision::new(
            &sources,
            WeatherKind::Clear,
            SessionId(1),
            Handle::default(),
        );
        vision.advance(&sources, WeatherKind::Rain, SessionId(1), 0);
        vision.advance(&sources, WeatherKind::Rain, SessionId(1), 500);
        let halfway = vision.settings.controls;
        let sine_weight = std::f32::consts::FRAC_1_SQRT_2;
        assert!((halfway.film_blend.x - (1. - sine_weight)).abs() < 0.000001);
        assert!((halfway.film_gamma.x - (1. + (0.887097 - 1.) * sine_weight)).abs() < 0.000001);
        assert!(!vision.needs_update(WeatherKind::Rain, SessionId(1), 500));
        vision.advance(&sources, WeatherKind::Clear, SessionId(1), 500);
        assert_eq!(vision.settings.controls, halfway);
        vision.advance(&sources, WeatherKind::Clear, SessionId(1), 1000);
        assert!(
            (vision.settings.controls.film_blend.x
                - (halfway.film_blend.x + (1. - halfway.film_blend.x) * sine_weight))
                .abs()
                < 0.000001
        );
        vision.advance(&sources, WeatherKind::Clear, SessionId(1), 1500);
        assert_eq!(
            vision.settings.controls,
            sources.controls(WeatherKind::Clear)
        );
        assert!(!vision.needs_update(WeatherKind::Clear, SessionId(1), 2000));
        // A new session or source-clock rewind snaps to its actual selected bank.
        vision.advance(&sources, WeatherKind::Snow, SessionId(2), 0);
        assert_eq!(
            vision.settings.controls,
            sources.controls(WeatherKind::Snow)
        );
        vision.advance(&sources, WeatherKind::Clear, SessionId(2), 10);
        vision.advance(&sources, WeatherKind::Clear, SessionId(2), 510);
        vision.advance(&sources, WeatherKind::Clear, SessionId(2), 0);
        assert_eq!(
            vision.settings.controls,
            sources.controls(WeatherKind::Clear)
        );
        assert_eq!(vision.settings.lut_banks, [0, 0]);
    }
}
