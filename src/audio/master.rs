use super::{DynamicsDefinition, EqualizerDefinition, MasterDefinition};

struct Equalizer {
    coefficients: [f32; 5],
    memory: [[f32; 4]; 2],
}
impl Equalizer {
    fn new(definition: &EqualizerDefinition, band: usize) -> Self {
        let mut coefficients = [1., 0., 0., 0., 0.];
        if definition.enabled {
            let amplitude = 10_f64.powf((definition.gain * 0.025) as f64) as f32;
            let angle = (definition.frequency / 48000. * 2.).max(1e-6) * std::f32::consts::PI;
            let (sin, cos) = angle.sin_cos();
            let alpha = sin / (definition.quality * 2.);
            coefficients = if matches!(band, 1 | 2) {
                let inverse = 1. / (1. + alpha / amplitude);
                [
                    inverse * (1. + alpha * amplitude),
                    inverse * -2. * cos,
                    inverse * (1. - alpha * amplitude),
                    inverse * -2. * cos,
                    inverse * (1. - alpha / amplitude),
                ]
            } else {
                let sum = amplitude + 1.;
                let difference = amplitude - 1.;
                let beta = amplitude.sqrt() * 2. * alpha;
                if band == 0 {
                    let numerator = sum - difference * cos;
                    let denominator = sum + difference * cos;
                    let inverse = 1. / (denominator + beta);
                    [
                        inverse * (numerator + beta) * amplitude,
                        inverse * (difference - sum * cos) * amplitude * 2.,
                        inverse * (numerator - beta) * amplitude,
                        inverse * (sum * cos + difference) * -2.,
                        inverse * (denominator - beta),
                    ]
                } else {
                    let numerator = sum + difference * cos;
                    let denominator = sum - difference * cos;
                    let inverse = 1. / (denominator + beta);
                    [
                        inverse * (numerator + beta) * amplitude,
                        inverse * (sum * cos + difference) * amplitude * -2.,
                        inverse * (numerator - beta) * amplitude,
                        inverse * (difference - sum * cos) * 2.,
                        inverse * (denominator - beta),
                    ]
                }
            };
        }
        Self {
            coefficients,
            memory: [[0.; 4]; 2],
        }
    }
    fn sample(&mut self, input: f32, channel: usize) -> f32 {
        let [b0, b1, b2, a1, a2] = self.coefficients;
        let [x1, x2, y1, y2] = self.memory[channel];
        let output = b0 * input + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        self.memory[channel] = [input, x1, output, y1];
        output
    }
}

struct Dynamics {
    definition: DynamicsDefinition,
    attack: f32,
    release: f32,
    exponent: f32,
    makeup: f32,
    detector: [f32; 2],
    gain: [f32; 2],
    highpass: [[f32; 2]; 2],
}
impl Dynamics {
    fn new(definition: &DynamicsDefinition) -> Self {
        let coefficient = |seconds: f32| {
            let samples = seconds.max(1e-6) * 48000.;
            let squared = samples * samples;
            (((squared as f64 * 4. + 1.).sqrt() - 1.) / (squared as f64 * 2.)) as f32
        };
        let exponent = if definition.enabled {
            1. / definition.ratio - 1.
        } else {
            0.
        };
        let threshold_db = 20. * definition.threshold.max(1.52879e-05).log10();
        let makeup = 10_f64.powf((threshold_db * exponent * 0.05) as f64) as f32;
        Self {
            definition: definition.clone(),
            attack: coefficient(definition.attack),
            release: coefficient(definition.release),
            exponent,
            makeup,
            detector: [0.; 2],
            gain: [makeup; 2],
            highpass: [[0.; 2]; 2],
        }
    }
    fn sample(&mut self, input: f32, channel: usize, frame: usize) -> f32 {
        let definition = &self.definition;
        if !definition.enabled {
            return input;
        }
        let input = input * definition.pre_gain;
        let filtered = input - self.highpass[channel][0] + self.highpass[channel][1] * 0.995;
        self.highpass[channel] = [input, filtered];
        let peak = if frame < 4 { filtered } else { filtered.abs() };
        let previous = self.detector[channel];
        let envelope = (peak - previous).max(0.) * self.attack + previous - previous * self.release
            + 2.3283064e-10;
        if frame & 31 == 0 || previous + 1.52879e-05 < envelope {
            self.gain[channel] = if envelope < definition.threshold {
                self.makeup
            } else {
                (envelope * definition.threshold).powf(self.exponent) / self.makeup
            };
        }
        self.detector[channel] = envelope;
        filtered * self.gain[channel] * definition.post_gain * 0.5
    }
}

pub(super) struct Master {
    gain: f32,
    equalizers: [Equalizer; 4],
    compressor: Dynamics,
    limiter: Dynamics,
}
impl Master {
    pub(super) fn new(definition: &MasterDefinition) -> Self {
        Self {
            gain: definition.gain,
            equalizers: std::array::from_fn(|band| {
                Equalizer::new(&definition.equalizers[band], band)
            }),
            compressor: Dynamics::new(&definition.compressor),
            limiter: Dynamics::new(&definition.limiter),
        }
    }
    pub(super) fn sample(&mut self, mut input: [f32; 2], frame: usize) -> [f32; 2] {
        for (channel, sample) in input.iter_mut().enumerate() {
            *sample = self.compressor.sample(*sample, channel, frame);
            *sample *= self.gain;
            for equalizer in &mut self.equalizers {
                *sample = equalizer.sample(*sample, channel);
            }
            *sample = self.limiter.sample(*sample, channel, frame);
        }
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_shelves_preserve_their_dc_and_nyquist_gains() {
        for band in [0, 3] {
            let definition = EqualizerDefinition {
                enabled: true,
                gain: 5.,
                frequency: 1000.,
                quality: 0.7,
            };
            let eq = Equalizer::new(&definition, band);
            let [b0, b1, b2, a1, a2] = eq.coefficients;
            let dc = (b0 + b1 + b2) / (1. + a1 + a2);
            let nyquist = (b0 - b1 + b2) / (1. - a1 + a2);
            let boosted = 10_f32.powf(5. / 20.);
            assert!((dc - if band == 0 { boosted } else { 1. }).abs() < 0.0001);
            assert!((nyquist - if band == 3 { boosted } else { 1. }).abs() < 0.0001);
        }
    }

    #[test]
    fn native_dynamics_tracks_each_channel_and_bypasses_disabled_processing() {
        let mut definition = DynamicsDefinition {
            enabled: false,
            pre_gain: 1.,
            post_gain: 2.,
            threshold: 0.5,
            ratio: 4.,
            attack: 0.001,
            release: 0.1,
        };
        let mut dynamics = Dynamics::new(&definition);
        assert_eq!(dynamics.sample(0.25, 0, 0), 0.25);
        definition.enabled = true;
        let mut dynamics = Dynamics::new(&definition);
        for frame in 0..256 {
            assert!(dynamics.sample(1., 0, frame).is_finite());
            assert_eq!(dynamics.sample(0., 1, frame), 0.);
        }
        assert!(dynamics.detector[0] > dynamics.detector[1]);
    }
}
