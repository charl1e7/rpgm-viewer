use crate::types::*;

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct Decrypter {
    // Encryption Fields
    pub key: Option<Key>,

    // Option Fields
    ignore_fake_header: bool,

    // Fake-Header Info Fields
    header_len: Option<usize>,
    signature: Option<String>,
    version: Option<String>,
    remain: Option<String>,

    // Header Lengths
    png_header_len: Option<usize>,
    ogg_header_len: Option<usize>,
    m4a_header_len: Option<usize>,
}

impl Decrypter {
    const DEFAULT_HEADER_LEN: usize = 16;
    const DEFAULT_SIGNATURE: &'static str = "5250474d56000000";
    const DEFAULT_VERSION: &'static str = "000301";
    const DEFAULT_REMAIN: &'static str = "0000000000";
    const PNG_HEADER_BYTES: &'static str = "89 50 4E 47 0D 0A 1A 0A 00 00 00 0D 49 48 44 52";
    const OGG_HEADER_BYTES: &'static str =
        "4F 67 67 53 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00";
    const M4A_HEADER_BYTES: &'static str = "00 00 00 20 66 74 79 70 4D 34 41 20 00 00 00 00";

    pub fn new(key: Option<Key>) -> Self {
        Decrypter {
            key,
            ignore_fake_header: false,
            header_len: None,
            signature: None,
            version: None,
            remain: None,
            png_header_len: None,
            ogg_header_len: None,
            m4a_header_len: None,
        }
    }

    pub fn from_file(file_contents: &[u8]) -> Option<Self> {
        let key = Self::detect_key_from_file(file_contents);
        if let Some(key) = key {
            Some(Self::new(Some(key)))
        } else {
            None
        }
    }

    pub fn verify_fake_header(&self, file_header: &[u8]) -> bool {
        let fake_header = self.build_fake_header();
        if file_header.len() < self.get_header_len() {
            return false;
        }

        for i in 0..self.get_header_len() {
            if file_header[i] != fake_header[i] {
                return false;
            }
        }
        true
    }

    fn build_fake_header(&self) -> Vec<u8> {
        let mut fake_header = vec![0u8; self.get_header_len()];
        let header_structure = format!(
            "{}{}{}",
            self.get_signature(),
            self.get_version(),
            self.get_remain()
        );

        for i in 0..self.get_header_len() {
            let hex_str = &header_structure[i * 2..i * 2 + 2];
            fake_header[i] = u8::from_str_radix(hex_str, 16).unwrap_or(0);
        }

        fake_header
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(Error::EmptyFile);
        }

        if !self.ignore_fake_header {
            let header = &data[0..self.get_header_len()];
            if !self.verify_fake_header(header) {
                return Err(Error::InvalidHeader);
            }
        }

        // Rem fake header
        let mut content = data[self.get_header_len()..].to_vec();

        self.xor_bytes(&mut content);

        Ok(content)
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(Error::EmptyFile);
        }

        let mut content = data.to_vec();
        self.xor_bytes(&mut content);

        let fake_header = self.build_fake_header();
        let mut result = Vec::with_capacity(content.len() + self.get_header_len());
        result.extend_from_slice(&fake_header);
        result.extend_from_slice(&content);

        if !self.verify_fake_header(&result[0..self.get_header_len()]) {
            return Err(Error::InvalidHeader);
        }

        Ok(result)
    }

    fn xor_bytes(&self, data: &mut [u8]) {
        if let Some(key) = &self.key {
            let key_bytes = key.as_bytes();
            for i in 0..self.get_header_len().min(data.len()).min(key_bytes.len()) {
                data[i] ^= key_bytes[i];
            }
        }
    }

    pub fn restore_header(&self, data: &[u8], file_type: FileExtension) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(Error::EmptyFile);
        }

        let has_correct_header = match file_type {
            FileExtension::OGG | FileExtension::RPGMVO | FileExtension::OGG_ if data.len() >= 4 => {
                &data[0..4] == &[0x4F, 0x67, 0x67, 0x53]
            } // "OggS"
            FileExtension::PNG | FileExtension::RPGMVP | FileExtension::PNG_ if data.len() >= 8 => {
                &data[0..8] == &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
            } // PNG signature
            FileExtension::M4A | FileExtension::RPGMVM | FileExtension::M4A_ if data.len() >= 8 => {
                &data[4..8] == b"ftyp"
            } // M4A signature
            _ => false,
        };

        if has_correct_header {
            return Ok(data.to_vec());
        }

        let fake_header_len = self.get_header_len();
        let (correct_header_len, header_bytes) = match file_type {
            FileExtension::PNG | FileExtension::RPGMVP | FileExtension::PNG_ => (
                self.png_header_len.unwrap_or(fake_header_len),
                Self::PNG_HEADER_BYTES,
            ),
            FileExtension::OGG | FileExtension::RPGMVO | FileExtension::OGG_ => {
                (self.ogg_header_len.unwrap_or(28), Self::OGG_HEADER_BYTES)
            }
            FileExtension::M4A | FileExtension::RPGMVM | FileExtension::M4A_ => (
                self.m4a_header_len.unwrap_or(fake_header_len),
                Self::M4A_HEADER_BYTES,
            ),
        };

        let header = Self::get_header_bytes(header_bytes, correct_header_len);

        let has_fake_header = if data.len() >= self.get_header_len() {
            self.verify_fake_header(&data[0..self.get_header_len()])
        } else {
            false
        };

        let content = if has_fake_header {
            if data.len() < fake_header_len {
                return Err(Error::InvalidHeader);
            }
            &data[fake_header_len..]
        } else {
            data
        };

        let mut result = Vec::with_capacity(content.len() + correct_header_len);
        result.extend_from_slice(&header);
        result.extend_from_slice(content);

        Ok(result)
    }

    fn get_header_bytes(header_str: &str, header_len: usize) -> Vec<u8> {
        let header_to_restore: Vec<&str> = header_str.split(' ').collect();
        let len = header_len.min(header_to_restore.len());
        let mut restored_header = vec![0u8; len];

        for i in 0..len {
            if let Ok(byte) = u8::from_str_radix(header_to_restore[i], 16) {
                restored_header[i] = byte;
            }
        }

        restored_header
    }

    pub fn get_header_len(&self) -> usize {
        self.header_len.unwrap_or(Self::DEFAULT_HEADER_LEN)
    }

    fn get_signature(&self) -> String {
        self.signature
            .clone()
            .unwrap_or_else(|| Self::DEFAULT_SIGNATURE.to_string())
    }

    fn get_version(&self) -> String {
        self.version
            .clone()
            .unwrap_or_else(|| Self::DEFAULT_VERSION.to_string())
    }

    fn get_remain(&self) -> String {
        self.remain
            .clone()
            .unwrap_or_else(|| Self::DEFAULT_REMAIN.to_string())
    }

    pub fn detect_key_from_file(file_contents: &[u8]) -> Option<Key> {
        let header_len = Self::new(None).get_header_len();
        Self::detect_encryption_code(file_contents, header_len)
    }

    fn detect_encryption_code(data: &[u8], header_len: usize) -> Option<Key> {
        if let Some(key) = Key::from_png_header(header_len, data) {
            return Some(key);
        }

        if let Ok(text) = String::from_utf8(data.to_vec()) {
            if let Some(key) = Key::from_json(&text) {
                return Some(key);
            }

            Key::from_rpg_core(&text)
        } else {
            None
        }
    }

    pub fn byte_to_hex(byte: u8) -> String {
        format!("{:02x}", byte)
    }

    pub fn check_hex_chars(s: &str) -> bool {
        s.chars().all(|c| c.is_ascii_hexdigit())
    }

    pub fn helper_show_bits(byte: u8) -> String {
        format!("{:08b}", byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_fake_header() {
        let key = Key::new("deadbeef").unwrap();
        let decrypter = Decrypter::new(Some(key));
        let fake_header = decrypter.build_fake_header();
        assert!(decrypter.verify_fake_header(&fake_header));
    }

    #[test]
    fn test_encryption_decryption() -> Result<()> {
        let key = Key::new("deadbeef").unwrap();
        let decrypter = Decrypter::new(Some(key));
        let test_data = b"Hello, World!";

        let encrypted = decrypter.encrypt(test_data)?;
        let decrypted = decrypter.decrypt(&encrypted)?;

        assert_eq!(&decrypted, test_data);
        Ok(())
    }
}
