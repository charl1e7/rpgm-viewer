use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use symphonia::core::{
    audio::{SampleBuffer, Signal},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::{FormatOptions, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleFormat, StreamConfig,
};

use rpgm_enc::Decrypter;

pub mod ui;

#[derive(Clone)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Duration,
    pub filename: String,
}

impl Default for TrackMetadata {
    fn default() -> Self {
        Self {
            title: None,
            artist: None,
            album: None,
            duration: Duration::from_secs(0),
            filename: "Unknown".to_string(),
        }
    }
}

#[derive(Default)]
pub struct AudioState {
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    current_audio: Option<PathBuf>,
    current_metadata: Arc<Mutex<TrackMetadata>>,
    pub is_playing: bool,
    stream: Option<cpal::Stream>,
    device: Option<Device>,
    sample_rate: u32,
    read_position: Arc<Mutex<usize>>,
    total_samples: Arc<Mutex<usize>>,
    volume: Arc<Mutex<f32>>,
}

impl AudioState {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host.default_output_device();

        Self {
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            current_audio: None,
            current_metadata: Arc::new(Mutex::new(TrackMetadata::default())),
            is_playing: false,
            stream: None,
            device,
            sample_rate: 44100,
            read_position: Arc::new(Mutex::new(0)),
            total_samples: Arc::new(Mutex::new(0)),
            volume: Arc::new(Mutex::new(1.0)),
        }
    }

    pub fn play_audio(&mut self, path: &Path, decrypter: &Decrypter) -> Result<(), String> {
        self.stop_audio();

        let data = if path.extension().map_or(false, |ext| {
            matches!(ext.to_str().unwrap_or(""), "ogg_" | "rpgmvo")
        }) {
            let file_data = std::fs::read(path)
                .map_err(|e| format!("Failed to read encrypted audio file: {}", e))?;

            decrypter
                .decrypt(&file_data)
                .map_err(|e| format!("Failed to decrypt audio: {}", e))?
        } else {
            std::fs::read(path).map_err(|e| format!("Failed to read audio file: {}", e))?
        };

        let cursor = std::io::Cursor::new(data);
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(ext.to_str().unwrap_or(""));
        }

        let meta_opts = MetadataOptions::default();
        let fmt_opts = FormatOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| format!("Error probing media: {}", e))?;

        let mut format = probed.format;

        let mut metadata = TrackMetadata::default();
        metadata.filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if let Some(metadata_rev) = format.metadata().current() {
            for tag in metadata_rev.tags() {
                match tag.std_key {
                    Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                        metadata.title = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Artist) => {
                        metadata.artist = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Album) => {
                        metadata.album = Some(tag.value.to_string());
                    }
                    _ => {}
                }
            }
        }

        let track = format
            .default_track()
            .ok_or("No default track found in the audio file")?;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Error creating decoder: {}", e))?;

        if let Some(time_base) = track.codec_params.time_base {
            if let Some(n_frames) = track.codec_params.n_frames {
                let duration = n_frames as f64 * time_base.numer as f64 / time_base.denom as f64;
                metadata.duration = Duration::from_secs_f64(duration);
            }
        }

        let mut audio_buffer = Vec::new();
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        self.sample_rate = sample_rate;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(_))
                | Err(symphonia::core::errors::Error::ResetRequired) => {
                    break;
                }
                Err(e) => {
                    return Err(format!("Error reading packet: {}", e));
                }
            };

            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(symphonia::core::errors::Error::IoError(_)) => {
                    break;
                }
                Err(e) => {
                    log::warn!("Error decoding packet: {}", e);
                    continue;
                }
            };

            let spec = *decoded.spec();

            let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);

            sample_buffer.copy_interleaved_ref(decoded);

            let samples = sample_buffer.samples();

            audio_buffer.extend_from_slice(samples);
        }

        *self.audio_buffer.lock().unwrap() = audio_buffer;
        *self.total_samples.lock().unwrap() = self.audio_buffer.lock().unwrap().len();
        *self.read_position.lock().unwrap() = 0;
        *self.current_metadata.lock().unwrap() = metadata;

        self.start_playback()?;

        self.current_audio = Some(path.to_path_buf());
        self.is_playing = true;

        Ok(())
    }

    fn start_playback(&mut self) -> Result<(), String> {
        if self.device.is_none() {
            self.device = cpal::default_host().default_output_device();
        }

        let device = self.device.as_ref().ok_or("No audio device available")?;

        let supported_config = device
            .supported_output_configs()
            .map_err(|e| format!("Error getting supported configs: {}", e))?
            .find(|config| config.sample_format() == SampleFormat::F32)
            .ok_or("No supported audio format found")?
            .with_max_sample_rate();

        let config: StreamConfig = supported_config.into();

        let audio_buffer = self.audio_buffer.clone();
        let read_position = self.read_position.clone();
        let total_samples = self.total_samples.clone();
        let volume = self.volume.clone();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut pos = read_position.lock().unwrap();
                    let total = *total_samples.lock().unwrap();
                    let buffer = audio_buffer.lock().unwrap();
                    let current_volume = *volume.lock().unwrap();

                    for sample in data.iter_mut() {
                        if *pos < total {
                            *sample = buffer[*pos] * current_volume;
                            *pos += 1;
                        } else {
                            *sample = 0.0;
                        }
                    }
                },
                |err| log::error!("Error in audio stream: {}", err),
                None,
            )
            .map_err(|e| format!("Error building audio stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Error playing audio: {}", e))?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop_audio(&mut self) {
        self.stream = None;
        *self.read_position.lock().unwrap() = 0;
        self.is_playing = false;
        self.current_audio = None;
    }

    pub fn pause_audio(&mut self) {
        if let Some(stream) = &self.stream {
            let _ = stream.pause();
            self.is_playing = false;
        }
    }

    pub fn resume_audio(&mut self) {
        if let Some(stream) = &self.stream {
            let _ = stream.play();
            self.is_playing = true;
        }
    }

    pub fn seek_to_percent(&mut self, percent: f32) {
        let total = *self.total_samples.lock().unwrap();
        let new_pos = (total as f32 * percent.clamp(0.0, 1.0)) as usize;
        *self.read_position.lock().unwrap() = new_pos;
    }

    pub fn get_current_position(&self) -> f32 {
        let pos = *self.read_position.lock().unwrap();
        let total = *self.total_samples.lock().unwrap();

        if total == 0 {
            return 0.0;
        }

        pos as f32 / total as f32
    }

    pub fn get_current_time(&self) -> Duration {
        let metadata = self.current_metadata.lock().unwrap();

        if self.sample_rate == 0 || *self.total_samples.lock().unwrap() == 0 {
            return Duration::from_secs(0);
        }

        let total_duration = metadata.duration;
        let progress = self.get_current_position();

        Duration::from_secs_f64(total_duration.as_secs_f64() * progress as f64)
    }

    pub fn get_current_metadata(&self) -> TrackMetadata {
        self.current_metadata.lock().unwrap().clone()
    }

    pub fn set_volume(&mut self, volume: f32) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }

    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }
}
