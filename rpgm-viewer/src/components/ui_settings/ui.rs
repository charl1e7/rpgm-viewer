use super::UiSettings;
use crate::components::file_browser::FileBrowser;

pub struct UiSettingsWindow;

impl UiSettingsWindow {
    pub fn show(ctx: &egui::Context, settings: &mut UiSettings, file_browser: &mut FileBrowser) {
        egui::Window::new("UI Settings")
            .open(&mut settings.show_ui_settings)
            .show(ctx, |ui| {
                ui.checkbox(&mut settings.show_thumbnails, "Show Thumbnails");
                if settings.show_thumbnails {
                    ui.add(
                        egui::Slider::new(&mut settings.thumbnail_size, 16.0..=128.0)
                            .text("Thumbnail Size"),
                    );

                    ui.collapsing("Thumbnail Cache Settings", |ui| {
                        ui.add(
                            egui::Slider::new(&mut settings.thumbnail_compression_size, 32..=1024)
                                .text("Thumbnail Resolution"),
                        );

                        ui.add(
                            egui::Slider::new(&mut settings.cache_update, 10..=300)
                                .text("Cache Update Interval (sec)"),
                        );

                        if ui.button("Clear Thumbnail Cache").clicked() {
                            file_browser.clear_thumbnail_cache();
                        }
                    });
                }

                ui.add(egui::Slider::new(&mut settings.ui_scale, 1.0..=3.0).text("UI Scale"));

                ui.add(egui::Slider::new(&mut settings.font_size, 8.0..=32.0).text("Font Size"));

                ui.checkbox(&mut settings.show_logger, "Show Logger");
            });
    }
}
