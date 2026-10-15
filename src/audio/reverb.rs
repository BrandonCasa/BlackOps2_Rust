use bevy::{
    audio::{ChannelCount, Decodable, Sample, SampleRate, Source},
    prelude::*,
};
use std::{
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Copy)]
pub(super) struct Preset([f32; 16], f32);
impl Default for Preset {
    fn default() -> Self {
        Self(
            [
                0.5, 0.8, 0.8, 1., 1., 1., 0., 0., 0., 0., 0.5, 0., 1., 1., 0.5, 1.,
            ],
            1.,
        )
    }
}
impl Preset {
    fn clamped(self) -> Self {
        Self(
            std::array::from_fn(|index| match index {
                3..=5 => self.0[index].clamp(0., 4.),
                12 | 13 => self.0[index].clamp(0.1, 16.),
                15 => self.0[index].clamp(0.01, 1.),
                _ => self.0[index].clamp(0., 1.),
            }),
            self.1,
        )
    }
}
#[derive(Resource)]
pub(crate) struct ListenerRoom {
    state: Arc<RwLock<Preset>>,
    room: String,
    from: f32,
    elapsed: f32,
    mixer: Arc<Mutex<WetMixer>>,
    pub(super) started: bool,
}
impl Default for ListenerRoom {
    fn default() -> Self {
        let state = Arc::new(RwLock::new(Preset::default()));
        Self {
            state: state.clone(),
            room: String::new(),
            from: 1.,
            elapsed: 2.,
            mixer: Arc::new(Mutex::new(WetMixer {
                network: Network::new(48000),
                room: state,
                voices: Vec::new(),
                gain: 1.,
                master: None,
            })),
            started: false,
        }
    }
}
pub(super) fn update(
    time: Res<Time>,
    weather: Res<crate::environment::weather::Weather>,
    game: Option<Res<crate::origins::GameDefinition>>,
    mut room: ResMut<ListenerRoom>,
) {
    let Some(game) = game else {
        return;
    };
    {
        let mut mixer = room.mixer.lock().unwrap_or_else(|error| error.into_inner());
        if mixer.master.is_none() {
            mixer.master = game.audio.master.as_ref().map(super::master::Master::new);
        }
        mixer.gain = game
            .audio
            .master
            .as_ref()
            .map_or(1., |master| master.buses[0]);
    }
    if room.room != weather.ambient_room || room.room.is_empty() {
        let Some(profile) = game
            .audio
            .rooms
            .get(&weather.ambient_room)
            .or_else(|| game.audio.rooms.get("tomb_outdoor"))
        else {
            return;
        };
        let previous = room
            .state
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .1;
        room.from = previous;
        room.state
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .0 = profile.parameters;
        room.room = weather.ambient_room.clone();
        room.elapsed = 0.;
    }
    room.elapsed = (room.elapsed + time.delta_secs()).min(2.);
    room.state
        .write()
        .unwrap_or_else(|error| error.into_inner())
        .1 = room.from + (0.75 - room.from) * room.elapsed / 2.;
}
struct Network {
    parameters: Preset,
    ring: [Vec<f32>; 4],
    early_delay: [usize; 16],
    late_delay: [usize; 16],
    early_matrix: [[f32; 4]; 4],
    late_matrix: [[f32; 4]; 4],
    input: [f32; 4],
    early: [f32; 4],
    late: [f32; 4],
    damped: [f32; 4],
    highpass: [[f32; 4]; 2],
    cursor: usize,
    rate: f32,
}
impl Network {
    fn new(rate: u32) -> Self {
        let size = ((32768. * rate as f32 / 48000.).ceil() as usize).next_power_of_two();
        let mut network = Self {
            parameters: Preset::default(),
            ring: std::array::from_fn(|_| vec![0.; size]),
            early_delay: [0; 16],
            late_delay: [0; 16],
            early_matrix: [[0.; 4]; 4],
            late_matrix: [[0.; 4]; 4],
            input: [0.; 4],
            early: [0.; 4],
            late: [0.; 4],
            damped: [0.; 4],
            highpass: [[0.; 4]; 2],
            cursor: 0,
            rate: rate as f32,
        };
        network.update(Preset::default());
        network
    }
    fn update(&mut self, target: Preset) {
        let target = target.clamped();
        let fraction = (1. - target.0[0]) * 256. / (self.rate * 0.1);
        for index in 1..16 {
            self.parameters.0[index] =
                target.0[index] * fraction + self.parameters.0[index] * (1. - fraction);
        }
        let reflect = self.parameters.0[10];
        let near = std::f32::consts::FRAC_1_SQRT_2;
        let side = ((reflect - 1.) * 0.35355338 + near).max(0.);
        let opposite = ((reflect - 1.) * near + near).max(0.);
        let rear = ((reflect - 1.) * 1.0606601 + near).max(0.);
        self.early_matrix = [
            [near, rear, opposite, side],
            [rear, near, side, opposite],
            [opposite, side, near, rear],
            [side, opposite, rear, near],
        ];
        let (sin, cos) = (self.parameters.0[14] * std::f32::consts::FRAC_PI_2).sin_cos();
        self.late_matrix = [
            [cos, sin, 0., 0.],
            [sin, -cos, 0., 0.],
            [0., 0., -cos, sin],
            [0., 0., sin, cos],
        ];
        let early = [
            941., 149., 673., 277., 211., 797., 421., 577., 751., 349., 1009., 173., 499., 613.,
            239., 877.,
        ];
        let late = [
            1103., 1283., 0., 0., 1439., 1619., 0., 0., 0., 0., 1759., 1949., 0., 0., 751., 941.,
        ];
        for index in 0..16 {
            self.early_delay[index] =
                (early[index] * target.0[12] * self.rate / 48000.).round_ties_even() as usize;
            self.late_delay[index] =
                (late[index] * target.0[13] * self.rate / 48000.).round_ties_even() as usize;
        }
    }
    fn sample(&mut self, input: [f32; 4]) -> [f32; 4] {
        let parameters = self.parameters.0;
        let mask = self.ring[0].len() - 1;
        for output in 0..4 {
            let mut early = 0.;
            let mut late = 0.;
            for channel in 0..4 {
                let index = output * 4 + channel;
                early += self.ring[channel]
                    [self.cursor.wrapping_sub(self.early_delay[index]) & mask]
                    * self.early_matrix[output][channel];
                late += self.ring[channel][self.cursor.wrapping_sub(self.late_delay[index]) & mask]
                    * self.late_matrix[output][channel];
            }
            self.early[output] =
                self.early[output] * parameters[6] + early * (1. - parameters[6]) * parameters[1];
            self.late[output] =
                self.late[output] * parameters[9] + late * (1. - parameters[9]) * parameters[2];
        }
        let mix = std::f32::consts::FRAC_1_SQRT_2;
        let mixed = [
            (self.late[3] + self.late[2]) * mix,
            (-self.late[2] + self.late[3]) * mix,
            (-self.late[1] + self.late[0]) * mix,
            (self.late[1] + self.late[0]) * mix,
        ];
        let highpass = parameters[15].powi(2);
        let mut output = [0.; 4];
        for channel in 0..4 {
            self.input[channel] =
                input[channel] * (1. - parameters[8]) + self.input[channel] * parameters[8];
            self.damped[channel] =
                mixed[channel] * (1. - parameters[7]) + self.damped[channel] * parameters[7];
            self.highpass[0][channel] = (self.input[channel] * parameters[11]
                + self.damped[channel]
                + self.early[channel] * parameters[3])
                * parameters[5]
                * 3.
                * (highpass - 1.)
                - self.highpass[0][channel] * highpass;
            self.highpass[1][channel] =
                self.highpass[0][channel] * (highpass - 1.) - self.highpass[1][channel] * highpass;
            self.ring[channel][self.cursor] = mixed[channel] + self.input[channel];
            output[channel] = self.highpass[1][channel];
        }
        self.cursor = (self.cursor + 1) & mask;
        output
    }
}
pub(super) struct VoiceControl {
    pub(super) gain: Arc<AtomicU32>,
    pub(super) filter: Arc<AtomicU32>,
    pub(super) speed: AtomicU32,
    pub(super) paused: AtomicBool,
    pub(super) stopped: AtomicBool,
    pub(super) dry: AtomicU32,
    pub(super) spatial: [AtomicU32; 2],
    pub(super) done: Arc<AtomicBool>,
    pub(super) positioned: bool,
    pan: [[f32; 2]; 2],
}
impl VoiceControl {
    pub(super) fn new(
        gain: Arc<AtomicU32>,
        filter: Arc<AtomicU32>,
        speed: f32,
        paused: bool,
        pan: [[f32; 2]; 2],
        dry: f32,
        done: Arc<AtomicBool>,
        positioned: bool,
    ) -> Self {
        Self {
            gain,
            filter,
            speed: AtomicU32::new(speed.to_bits()),
            paused: AtomicBool::new(paused),
            stopped: AtomicBool::new(false),
            dry: AtomicU32::new(dry.to_bits()),
            spatial: std::array::from_fn(|_| AtomicU32::new(1_f32.to_bits())),
            done,
            positioned,
            pan,
        }
    }
}
struct Reader {
    source: Box<dyn Source<Item = Sample> + Send>,
    channels: usize,
    rate: u32,
    current: Option<[f32; 2]>,
    next: Option<[f32; 2]>,
    fraction: u32,
    frame: usize,
    coefficients: [f32; 2],
    previous: [f32; 2],
    control: Arc<VoiceControl>,
}
impl Reader {
    fn new(input: &AudioSource, looping: bool, control: Arc<VoiceControl>) -> Self {
        let source: Box<dyn Source<Item = Sample> + Send> = if looping {
            Box::new(
                rodio::Decoder::new_looped(std::io::Cursor::new(input.bytes.clone()))
                    .expect("native looping audio must contain a supported encoded source"),
            )
        } else {
            Box::new(input.decoder())
        };
        let channels = source.channels().get() as usize;
        let rate = source.sample_rate().get();
        let mut reader = Self {
            source,
            channels,
            rate,
            current: None,
            next: None,
            fraction: 0,
            frame: 0,
            coefficients: [1., 0.],
            previous: [0.; 2],
            control,
        };
        reader.current = reader.read();
        reader.next = reader.read();
        reader
    }
    fn read(&mut self) -> Option<[f32; 2]> {
        let left = self.source.next()?;
        let mut right = left;
        for channel in 1..self.channels {
            let sample = self.source.next()?;
            if channel == 1 {
                right = sample;
            }
        }
        Some([left, right])
    }
    fn sample(&mut self) -> Option<[f32; 2]> {
        if self.control.stopped.load(Ordering::Acquire) {
            return None;
        }
        if self.control.paused.load(Ordering::Relaxed) {
            return Some([0.; 2]);
        }
        let current = self.current?;
        let next = self.next.unwrap_or([0.; 2]);
        let fraction = self.fraction as f32 * 3.051851e-05;
        let mut sample = std::array::from_fn(|channel| {
            current[channel] * (1. - fraction) + next[channel] * fraction
        });
        let speed = f32::from_bits(self.control.speed.load(Ordering::Relaxed));
        let step = (speed * self.rate as f32 / 48000. * 32768.).round_ties_even() as u32;
        self.fraction += step;
        while self.fraction >= 32768 {
            self.fraction -= 32768;
            self.current = self.next;
            self.next = self.read();
        }
        if self.frame % 256 == 0 {
            self.coefficients = low_pass(
                f32::from_bits(self.control.filter.load(Ordering::Relaxed)),
                48000.,
            );
        }
        if self.coefficients != [1., 0.] {
            for channel in 0..2 {
                sample[channel] = sample[channel] * self.coefficients[0]
                    - self.previous[channel] * self.coefficients[1];
                self.previous[channel] = sample[channel];
            }
        }
        self.frame += 1;
        for channel in 0..2 {
            sample[channel] *= self.control.pan[usize::from(self.channels > 1)][channel];
        }
        Some(sample)
    }
}
struct WetMixer {
    network: Network,
    room: Arc<RwLock<Preset>>,
    voices: Vec<Reader>,
    gain: f32,
    master: Option<super::master::Master>,
}
impl WetMixer {
    fn render(&mut self) -> Vec<f32> {
        let preset = *self.room.read().unwrap_or_else(|error| error.into_inner());
        self.network.update(preset);
        let mut output = Vec::with_capacity(512);
        for frame in 0..256 {
            let mut input = [0.; 4];
            let mut dry = [0.; 2];
            self.voices.retain_mut(|voice| {
                let Some(sample) = voice.sample() else {
                    voice.control.done.store(true, Ordering::Release);
                    return false;
                };
                let gain = f32::from_bits(voice.control.dry.load(Ordering::Relaxed));
                for channel in 0..2 {
                    dry[channel] += if voice.control.positioned {
                        (sample[0] + sample[1])
                            * 0.5
                            * gain
                            * f32::from_bits(voice.control.spatial[channel].load(Ordering::Relaxed))
                    } else {
                        sample[channel] * gain
                    };
                }
                let gain = (f32::from_bits(voice.control.gain.load(Ordering::Relaxed)) * preset.1)
                    .clamp(0., 1.);
                input[0] += sample[0] * gain;
                input[2] += sample[1] * gain;
                true
            });
            let wet = self.network.sample(input);
            let mixed = [dry[0] + wet[0] * self.gain, dry[1] + wet[2] * self.gain];
            output.extend(
                self.master
                    .as_mut()
                    .map_or(mixed, |master| master.sample(mixed, frame)),
            );
        }
        output
    }
}
impl ListenerRoom {
    pub(super) fn enqueue(&self, input: &AudioSource, looping: bool, control: Arc<VoiceControl>) {
        self.mixer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .voices
            .push(Reader::new(input, looping, control));
    }
    pub(super) fn bus(&self) -> RoomBus {
        RoomBus(self.mixer.clone())
    }
}
#[derive(Asset, TypePath)]
pub(crate) struct RoomBus(Arc<Mutex<WetMixer>>);
impl Decodable for RoomBus {
    type Decoder = BusSamples;
    fn decoder(&self) -> BusSamples {
        BusSamples {
            mixer: self.0.clone(),
            pending: Vec::new(),
            next: 0,
        }
    }
}
pub(crate) struct BusSamples {
    mixer: Arc<Mutex<WetMixer>>,
    pending: Vec<f32>,
    next: usize,
}
impl Iterator for BusSamples {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        if self.next == self.pending.len() {
            self.pending = self
                .mixer
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .render();
            self.next = 0;
        }
        let value = self.pending[self.next];
        self.next += 1;
        Some(value)
    }
}
impl Source for BusSamples {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(48000).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
#[derive(Asset, TypePath)]
pub(super) struct RoomClip {
    dry_done: Arc<AtomicBool>,
    control: Arc<VoiceControl>,
}
impl RoomClip {
    pub(super) fn new(dry_done: Arc<AtomicBool>, control: Arc<VoiceControl>) -> Self {
        Self { dry_done, control }
    }
}
impl Decodable for RoomClip {
    type Decoder = RoomSamples;
    fn decoder(&self) -> RoomSamples {
        RoomSamples {
            dry_done: self.dry_done.clone(),
            control: self.control.clone(),
        }
    }
}
pub(super) struct RoomSamples {
    dry_done: Arc<AtomicBool>,
    control: Arc<VoiceControl>,
}
impl Iterator for RoomSamples {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        if self.dry_done.load(Ordering::Acquire) || self.control.stopped.load(Ordering::Acquire) {
            return None;
        }
        Some(0.)
    }
}
impl Source for RoomSamples {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(48000).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
fn low_pass(filter: f32, rate: f32) -> [f32; 2] {
    let gain = (-(1. - filter) * 35.).clamp(-80., 0.);
    if gain >= -1e-6 {
        return [1., 0.];
    }
    let shape = 0.25;
    let power = 10_f64.powf(((1. - shape) * gain * 0.1) as f64) as f32;
    let cosine = (31415.928 / rate).cos();
    let pole = ((1. - cosine * power)
        - ((1. - cosine) * power * 2. - (1. - cosine * cosine) * power * power).sqrt())
        / (1. - power);
    let feed = 10_f64.powf((shape * gain * 0.05) as f64) as f32 * (1. - pole);
    [feed, -pole]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_room_bus_keeps_reflections_after_source_retirement() {
        let mut preset = Preset::default();
        preset.0[15] = 0.5;
        let room = Arc::new(RwLock::new(preset));
        let mut mixer = WetMixer {
            network: Network::new(48000),
            room,
            voices: Vec::new(),
            gain: 1.,
            master: None,
        };
        mixer.network.parameters = preset;
        mixer.network.update(preset);
        mixer.network.sample([1., 0., 0., 0.]);
        let early = mixer.render();
        assert!(early.iter().all(|sample| *sample == 0.));
        let tail = (0..3).flat_map(|_| mixer.render()).collect::<Vec<_>>();
        assert!(mixer.voices.is_empty());
        assert!(tail.iter().any(|sample| sample.abs() > 0.00001));
        assert!(tail.iter().all(|sample| sample.is_finite()));
    }
    #[test]
    fn native_duck_filter_preserves_bypass_and_has_source_dc_and_five_kilohertz_gains() {
        assert_eq!(low_pass(1., 48000.), [1., 0.]);
        let filter = 0.85;
        let gain = -(1. - filter) * 35.;
        let [feed, feedback] = low_pass(filter, 48000.);
        assert!((feed / (1. + feedback) - 10_f32.powf(gain * 0.25 * 0.05)).abs() < 0.00001);
        let response = feed
            / (1. + feedback * feedback + 2. * feedback * (31415.928_f32 / 48000.).cos()).sqrt();
        assert!((response - 10_f32.powf(gain * 0.05)).abs() < 0.00001);
    }
}
