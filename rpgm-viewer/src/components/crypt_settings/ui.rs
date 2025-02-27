use rfd;
use std::path::PathBuf;

use crate::components::crypt_manager::CryptManager;

pub struct CryptSettingsWindow;

impl CryptSettingsWindow {
    pub fn show(ctx: &egui::Context, settings: &mut CryptManager) {
        if let Some(root) = settings.current_folder.clone() {
            if let Some(crypt_settings) = settings.get_mut_settings() {
                let initial_key_hex = if let Some(key) = &mut crypt_settings.encryption_key {
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
                let crypt_path = crypt_settings.crypt_path.clone();
                let mut show_settings = crypt_settings.show_settings;

                let mut new_key_hex = None;
                let mut new_decrypt_path = decrypt_path.clone();
                let mut new_crypt_path = crypt_path.clone();

                egui::Window::new("Crypt Settings")
                    .open(&mut show_settings)
                    .show(ctx, |ui| {
                        let mut key_hex = initial_key_hex.clone();
                        ui.horizontal(|ui| {
                            ui.label("Encryption Key (HEX):");
                            if ui.text_edit_singleline(&mut key_hex).changed() {
                                new_key_hex = Some(key_hex);
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
                                new_decrypt_path = Some(PathBuf::from(path));
                            }
                            if ui.button("Browse...").clicked() {
                                if let Some(path) =
                                    rfd::FileDialog::new().set_directory(&root).pick_folder()
                                {
                                    new_decrypt_path = Some(path);
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Crypt Path:");
                            let mut path = match &crypt_path {
                                Some(path) => path.to_string_lossy().into_owned(),
                                None => String::new(),
                            };
                            if ui.text_edit_singleline(&mut path).changed() {
                                new_crypt_path = Some(PathBuf::from(path));
                            }
                            if ui.button("Browse...").clicked() {
                                if let Some(path) =
                                    rfd::FileDialog::new().set_directory(&root).pick_folder()
                                {
                                    new_crypt_path = Some(path);
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("Reset Directory").clicked() {
                            new_decrypt_path = Some(root.clone());
                            new_crypt_path = Some(root.clone());
                        }
                    });

                if let Some(crypt_settings) = settings.get_mut_settings() {
                    crypt_settings.show_settings = show_settings;
                    crypt_settings.rpgmaker_version = version;
                    crypt_settings.decrypt_path = new_decrypt_path;
                    crypt_settings.crypt_path = new_crypt_path;
                }
                if let Some(key_hex) = new_key_hex {
                    settings.handle_key_hex_input(key_hex);
                }
            }
        }
    }
}
