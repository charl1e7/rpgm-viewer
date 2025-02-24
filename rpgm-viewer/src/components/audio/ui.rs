use super::AudioState;

impl AudioState {
    pub fn show(&mut self, ctx: &egui::Context) {
        if self.current_audio.is_some() {
            egui::TopBottomPanel::bottom("audio_controls").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.is_playing {
                        if ui.button("⏸").clicked() {
                            self.pause_audio();
                        }
                    } else {
                        if ui.button("▶").clicked() {
                            self.resume_audio();
                        }
                    }
                    if ui.button("⏹").clicked() {
                        self.stop_audio();
                    }
                    if let Some(path) = &self.current_audio {
                        ui.label(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                });
            });
        }
    }
}
