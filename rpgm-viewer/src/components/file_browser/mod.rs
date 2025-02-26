pub mod file_entry;
pub mod thumbnail_cache;
pub mod ui;
use std::path::PathBuf;
use std::time::Duration;

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
    last_update_time: Option<std::time::SystemTime>,
    #[serde(skip)]
    all_thumbnails_loaded: bool,
    #[serde(skip)]
    last_thumbnail_max_size: usize,
    #[serde(skip)]
    last_thumbnail_ttl_secs: u64,
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
            thumbnail_cache: ThumbnailCache::new(
                ui_settings.get_thumbnail_cache_size(),
                ui_settings.get_thumbnail_cache_ttl(),
            ),
            all_thumbnails_loaded: false,
            last_thumbnail_max_size: ui_settings.get_thumbnail_cache_size(),
            last_thumbnail_ttl_secs: ui_settings.get_thumbnail_cache_ttl().as_secs(),
            last_show_thumbnails: ui_settings.show_thumbnails,
            last_thumbnail_compression_size: ui_settings.get_thumbnail_compression_size(),
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

    pub fn update_thumbnail_cache_settings(&mut self, ui_settings: &UiSettings) {
        let current_max_size = ui_settings.get_thumbnail_cache_size();
        let current_ttl = ui_settings.get_thumbnail_cache_ttl();
        let current_ttl_secs = current_ttl.as_secs();

        if self.last_thumbnail_max_size != current_max_size {
            self.thumbnail_cache.set_max_size(current_max_size);
            self.last_thumbnail_max_size = current_max_size;
        }

        if self.last_thumbnail_ttl_secs != current_ttl_secs {
            self.thumbnail_cache.set_ttl(current_ttl);
            self.last_thumbnail_ttl_secs = current_ttl_secs;
        }
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
