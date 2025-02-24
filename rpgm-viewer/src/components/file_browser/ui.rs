use super::file_entry::FileEntry;
use super::FileBrowser;
use std::collections::HashMap;
use std::path::Path;

use crate::components::audio::AudioState;
use crate::components::crypt_manager::CryptManager;
use crate::components::image_viewer::ImageViewer;
use crate::components::ui_settings::UiSettings;
use log::{info, trace};
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
        ui.heading("Files");
        self.show_search_bar(ui);
        ui.separator();

        if let Some(root) = &crypt_manager.current_folder {
            let root = root.clone();
            self.update_entries_cache(&root, crypt_manager, ui_settings);
            let entries = self.get_filtered_entries(&root);
            self.show_file_list(ui, ctx, entries, crypt_manager, ui_settings, audio);
        }
    }

    fn show_search_bar(&mut self, ui: &mut egui::Ui) -> bool {
        ui.horizontal(|ui| {
            ui.label("🔍");
            let changed = ui.text_edit_singleline(&mut self.search_query).changed();
            if !self.search_query.is_empty() {
                if ui.button("✖").clicked() {
                    self.search_query.clear();
                    self.search_results_cache = None;
                    self.entries_cache = None;
                    return true;
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

        if self.entries_cache.is_none() || self.last_expanded_state != expanded_folders {
            let mut new_entries =
                FileEntry::recursive_collect_entries_flat(root, 0, &expanded_folders);
            self.preserve_thumbnails(&mut new_entries, ui_settings);
            self.entries_cache = Some(new_entries);
            self.last_expanded_state = expanded_folders.clone();
        }
    }

    fn preserve_thumbnails(&self, new_entries: &mut Vec<FileEntry>, ui_settings: &UiSettings) {
        if !ui_settings.show_thumbnails {
            for entry in new_entries.iter_mut() {
                entry.thumbnail = None;
            }
            return;
        }

        if let Some(old_entries) = self.entries_cache.as_ref() {
            let old_thumbnails: HashMap<_, _> = old_entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .thumbnail
                        .as_ref()
                        .map(|thumb| (entry.path.clone(), thumb.clone()))
                })
                .collect();

            for entry in new_entries.iter_mut() {
                if let Some(thumb) = old_thumbnails.get(&entry.path) {
                    entry.thumbnail = Some(thumb.clone());
                }
            }
        }
    }

    fn get_filtered_entries(&mut self, root: &Path) -> Vec<FileEntry> {
        if self.search_query.is_empty() {
            self.search_results_cache = None;
            self.entries_cache.as_ref().unwrap().clone()
        } else {
            self.update_search_results(root)
        }
    }

    fn update_search_results(&mut self, root: &Path) -> Vec<FileEntry> {
        if self.search_results_cache.is_none()
            || self.search_results_cache.as_ref().unwrap().0 != self.search_query
        {
            let all_entries = FileEntry::recursive_collect_all_entries_flat(root, 0);
            let query = self.search_query.to_lowercase();

            let filtered_entries: Vec<_> = all_entries
                .into_iter()
                .filter(|entry| {
                    if let Ok(relative_path) = entry.path.strip_prefix(root) {
                        relative_path
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&query)
                    } else {
                        entry.name().to_lowercase().contains(&query)
                    }
                })
                .collect();

            self.search_results_cache = Some((self.search_query.clone(), filtered_entries));
        }
        self.search_results_cache.as_ref().unwrap().1.clone()
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
        let should_load_thumbnails = ui_settings.show_thumbnails;

        if should_load_thumbnails {
            if let Some(decrypter) = crypt_manager.get_decrypter() {
                self.process_thumbnails(ui, ctx, &mut entries, &decrypter, ui_settings);
            }
        }

        egui::ScrollArea::vertical()
            .id_source("file_list_scroll")
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

    fn process_thumbnails(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        entries: &mut Vec<FileEntry>,
        decrypter: &Decrypter,
        ui_settings: &UiSettings,
    ) {
        if !ui_settings.show_thumbnails {
            return;
        }

        let all_loaded = entries
            .iter()
            .all(|entry| entry.is_folder || entry.thumbnail.is_some());
        if all_loaded {
            trace!("All thumbnails already loaded, skipping processing");
            return;
        }

        let visible_rect = ui.clip_rect();
        let mut processed_this_frame = 0;
        const MAX_THUMBNAILS_PER_FRAME: usize = 4;

        for entry in entries.iter_mut() {
            if self.process_single_thumbnail(entry, ui, ctx, decrypter, &visible_rect) {
                processed_this_frame += 1;
                trace!("Processed thumbnail for: {:?}", entry.path);
                if processed_this_frame >= MAX_THUMBNAILS_PER_FRAME {
                    break;
                }
            }
        }

        self.update_caches(entries);
    }

    fn process_single_thumbnail(
        &mut self,
        entry: &mut FileEntry,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        decrypter: &Decrypter,
        visible_rect: &egui::Rect,
    ) -> bool {
        if entry.is_folder {
            return false;
        }

        let entry_rect = ui.max_rect();
        if !entry_rect.intersects(*visible_rect) {
            return false;
        }

        if entry.thumbnail.is_none() && !self.thumbnail_cache.is_pending(&entry.path) {
            self.thumbnail_cache.mark_pending(entry.path.clone());

            let processed = if entry.path.extension().map_or(false, |ext| {
                matches!(
                    ext.to_str().unwrap_or(""),
                    "png" | "png_" | "rpgmvp" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
                )
            }) {
                if let Some(texture) = self.load_thumbnail(&entry.path, ctx, decrypter) {
                    entry.thumbnail = Some(texture);
                    true
                } else {
                    false
                }
            } else {
                false
            };

            self.thumbnail_cache.unmark_pending(&entry.path);
            return processed;
        }
        false
    }

    fn update_caches(&mut self, entries: &Vec<FileEntry>) {
        if !self.search_query.is_empty() {
            self.search_results_cache = Some((self.search_query.clone(), entries.clone()));
        } else {
            self.entries_cache = Some(entries.clone());
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
        ui.horizontal(|ui| {
            if entry.nesting_level > 0 {
                ui.add_space(entry.nesting_level as f32 * 20.0);
            }

            if entry.is_folder {
                self.show_folder_entry(ui, entry, crypt_manager);
            } else {
                self.show_file_icon(ui, entry, ui_settings);
                if ui_settings.show_thumbnails && entry.thumbnail.is_some() {
                    ui.set_min_height(ui_settings.thumbnail_size);
                }
                let response = ui.button(&entry.name());
                if response.clicked() {
                    self.handle_file_click(entry, ctx, crypt_manager, audio);
                }
                response.context_menu(|ui| self.show_file_context_menu(ui, entry, crypt_manager));
            }
        });
    }

    fn show_folder_entry(
        &self,
        ui: &mut egui::Ui,
        entry: &FileEntry,
        crypt_manager: &mut CryptManager,
    ) {
        ui.label("📁");
        let response = ui.button(&entry.name());
        if response.clicked() {
            let crypt_settings = crypt_manager.get_mut_settings().unwrap();
            crypt_settings.toggle_folder_expansion(&entry.path);
        }

        response.context_menu(|ui| self.show_folder_context_menu(ui, entry, crypt_manager));
    }

    fn show_folder_context_menu(
        &self,
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
            ui.close_menu();
        }

        ui.separator();

        if ui.button("Encrypt All Files").clicked() {
            if let Err(e) = crypt_manager.encrypt_folder(&entry.path) {
                eprintln!("Failed to encrypt folder: {}", e);
            }
            ui.close_menu();
        }

        if ui.button("Decrypt All Files").clicked() {
            if let Err(e) = crypt_manager.decrypt_folder(&entry.path) {
                eprintln!("Failed to decrypt folder: {}", e);
            }
            ui.close_menu();
        }
    }

    fn show_file_icon(&self, ui: &mut egui::Ui, entry: &FileEntry, ui_settings: &UiSettings) {
        if ui_settings.show_thumbnails {
            if let Some(texture) = entry.thumbnail.as_ref() {
                let display_size = ui_settings.thumbnail_size as f32;
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

        let icon = if entry.path.extension().map_or(false, |ext| {
            matches!(
                ext.to_str().unwrap_or(""),
                "png" | "png_" | "rpgmvp" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
            )
        }) {
            if entry.is_encrypted {
                "🔒"
            } else {
                "🖼"
            }
        } else if entry.path.extension().map_or(false, |ext| {
            matches!(
                ext.to_str().unwrap_or(""),
                "ogg" | "ogg_" | "rpgmvo" | "mp3"
            )
        }) {
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

    fn handle_file_click(
        &mut self,
        entry: &FileEntry,
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        audio: &mut AudioState,
    ) {
        let decrypter = crypt_manager.get_decrypter().unwrap();
        if entry.path.extension().map_or(false, |ext| {
            matches!(
                ext.to_str().unwrap_or(""),
                "ogg" | "ogg_" | "rpgmvo" | "mp3"
            )
        }) {
            if let Err(e) = audio.play_audio(&entry.path, decrypter) {
                eprintln!("Failed to play audio: {}", e);
            }
        } else {
            match ImageViewer::load_image(&entry.path, ctx, Some(&decrypter)) {
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
        &self,
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
                ui.close_menu();
            }
            ui.separator();
        }

        if entry.is_encrypted {
            if ui.button("Decrypt").clicked() {
                if let Err(e) = crypt_manager.decrypt_image(&entry.path) {
                    eprintln!("Failed to decrypt: {}", e);
                }
                ui.close_menu();
            }
        } else {
            if ui.button("Encrypt").clicked() {
                if let Err(e) = crypt_manager.encrypt_image(&entry.path) {
                    eprintln!("Failed to encrypt: {}", e);
                }
                ui.close_menu();
            }
        }
    }

    fn load_thumbnail(
        &mut self,
        path: &Path,
        ctx: &egui::Context,
        decrypter: &Decrypter,
    ) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.thumbnail_cache.get(path) {
            return Some(texture);
        }

        let file_data = std::fs::read(path).ok()?;
        let mut rpg_file = rpgm_enc::RPGFile::new(path.to_path_buf()).ok()?;
        rpg_file.set_content(file_data);

        let image_data = if rpg_file.is_encrypted() {
            match decrypter.decrypt(rpg_file.content().unwrap()) {
                Ok(content) => content,
                Err(_) => return None,
            }
        } else {
            rpg_file.content().unwrap_or_default().to_vec()
        };

        if let Ok(img) = image::load_from_memory(&image_data) {
            let thumb_size = 64u32;
            let thumbnail = img.thumbnail(thumb_size, thumb_size);
            let image_buffer = thumbnail.to_rgb8();
            let dimensions = [thumbnail.width() as usize, thumbnail.height() as usize];

            let texture = ctx.load_texture(
                format!("thumb_{}", path.file_name().unwrap().to_string_lossy()),
                egui::ColorImage::from_rgb([dimensions[0], dimensions[1]], image_buffer.as_raw()),
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Linear,
                    minification: egui::TextureFilter::Linear,
                    ..Default::default()
                },
            );

            self.thumbnail_cache
                .insert(path.to_path_buf(), texture.clone());
            Some(texture)
        } else {
            None
        }
    }
}
