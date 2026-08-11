use crate::types::*;

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct Decrypter {
    pub key: Option<Key>,
    ignore_fake_header: bool,
    header_len: Option<usize>,
    signature: Option<String>,
    version: Option<String>,
    remain: Option<String>,
    fake_header_cache: Vec<u8>,
}

impl Decrypter {
    const DEFAULT_HEADER_LEN: usize = 16;
    const DEFAULT_SIGNATURE: &'static str = "5250474d56000000";
    const DEFAULT_VERSION: &'static str = "000301";
    const DEFAULT_REMAIN: &'static str = "0000000000";

    pub fn new(key: Option<Key>) -> Self {
        let mut decrypter = Decrypter {
            key,
            ignore_fake_header: false,
            header_len: None,
            signature: None,
            version: None,
            remain: None,
            fake_header_cache: Vec::new(),
        };
        decrypter.rebuild_fake_header();
        decrypter
    }

    pub fn rebuild_fake_header(&mut self) {
        let header_len = self.get_header_len();
        let header_structure = format!(
            "{}{}{}",
            self.get_signature(),
            self.get_version(),
            self.get_remain()
        );

        let mut fake_header = vec![0u8; header_len];
        for i in 0..header_len {
            if i * 2 + 2 <= header_structure.len() {
                let hex_str = &header_structure[i * 2..i * 2 + 2];
                fake_header[i] = u8::from_str_radix(hex_str, 16).unwrap_or(0);
            }
        }
        self.fake_header_cache = fake_header;
    }

    pub fn build_fake_header(&self) -> &[u8] {
        &self.fake_header_cache
    }

    pub fn verify_fake_header(&self, file_header: &[u8]) -> bool {
        let fake_header = self.build_fake_header();
        if file_header.len() < self.get_header_len() {
            return false;
        }
        file_header[..self.get_header_len()] == fake_header[..self.get_header_len()]
    }

    pub fn decrypt(&self, data: &[u8], file_type: FileExtension) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(Error::EmptyFile);
        }

        if matches!(
            file_type,
            FileExtension::M4A | FileExtension::M4A_ | FileExtension::RPGMVM
        ) {
            return Ok(data.to_vec());
        }

        let header_len = self.get_header_len();
        if data.len() < header_len {
            return Err(Error::InvalidHeader);
        }

        if !self.ignore_fake_header {
            let header = &data[0..header_len];
            if !self.verify_fake_header(header) {
                return Err(Error::InvalidHeader);
            }
        }

        let mut content = data[header_len..].to_vec();
        self.xor_bytes(&mut content);
        Ok(content)
    }

    pub fn encrypt(&self, data: &[u8], file_type: FileExtension) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Err(Error::EmptyFile);
        }

        // M4A does not get encrypted — just return the data
        if matches!(
            file_type,
            FileExtension::M4A | FileExtension::M4A_ | FileExtension::RPGMVM
        ) {
            return Ok(data.to_vec());
        }

        let mut content = data.to_vec();
        self.xor_bytes(&mut content);

        let fake_header = self.build_fake_header();
        let mut result = Vec::with_capacity(content.len() + self.get_header_len());
        result.extend_from_slice(fake_header);
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
                &data[0..4] == b"OggS"
            }
            FileExtension::PNG | FileExtension::RPGMVP | FileExtension::PNG_ if data.len() >= 8 => {
                &data[0..8] == &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
            }
            FileExtension::M4A | FileExtension::RPGMVM | FileExtension::M4A_ if data.len() >= 8 => {
                &data[4..8] == b"ftyp"
            }
            _ => false,
        };

        if has_correct_header {
            return Ok(data.to_vec());
        }

        match file_type {
            FileExtension::PNG | FileExtension::RPGMVP | FileExtension::PNG_ => {
                let fake_header_len = self.get_header_len();
                let header = &PNG_HEADER_BYTES[..fake_header_len.min(PNG_HEADER_BYTES.len())];

                let has_fake_header = data.len() >= fake_header_len
                    && self.verify_fake_header(&data[0..fake_header_len]);
                let content = if has_fake_header {
                    if data.len() < fake_header_len {
                        return Err(Error::InvalidHeader);
                    }
                    &data[fake_header_len..]
                } else {
                    data
                };

                let mut result = Vec::with_capacity(content.len() + header.len());
                result.extend_from_slice(header);
                result.extend_from_slice(content);
                Ok(result)
            }
            _ => Err(Error::InvalidHeader),
        }
    }

    pub fn get_header_len(&self) -> usize {
        self.header_len.unwrap_or(Self::DEFAULT_HEADER_LEN)
    }

    fn get_signature(&self) -> &str {
        self.signature.as_deref().unwrap_or(Self::DEFAULT_SIGNATURE)
    }

    fn get_version(&self) -> &str {
        self.version.as_deref().unwrap_or(Self::DEFAULT_VERSION)
    }

    fn get_remain(&self) -> &str {
        self.remain.as_deref().unwrap_or(Self::DEFAULT_REMAIN)
    }

    pub fn detect_key(data: &[u8], file_type: FileExtension) -> Option<Key> {
        match file_type {
            FileExtension::PNG | FileExtension::RPGMVP | FileExtension::PNG_ => {
                Key::from_png_header(Self::DEFAULT_HEADER_LEN, data)
            }
            FileExtension::OGG | FileExtension::RPGMVO | FileExtension::OGG_ => {
                Key::from_ogg_header(Self::DEFAULT_HEADER_LEN, data)
            }
            _ => None,
        }
    }

    pub fn set_header_params(&mut self, len: usize, sig: &str, ver: &str, rem: &str) {
        self.header_len = Some(len);
        self.signature = Some(sig.to_string());
        self.version = Some(ver.to_string());
        self.remain = Some(rem.to_string());
        self.rebuild_fake_header();
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
        let encrypted = decrypter.encrypt(test_data, FileExtension::PNG_)?;
        let decrypted = decrypter.decrypt(&encrypted, FileExtension::PNG_)?;
        assert_eq!(&decrypted, test_data);
        Ok(())
    }
}
