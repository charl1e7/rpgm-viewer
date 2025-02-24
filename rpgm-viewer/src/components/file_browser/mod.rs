pub mod file_entry;
pub mod thumbnail_cache;
pub mod ui;
use std::collections::HashMap;
use std::collections::HashSet;
use std::default;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::components::crypt_manager::CryptManager;
use crate::components::ui_settings::UiSettings;
use file_entry::FileEntry;
use log::info;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
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
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            search_results_cache: None,
            current_image: None,
            entries_cache: None,
            last_expanded_state: Vec::new(),
            thumbnail_cache: ThumbnailCache::new(
                100,                      // max 100 thumb in cache
                Duration::from_secs(300), // cache for 5 minutes
            ),
            
        }
    }
}
