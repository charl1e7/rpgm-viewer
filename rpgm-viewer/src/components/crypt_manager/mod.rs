use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use log::info;

use crate::components::file_browser;

use super::{
    crypt_settings::CryptSettings,
    file_browser::{FileBrowser, file_entry::FileEntry},
};

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct CryptManager {
    settings: HashMap<PathBuf, CryptSettings>,
    pub current_folder: Option<PathBuf>,
}

impl CryptManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_mut_settings(&mut self) -> Option<&mut CryptSettings> {
        if let Some(current_folder) = &self.current_folder {
            self.settings.get_mut(current_folder)
        } else {
            None
        }
    }

    pub fn get_settings(&self) -> Option<&CryptSettings> {
        if let Some(current_folder) = &self.current_folder {
            self.settings.get(current_folder)
        } else {
            None
        }
    }

    pub fn toggle_settings(&mut self) {
        if let Some(current_folder) = &self.current_folder {
            if let Some(settings) = self.settings.get_mut(current_folder) {
                settings.toggle_settings();
            }
        }
    }

    pub fn show_settings(&self) -> bool {
        if let Some(current_folder) = &self.current_folder {
            self.settings
                .get(current_folder)
                .map_or(false, |settings| settings.show_settings)
        } else {
            false
        }
    }

    pub fn get_decrypter(&self) -> Option<&rpgm_enc::Decrypter> {
        if let Some(current_folder) = &self.current_folder {
            self.settings
                .get(current_folder)
                .and_then(|settings| settings.decrypter.as_ref())
        } else {
            None
        }
    }

    fn ext_from_path(path: &Path) -> Option<rpgm_enc::FileExtension> {
        let ext_str = path.extension()?.to_str()?;
        rpgm_enc::FileExtension::from_str(ext_str)
    }

    pub fn try_extract_key(&self, path: &Path) -> Option<rpgm_enc::Key> {
        let ext = Self::ext_from_path(path)?;
        if !ext.is_encrypted() {
            info!("Skipping key extraction - not encrypted: {:?}", path);
            return None;
        }

        info!("Attempting to extract key from file: {:?}", path);
        let file_data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                info!("Failed to read file: {:?}, {}", path, e);
                return None;
            }
        };

        match rpgm_enc::Decrypter::detect_key(&file_data, ext) {
            Some(key) => {
                info!("Successfully extracted key: {}", key.as_str());
                Some(key)
            }
            None => {
                info!("Failed to extract key from {:?}", path);
                None
            }
        }
    }

    pub fn update_encryption_key(&mut self, key: &rpgm_enc::Key) {
        info!("Setting encryption key: {}", key.as_str());
        if let Some(crypt_settings) = self.get_mut_settings() {
            crypt_settings.encryption_key = Some(key.clone());
            crypt_settings.decrypter = Some(rpgm_enc::Decrypter::new(Some(key.clone())));
        }
    }

    pub fn set_current_directory(&mut self, path: PathBuf, file_browser: Option<&mut FileBrowser>) {
        info!("Setting current directory to: {}", path.display());
        if let Some(browser) = file_browser {
            browser.reset_cache();
        }

        self.current_folder = Some(path.clone());
        let mut settings = CryptSettings::default();
        settings.decrypt_path = Some(path.clone());
        settings.crypt_path = Some(path.clone());
        self.settings.insert(path.clone(), settings);

        if let Some(settings) = self.settings.get(&path) {
            if settings.encryption_key.is_none() {
                let walker = walkdir::WalkDir::new(&path)
                    .into_iter()
                    .filter_map(|e| e.ok());

                for entry in walker {
                    let file_path = entry.path().to_path_buf();
                    let ext_str = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

                    if matches!(ext_str, "png_" | "rpgmvp" | "ogg_" | "rpgmvo") {
                        if let Some(key) = self.try_extract_key(&file_path) {
                            info!("Found key in: {} -> {}", file_path.display(), key.as_str());
                            self.update_encryption_key(&key);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn handle_key_hex_input(&mut self, hex_str: String) {
        let hex_str = hex_str.replace(" ", "");
        if let Some(key) = rpgm_enc::Key::new(&hex_str) {
            self.update_encryption_key(&key);
        }
    }

    pub fn encrypt_file(&self, path: &Path) -> Result<(), String> {
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        let ext = Self::ext_from_path(path).ok_or("Unknown file extension")?;
        let decrypter = self.get_decrypter().ok_or("No encryption key set")?;
        let encrypted_data = decrypter
            .encrypt(&file_data, ext)
            .map_err(|e| e.to_string())?;
        std::fs::write(path, encrypted_data).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn decrypt_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        let ext = Self::ext_from_path(path).ok_or("Unknown file extension")?;
        let decrypter = self.get_decrypter().ok_or("No decryption key set")?;
        let decrypted_content = decrypter
            .decrypt(&file_data, ext)
            .map_err(|e| format!("Decryption failed: {}", e))?;
        Ok(decrypted_content)
    }

    pub fn decrypt_file_with_header(&self, path: &Path) -> Result<Vec<u8>, String> {
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        let ext = Self::ext_from_path(path).ok_or("Unknown file extension")?;
        let decrypter = self.get_decrypter().ok_or("No decryption key set")?;

        let decrypted_content = decrypter
            .decrypt(&file_data, ext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        let restored_content = decrypter
            .restore_header(&decrypted_content, ext)
            .map_err(|e| format!("Header restoration failed: {}", e))?;

        Ok(restored_content)
    }

    pub fn is_file_encrypted(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| {
            matches!(
                ext.to_str().unwrap_or(""),
                "png_" | "rpgmvp" | "m4a_" | "rpgmvm" | "ogg_" | "rpgmvo"
            )
        })
    }

    pub fn encrypt_folder(
        &mut self,
        path: &std::path::Path,
        file_browser: &mut FileBrowser,
    ) -> Result<(), String> {
        let entries = FileEntry::recursive_collect_all_entries_flat(path, 0);
        let mut errors = Vec::new();

        for entry in entries {
            if !entry.is_folder && !entry.is_encrypted {
                if let Err(e) = self.encrypt_image(&entry.path, file_browser) {
                    errors.push(format!("Failed to encrypt {}: {}", entry.path.display(), e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    pub fn decrypt_folder(
        &mut self,
        path: &std::path::Path,
        file_browser: &mut FileBrowser,
    ) -> Result<(), String> {
        let entries = FileEntry::recursive_collect_all_entries_flat(path, 0);
        let mut errors = Vec::new();

        for entry in entries {
            if !entry.is_folder && entry.is_encrypted {
                if let Err(e) = self.decrypt_image(&entry.path, file_browser) {
                    errors.push(format!("Failed to decrypt {}: {}", entry.path.display(), e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    pub fn encrypt_image(
        &mut self,
        path: &std::path::Path,
        file_browser: &mut FileBrowser,
    ) -> Result<(), String> {
        let root = self.current_folder.clone().ok_or("No root folder set")?;
        let crypt_settings = self.get_settings().ok_or("No settings set")?;
        let rpgmaker_version = crypt_settings.rpgmaker_version;
        let crypt_path = crypt_settings
            .crypt_path
            .clone()
            .unwrap_or_else(|| root.clone());
        let decrypter = self.get_decrypter().ok_or("No encryption key set")?;

        info!("Starting encryption of file: {}", path.display());
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        info!("Read file content, size: {}", file_data.len());

        let ext = Self::ext_from_path(path).ok_or("Unknown file extension")?;

        let encrypted_data = decrypter
            .encrypt(&file_data, ext)
            .map_err(|e| e.to_string())?;
        info!(
            "Data encrypted successfully, size: {}",
            encrypted_data.len()
        );

        let new_ext = ext.convert(false, rpgmaker_version);
        info!("Converted to encrypted extension: {:?}", new_ext);

        let output_path = {
            let relative_path = path
                .strip_prefix(&root)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;
            let mut full_path = crypt_path.join(relative_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }
            full_path.set_extension(new_ext.to_str());
            info!("Final output path: {}", full_path.display());
            full_path
        };

        std::fs::write(&output_path, &encrypted_data).map_err(|e| e.to_string())?;

        if output_path != path {
            let _ = std::fs::remove_file(path);
        }

        info!(
            "Successfully wrote encrypted file to: {}",
            output_path.display()
        );
        file_browser.reset_cache();
        Ok(())
    }

    pub fn decrypt_image(
        &mut self,
        path: &std::path::Path,
        file_browser: &mut FileBrowser,
    ) -> Result<(), String> {
        let root = self.current_folder.clone().ok_or("No root folder set")?;
        let crypt_settings = self.get_settings().ok_or("No settings set")?;
        let decrypt_path = crypt_settings
            .decrypt_path
            .clone()
            .unwrap_or_else(|| root.clone());
        let decrypter = self.get_decrypter().ok_or("No encryption key set")?;

        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        info!(
            "Original encrypted data first 32 bytes: {:02X?}",
            &file_data[..32.min(file_data.len())]
        );

        let ext = Self::ext_from_path(path).ok_or("Unknown file extension")?;
        if !ext.is_encrypted() {
            return Err("File is not encrypted".to_string());
        }
        info!("Detected file type: {:?}", ext);

        let decrypted_content = decrypter
            .decrypt(&file_data, ext)
            .map_err(|e| format!("Decryption failed: {}", e))?;
        info!(
            "Decrypted content first 32 bytes: {:02X?}",
            &decrypted_content[..32.min(decrypted_content.len())]
        );

        let restored_content = decrypter
            .restore_header(&decrypted_content, ext)
            .map_err(|e| format!("Header restoration failed: {}", e))?;
        info!(
            "Restored content first 32 bytes: {:02X?}",
            &restored_content[..32.min(restored_content.len())]
        );

        let new_ext = ext.convert(true, crypt_settings.rpgmaker_version);

        let output_path = {
            let relative_path = path
                .strip_prefix(&root)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;
            let mut full_path = decrypt_path.join(relative_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }
            full_path.set_extension(new_ext.to_str());
            info!("Final output path: {}", full_path.display());
            full_path
        };

        std::fs::write(&output_path, &restored_content).map_err(|e| e.to_string())?;
        info!(
            "Successfully wrote decrypted file to: {}",
            output_path.display()
        );
        file_browser.reset_cache();
        Ok(())
    }
}
