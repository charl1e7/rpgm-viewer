use rfd;
use std::path::PathBuf;

use crate::components::crypt_manager::CryptManager;

pub struct CryptSettingsWindow;

impl CryptSettingsWindow {
    pub fn show(ctx: &egui::Context, settings: &mut CryptManager) {
        if let Some(root) = settings.current_folder.clone() {
            if let Some(crypt_settings) = settings.get_mut_settings() {
                let key_hex = if let Some(key) = &mut crypt_settings.encryption_key {
                    key.as_str()
                        .bytes()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    String::new()
                };

                let mut version = crypt_settings.rpgmaker_version;
                let decrypt_path = crypt_settings.decrypt_path.clone();
                let mut show_settings = crypt_settings.show_settings;

                egui::Window::new("Crypt Settings")
                    .open(&mut show_settings)
                    .show(ctx, |ui| {
                        let mut key_hex = key_hex.clone();
                        ui.horizontal(|ui| {
                            ui.label("Encryption Key (HEX):");
                            if ui.text_edit_singleline(&mut key_hex).changed() {
                                settings.handle_key_hex_input(&root, key_hex.clone());
                            }
                        });

                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("RPG Maker Version:");
                            egui::ComboBox::new("rpgmaker_version", "")
                                .selected_text(format!("{:?}", version))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut version,
                                        rpgm_enc::RPGMakerVersion::MV,
                                        "MV",
                                    );
                                    ui.selectable_value(
                                        &mut version,
                                        rpgm_enc::RPGMakerVersion::MZ,
                                        "MZ",
                                    );
                                });
                        });

                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Decrypt Path:");
                            let mut path = match &decrypt_path {
                                Some(path) => path.to_string_lossy().into_owned(),
                                None => String::new(),
                            };
                            if ui.text_edit_singleline(&mut path).changed() {
                                settings.get_mut_settings().unwrap().decrypt_path =
                                    Some(PathBuf::from(path));
                            }
                            if ui.button("Browse...").clicked() {
                                if let Some(path) =
                                    rfd::FileDialog::new().set_directory(root).pick_folder()
                                {
                                    settings.get_mut_settings().unwrap().decrypt_path = Some(path);
                                }
                            }
                        });
                    });

                if let Some(crypt_settings) = settings.get_mut_settings() {
                    crypt_settings.show_settings = show_settings;
                    crypt_settings.rpgmaker_version = version;
                }
            }
        }
    }
}
