use std::{
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rodio::{Decoder, OutputStream, Sink};
use rpgm_enc::Decrypter;


pub mod ui;
#[derive(Default)]
pub struct AudioState {
    _stream: Option<OutputStream>,
    sink: Option<Arc<Mutex<Sink>>>,
    current_audio: Option<PathBuf>,
    pub is_playing: bool,
}

impl AudioState {
    pub fn play_audio(&mut self, path: &Path, decrypter: &Decrypter) -> Result<(), String> {
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().stop();
        }

        if self._stream.is_none() {
            let (stream, stream_handle) = OutputStream::try_default()
                .map_err(|e| format!("Failed to initialize audio: {}", e))?;
            let sink = Sink::try_new(&stream_handle)
                .map_err(|e| format!("Failed to create audio sink: {}", e))?;
            self._stream = Some(stream);
            self.sink = Some(Arc::new(Mutex::new(sink)));
        }

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
        let reader = BufReader::new(cursor);

        let source = Decoder::new(reader).map_err(|e| format!("Failed to decode audio: {}", e))?;

        if let Some(sink) = &self.sink {
            sink.lock().unwrap().append(source);
            sink.lock().unwrap().play();
            self.current_audio = Some(path.to_path_buf());
            self.is_playing = true;
        }

        Ok(())
    }

    pub fn stop_audio(&mut self) {
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().stop();
            self.is_playing = false;
        }
    }

    pub fn pause_audio(&mut self) {
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().pause();
            self.is_playing = false;
        }
    }

    pub fn resume_audio(&mut self) {
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().play();
            self.is_playing = true;
        }
    }
}
