pub mod ui;

use std::path::PathBuf;

use log::{debug, error, trace};
use rpgm_enc::{Decrypter, FileExtension};

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct ImageViewer {}

impl ImageViewer {
    pub fn load_image(
        path: &std::path::Path,
        ctx: &egui::Context,
        decrypter: Option<Decrypter>,
    ) -> Option<egui::TextureHandle> {
        let file_data = std::fs::read(path).ok()?;

        let ext_str = path.extension()?.to_str()?;
        let ext = FileExtension::from_str(ext_str)?;

        let decrypter = match decrypter {
            Some(d) => d,
            None => {
                let key = Decrypter::detect_key(&file_data, ext)?;
                Decrypter::new(Some(key))
            }
        };

        debug!(
            "Image state: encrypted={}, ext={:?}",
            ext.is_encrypted(),
            ext
        );

        let image_data = if ext.is_encrypted() {
            trace!("File is encrypted, attempting to decrypt");
            match decrypter.decrypt(&file_data, ext) {
                Ok(content) => {
                    trace!("Successfully decrypted content, size: {}", content.len());
                    match decrypter.restore_header(&content, ext) {
                        Ok(restored) => restored,
                        Err(_) => content,
                    }
                }
                Err(e) => {
                    error!("Decryption failed: {}", e);
                    return None;
                }
            }
        } else {
            trace!("File is not encrypted, using original content");
            file_data
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
