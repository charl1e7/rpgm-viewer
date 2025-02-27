use std::time::Duration;
pub mod ui;
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct UiSettings {
    pub show_logger: bool,
    pub show_thumbnails: bool,
    pub thumbnail_size: f32,
    pub ui_scale: f32,
    pub font_size: f32,
    pub show_settings: bool,
    pub show_ui_settings: bool,
    pub thumbnail_compression_size: u32,
    pub cache_update: u64,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            show_logger: false,
            show_thumbnails: true,
            thumbnail_size: 128.0,
            ui_scale: 1.3,
            font_size: 15.0,
            show_settings: false,
            show_ui_settings: false,
            thumbnail_compression_size: 256,
            cache_update: 60,
        }
    }
}

impl UiSettings {
    pub fn apply(&mut self, ctx: &egui::Context) {
        ctx.set_pixels_per_point(self.ui_scale);
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(self.font_size * 1.2, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(self.font_size, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(self.font_size, egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(self.font_size, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(self.font_size * 0.8, egui::FontFamily::Proportional),
            ),
        ]
        .into();
        ctx.set_style(style);
    }

    pub fn toggle_ui_settings(&mut self) {
        self.show_ui_settings = !self.show_ui_settings;
    }

    pub fn toggle_logger(&mut self) {
        self.show_logger = !self.show_logger;
    }

    pub fn set_thumbnail_size(&mut self, size: f32) {
        self.thumbnail_size = size;
    }

    pub fn set_ui_scale(&mut self, scale: f32) {
        self.ui_scale = scale;
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    pub fn get_thumbnail_size(&self) -> f32 {
        self.thumbnail_size
    }

    pub fn should_show_thumbnails(&self) -> bool {
        self.show_thumbnails
    }

    pub fn toggle_thumbnails(&mut self) {
        self.show_thumbnails = !self.show_thumbnails;
    }

    pub fn get_thumbnail_compression_size(&self) -> u32 {
        self.thumbnail_compression_size
    }

    pub fn get_cache_update_interval(&self) -> Duration {
        Duration::from_secs(self.cache_update)
    }
}
