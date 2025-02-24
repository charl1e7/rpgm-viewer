use std::path::{Path, PathBuf};
use crate::types::*;

pub struct RPGFile {
    path: PathBuf,
    content: Option<Vec<u8>>,
    version: RPGMakerVersion,
}

impl RPGFile {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Self {
            path,
            content: None,
            version: RPGMakerVersion::MV, 
        })
    }

    pub fn set_version(&mut self, version: RPGMakerVersion) {
        self.version = version;
    }

    pub fn get_version(&self) -> RPGMakerVersion {
        self.version
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn content(&self) -> Option<&[u8]> {
        self.content.as_deref()
    }

    pub fn set_content(&mut self, content: Vec<u8>) {
        self.content = Some(content);
    }

    pub fn extension(&self) -> Option<FileExtension> {
        self.path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(FileExtension::from_str)
    }

    pub fn convert_extension(&mut self, to_normal: bool) {
        if let Some(current_ext) = self.extension() {
            let new_ext = current_ext.convert(to_normal, self.version);
            if let Some(parent) = self.path.parent() {
                if let Some(stem) = self.path.file_stem() {
                    self.path = parent.join(stem).with_extension(new_ext.to_str());
                }
            }
        }
    }

    pub fn is_encrypted(&self) -> bool {
        self.extension().map_or(false, |ext| ext.is_encrypted())
    }

    pub fn is_image(&self) -> bool {
        self.extension().map_or(false, |ext| ext.get_file_type() == FileType::Image)
    }

    pub fn mime_type(&self) -> Option<&'static str> {
        self.extension().map(|ext| ext.get_mime_type())
    }
}
