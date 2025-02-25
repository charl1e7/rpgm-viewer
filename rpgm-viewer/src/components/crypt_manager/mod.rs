use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use log::info;

use crate::components::file_browser;

use super::{
    crypt_settings::CryptSettings,
    file_browser::{file_entry::FileEntry, FileBrowser},
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

    pub fn try_extract_key(&self, path: &Path) -> Option<rpgm_enc::Key> {
        if !path
            .extension()
            .map_or(false, |ext| ext == "png_" || ext == "rpgmvp")
        {
            info!(
                "Skipping key extraction - file is not png_ or rpgmvp: {:?}",
                path
            );
            return None;
        }

        info!("Attempting to extract key from file: {:?}", path);
        if let Ok(file_data) = std::fs::read(path) {
            let header_len = rpgm_enc::Decrypter::new(None).get_header_len();
            info!(
                "File size: {}, header length: {}",
                file_data.len(),
                header_len
            );
            info!(
                "First 32 bytes of file: {:02X?}",
                &file_data[..32.min(file_data.len())]
            );

            match rpgm_enc::Decrypter::detect_key_from_file(&file_data) {
                Some(key) => {
                    info!("Successfully extracted key: {}", key.as_str());
                    Some(key)
                }
                None => {
                    info!("Failed to extract key - no valid encryption code found");
                    None
                }
            }
        } else {
            info!("Failed to read file: {:?}", path);
            None
        }
    }

    pub fn update_encryption_key(&mut self, key: &rpgm_enc::Key) {
        info!("Setting encryption key: {}", key.as_str());

        if let Some(crypt_settings) = self.get_mut_settings() {
            info!(
                "Previous encryption key: {:?}",
                crypt_settings.encryption_key.as_ref().map(|k| k.as_str())
            );
            crypt_settings.encryption_key = Some(key.clone());
            info!("Updated settings with new key: {}", key.as_str());
            crypt_settings.decrypter = Some(rpgm_enc::Decrypter::new(Some(key.clone())));
            info!("Created new decrypter with key: {}", key.as_str());
        }
    }

    pub fn set_current_directory(&mut self, path: PathBuf, file_browser: Option<&mut FileBrowser>) {
        info!("Setting current directory to: {}", path.display());

        // Reset the file browser cache if provided
        if let Some(browser) = file_browser {
            browser.reset_cache();
        }

        self.current_folder = Some(path.clone());
        self.settings.insert(path.clone(), CryptSettings::default());
        if let Some(crypt_settings) = self.get_settings() {
            if crypt_settings.encryption_key.is_none() {
                let walker = walkdir::WalkDir::new(&path)
                    .into_iter()
                    .filter_map(|e| e.ok());
                for entry in walker {
                    let file_path = entry.path().to_path_buf();
                    info!("Checking file: {}", file_path.display());
                    if file_path.extension().map_or(false, |ext| {
                        matches!(ext.to_str().unwrap_or(""), "png_" | "rpgmvp")
                    }) {
                        if let Some(key) = self.try_extract_key(&file_path) {
                            info!(
                                "File is a valid key file: {} {}",
                                file_path.display(),
                                key.as_str()
                            );
                            self.update_encryption_key(&key);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn handle_key_hex_input(&mut self, _root: &Path, hex_str: String) {
        let hex_str = hex_str.replace(" ", "");
        let key_bytes = (0..hex_str.len())
            .step_by(2)
            .filter_map(|i| {
                if i + 2 <= hex_str.len() {
                    u8::from_str_radix(&hex_str[i..i + 2], 16).ok()
                } else {
                    None
                }
            })
            .collect::<Vec<u8>>();

        let key_str = String::from_utf8_lossy(&key_bytes).to_string();
        if let Ok(key) = rpgm_enc::Key::from_str(&key_str) {
            self.update_encryption_key(&key);
        }
    }

    pub fn encrypt_file(&self, path: &Path) -> Result<(), String> {
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut rpg_file = rpgm_enc::RPGFile::new(path.to_path_buf()).map_err(|e| e.to_string())?;
        rpg_file.set_content(file_data);

        let decrypter = self.get_decrypter().ok_or("No encryption key set")?;

        let encrypted_data = decrypter
            .encrypt(rpg_file.content().unwrap())
            .map_err(|e| e.to_string())?;
        rpg_file.set_content(encrypted_data);

        rpg_file.convert_extension(false);

        std::fs::write(&path, rpg_file.content().unwrap()).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn decrypt_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        let decrypter = self.get_decrypter().ok_or("No decryption key set")?;

        let decrypted_content = decrypter
            .decrypt(&file_data)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        Ok(decrypted_content)
    }

    pub fn decrypt_file_with_header(&self, path: &Path) -> Result<Vec<u8>, String> {
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut rpg_file = rpgm_enc::RPGFile::new(path.to_path_buf()).map_err(|e| e.to_string())?;
        rpg_file.set_content(file_data);

        let decrypter = self.get_decrypter().ok_or("No decryption key set")?;

        let decrypted_content = decrypter
            .decrypt(rpg_file.content().unwrap())
            .map_err(|e| format!("Decryption failed: {}", e))?;

        let file_ext = rpg_file
            .extension()
            .ok_or("Could not determine file extension")?;

        let restored_content = decrypter
            .restore_header(&decrypted_content, file_ext)
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
        let entries = FileEntry::recursive_collect_entries_flat(path, 0, &[]);
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
        let entries = FileEntry::recursive_collect_entries_flat(path, 0, &[]);
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
        let decrypt_path = crypt_settings
            .decrypt_path
            .clone()
            .unwrap_or_else(|| root.clone());

        let decrypter = self.get_decrypter().ok_or("No encryption key set")?;

        info!("Starting encryption of file: {}", path.display());
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        info!("Read file content, size: {}", file_data.len());

        let mut rpg_file = rpgm_enc::RPGFile::new(path.to_path_buf()).map_err(|e| e.to_string())?;
        rpg_file.set_version(rpgmaker_version);
        rpg_file.set_content(file_data);
        info!(
            "Created RPGFile, initial extension: {:?}",
            rpg_file.extension()
        );

        let encrypted_data = decrypter
            .encrypt(rpg_file.content().unwrap())
            .map_err(|e| e.to_string())?;
        rpg_file.set_content(encrypted_data);
        info!(
            "Data encrypted successfully, size: {}",
            rpg_file.content().unwrap().len()
        );

        rpg_file.convert_extension(false);
        info!(
            "Converted to encrypted extension: {:?}",
            rpg_file.extension()
        );

        let output_path = {
            let relative_path = path
                .strip_prefix(&root)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;

            let mut full_path = decrypt_path.join(relative_path);

            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }

            if let Some(ext) = rpg_file.extension() {
                full_path.set_extension(ext.to_str());
            }

            info!("Final output path: {}", full_path.display());
            full_path
        };

        std::fs::write(&output_path, rpg_file.content().unwrap()).map_err(|e| e.to_string())?;
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

        let mut rpg_file = rpgm_enc::RPGFile::new(path.to_path_buf()).map_err(|e| e.to_string())?;
        rpg_file.set_content(file_data);

        if !rpg_file.is_encrypted() {
            return Err("File is not encrypted".to_string());
        }

        let file_ext = rpg_file
            .extension()
            .ok_or("Could not determine file extension")?;
        info!("Detected file type: {:?}", file_ext);

        let decrypted_content = decrypter
            .decrypt(rpg_file.content().unwrap())
            .map_err(|e| format!("Decryption failed: {}", e))?;
        info!(
            "Decrypted content first 32 bytes: {:02X?}",
            &decrypted_content[..32.min(decrypted_content.len())]
        );

        let restored_content = decrypter
            .restore_header(&decrypted_content, file_ext)
            .map_err(|e| format!("Header restoration failed: {}", e))?;
        info!(
            "Restored content first 32 bytes: {:02X?}",
            &restored_content[..32.min(restored_content.len())]
        );

        rpg_file.set_content(restored_content);
        rpg_file.convert_extension(true);

        let output_path = {
            let relative_path = path
                .strip_prefix(&root)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;

            let mut full_path = decrypt_path.join(relative_path);

            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }

            if let Some(ext) = rpg_file.extension() {
                full_path.set_extension(ext.to_str());
            }

            info!("Final output path: {}", full_path.display());
            full_path
        };

        std::fs::write(&output_path, rpg_file.content().unwrap()).map_err(|e| e.to_string())?;
        info!(
            "Successfully wrote decrypted file to: {}",
            output_path.display()
        );

        file_browser.reset_cache();
        Ok(())
    }
}
