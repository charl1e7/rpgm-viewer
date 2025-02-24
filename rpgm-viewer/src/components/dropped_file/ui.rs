use std::path::PathBuf;

use log::{debug, info, trace};

use crate::components::{
    crypt_manager::CryptManager, file_browser::FileBrowser, image_viewer::ImageViewer,
};

use super::DroppedFile;

impl DroppedFile {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        crypt_manager: &mut CryptManager,
        file_browser: &mut FileBrowser,
    ) {
        // handle pending loads from prev frame
        if let Some(path) = self.pending_load.take() {
            if let Some(texture) = ImageViewer::load_image(&path, ctx, None) {
                file_browser.current_image = Some((path, texture));
            }
        }

        // Handle dnd
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            ctx.input(|i| {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        debug!("Dropped path: {}", path.display());
                        if path.is_dir() {
                            trace!("Setting current directory: {}", path.display());
                            crypt_manager.set_current_directory(path.to_path_buf());
                        } else if let Some(ext) = path.extension() {
                            trace!("Dropped file extension: {}", ext.to_string_lossy());
                            if let Some(ext_str) = ext.to_str() {
                                if ["png", "jpg", "jpeg", "gif", "bmp", "webp", "png_", "rpgmvp"]
                                    .contains(&ext_str.to_lowercase().as_str())
                                {
                                    trace!("Scheduling image load for next frame");
                                    self.pending_load = Some(path.to_path_buf());
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}
