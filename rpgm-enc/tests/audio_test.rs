use rpgm_enc::{Decrypter, FileExtension, Key, RPGMakerVersion, Result};

fn verify_image_format(data: &[u8]) -> bool {
    image::load_from_memory(data).is_ok()
}

fn verify_audio_format(data: &[u8], format_hint: &str) -> bool {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let source = std::io::Cursor::new(Vec::from(data));
    let mss = MediaSourceStream::new(Box::new(source), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(format_hint);

    let mut format = match symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    ) {
        Ok(f) => f,
        Err(e) => {
            println!("Probe failed: {:?}", e);
            return false;
        }
    };

    let track = match format.default_track(TrackType::Audio) {
        Some(t) => t,
        None => {
            println!("No audio track found");
            return false;
        }
    };

    let audio_params = match track.codec_params.as_ref().and_then(|p| p.audio()) {
        Some(p) => p,
        None => {
            println!("Track has no audio codec parameters");
            return false;
        }
    };

    let mut decoder = match symphonia::default::get_codecs()
        .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
    {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to create audio decoder: {:?}", e);
            return false;
        }
    };

    println!("Symphonia successfully created audio decoder");

    let mut packets = 0usize;
    while packets < 5 {
        match format.next_packet() {
            Ok(Some(packet)) => match decoder.decode(&packet) {
                Ok(_) => packets += 1,
                Err(e) => {
                    println!("Decode error on packet {}: {:?}", packets, e);
                    return false;
                }
            },
            Ok(None) => break,
            Err(e) => {
                println!("Error reading packet: {:?}", e);
                return false;
            }
        }
    }

    packets > 0
}

#[test]
fn test_extension_conversion() {
    assert_eq!(
        FileExtension::RPGMVP.convert(true, RPGMakerVersion::MV),
        FileExtension::PNG
    );
    assert_eq!(
        FileExtension::RPGMVO.convert(true, RPGMakerVersion::MV),
        FileExtension::OGG
    );
    assert_eq!(
        FileExtension::PNG.convert(false, RPGMakerVersion::MZ),
        FileExtension::PNG_
    );
    assert_eq!(
        FileExtension::OGG.convert(false, RPGMakerVersion::MZ),
        FileExtension::OGG_
    );
}

#[test]
fn test_extension_properties() {
    assert!(FileExtension::RPGMVP.is_encrypted());
    assert!(!FileExtension::PNG.is_encrypted());

    assert_eq!(FileExtension::PNG.get_mime_type(), "image/png");
    assert_eq!(FileExtension::OGG.get_mime_type(), "audio/ogg");
    assert_eq!(FileExtension::M4A.get_mime_type(), "audio/m4a");
}

#[test]
fn test_key_from_png() -> Result<()> {
    let test_png = include_bytes!("test_data/test.png_");

    let key = Decrypter::detect_key(test_png, FileExtension::PNG_)
        .expect("Failed to detect key from PNG");
    println!("PNG key: {}", key.as_str());

    let decrypter = Decrypter::new(Some(key));
    let decrypted = decrypter.decrypt(test_png, FileExtension::PNG_)?;
    let restored = decrypter.restore_header(&decrypted, FileExtension::PNG)?;

    assert!(verify_image_format(&restored));
    Ok(())
}

#[test]
fn test_key_from_audio_only() -> Result<()> {
    let test_ogg = include_bytes!("test_data/test.ogg_");

    let key = Key::from_ogg_header(16, test_ogg).expect("Failed to extract key from OGG");
    println!("Audio-only key: {}", key.as_str());

    let decrypter = Decrypter::new(Some(key));
    let decrypted = decrypter.decrypt(test_ogg, FileExtension::OGG_)?;

    assert_eq!(&decrypted[0..4], b"OggS");
    assert!(verify_audio_format(&decrypted, "ogg"));

    let restored = decrypter.restore_header(&decrypted, FileExtension::OGG_)?;
    assert_eq!(&restored[0..4], b"OggS");

    Ok(())
}

#[test]
fn test_detect_key_with_hint() -> Result<()> {
    let test_ogg = include_bytes!("test_data/test.ogg_");

    let key = Decrypter::detect_key(test_ogg, FileExtension::OGG_)
        .expect("detect_key should find key in OGG");

    let decrypter = Decrypter::new(Some(key));
    let decrypted = decrypter.decrypt(test_ogg, FileExtension::OGG_)?;

    assert_eq!(&decrypted[0..4], b"OggS");
    assert!(verify_audio_format(&decrypted, "ogg"));
    Ok(())
}

#[test]
fn test_full_audio_only_pipeline() -> Result<()> {
    let test_ogg = include_bytes!("test_data/test.ogg_");

    let key = Decrypter::detect_key(test_ogg, FileExtension::OGG_)
        .expect("Should extract key from audio");

    let decrypter = Decrypter::new(Some(key));
    let decrypted = decrypter.decrypt(test_ogg, FileExtension::OGG_)?;
    let final_data = decrypter.restore_header(&decrypted, FileExtension::OGG_)?;

    assert!(verify_audio_format(&final_data, "ogg"));
    Ok(())
}

#[test]
fn test_m4a_passthrough() -> Result<()> {
    let key = Key::new("deadbeef").unwrap();
    let decrypter = Decrypter::new(Some(key));

    let mut m4a = vec![0x00, 0x00, 0x00, 0x20];
    m4a.extend_from_slice(b"ftypM4A ");
    m4a.extend_from_slice(&[0u8; 8]);

    let encrypted = decrypter.encrypt(&m4a, FileExtension::M4A_)?;
    assert_eq!(encrypted, m4a);

    let decrypted = decrypter.decrypt(&m4a, FileExtension::M4A_)?;
    assert_eq!(decrypted, m4a);
    Ok(())
}
