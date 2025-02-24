#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum RPGMakerVersion {
    #[default]
    MV,
    MZ,
}

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[derive(Serialize, Deserialize)]
pub struct Key {
    #[serde(rename = "key")]
    raw: String,
    bytes: Vec<u8>,
}

impl Key {
    pub fn new(key: &str) -> Option<Self> {
        if !Self::is_valid_hex(key) {
            return None;
        }

        let bytes = key.as_bytes()
            .chunks(2)
            .filter_map(|chunk| {
                let hex = std::str::from_utf8(chunk).ok()?;
                u8::from_str_radix(hex, 16).ok()
            })
            .collect();

        Some(Self {
            raw: key.to_string(),
            bytes,
        })
    }

    pub fn from_png_header(header_len: usize, data: &[u8]) -> Option<Self> {
        if data.len() < header_len * 2 {
            return None;
        }

        let file_header = &data[header_len..header_len * 2];
        let png_header = Self::get_png_header_bytes(header_len);
        let mut key = String::with_capacity(header_len * 2);

        for i in 0..header_len {
            let key_byte = png_header[i] ^ file_header[i];
            key.push_str(&format!("{:02x}", key_byte));
        }

        Self::new(&key)
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str::<serde_json::Value>(json).ok()
            .and_then(|v| v.get("encryptionKey")
                .and_then(|k| k.as_str())
                .map(|s| s.to_string()))
            .and_then(|key| Self::new(&key))
    }

    pub fn from_rpg_core(content: &str) -> Option<Self> {
        content.lines()
            .find(|line| line.contains("this._encryptionKey"))
            .and_then(|line| line.split('"').nth(1))
            .map(|s| s.to_string())
            .and_then(|key| Self::new(&key))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    fn is_valid_hex(s: &str) -> bool {
        s.chars().all(|c| c.is_ascii_hexdigit())
    }

    fn get_png_header_bytes(header_len: usize) -> Vec<u8> {
        const PNG_HEADER: &str = "89 50 4E 47 0D 0A 1A 0A 00 00 00 0D 49 48 44 52";
        PNG_HEADER.split(' ')
            .take(header_len)
            .filter_map(|hex| u8::from_str_radix(hex, 16).ok())
            .collect()
    }
}

impl TryFrom<String> for Key {
    type Error = Error;

    fn try_from(key: String) -> std::result::Result<Self, Self::Error> {
        Self::new(&key).ok_or(Error::InvalidKey)
    }
}

impl std::str::FromStr for Key {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::new(s).ok_or(Error::InvalidKey)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Image,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtension {
    // Normal extensions
    PNG,
    OGG,
    M4A,
    
    // Encrypted MV extensions
    RPGMVP,  // PNG
    RPGMVO,  // OGG
    RPGMVM,  // M4A
    
    // Encrypted MZ extensions
    PNG_,    // PNG
    OGG_,    // OGG
    M4A_,    // M4A
}

impl FileExtension {
    pub fn from_str(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(Self::PNG),
            "ogg" => Some(Self::OGG),
            "m4a" => Some(Self::M4A),
            "rpgmvp" => Some(Self::RPGMVP),
            "rpgmvo" => Some(Self::RPGMVO),
            "rpgmvm" => Some(Self::RPGMVM),
            "png_" => Some(Self::PNG_),
            "ogg_" => Some(Self::OGG_),
            "m4a_" => Some(Self::M4A_),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::PNG => "png",
            Self::OGG => "ogg",
            Self::M4A => "m4a",
            Self::RPGMVP => "rpgmvp",
            Self::RPGMVO => "rpgmvo",
            Self::RPGMVM => "rpgmvm",
            Self::PNG_ => "png_",
            Self::OGG_ => "ogg_",
            Self::M4A_ => "m4a_",
        }
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, 
            Self::RPGMVP | Self::RPGMVO | Self::RPGMVM |
            Self::PNG_ | Self::OGG_ | Self::M4A_
        )
    }

    pub fn get_mime_type(&self) -> &'static str {
        match self {
            Self::PNG | Self::RPGMVP | Self::PNG_ => "image/png",
            Self::OGG | Self::RPGMVO | Self::OGG_ => "audio/ogg",
            Self::M4A | Self::RPGMVM | Self::M4A_ => "audio/m4a",
        }
    }

    pub fn get_file_type(&self) -> FileType {
        match self {
            Self::PNG | Self::RPGMVP | Self::PNG_ => FileType::Image,
            Self::OGG | Self::RPGMVO | Self::OGG_ |
            Self::M4A | Self::RPGMVM | Self::M4A_ => FileType::Audio,
        }
    }

    pub fn convert(&self, to_normal: bool, version: RPGMakerVersion) -> Self {
        if to_normal {
            match self {
                Self::RPGMVP | Self::PNG_ => Self::PNG,
                Self::RPGMVO | Self::OGG_ => Self::OGG,
                Self::RPGMVM | Self::M4A_ => Self::M4A,
                _ => *self,
            }
        } else {
            match (self, version) {
                (Self::RPGMVP | Self::PNG_, _) => *self,
                (Self::RPGMVO | Self::OGG_, _) => *self,
                (Self::RPGMVM | Self::M4A_, _) => *self,
                
                (Self::PNG, RPGMakerVersion::MZ) => Self::PNG_,
                (Self::PNG, RPGMakerVersion::MV) => Self::RPGMVP,
                (Self::OGG, RPGMakerVersion::MZ) => Self::OGG_,
                (Self::OGG, RPGMakerVersion::MV) => Self::RPGMVO,
                (Self::M4A, RPGMakerVersion::MZ) => Self::M4A_,
                (Self::M4A, RPGMakerVersion::MV) => Self::RPGMVM,
                _ => *self,
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid file extension: {0}")]
    InvalidExtension(String),
    
    #[error("Invalid encryption key")]
    InvalidKey,
    
    #[error("Invalid file header")]
    InvalidHeader,
    
    #[error("File is empty")]
    EmptyFile,
    
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    
    #[error("Failed to detect encryption key")]
    KeyDetectionFailed,
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension_conversion() {
        assert_eq!(FileExtension::RPGMVP.convert(true, RPGMakerVersion::MV), FileExtension::PNG);
        assert_eq!(FileExtension::RPGMVO.convert(true, RPGMakerVersion::MV), FileExtension::OGG);
        assert_eq!(FileExtension::RPGMVM.convert(true, RPGMakerVersion::MV), FileExtension::M4A);
        assert_eq!(FileExtension::PNG_.convert(true, RPGMakerVersion::MZ), FileExtension::PNG);
        assert_eq!(FileExtension::OGG_.convert(true, RPGMakerVersion::MZ), FileExtension::OGG);
        assert_eq!(FileExtension::M4A_.convert(true, RPGMakerVersion::MZ), FileExtension::M4A);

        assert_eq!(FileExtension::PNG.convert(false, RPGMakerVersion::MV), FileExtension::RPGMVP);
        assert_eq!(FileExtension::OGG.convert(false, RPGMakerVersion::MV), FileExtension::RPGMVO);
        assert_eq!(FileExtension::M4A.convert(false, RPGMakerVersion::MV), FileExtension::RPGMVM);

        assert_eq!(FileExtension::PNG.convert(false, RPGMakerVersion::MZ), FileExtension::PNG_);
        assert_eq!(FileExtension::OGG.convert(false, RPGMakerVersion::MZ), FileExtension::OGG_);
        assert_eq!(FileExtension::M4A.convert(false, RPGMakerVersion::MZ), FileExtension::M4A_);

        assert_eq!(FileExtension::RPGMVP.convert(false, RPGMakerVersion::MZ), FileExtension::RPGMVP);
        assert_eq!(FileExtension::PNG_.convert(false, RPGMakerVersion::MV), FileExtension::PNG_);
        assert_eq!(FileExtension::RPGMVO.convert(false, RPGMakerVersion::MZ), FileExtension::RPGMVO);
        assert_eq!(FileExtension::OGG_.convert(false, RPGMakerVersion::MV), FileExtension::OGG_);
    }

    #[test]
    fn test_file_extension_from_str() {
        // Test normal extensions
        assert_eq!(FileExtension::from_str("png"), Some(FileExtension::PNG));
        assert_eq!(FileExtension::from_str("ogg"), Some(FileExtension::OGG));
        assert_eq!(FileExtension::from_str("m4a"), Some(FileExtension::M4A));

        // Test RPG Maker MV extensions
        assert_eq!(FileExtension::from_str("rpgmvp"), Some(FileExtension::RPGMVP));
        assert_eq!(FileExtension::from_str("rpgmvo"), Some(FileExtension::RPGMVO));
        assert_eq!(FileExtension::from_str("rpgmvm"), Some(FileExtension::RPGMVM));

        // Test RPG Maker MZ extensions
        assert_eq!(FileExtension::from_str("png_"), Some(FileExtension::PNG_));
        assert_eq!(FileExtension::from_str("ogg_"), Some(FileExtension::OGG_));
        assert_eq!(FileExtension::from_str("m4a_"), Some(FileExtension::M4A_));

        // Test case insensitivity
        assert_eq!(FileExtension::from_str("PNG"), Some(FileExtension::PNG));
        assert_eq!(FileExtension::from_str("RPGMVP"), Some(FileExtension::RPGMVP));
        assert_eq!(FileExtension::from_str("PNG_"), Some(FileExtension::PNG_));

        // Test invalid extension
        assert_eq!(FileExtension::from_str("invalid"), None);
    }

    #[test]
    fn test_file_extension_properties() {
        // Test is_encrypted
        assert!(FileExtension::RPGMVP.is_encrypted());
        assert!(FileExtension::PNG_.is_encrypted());
        assert!(!FileExtension::PNG.is_encrypted());

        // Test get_mime_type
        assert_eq!(FileExtension::PNG.get_mime_type(), "image/png");
        assert_eq!(FileExtension::RPGMVP.get_mime_type(), "image/png");
        assert_eq!(FileExtension::PNG_.get_mime_type(), "image/png");
        assert_eq!(FileExtension::OGG.get_mime_type(), "audio/ogg");
        assert_eq!(FileExtension::M4A.get_mime_type(), "audio/m4a");

        // Test get_file_type
        assert_eq!(FileExtension::PNG.get_file_type(), FileType::Image);
        assert_eq!(FileExtension::RPGMVP.get_file_type(), FileType::Image);
        assert_eq!(FileExtension::PNG_.get_file_type(), FileType::Image);
        assert_eq!(FileExtension::OGG.get_file_type(), FileType::Audio);
        assert_eq!(FileExtension::RPGMVO.get_file_type(), FileType::Audio);
        assert_eq!(FileExtension::OGG_.get_file_type(), FileType::Audio);
    }
} 