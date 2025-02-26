pub mod ui;
use std::path::PathBuf;

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct DroppedFile {
    #[serde(skip)]
    pending_load: Option<PathBuf>,
}
