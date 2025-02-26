use super::AudioState;
use egui::{Color32, Frame, Id, RichText, Rounding, Stroke, Vec2};
use std::time::Duration;

impl AudioState {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let metadata = self.get_current_metadata();
        let current_time = self.get_current_time();
        let total_duration = metadata.duration;
        let mut volume = self.get_volume();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if let Some(title) = &metadata.title {
                        ui.add(egui::Label::new(RichText::new(title).size(18.0).strong()));
                    } else {
                        ui.add(egui::Label::new(
                            RichText::new(&metadata.filename).size(18.0).strong(),
                        ));
                    }

                    let mut info_text = String::new();
                    if let Some(artist) = &metadata.artist {
                        info_text.push_str(artist);
                        if metadata.album.is_some() {
                            info_text.push_str(" - ");
                        }
                    }
                    if let Some(album) = &metadata.album {
                        info_text.push_str(album);
                    }

                    if !info_text.is_empty() {
                        ui.add(egui::Label::new(RichText::new(info_text).size(14.0)));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let slider = egui::Slider::new(&mut volume, 0.0..=1.0)
                        .show_value(false)
                        .text("Volume");

                    if ui.add_sized([100.0, 20.0], slider).changed() {
                        self.set_volume(volume);
                    }

                    if volume > 0.5 {
                        ui.label("🔊");
                    } else if volume > 0.0 {
                        ui.label("🔉");
                    } else {
                        ui.label("🔈");
                    }

                    ui.add_space(20.0);

                    if ui.button(RichText::new("⏹").size(18.0)).clicked() {
                        self.stop_audio();
                    }

                    if self.is_playing {
                        if ui.button(RichText::new("⏸").size(18.0)).clicked() {
                            self.pause_audio();
                        }
                    } else {
                        if ui.button(RichText::new("▶").size(18.0)).clicked() {
                            self.resume_audio();
                        }
                    }
                });
            });

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let time_text = format!("{}", format_duration(current_time));
                ui.label(time_text);

                let mut current_pos = self.get_current_position();

                let timeline_response = ui.add(
                    egui::Slider::new(&mut current_pos, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true),
                );

                if timeline_response.drag_stopped() || timeline_response.clicked() {
                    self.seek_to_percent(current_pos);
                }

                let duration_text = format!("{}", format_duration(total_duration));
                ui.label(duration_text);
            });
        });
    }
}

// as MM:SS
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
