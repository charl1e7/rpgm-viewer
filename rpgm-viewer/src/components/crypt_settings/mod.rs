pub mod ui;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct CryptSettings {
    pub(crate) encryption_key: Option<rpgm_enc::Key>,
    pub(crate) root_folder: Option<PathBuf>,
    pub(crate) expanded_folders: HashSet<PathBuf>,
    pub(crate) decrypt_path: Option<PathBuf>,
    pub(crate) rpgmaker_version: rpgm_enc::RPGMakerVersion,
    pub(crate) show_settings: bool,
    pub(crate) decrypter: Option<rpgm_enc::Decrypter>,
}

impl CryptSettings {
    pub fn show_settings(&self) -> bool {
        self.show_settings
    }

    pub fn set_show_settings(&mut self, show: bool) {
        self.show_settings = show;
    }

    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
    }
    pub fn is_folder_expanded(&self, path: &PathBuf) -> bool {
        self.expanded_folders.contains(path)
    }

    pub fn toggle_folder_expansion(&mut self, path: &Path) {
        if self.expanded_folders.contains(path) {
            self.expanded_folders.remove(path);
        } else {
            self.expanded_folders.insert(path.to_path_buf());
        }
    }

    pub fn update_encryption_key(&mut self, key: &rpgm_enc::Key) {
        self.encryption_key = Some(key.clone());
    }

    pub fn get_expanded_folders(&self) -> Vec<PathBuf> {
        self.expanded_folders.iter().cloned().collect()
    }
}
