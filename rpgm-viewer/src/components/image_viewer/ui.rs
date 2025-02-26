use log::info;

use crate::components::{crypt_manager::CryptManager, file_browser::FileBrowser};

use super::ImageViewer;

impl ImageViewer {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        file_browser: &mut FileBrowser,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((path, texture)) = &file_browser.current_image {
                egui::containers::Frame::new().show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            let available_size = ui.available_size();
                            let texture_size = texture.size_vec2();
                            let aspect_ratio = texture_size.x / texture_size.y;

                            let mut size = available_size;
                            if size.x * texture_size.y > size.y * texture_size.x {
                                size.x = size.y * aspect_ratio;
                            } else {
                                size.y = size.x / aspect_ratio;
                            }

                            ui.add(egui::Image::new(texture).fit_to_exact_size(size));
                        },
                    );
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.4);
                    ui.heading("Welcome to Image Viewer");
                    ui.add_space(20.0);
                    if ui.button("📁 Open Folder...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            crypt_manager.set_current_directory(path, Some(file_browser));
                        }
                    }
                    ui.add_space(10.0);
                    if ui.button("🖼 Open Image...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
                            .pick_file()
                        {
                            if let Some(decrypter) = crypt_manager.get_decrypter() {
                                match Self::load_image(&path, ctx, Some(decrypter)) {
                                    Some(texture) => {
                                        file_browser.current_image =
                                            Some((path.to_path_buf(), texture));
                                    }
                                    None => {
                                        info!("Failed to load image, resetting to welcome screen");
                                        file_browser.current_image = None;
                                    }
                                }
                            }
                        }
                    }
                });
            }
        });
    }
}
