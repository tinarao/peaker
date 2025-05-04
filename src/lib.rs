use std::{fs::File, path::Path};

use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::DecoderOptions,
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

type Err = Box<dyn std::error::Error>;

fn open_file(file_path: &str) -> Result<File, Err> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("Pizda".into());
    }

    return match File::open(file_path) {
        Ok(f) => Ok(f),
        Err(e) => Err(Box::new(e)),
    };
}

/// Reads PCM content from .mp3 file
pub fn decode(file_path: &str) -> Result<Vec<i16>, Err> {
    let file = open_file(file_path)?;
    let mut samples: Vec<i16> = Vec::new();

    let format_opts = FormatOptions {
        enable_gapless: true,
        ..FormatOptions::default()
    };

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let probe = symphonia::default::get_probe();
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let mut probed = match probe.format(&hint, mss, &format_opts, &MetadataOptions::default()) {
        Ok(pr) => pr,
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let track = match probed.format.default_track() {
        Some(t) => t,
        None => return Err("Audio track not found".into()),
    };

    println!("Format: {:#?}", track.codec_params.sample_format);

    let mut decoder = match symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
    {
        Ok(d) => d,
        Err(e) => return Err(e.into()),
    };

    let mut packet_count = 0;

    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(_)) => break,
            Err(e) => {
                return Err(e.into());
            }
        };

        match decoder.decode(&packet)? {
            // здесь становится запутанно
            // как я понял, дефолтный стандарт для mp3 - S16
            // F32 по сути не должен встретиться, но на всякий случай
            // этот кейс отрабатывается конвертацией
            AudioBufferRef::S16(b) => {
                samples.extend(b.chan(0));
            }
            AudioBufferRef::F32(buf) => {
                let converted: Vec<i16> = buf
                    .chan(0)
                    .iter()
                    .map(|v| (v * i16::MAX as f32) as i16)
                    .collect();

                samples.extend(converted);
            }
            _ => return Err("Unsupported format!".into()),
        };

        packet_count += 1;

        if packet_count % 100 == 0 {
            println!("Analyzed {} packets", &packet_count);
        }

        if packet_count > 100000 {
            // Файл может быть битым - не содержать корректного EOF
            // На этот случай нужна эта отсечка - в случае
            // беды, просто крашим луп.
            break;
        }
    }

    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonexistent_file() {
        let result = decode("non-existent.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn wrong_format() {
        let result = decode("wav-file.wav");
        assert!(result.is_err());
    }

    #[test]
    fn correct_case() {
        let result = decode("audio.mp3");
        assert!(result.is_ok());
    }
}
