use super::FileBrowser;
use super::file_entry::FileEntry;
use std::path::{Path, PathBuf};

use crate::components::audio::AudioState;
use crate::components::crypt_manager::CryptManager;
use crate::components::image_viewer::ImageViewer;
use crate::components::ui_settings::UiSettings;
use log::{debug, error, info, trace};
use rpgm_enc::Decrypter;

impl FileBrowser {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        ui_settings: &UiSettings,
        audio: &mut AudioState,
    ) {
        let current_show_thumbnails = ui_settings.show_thumbnails;
        let current_thumbnail_size = ui_settings.get_thumbnail_compression_size();

        if self.last_show_thumbnails != current_show_thumbnails
            || self.last_thumbnail_compression_size != current_thumbnail_size
        {
            self.all_thumbnails_loaded = false;
            self.last_show_thumbnails = current_show_thumbnails;
            self.last_thumbnail_compression_size = current_thumbnail_size;
        }

        ui.heading("Files");
        self.show_search_bar(ui);
        ui.separator();

        if let Some(root) = &crypt_manager.current_folder {
            let root = root.clone();
            self.check_and_update_cache(&root, ui_settings);
            self.update_entries_cache(&root, crypt_manager, ui_settings);
            let entries = self.get_filtered_entries(&root);
            self.show_file_list(ui, ctx, entries, crypt_manager, ui_settings, audio);
        }

        self.show_delete_confirmation_dialog(ctx);
    }

    fn show_search_bar(&mut self, ui: &mut egui::Ui) -> bool {
        ui.horizontal(|ui| {
            ui.label("🔍");
            let search_field = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .desired_width((ui.available_width() - 30.0).max(10.0)),
            );
            let changed = search_field.changed();

            if changed {
                self.all_thumbnails_loaded = false;
            }

            if !self.search_query.is_empty() {
                if ui.button("✖").clicked() {
                    self.search_query.clear();
                    self.search_results_cache = None;
                    self.all_thumbnails_loaded = false;
                    return true;
                }
            }

            if !self.search_query.is_empty() {
                if let Some(search_results) = &self.search_results_cache {
                    ui.label(format!("{} results", search_results.1.len()));
                }
            }

            changed
        })
        .inner
    }

    fn update_entries_cache(
        &mut self,
        root: &Path,
        crypt_manager: &CryptManager,
        ui_settings: &UiSettings,
    ) {
        let crypt_settings = crypt_manager.get_settings().unwrap();
        let expanded_folders = crypt_settings.get_expanded_folders();

        let dir_metadata = std::fs::metadata(root).ok().and_then(|m| m.modified().ok());
        let needs_update = self.entries_cache.is_none()
            || self.last_expanded_state != expanded_folders
            || self.last_update_time.map_or(true, |last| {
                dir_metadata.map_or(true, |current| current > last)
            });

        if needs_update {
            let mut new_entries =
                FileEntry::recursive_collect_entries_flat(root, 0, &expanded_folders);
            self.preserve_thumbnails(&mut new_entries, ui_settings);
            self.entries_cache = Some(new_entries);
            self.last_expanded_state = expanded_folders.clone();
            self.last_update_time = dir_metadata;
            self.all_thumbnails_loaded = false;
        }
    }

    fn preserve_thumbnails(&mut self, new_entries: &mut Vec<FileEntry>, ui_settings: &UiSettings) {
        if !ui_settings.show_thumbnails {
            for entry in new_entries.iter_mut() {
                entry.thumbnail = None;
            }
            return;
        }

        for entry in new_entries.iter_mut() {
            if let Some(texture) = self.thumbnail_cache.get(&entry.path) {
                entry.thumbnail = Some(texture);
            } else {
                trace!("No thumbnail in cache to preserve for: {:?}", entry.path);
            }
        }
    }

    fn get_filtered_entries(&mut self, root: &Path) -> Vec<FileEntry> {
        if self.search_query.is_empty() {
            self.search_results_cache = None;
            return self.entries_cache.as_ref().unwrap().clone();
        } else {
            return self.update_search_results(root);
        }
    }

    fn update_search_results(&mut self, root: &Path) -> Vec<FileEntry> {
        if self.search_results_cache.is_none()
            || self.search_results_cache.as_ref().unwrap().0 != self.search_query
        {
            debug!("Updating search results for query: {}", self.search_query);
            let all_entries = FileEntry::recursive_collect_all_entries_flat(root, 0);
            let query = self.search_query.to_lowercase();

            let mut filtered_entries: Vec<_> = all_entries
                .into_iter()
                .filter(|entry| self.entry_matches_search(entry, root, &query))
                .collect();

            self.preserve_thumbnails(&mut filtered_entries, &UiSettings::default());

            debug!("Found {} matches for '{}'", filtered_entries.len(), query);
            self.search_results_cache = Some((self.search_query.clone(), filtered_entries));
        }
        self.search_results_cache.as_ref().unwrap().1.clone()
    }

    fn entry_matches_search(&self, entry: &FileEntry, root: &Path, query: &str) -> bool {
        if let Ok(relative_path) = entry.path.strip_prefix(root) {
            if relative_path
                .to_string_lossy()
                .to_lowercase()
                .contains(query)
            {
                return true;
            }
        }

        entry.name().to_lowercase().contains(query)
    }

    fn show_file_list(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        mut entries: Vec<FileEntry>,
        crypt_manager: &mut CryptManager,
        ui_settings: &UiSettings,
        audio: &mut AudioState,
    ) {
        if ui_settings.show_thumbnails {
            self.process_thumbnails(ctx, &mut entries);
        }

        self.render_file_list(ui, &entries, ctx, crypt_manager, audio, ui_settings);
    }

    fn render_file_list(
        &mut self,
        ui: &mut egui::Ui,
        entries: &[FileEntry],
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        audio: &mut AudioState,
        ui_settings: &UiSettings,
    ) {
        egui::ScrollArea::both()
            .id_salt("file_list_scroll")
            .auto_shrink([false; 2])
            .stick_to_bottom(false)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for entry in entries.iter() {
                        self.show_entry_row(ui, entry, ctx, crypt_manager, audio, ui_settings);
                    }
                });
            });
    }

    fn process_thumbnails(&mut self, ctx: &egui::Context, entries: &mut Vec<FileEntry>) {
        let loaded_thumbnails = self.thumbnail_cache.process_results(ctx);
        if !loaded_thumbnails.is_empty() {
            debug!("Received {} new thumbnails", loaded_thumbnails.len());
            self.apply_loaded_thumbnails(entries, loaded_thumbnails);
        }
        self.update_caches(entries);
    }

    fn apply_loaded_thumbnails(
        &mut self,
        entries: &mut Vec<FileEntry>,
        loaded_thumbnails: Vec<(PathBuf, egui::TextureHandle)>,
    ) {
        for (path, texture) in loaded_thumbnails {
            let mut found = false;
            for entry in entries.iter_mut() {
                if entry.path == path {
                    entry.thumbnail = Some(texture.clone());
                    found = true;
                    break;
                }
            }
            if !found {
                debug!("Thumbnail for file not found in the list: {:?}", path);
            }
        }
    }

    fn update_caches(&mut self, entries: &[FileEntry]) {
        if !self.search_query.is_empty() {
            self.search_results_cache = Some((self.search_query.clone(), entries.to_vec()));
        } else {
            self.entries_cache = Some(entries.to_vec());
        }
    }

    fn show_entry_row(
        &mut self,
        ui: &mut egui::Ui,
        entry: &FileEntry,
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        audio: &mut AudioState,
        ui_settings: &UiSettings,
    ) {
        let row_response = ui.horizontal(|ui| {
            if entry.nesting_level > 0 {
                let indent_amount = if ui_settings.show_thumbnails {
                    entry.nesting_level as f32 * 12.0
                } else {
                    entry.nesting_level as f32 * 20.0
                };
                ui.add_space(indent_amount);
            }

            if entry.is_folder {
                self.show_folder_entry(ui, entry, crypt_manager);
            } else {
                if ui_settings.show_thumbnails && entry.thumbnail.is_some() {
                    ui.set_min_height(ui_settings.thumbnail_size);

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        self.show_file_icon(ui, entry, ui_settings);

                        let response = ui.add(egui::Button::new(&entry.name()).truncate());
                        if response.clicked() {
                            self.handle_file_click(entry, ctx, crypt_manager, audio);
                        }
                        response.context_menu(|ui| {
                            self.show_file_context_menu(ui, entry, crypt_manager)
                        });
                    });
                } else {
                    self.show_file_icon(ui, entry, ui_settings);
                    let response = ui.add(egui::Button::new(&entry.name()).truncate());
                    if response.clicked() {
                        self.handle_file_click(entry, ctx, crypt_manager, audio);
                    }
                    response
                        .context_menu(|ui| self.show_file_context_menu(ui, entry, crypt_manager));
                }
            }
        });

        if ui_settings.show_thumbnails
            && !entry.is_folder
            && self.is_image_file(&entry.path)
            && entry.thumbnail.is_none()
            && ui.is_rect_visible(row_response.response.rect)
        {
            if let Some(decrypter) = crypt_manager.get_decrypter() {
                self.thumbnail_cache.request_thumbnail(
                    &entry.path,
                    decrypter,
                    ui_settings.get_thumbnail_compression_size(),
                );
            }
        }
    }

    fn show_folder_entry(
        &mut self,
        ui: &mut egui::Ui,
        entry: &FileEntry,
        crypt_manager: &mut CryptManager,
    ) {
        ui.label("📁");
        let response = ui.add(egui::Button::new(&entry.name()).truncate());

        if response.clicked() {
            let crypt_settings = crypt_manager.get_mut_settings().unwrap();
            crypt_settings.toggle_folder_expansion(&entry.path);
        }

        response.context_menu(|ui| self.show_folder_context_menu(ui, entry, crypt_manager));
    }

    fn show_folder_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        entry: &FileEntry,
        crypt_manager: &mut CryptManager,
    ) {
        if ui.button("Extract Key from PNG_/RPGMVP...").clicked() {
            let entries = FileEntry::collect_entries(&entry.path);
            for entry in entries {
                if entry
                    .path
                    .extension()
                    .map_or(false, |ext| ext == "png_" || ext == "rpgmvp")
                {
                    if let Some(key) = crypt_manager.try_extract_key(&entry.path) {
                        crypt_manager.update_encryption_key(&key);
                        break;
                    }
                }
            }
            ui.close();
        }

        ui.separator();

        if ui.button("Encrypt All Files").clicked() {
            match crypt_manager.encrypt_folder(&entry.path, self) {
                Ok(_) => info!("Successfully encrypted folder: {:?}", entry.path),
                Err(e) => error!("Failed to encrypt folder {:?}: {}", entry.path, e),
            }
            ui.close();
        }

        if ui.button("Decrypt All Files").clicked() {
            match crypt_manager.decrypt_folder(&entry.path, self) {
                Ok(_) => info!("Successfully decrypted folder: {:?}", entry.path),
                Err(e) => error!("Failed to decrypt folder {:?}: {}", entry.path, e),
            }
            ui.close();
        }

        ui.separator();

        if ui.button("🗑 Delete").clicked() {
            self.show_delete_confirmation = Some((entry.path.clone(), true));
            ui.close();
        }
    }

    fn show_file_icon(&self, ui: &mut egui::Ui, entry: &FileEntry, ui_settings: &UiSettings) {
        if ui_settings.show_thumbnails {
            if let Some(texture) = entry.thumbnail.as_ref() {
                let display_size = ui_settings.thumbnail_size;

                ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(egui::vec2(display_size, display_size))
                        .maintain_aspect_ratio(true)
                        .texture_options(egui::TextureOptions {
                            magnification: egui::TextureFilter::Linear,
                            minification: egui::TextureFilter::Linear,
                            ..Default::default()
                        }),
                );
                return;
            }
        }

        let icon = if self.is_image_file(&entry.path) {
            if entry.is_encrypted { "🔒" } else { "🖼" }
        } else if self.is_audio_file(&entry.path) {
            if entry.is_encrypted {
                "🔒🎵"
            } else {
                "🎵"
            }
        } else {
            "📄"
        };
        ui.label(icon);
    }

    fn is_image_file(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| {
            matches!(
                ext.to_str().unwrap_or(""),
                "png" | "png_" | "rpgmvp" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
            )
        })
    }

    fn is_audio_file(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| {
            matches!(
                ext.to_str().unwrap_or(""),
                "ogg" | "ogg_" | "rpgmvo" | "mp3" | "m4a" | "m4a_" | "rpgmvm"
            )
        })
    }

    fn handle_file_click(
        &mut self,
        entry: &FileEntry,
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        audio: &mut AudioState,
    ) {
        if self.is_audio_file(&entry.path) {
            if let Some(decrypter) = crypt_manager.get_decrypter() {
                match std::fs::read(&entry.path) {
                    Ok(raw_bytes) => {
                        if let Err(e) = audio.play_audio(&entry.name(), &raw_bytes, decrypter) {
                            error!("Failed to play audio file {:?}: {}", entry.path, e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to read audio file {:?}: {}", entry.path, e);
                    }
                }
            } else {
                error!(
                    "Cannot play encrypted audio {:?}: No decryption key set",
                    entry.path
                );
            }
        } else {
            let decrypter = crypt_manager.get_decrypter().cloned();
            match ImageViewer::load_image(&entry.path, ctx, decrypter) {
                Some(texture) => {
                    self.current_image = Some((entry.path.clone(), texture));
                }
                None => {
                    info!("Failed to load image, resetting to welcome screen");
                    self.current_image = None;
                }
            }
        }
    }

    fn show_file_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        entry: &FileEntry,
        crypt_manager: &mut CryptManager,
    ) {
        if entry
            .path
            .extension()
            .map_or(false, |ext| ext == "png_" || ext == "rpgmvp")
        {
            if ui.button("Extract Key").clicked() {
                if let Some(key) = crypt_manager.try_extract_key(&entry.path) {
                    let crypt_settings = crypt_manager.get_mut_settings().unwrap();
                    crypt_settings.update_encryption_key(&key);
                }
                ui.close();
            }
            ui.separator();
        }

        if entry.is_encrypted {
            if ui.button("Decrypt").clicked() {
                if let Err(e) = crypt_manager.decrypt_image(&entry.path, self) {
                    error!("Failed to decrypt {:?}: {}", entry.path, e);
                }
                ui.close();
            }
        } else {
            if ui.button("Encrypt").clicked() {
                if let Err(e) = crypt_manager.encrypt_image(&entry.path, self) {
                    error!("Failed to encrypt {:?}: {}", entry.path, e);
                }
                ui.close();
            }
        }

        ui.separator();

        if ui.button("🗑 Delete").clicked() {
            self.show_delete_confirmation = Some((entry.path.clone(), false));
            ui.close();
        }
    }

    fn show_delete_confirmation_dialog(&mut self, ctx: &egui::Context) {
        if let Some((path, is_folder)) = &self.show_delete_confirmation {
            let path = path.clone();
            let is_folder = *is_folder;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("⚠️Warning");
                        ui.label(format!(
                            "Are you sure you want to delete {}?",
                            if is_folder {
                                "this folder"
                            } else {
                                "this file"
                            }
                        ));
                        ui.label(path.to_string_lossy().to_string());
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_delete_confirmation = None;
                            }
                            if ui.button("Delete").clicked() {
                                if is_folder {
                                    if let Err(e) = std::fs::remove_dir_all(&path) {
                                        error!("Failed to delete folder {:?}: {}", path, e);
                                    } else {
                                        info!("Successfully deleted folder: {:?}", path);
                                    }
                                } else {
                                    if let Err(e) = std::fs::remove_file(&path) {
                                        error!("Failed to delete file {:?}: {}", path, e);
                                    } else {
                                        info!("Successfully deleted file: {:?}", path);
                                    }
                                }
                                self.show_delete_confirmation = None;
                            }
                        });
                    });
                });
        }
    }
}