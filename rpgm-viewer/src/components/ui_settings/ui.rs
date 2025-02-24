use super::UiSettings;

pub struct UiSettingsWindow;

impl UiSettingsWindow {
    pub fn show(ctx: &egui::Context, settings: &mut UiSettings) {
        egui::Window::new("UI Settings")
            .open(&mut settings.show_ui_settings)
            .show(ctx, |ui| {
                ui.checkbox(&mut settings.show_thumbnails, "Show Thumbnails");
                if settings.show_thumbnails {
                    ui.add(
                        egui::Slider::new(&mut settings.thumbnail_size, 16.0..=128.0)
                            .text("Thumbnail Size"),
                    );
                }

                ui.add(egui::Slider::new(&mut settings.ui_scale, 1.0..=3.0).text("UI Scale"));

                ui.add(egui::Slider::new(&mut settings.font_size, 8.0..=32.0).text("Font Size"));

                ui.checkbox(&mut settings.show_logger, "Show Logger");
            });
    }
}
