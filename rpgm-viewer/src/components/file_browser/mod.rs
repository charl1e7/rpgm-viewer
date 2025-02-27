pub mod file_entry;
pub mod thumbnail_cache;
pub mod ui;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::components::ui_settings::UiSettings;
use file_entry::FileEntry;
use log::info;
use thumbnail_cache::ThumbnailCache;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct FileBrowser {
    search_query: String,
    #[serde(skip)]
    search_results_cache: Option<(String, Vec<FileEntry>)>,
    #[serde(skip)]
    pub current_image: Option<(PathBuf, egui::TextureHandle)>,
    #[serde(skip)]
    thumbnail_cache: ThumbnailCache,
    #[serde(skip)]
    entries_cache: Option<Vec<FileEntry>>,
    #[serde(skip)]
    last_expanded_state: Vec<PathBuf>,
    #[serde(skip)]
    last_update_time: Option<SystemTime>,
    #[serde(skip)]
    last_cache_check: Option<SystemTime>,
    #[serde(skip)]
    all_thumbnails_loaded: bool,
    #[serde(skip)]
    last_show_thumbnails: bool,
    #[serde(skip)]
    last_thumbnail_compression_size: u32,
    #[serde(skip)]
    pub show_delete_confirmation: Option<(PathBuf, bool)>,
}

impl Default for FileBrowser {
    fn default() -> Self {
        let ui_settings = UiSettings::default();
        Self {
            search_query: String::new(),
            search_results_cache: None,
            current_image: None,
            entries_cache: None,
            last_update_time: None,
            last_expanded_state: Vec::new(),
            thumbnail_cache: ThumbnailCache::new(),
            all_thumbnails_loaded: false,
            last_show_thumbnails: ui_settings.show_thumbnails,
            last_thumbnail_compression_size: ui_settings.get_thumbnail_compression_size(),
            last_cache_check: None,
            show_delete_confirmation: None,
        }
    }
}

impl FileBrowser {
    pub fn reset_cache(&mut self) {
        self.entries_cache = None;
        self.search_results_cache = None;
        self.all_thumbnails_loaded = false;
    }

    pub fn check_and_update_cache(&mut self, root: &PathBuf, ui_settings: &UiSettings) {
        let now = SystemTime::now();
        let cache_update_interval = ui_settings.get_cache_update_interval();

        if let Some(last_check) = self.last_cache_check {
            if now
                .duration_since(last_check)
                .unwrap_or(Duration::from_secs(0))
                < cache_update_interval
            {
                return;
            }
        }

        self.thumbnail_cache.update_cache(root);
        self.last_cache_check = Some(now);
    }

    pub fn get_thumbnail_compression_size(&self, ui_settings: &UiSettings) -> u32 {
        ui_settings.get_thumbnail_compression_size()
    }

    pub fn clear_thumbnail_cache(&mut self) {
        info!("Clearing thumbnail cache");
        self.thumbnail_cache.clear_cache();
        self.reset_cache();
    }
}
