use log::{debug, info, trace};

use crate::components::audio::AudioState;
use crate::components::crypt_manager::CryptManager;
use crate::components::crypt_settings::ui::CryptSettingsWindow;
use crate::components::dropped_file::DroppedFile;
use crate::components::file_browser::FileBrowser;
use crate::components::image_viewer::ImageViewer;
use crate::components::ui_settings::UiSettings;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct ImageViewerApp {
    crypt_settings: CryptManager,
    ui_settings: UiSettings,
    file_browser: FileBrowser,
    dropped_file: DroppedFile,
    image_viewer: ImageViewer,
    #[serde(skip)]
    audio: AudioState,
}

impl ImageViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            let mut app: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.audio = AudioState::new();
            return app;
        }

        let mut app = Self::default();
        app.audio = AudioState::new();
        app
    }
}

impl eframe::App for ImageViewerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_settings.apply(ctx);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.file_browser.current_image = None;
            debug!("Esc pressed, current_image reset to None");
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Menu", |ui| {
                    if ui.button("Open Folder...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.crypt_settings
                                .set_current_directory(path, Some(&mut self.file_browser));
                        }
                    }
                    ui.separator();
                    if ui.button("Crypt Settings").clicked() {
                        self.crypt_settings.toggle_settings();
                    }
                    if ui.button("UI Settings").clicked() {
                        self.ui_settings.toggle_ui_settings();
                    }
                    if !cfg!(target_arch = "wasm32") {
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });
            });
        });

        if self.ui_settings.show_ui_settings {
            use crate::components::ui_settings::ui::UiSettingsWindow;
            UiSettingsWindow::show(ctx, &mut self.ui_settings, &mut self.file_browser);
        }

        if self.crypt_settings.show_settings() {
            CryptSettingsWindow::show(ctx, &mut self.crypt_settings);
        }

        egui::SidePanel::left("files_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.file_browser.show(
                    ui,
                    ctx,
                    &mut self.crypt_settings,
                    &self.ui_settings,
                    &mut self.audio,
                );
            });

        if self.audio.is_audio_loaded() {
            egui::TopBottomPanel::bottom("audio_player")
                .min_height(60.0)
                .show(ctx, |ui| {
                    self.audio.show(ui);
                });
        }

        self.image_viewer
            .show(ctx, &mut self.crypt_settings, &mut self.file_browser);

        if self.ui_settings.show_logger {
            egui::Window::new("Log")
                .open(&mut self.ui_settings.show_logger)
                .show(ctx, |ui| {
                    egui_logger::logger_ui().show(ui);
                });
        }

        self.dropped_file
            .show(ctx, &mut self.crypt_settings, &mut self.file_browser);
    }
}
