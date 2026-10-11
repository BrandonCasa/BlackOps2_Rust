use bevy::{
    audio::{ChannelCount, Decodable, Sample, SampleRate, Source},
    prelude::*,
};
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Copy)]
pub(super) struct Preset {
    early: f32,
    late: f32,
    gain: f32,
    feedback: f32,
    damping: f32,
}
pub(super) fn preset(room: &str) -> Preset {
    let name = match room {
        "tomb_robot_head" => "tomb_bot_head",
        "tomb_bunker_sml" | "tomb_bunker_sml_prtl" => "tomb_bunker_smallroom",
        "tomb_bunker_med" | "tomb_bunker_med_prtl" => "tomb_bunker_mediumroom",
        "tomb_bunker_lrg" | "tomb_bunker_lrg_prtl" => "tomb_bunker_largeroom",
        "tomb_spiral_staircase" => "tomb_stairs",
        "tomb_spiral_staircase_bot" => "tomb_stairs_big",
        "tomb_large_chamber" => "tomb_cave_2",
        "tomb_overhang_prtl" => "tomb_overhang",
        "tomb_church_prtl" => "tomb_church",
        other => other,
    };
    match name {
        "tomb_tunnel" => Preset {
            early: 0.840000,
            late: 0.492188,
            gain: 1.701607,
            feedback: 0.874023,
            damping: 0.309570,
        },
        "tomb_stairs_big" => Preset {
            early: 0.623047,
            late: 0.730469,
            gain: 1.674280,
            feedback: 0.677734,
            damping: 0.521484,
        },
        "tomb_stairs" => Preset {
            early: 0.502930,
            late: 0.534180,
            gain: 1.470883,
            feedback: 0.677734,
            damping: 0.521484,
        },
        "tomb_overhang" => Preset {
            early: 0.500000,
            late: 0.680000,
            gain: 0.801505,
            feedback: 0.639648,
            damping: 0.316406,
        },
        "tomb_outdoor" => Preset {
            early: 0.304688,
            late: 0.541016,
            gain: 0.965530,
            feedback: 0.210000,
            damping: 0.394531,
        },
        "tomb_church" => Preset {
            early: 0.500000,
            late: 0.680000,
            gain: 0.487901,
            feedback: 0.230000,
            damping: 0.440000,
        },
        "tomb_chamber" => Preset {
            early: 0.205000,
            late: 0.300000,
            gain: 1.024570,
            feedback: 0.820000,
            damping: 0.350000,
        },
        "tomb_cave_2" => Preset {
            early: 0.840000,
            late: 0.575000,
            gain: 0.646470,
            feedback: 0.740000,
            damping: 0.580000,
        },
        "tomb_cave" => Preset {
            early: 0.599609,
            late: 0.575000,
            gain: 0.883271,
            feedback: 0.689453,
            damping: 0.250977,
        },
        "tomb_bunker_smallroom" => Preset {
            early: 0.441406,
            late: 0.460938,
            gain: 1.191714,
            feedback: 0.856445,
            damping: 0.401367,
        },
        "tomb_bunker_mediumroom" => Preset {
            early: 0.603516,
            late: 0.583008,
            gain: 2.134504,
            feedback: 0.426758,
            damping: 0.389648,
        },
        "tomb_bunker_largeroom" => Preset {
            early: 0.633789,
            late: 0.595000,
            gain: 1.701607,
            feedback: 0.367188,
            damping: 0.673828,
        },
        "tomb_bot_head" => Preset {
            early: 0.385742,
            late: 0.539062,
            gain: 1.389853,
            feedback: 0.825000,
            damping: 0.701172,
        },

        _ => Preset {
            early: 0.304688,
            late: 0.541016,
            gain: 0.965530,
            feedback: 0.21,
            damping: 0.394531,
        },
    }
}
impl Preset {
    fn blend(self, target: Self, t: f32) -> Self {
        Self {
            early: self.early + (target.early - self.early) * t,
            late: self.late + (target.late - self.late) * t,
            gain: self.gain + (target.gain - self.gain) * t,
            feedback: self.feedback + (target.feedback - self.feedback) * t,
            damping: self.damping + (target.damping - self.damping) * t,
        }
    }
}
#[derive(Resource)]
pub(super) struct ListenerRoom {
    pub state: Arc<RwLock<Preset>>,
    room: String,
    from: Preset,
    target: Preset,
    elapsed: f32,
}
impl Default for ListenerRoom {
    fn default() -> Self {
        let p = preset("tomb_outdoor");
        Self {
            state: Arc::new(RwLock::new(p)),
            room: String::new(),
            from: p,
            target: p,
            elapsed: 2.,
        }
    }
}
pub(super) fn update(
    time: Res<Time>,
    weather: Res<crate::environment::weather::Weather>,
    mut room: ResMut<ListenerRoom>,
) {
    if room.room != weather.ambient_room {
        let previous = *room.state.read().unwrap_or_else(|e| e.into_inner());
        room.from = previous;
        room.target = preset(&weather.ambient_room);
        room.room = weather.ambient_room.clone();
        room.elapsed = 0.;
    }
    room.elapsed = (room.elapsed + time.delta_secs()).min(2.);
    let value = room.from.blend(room.target, room.elapsed / 2.);
    *room.state.write().unwrap_or_else(|e| e.into_inner()) = value;
}
#[derive(Clone)]
pub(super) enum Input {
    Cached(super::decoded::DecodedClip),
    Stream(AudioSource),
}
#[derive(Asset, TypePath)]
pub(super) struct RoomClip {
    input: Input,
    room: Arc<RwLock<Preset>>,
    pub wet: Arc<AtomicU32>,
    pub dry_done: Arc<AtomicBool>,
    looping: bool,
}
impl RoomClip {
    pub(super) fn new(
        input: Input,
        room: &ListenerRoom,
        wet: Arc<AtomicU32>,
        dry_done: Arc<AtomicBool>,
        looping: bool,
    ) -> Self {
        Self {
            input,
            room: room.state.clone(),
            wet,
            dry_done,
            looping,
        }
    }
}
impl Decodable for RoomClip {
    type Decoder = RoomSamples;
    fn decoder(&self) -> RoomSamples {
        let source: Box<dyn Source<Item = Sample> + Send> = match &self.input {
            Input::Cached(clip) => Box::new(clip.decoder()),
            Input::Stream(source) => Box::new(source.decoder()),
        };
        let channels = source.channels();
        let rate = source.sample_rate();
        let duration = if self.looping {
            None
        } else {
            source.total_duration()
        };
        let source: Box<dyn Source<Item = Sample> + Send> = if self.looping {
            Box::new(source.repeat_infinite())
        } else {
            source
        };
        let size = (rate.get() as usize * channels.get() as usize / 5).max(1);
        let p = *self.room.read().unwrap_or_else(|e| e.into_inner());
        RoomSamples {
            source,
            channels,
            rate,
            duration,
            room: self.room.clone(),
            wet: self.wet.clone(),
            dry_done: self.dry_done.clone(),
            preset: p,
            cursor: 0,
            early: vec![0.; size],
            late: vec![0.; size],
            filter: vec![0.; channels.get() as usize],
            tail: rate.get() as usize * channels.get() as usize * 2,
            remaining: None,
        }
    }
}
pub(super) struct RoomSamples {
    source: Box<dyn Source<Item = Sample> + Send>,
    channels: ChannelCount,
    rate: SampleRate,
    duration: Option<Duration>,
    room: Arc<RwLock<Preset>>,
    wet: Arc<AtomicU32>,
    dry_done: Arc<AtomicBool>,
    preset: Preset,
    cursor: usize,
    early: Vec<f32>,
    late: Vec<f32>,
    filter: Vec<f32>,
    tail: usize,
    remaining: Option<usize>,
}
impl Iterator for RoomSamples {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        let input = if let Some(input) = self.source.next() {
            input
        } else {
            self.dry_done.store(true, Ordering::Release);
            let remaining = self.remaining.get_or_insert(self.tail);
            if *remaining == 0 {
                return None;
            }
            *remaining -= 1;
            0.
        };
        if self.cursor % (256 * self.channels.get() as usize) == 0 {
            self.preset = *self.room.read().unwrap_or_else(|e| e.into_inner());
        }
        let unit = self.rate.get() as f32 * self.channels.get() as f32;
        let e_delay = ((self.preset.early * 0.1).clamp(0.01, 0.1) * unit) as usize;
        let l_delay = ((self.preset.late * 0.2).clamp(0.03, 0.2) * unit) as usize;
        let e = self.cursor % self.early.len();
        let l = self.cursor % self.late.len();
        let c = self.cursor % self.filter.len();
        let early = self.early
            [(e + self.early.len() - e_delay.min(self.early.len() - 1)) % self.early.len()];
        let late =
            self.late[(l + self.late.len() - l_delay.min(self.late.len() - 1)) % self.late.len()];
        self.filter[c] += (late - self.filter[c]) * (1. - self.preset.damping.clamp(0., 0.95));
        self.early[e] = input;
        self.late[l] = input + early * 0.25 + self.filter[c] * self.preset.feedback.clamp(0., 0.85);
        self.cursor += 1;
        let wet = f32::from_bits(self.wet.load(Ordering::Relaxed));
        Some(
            (input + (early * 0.2 + self.filter[c] * 0.25) * self.preset.gain * wet * 0.75)
                .clamp(-1., 1.),
        )
    }
}
impl Source for RoomSamples {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        self.channels
    }
    fn sample_rate(&self) -> SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        self.duration.map(|d| d + Duration::from_secs(2))
    }
}
