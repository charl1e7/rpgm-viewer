pub mod ui;
use std::path::PathBuf;

use log::{debug, error, trace};
use rpgm_enc::Decrypter;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct ImageViewer {
    file_notes: std::collections::HashMap<PathBuf, String>,
}

impl ImageViewer {
    pub fn load_image(
        path: &std::path::Path,
        ctx: &egui::Context,
        decrypter: Option<&Decrypter>,
    ) -> Option<egui::TextureHandle> {
        trace!("Loading image from path: {:?}", path);
        let file_data = std::fs::read(path).ok()?;
        trace!("Original file size: {}", file_data.len());
        let decrypter = match decrypter {
            Some(decrypter) => decrypter,
            None => {
                trace!("No decrypter provided, using default");
                let key = Decrypter::detect_key_from_file(&file_data);
                if let Some(key) = key {
                    trace!("Detected key: {}", key.as_str());
                    &Decrypter::new(Some(key))
                } else {
                    trace!("No key detected, returning None");
                    return None;
                }
            }
        };
        let mut rpg_file = rpgm_enc::RPGFile::new(path.to_path_buf()).ok()?;
        rpg_file.set_content(file_data);
        debug!(
            "RPGFile state: encrypted={}, extension={:?}, key={:?}",
            rpg_file.is_encrypted(),
            rpg_file.extension(),
            decrypter.key
        );

        let image_data = if rpg_file.is_encrypted() {
            trace!("File is encrypted, attempting to decrypt");
            match decrypter.decrypt(rpg_file.content().unwrap()) {
                Ok(content) => {
                    trace!("Successfully decrypted content, size: {}", content.len());
                    content
                }
                Err(e) => {
                    error!("Decryption failed: {}", e);
                    return None;
                }
            }
        } else {
            trace!("File is not encrypted, using original content");
            rpg_file.content().unwrap_or_default().to_vec()
        };

        match image::load_from_memory(&image_data) {
            Ok(img) => {
                debug!(
                    "Successfully loaded image: {}x{}",
                    img.width(),
                    img.height()
                );
                let size = [img.width() as _, img.height() as _];
                let image_buffer = img.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                trace!("Loading texture");
                Some(
                    ctx.load_texture(
                        path.file_name().unwrap().to_string_lossy(),
                        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                        egui::TextureOptions::default(),
                    )
                    .clone(),
                )
            }
            Err(e) => {
                error!("Failed to load image: {}", e);
                None
            }
        }
    }
}
