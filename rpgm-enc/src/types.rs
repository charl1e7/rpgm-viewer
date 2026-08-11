use serde::{Deserialize, Serialize};

pub const PNG_HEADER_BYTES: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RPGMakerVersion {
    #[default]
    MV,
    MZ,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
        if key.len() % 2 != 0 {
            return None;
        }

        let bytes = key
            .as_bytes()
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

        let known_header = &PNG_HEADER_BYTES[..header_len.min(PNG_HEADER_BYTES.len())];
        let key = Self::from_known_header(header_len, data, known_header)?;

        let payload_offset = header_len;
        if data.len() >= payload_offset + 29 {
            let compression = data[payload_offset + 26];
            let filter = data[payload_offset + 27];
            let interlace = data[payload_offset + 28];

            if compression == 0 && filter == 0 && (interlace == 0 || interlace == 1) {
                return Some(key);
            }
        }

        None
    }

    pub fn from_ogg_header(header_len: usize, data: &[u8]) -> Option<Self> {
        const KNOWN: &[u8; 14] = &[
            0x4F, 0x67, 0x67, 0x53, // "OggS"
            0x00, // stream_structure_version
            0x02, // header_type_flag (BOS)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // granule_position = 0
        ];

        if data.len() < header_len + 27 {
            return None;
        }
        let enc = &data[header_len..];

        let mut key_bytes: Vec<u8> = (0..14).map(|i| enc[i] ^ KNOWN[i]).collect();
        let serial_hi = [enc[16], enc[17]];

        if let Some([s0, s1]) = Self::find_ogg_serial_lo(&data[header_len + 16..], serial_hi) {
            key_bytes.push(enc[14] ^ s0);
            key_bytes.push(enc[15] ^ s1);
            let hex: String = key_bytes.iter().map(|b| format!("{:02x}", b)).collect();
            return Self::new(&hex);
        }

        Self::bruteforce_last_two_bytes(header_len, data, &key_bytes)
    }

    fn bruteforce_last_two_bytes(
    header_len: usize,
    data: &[u8],
    partial_key: &[u8],
) -> Option<Self> {
    let payload = &data[header_len..];

    let n_segments = payload[26] as usize;
    if payload.len() < 27 + n_segments {
        return None;
    }
    let data_len: usize = payload[27..27 + n_segments]
        .iter()
        .map(|&b| b as usize)
        .sum();
    let page_len = 27 + n_segments + data_len;
    if payload.len() < page_len {
        return None;
    }

    let stored_crc = u32::from_le_bytes([payload[22], payload[23], payload[24], payload[25]]);

    let mut dec = payload[..page_len].to_vec();

    for i in 0..14 {
        dec[i] ^= partial_key[i];
    }

    let orig_14 = dec[14];
    let orig_15 = dec[15];
    let orig_22 = dec[22];
    let orig_23 = dec[23];
    let orig_24 = dec[24];
    let orig_25 = dec[25];

    for lo in 0u16..=0xFFFF {
        let [b14, b15] = lo.to_le_bytes();

        dec[14] = orig_14 ^ b14;
        dec[15] = orig_15 ^ b15;
        dec[22] = 0;
        dec[23] = 0;
        dec[24] = 0;
        dec[25] = 0;

        if ogg_crc32(&dec) == stored_crc {
            let mut full_key = partial_key.to_vec();
            full_key.push(b14);
            full_key.push(b15);
            let hex: String = full_key.iter().map(|b| format!("{:02x}", b)).collect();
            return Self::new(&hex);
        }
    }
    None
}

    fn find_ogg_serial_lo(tail: &[u8], serial_hi: [u8; 2]) -> Option<[u8; 2]> {
        if tail.len() < 27 {
            return None;
        }
        for i in 0..tail.len() - 27 {
            if &tail[i..i + 4] == b"OggS" && tail[i + 4] == 0 {
                let s = &tail[i + 14..i + 18];
                if s[2] == serial_hi[0] && s[3] == serial_hi[1] {
                    return Some([s[0], s[1]]);
                }
            }
        }
        None
    }

    fn from_known_header(header_len: usize, data: &[u8], known_header: &[u8]) -> Option<Self> {
        if data.len() < header_len * 2 {
            return None;
        }

        let file_header = &data[header_len..header_len * 2];
        let mut key = String::with_capacity(header_len * 2);

        for i in 0..header_len {
            let known_byte = known_header.get(i).copied().unwrap_or(0);
            let key_byte = known_byte ^ file_header[i];
            key.push_str(&format!("{:02x}", key_byte));
        }

        Self::new(&key)
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|v| {
                v.get("encryptionKey")
                    .and_then(|k| k.as_str())
                    .map(|s| s.to_string())
            })
            .and_then(|key| Self::new(&key))
    }

    pub fn from_rpg_core(content: &str) -> Option<Self> {
        for line in content.lines() {
            if let Some(idx) = line.find("this._encryptionKey") {
                let rest = &line[idx + "this._encryptionKey".len()..];
                let mut quote_char = None;
                let mut start_idx = None;

                for (i, ch) in rest.char_indices() {
                    if ch == '\'' || ch == '"' {
                        if quote_char.is_none() {
                            quote_char = Some(ch);
                            start_idx = Some(i + ch.len_utf8());
                        } else if Some(ch) == quote_char {
                            if let Some(start) = start_idx {
                                let key_candidate = &rest[start..i];
                                if let Some(key) = Self::new(key_candidate) {
                                    return Some(key);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
        None
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    fn is_valid_hex(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
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
    PNG,
    OGG,
    M4A,
    RPGMVP,
    RPGMVO,
    RPGMVM,
    PNG_,
    OGG_,
    M4A_,
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
        matches!(
            self,
            Self::RPGMVP | Self::RPGMVO | Self::RPGMVM | Self::PNG_ | Self::OGG_ | Self::M4A_
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
            Self::OGG | Self::RPGMVO | Self::OGG_ | Self::M4A | Self::RPGMVM | Self::M4A_ => {
                FileType::Audio
            }
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
            match (self.get_file_type(), version) {
                (FileType::Image, RPGMakerVersion::MV) => Self::RPGMVP,
                (FileType::Image, RPGMakerVersion::MZ) => Self::PNG_,
                (FileType::Audio, RPGMakerVersion::MV) => match self {
                    Self::M4A | Self::RPGMVM | Self::M4A_ => Self::RPGMVM,
                    _ => Self::RPGMVO,
                },
                (FileType::Audio, RPGMakerVersion::MZ) => match self {
                    Self::M4A | Self::RPGMVM | Self::M4A_ => Self::M4A_,
                    _ => Self::OGG_,
                },
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Invalid encryption key")]
    InvalidKey,

    #[error("Invalid file header")]
    InvalidHeader,

    #[error("File is empty")]
    EmptyFile,

    #[error("Failed to detect encryption key")]
    KeyDetectionFailed,
}

pub type Result<T> = std::result::Result<T, Error>;

const OGG_CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i << 24;
        let mut j = 0;
        while j < 8 {
            if crc & 0x80000000 != 0 {
                crc = (crc << 1) ^ 0x04C11DB7;
            } else {
                crc <<= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

fn ogg_crc32(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &b in data {
        crc = (crc << 8) ^ OGG_CRC_TABLE[((crc >> 24) as u8 ^ b) as usize];
    }
    crc
}
