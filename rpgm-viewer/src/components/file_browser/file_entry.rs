#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct FileEntry {
    pub path: std::path::PathBuf,
    pub is_folder: bool,
    pub is_encrypted: bool,
    #[serde(skip)]
    pub thumbnail: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub nesting_level: usize,
}

impl FileEntry {
    pub fn recursive_collect_entries_flat(
        path: &std::path::Path,
        level: usize,
        expanded_folders: &[std::path::PathBuf],
    ) -> Vec<FileEntry> {
        let mut all_entries = Vec::new();

        if let Ok(dir_entries) = std::fs::read_dir(path) {
            let mut folders = Vec::new();
            let mut files = Vec::new();

            for entry in dir_entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    folders.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if [
                                "png", "png_", "rpgmvp", "m4a", "m4a_", "rpgmvm", "ogg", "ogg_",
                                "rpgmvo", "jpg", "jpeg", "gif", "bmp", "webp",
                            ]
                            .contains(&ext_str.to_lowercase().as_str())
                            {
                                files.push(path);
                            }
                        }
                    }
                }
            }

            folders.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

            for folder_path in folders {
                let mut entry = FileEntry::new(folder_path.clone(), true);
                entry.nesting_level = level;
                all_entries.push(entry);

                if expanded_folders.contains(&folder_path) {
                    all_entries.extend(Self::recursive_collect_entries_flat(
                        &folder_path,
                        level + 1,
                        expanded_folders,
                    ));
                }
            }

            for file_path in files {
                let mut entry = FileEntry::new(file_path, false);
                entry.nesting_level = level;
                all_entries.push(entry);
            }
        }

        all_entries
    }

    pub fn collect_entries(path: &std::path::Path) -> Vec<FileEntry> {
        let mut entries = Vec::new();

        if let Ok(dir_entries) = std::fs::read_dir(path) {
            for entry in dir_entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    entries.push(FileEntry::new(path, true));
                } else if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if [
                                "png", "png_", "rpgmvp", "m4a", "m4a_", "rpgmvm", "ogg", "ogg_",
                                "rpgmvo", "jpg", "jpeg", "gif", "bmp", "webp",
                            ]
                            .contains(&ext_str.to_lowercase().as_str())
                            {
                                entries.push(FileEntry::new(path, false));
                            }
                        }
                    }
                }
            }
        }

        entries.sort_by(|a, b| {
            if a.is_folder == b.is_folder {
                a.name().cmp(&b.name())
            } else {
                b.is_folder.cmp(&a.is_folder)
            }
        });

        entries
    }

    pub fn recursive_collect_all_entries_flat(
        path: &std::path::Path,
        level: usize,
    ) -> Vec<FileEntry> {
        let mut all_entries = Vec::new();

        if let Ok(dir_entries) = std::fs::read_dir(path) {
            let mut folders = Vec::new();
            let mut files = Vec::new();

            for entry in dir_entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    folders.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if [
                                "png", "png_", "rpgmvp", "m4a", "m4a_", "rpgmvm", "ogg", "ogg_",
                                "rpgmvo", "jpg", "jpeg", "gif", "bmp", "webp",
                            ]
                            .contains(&ext_str.to_lowercase().as_str())
                            {
                                files.push(path);
                            }
                        }
                    }
                }
            }

            folders.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

            for folder_path in folders {
                let mut entry = FileEntry::new(folder_path.clone(), true);
                entry.nesting_level = level;
                all_entries.push(entry);

                all_entries.extend(Self::recursive_collect_all_entries_flat(
                    &folder_path,
                    level + 1,
                ));
            }

            for file_path in files {
                let mut entry = FileEntry::new(file_path, false);
                entry.nesting_level = level;
                all_entries.push(entry);
            }
        }

        all_entries
    }
}

impl FileEntry {
    pub fn new(path: std::path::PathBuf, is_folder: bool) -> Self {
        let is_encrypted = if !is_folder {
            path.extension().map_or(false, |ext| {
                matches!(
                    ext.to_str().unwrap_or(""),
                    "png_" | "rpgmvp" | "m4a_" | "rpgmvm" | "ogg_" | "rpgmvo"
                )
            })
        } else {
            false
        };

        Self {
            path,
            is_folder,
            is_encrypted,
            thumbnail: None,
            nesting_level: 0,
        }
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
}
