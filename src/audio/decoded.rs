use bevy::{
    asset::{AssetLoader, LoadContext, LoadState, io::Reader},
    audio::{ChannelCount, Decodable, Sample, SampleRate, Source},
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::Arc,
    time::{Duration, Instant},
};

const CACHE_BYTES: usize = 64 * 1024 * 1024;
const MAX_CLIP_BYTES: usize = 8 * 1024 * 1024;
const MAX_CLIP_SECONDS: f32 = 10.;

#[derive(Asset, TypePath)]
pub(super) struct DecodedClip {
    samples: Arc<[Sample]>,
    channels: ChannelCount,
    rate: SampleRate,
    decode_nanos: u64,
}

impl Decodable for DecodedClip {
    type Decoder = ClipSamples;

    fn decoder(&self) -> ClipSamples {
        ClipSamples {
            samples: self.samples.clone(),
            cursor: 0,
            channels: self.channels,
            rate: self.rate,
        }
    }
}

pub(super) struct ClipSamples {
    samples: Arc<[Sample]>,
    cursor: usize,
    channels: ChannelCount,
    rate: SampleRate,
}

impl Iterator for ClipSamples {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = *self.samples.get(self.cursor)?;
        self.cursor += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.samples.len().saturating_sub(self.cursor);
        (remaining, Some(remaining))
    }
}

impl Source for ClipSamples {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len().saturating_sub(self.cursor))
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.samples.len() as f64 / self.channels.get() as f64 / self.rate.get() as f64,
        ))
    }
}

#[derive(Default, TypePath)]
pub(super) struct DecodedClipLoader;

impl AssetLoader for DecodedClipLoader {
    type Asset = DecodedClip;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<DecodedClip, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        AsyncComputeTaskPool::get()
            .spawn(async move {
                let _span = info_span!("audio_clip_decode", encoded_bytes = bytes.len()).entered();
                let started = Instant::now();
                let source = AudioSource {
                    bytes: bytes.into(),
                };
                let result = std::panic::catch_unwind(|| {
                    let mut decoder = source.decoder();
                    let channels = decoder.channels();
                    let rate = decoder.sample_rate();
                    let maximum = MAX_CLIP_BYTES / size_of::<Sample>();
                    let samples: Vec<_> = decoder.by_ref().take(maximum + 1).collect();
                    if samples.is_empty() || samples.len() > maximum {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Decoded short clip exceeds the PCM cache limit or is empty",
                        ));
                    }
                    Ok(DecodedClip {
                        samples: samples.into(),
                        channels,
                        rate,
                        decode_nanos: started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
                    })
                });
                result.unwrap_or_else(|_| {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Audio decoder failed",
                    ))
                })
            })
            .await
    }

    fn extensions(&self) -> &[&str] {
        &["oga", "ogg", "spx"]
    }
}

struct Entry {
    handle: Handle<DecodedClip>,
    used: u64,
    bytes: Option<usize>,
}

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct AudioDecodeStats {
    pub decoded_sources: u64,
    pub decode_worker_nanoseconds: u64,
    pub cache_hits: u64,
    pub evictions: u64,
    pub streaming_fallbacks: u64,
    pub cache_owned_pcm_bytes: usize,
    pub resident_pcm_bytes: usize,
}

#[derive(Resource, Default)]
pub(super) struct ClipCache {
    entries: HashMap<String, Entry>,
    failed: HashSet<String>,
    sequence: u64,
}

impl ClipCache {
    pub(super) fn clip(
        &mut self,
        server: &AssetServer,
        path: &str,
        seconds: f32,
        stats: &mut AudioDecodeStats,
    ) -> Option<Handle<DecodedClip>> {
        if seconds > MAX_CLIP_SECONDS || self.failed.contains(path) {
            return None;
        }
        self.sequence = self.sequence.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(path) {
            entry.used = self.sequence;
            stats.cache_hits += 1;
            return Some(entry.handle.clone());
        }
        let handle = server.load(path.to_owned());
        self.entries.insert(
            path.to_owned(),
            Entry {
                handle: handle.clone(),
                used: self.sequence,
                bytes: None,
            },
        );
        Some(handle)
    }
}

pub(super) fn maintain(
    mut commands: Commands,
    server: Res<AssetServer>,
    clips: Res<Assets<DecodedClip>>,
    mut cache: ResMut<ClipCache>,
    mut stats: ResMut<AudioDecodeStats>,
    players: Query<
        (Entity, &AudioPlayer<DecodedClip>),
        (Without<AudioSink>, Without<SpatialAudioSink>),
    >,
) {
    for entry in cache.entries.values_mut() {
        if entry.bytes.is_none()
            && let Some(clip) = clips.get(&entry.handle)
        {
            entry.bytes = Some(clip.samples.len() * size_of::<Sample>());
            stats.decoded_sources += 1;
            stats.decode_worker_nanoseconds = stats
                .decode_worker_nanoseconds
                .saturating_add(clip.decode_nanos);
        }
    }
    for (entity, player) in &players {
        if matches!(
            server.get_load_state(player.0.id()),
            Some(LoadState::Failed(_))
        ) && let Some(path) = server.get_path(player.0.id())
        {
            let path = path.to_string();
            cache.failed.insert(path.clone());
            cache.entries.remove(&path);
            commands
                .entity(entity)
                .remove::<AudioPlayer<DecodedClip>>()
                .insert(AudioPlayer::<AudioSource>::new(server.load(path)));
            stats.streaming_fallbacks += 1;
        }
    }
    let mut owned: usize = cache.entries.values().filter_map(|entry| entry.bytes).sum();
    while owned > CACHE_BYTES {
        let Some(path) = cache
            .entries
            .iter()
            .filter(|(_, entry)| entry.bytes.is_some())
            .min_by_key(|(_, entry)| entry.used)
            .map(|(path, _)| path.clone())
        else {
            break;
        };
        if let Some(entry) = cache.entries.remove(&path) {
            owned -= entry.bytes.unwrap_or(0);
            stats.evictions += 1;
        }
    }
    stats.cache_owned_pcm_bytes = owned;
    stats.resident_pcm_bytes = clips
        .iter()
        .map(|(_, clip)| clip.samples.len() * size_of::<Sample>())
        .sum();
}
