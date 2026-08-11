use rodio::{
    ChannelCount, Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate,
    buffer::SamplesBuffer, source::Source,
};
use rpgm_enc::{Decrypter, FileExtension};
use std::{path::Path, time::Duration};

pub mod ui;

#[derive(Clone, Default)]
pub struct TrackMetadata {
    pub filename: String,
    pub duration: Duration,
}

pub struct AudioState {
    _stream: Option<MixerDeviceSink>,
    player: Option<Player>,
    current_samples: Option<(ChannelCount, SampleRate, Vec<f32>)>,
    current_audio_name: Option<String>,
    current_metadata: TrackMetadata,
    volume: f32,
}

impl Default for AudioState {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioState {
    pub fn new() -> Self {
        let stream = Self::try_init_stream();
        Self {
            _stream: stream,
            player: None,
            current_samples: None,
            current_audio_name: None,
            current_metadata: TrackMetadata::default(),
            volume: 1.0,
        }
    }

    fn try_init_stream() -> Option<MixerDeviceSink> {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(mut stream) => {
                stream.log_on_drop(false);
                Some(stream)
            }
            Err(e) => {
                log::warn!("Audio output device not ready or blocked by browser: {}", e);
                None
            }
        }
    }

    pub fn play_audio(
        &mut self,
        filename: &str,
        raw: &[u8],
        decrypter: &Decrypter,
    ) -> Result<(), String> {
        self.stop_audio();

        if self._stream.is_none() {
            self._stream = Self::try_init_stream();
        }

        let stream = self
            ._stream
            .as_ref()
            .ok_or("No audio output device available. Please grant audio permissions.")?;

        let ext_str = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let data = if let Some(ext) = FileExtension::from_str(ext_str) {
            if ext.is_encrypted() {
                let decrypted = decrypter
                    .decrypt(raw, ext)
                    .map_err(|e| format!("Failed to decrypt audio: {}", e))?;
                decrypter
                    .restore_header(&decrypted, ext)
                    .map_err(|e| format!("Failed to restore audio header: {}", e))?
            } else {
                raw.to_vec()
            }
        } else {
            raw.to_vec()
        };

        let cursor = std::io::Cursor::new(data);
        let decoder = Decoder::try_from(cursor)
            .map_err(|e| format!("Failed to decode audio format: {}", e))?;

        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let samples: Vec<f32> = decoder.collect();

        let duration_secs =
            samples.len() as f64 / (sample_rate.get() as f64 * channels.get() as f64);
        let duration = Duration::from_secs_f64(duration_secs);

        let source = SamplesBuffer::new(channels, sample_rate, samples.clone());
        let player = Player::connect_new(stream.mixer());
        player.set_volume(self.volume);
        player.append(source);
        player.play();

        self.current_samples = Some((channels, sample_rate, samples));
        self.current_metadata = TrackMetadata {
            filename: filename.to_string(),
            duration,
        };
        self.player = Some(player);
        self.current_audio_name = Some(filename.to_string());

        Ok(())
    }

    pub fn stop_audio(&mut self) {
        if let Some(player) = self.player.take() {
            player.clear();
        }
        self.current_audio_name = None;
        self.current_samples = None;
    }

    pub fn pause_audio(&mut self) {
        if let Some(player) = &self.player {
            player.pause();
        }
    }

    pub fn resume_audio(&mut self) {
        if let Some(player) = &self.player {
            if player.empty() {
                self.replay_current();
            } else {
                player.play();
            }
        }
    }

    fn replay_current(&mut self) {
        if let (Some(stream), Some((channels, sample_rate, samples))) =
            (&self._stream, &self.current_samples)
        {
            let player = Player::connect_new(stream.mixer());
            player.set_volume(self.volume);
            let source = SamplesBuffer::new(*channels, *sample_rate, samples.clone());
            player.append(source);
            player.play();
            self.player = Some(player);
        }
    }

    pub fn seek_to_percent(&mut self, percent: f32) {
        let total = self.current_metadata.duration;
        if total <= Duration::ZERO {
            return;
        }
        let target = total.mul_f32(percent.clamp(0.0, 1.0));

        if let Some(player) = &self.player {
            if player.empty() {
                if let (Some(stream), Some((channels, sample_rate, samples))) =
                    (&self._stream, &self.current_samples)
                {
                    let new_player = Player::connect_new(stream.mixer());
                    new_player.set_volume(self.volume);
                    let source = SamplesBuffer::new(*channels, *sample_rate, samples.clone());
                    new_player.append(source);
                    let _ = new_player.try_seek(target);
                    new_player.play();
                    self.player = Some(new_player);
                    return;
                }
            }

            if player.try_seek(target).is_err() {
                if let (Some(stream), Some((channels, sample_rate, samples))) =
                    (&self._stream, &self.current_samples)
                {
                    let new_player = Player::connect_new(stream.mixer());
                    new_player.set_volume(self.volume);
                    let source = SamplesBuffer::new(*channels, *sample_rate, samples.clone());
                    new_player.append(source);
                    let _ = new_player.try_seek(target);
                    new_player.play();
                    self.player = Some(new_player);
                }
            }
        }
    }

    pub fn get_current_position(&self) -> f32 {
        let total = self.current_metadata.duration.as_secs_f32();
        if total <= 0.0 {
            return 0.0;
        }
        let pos = self
            .player
            .as_ref()
            .map(|p| {
                if p.empty() {
                    total
                } else {
                    p.get_pos().as_secs_f32()
                }
            })
            .unwrap_or(0.0);
        (pos / total).clamp(0.0, 1.0)
    }

    pub fn get_current_time(&self) -> Duration {
        self.player
            .as_ref()
            .map(|p| {
                if p.empty() {
                    self.current_metadata.duration
                } else {
                    p.get_pos()
                }
            })
            .unwrap_or(Duration::from_secs(0))
    }

    pub fn get_current_metadata(&self) -> TrackMetadata {
        self.current_metadata.clone()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = &self.player {
            player.set_volume(self.volume);
        }
    }

    pub fn get_volume(&self) -> f32 {
        self.volume
    }

    pub fn is_playing(&self) -> bool {
        self.player
            .as_ref()
            .map_or(false, |p| !p.is_paused() && !p.empty())
    }

    pub fn is_audio_loaded(&self) -> bool {
        self.current_audio_name.is_some()
    }
}
