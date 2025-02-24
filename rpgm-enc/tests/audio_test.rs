use rpgm_enc::{Decrypter, FileExtension, RPGFile, RPGMakerVersion, Result};
use std::fs;
use std::path::PathBuf;
use symphonia::core::codecs::CODEC_TYPE_VORBIS;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

fn verify_image_format(data: &[u8]) -> bool {
    match image::load_from_memory(data) {
        Ok(img) => {
            println!(
                "Successfully loaded image: {}x{}",
                img.width(),
                img.height()
            );
            true
        }
        Err(e) => {
            println!("Failed to verify image format: {:?}", e);
            false
        }
    }
}

fn verify_audio_format(data: &[u8], format_hint: &str) -> bool {
    let source = std::io::Cursor::new(Vec::from(data));
    let media_source = MediaSourceStream::new(Box::new(source), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(format_hint);

    let probe = get_probe();
    let fmt_opts = FormatOptions::default();
    let meta_opts = MetadataOptions::default();

    match probe.format(&hint, media_source, &fmt_opts, &meta_opts) {
        Ok(probed) => {
            let fmt = probed.format;

            if fmt.tracks().is_empty() {
                println!("No tracks found in the audio file");
                return false;
            }

            let track = &fmt.tracks()[0];

            if track.codec_params.codec != CODEC_TYPE_VORBIS {
                println!("Not a Vorbis audio track: {:?}", track.codec_params.codec);
                return false;
            }

            if track.codec_params.sample_rate.is_none() {
                println!("No sample rate information");
                return false;
            }

            if track.codec_params.channels.is_none() {
                println!("No channel information");
                return false;
            }

            true
        }
        Err(e) => {
            println!("Failed to verify format: {:?}", e);
            false
        }
    }
}

#[test]
fn test_ogg_conversion() -> Result<()> {
    let test_png = include_bytes!("test_data/test.png_");
    let key =
        Decrypter::detect_key_from_file(test_png).expect("Failed to detect key from PNG file");
    println!("Detected encryption key: {}", key.as_str());

    let test_data = include_bytes!("test_data/test.ogg_");
    println!(
        "Original encrypted data first 32 bytes: {:02X?}",
        &test_data[..32.min(test_data.len())]
    );

    {
        let mut rpg_file = RPGFile::new(PathBuf::from("test.ogg_"))?;
        rpg_file.set_version(RPGMakerVersion::MV);
        rpg_file.set_content(test_data.to_vec());

        assert!(rpg_file.is_encrypted());
        assert_eq!(rpg_file.extension(), Some(FileExtension::OGG_));

        rpg_file.convert_extension(true);
        assert_eq!(rpg_file.extension(), Some(FileExtension::OGG));

        rpg_file.convert_extension(false);
        assert_eq!(rpg_file.extension(), Some(FileExtension::RPGMVO));
    }

    {
        let mut rpg_file = RPGFile::new(PathBuf::from("test.ogg_"))?;
        rpg_file.set_version(RPGMakerVersion::MZ);
        rpg_file.set_content(test_data.to_vec());

        assert!(rpg_file.is_encrypted());
        assert_eq!(rpg_file.extension(), Some(FileExtension::OGG_));

        rpg_file.convert_extension(true);
        assert_eq!(rpg_file.extension(), Some(FileExtension::OGG));

        rpg_file.convert_extension(false);
        assert_eq!(rpg_file.extension(), Some(FileExtension::OGG_));
    }

    Ok(())
}

#[test]
fn test_png_conversion() -> Result<()> {
    let test_png = include_bytes!("test_data/test.png_");
    let key =
        Decrypter::detect_key_from_file(test_png).expect("Failed to detect key from PNG file");
    println!("Detected encryption key: {}", key.as_str());

    println!(
        "Original encrypted data first 32 bytes: {:02X?}",
        &test_png[..32.min(test_png.len())]
    );

    let mut rpg_file = RPGFile::new(PathBuf::from("test.png_"))?;
    rpg_file.set_content(test_png.to_vec());

    assert!(rpg_file.is_encrypted());
    assert!(rpg_file.is_image());
    assert_eq!(rpg_file.extension(), Some(FileExtension::PNG_));

    let decrypter = Decrypter::new(Some(key));
    let decrypted_content = decrypter.decrypt(rpg_file.content().unwrap())?;
    println!(
        "Decrypted content first 32 bytes: {:02X?}",
        &decrypted_content[..32.min(decrypted_content.len())]
    );

    let decrypted_path = "tests/test_data/decrypted_only_test.png";
    fs::write(decrypted_path, &decrypted_content)?;
    println!(
        "Saved decrypted (before header restoration) file to: {}",
        decrypted_path
    );

    let restored_content = decrypter.restore_header(&decrypted_content, FileExtension::PNG)?;
    println!(
        "Restored content first 32 bytes: {:02X?}",
        &restored_content[..32.min(restored_content.len())]
    );

    let output_path = "tests/test_data/decrypted_test.png";
    fs::write(output_path, &restored_content)?;
    println!("Saved final decrypted file to: {}", output_path);

    rpg_file.convert_extension(true);
    rpg_file.set_content(restored_content);

    assert_eq!(rpg_file.extension(), Some(FileExtension::PNG));
    assert!(!rpg_file.is_encrypted());
    assert_eq!(rpg_file.mime_type(), Some("image/png"));

    assert!(
        verify_image_format(rpg_file.content().unwrap()),
        "Failed to verify PNG format"
    );

    let _ = fs::remove_file(output_path);
    let _ = fs::remove_file(decrypted_path);
    Ok(())
}
