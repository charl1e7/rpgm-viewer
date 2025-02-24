pub mod ui;
use std::path::PathBuf;

use log::{debug, info, trace};

use crate::components::{crypt_manager::CryptManager, image_viewer::ImageViewer};

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct DroppedFile {
    pending_load: Option<PathBuf>,
}
