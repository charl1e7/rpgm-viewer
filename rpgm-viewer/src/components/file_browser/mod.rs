pub mod file_entry;
pub mod thumbnail_cache;
pub mod ui;
use std::path::PathBuf;
use std::time::Duration;

use file_entry::FileEntry;
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
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            search_results_cache: None,
            current_image: None,
            entries_cache: None,
            last_update_time: None,
            last_expanded_state: Vec::new(),
            thumbnail_cache: ThumbnailCache::new(
                100,                      // max 100 thumb in cache
                Duration::from_secs(300), // cache for 5 minutes
            ),
        }
    }
}

impl FileBrowser {
    pub fn reset_cache(&mut self) {
        self.entries_cache = None;
        self.search_results_cache = None;
    }
}
