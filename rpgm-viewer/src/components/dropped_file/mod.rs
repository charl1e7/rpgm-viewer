pub mod ui;
use std::path::PathBuf;



#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct DroppedFile {
    pending_load: Option<PathBuf>,
}
