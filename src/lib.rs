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

/// Defines how many packets to include in result.
/// For example - 2-minute 44100 hz & 192 kb/s
/// mp3 file returns Vec<i16> with length over 5500000
pub enum Precision {
    /// Every 5000 packets
    Ultralow = 5000,

    /// Every 5000 packets
    Low = 1000,

    /// Every 1000 packets
    Medium = 500,

    /// Every 100 packets
    High = 100,

    /// Every packet will be included
    Max = 1,
}

impl Precision {
    pub fn value(&self) -> usize {
        match self {
            Precision::Ultralow => 5000,
            Precision::Low => 1000,
            Precision::Medium => 500,
            Precision::High => 100,
            Precision::Max => 1,
        }
    }
}

pub struct DecoderConfig {
    /// How many packets to process. Defaults to 100000
    pub packets_limit: i32,

    /// Result may be heavy.
    pub precision: Precision,
}

impl DecoderConfig {
    pub fn default() -> Self {
        DecoderConfig {
            packets_limit: 100000,
            precision: Precision::Max,
        }
    }

    pub fn new(packets_limit: i32, precision: Precision) -> Self {
        DecoderConfig {
            packets_limit,
            precision,
        }
    }
}

pub struct Decoder {
    pub config: DecoderConfig,
    file_path: String,
}

impl Decoder {
    pub fn new(file_path: &str, cfg: DecoderConfig) -> Self {
        Decoder {
            config: cfg,
            file_path: file_path.to_string(),
        }
    }

    fn open_file(&self) -> Result<File, Err> {
        let path = Path::new(&self.file_path);
        if !path.exists() {
            return Err("Pizda".into());
        }

        return match File::open(&self.file_path) {
            Ok(f) => Ok(f),
            Err(e) => Err(Box::new(e)),
        };
    }

    pub fn decode(&self) -> Result<Vec<i16>, Err> {
        let file = self.open_file()?;
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

            let packets: Vec<i16> = match decoder.decode(&packet)? {
                // здесь становится запутанно
                // как я понял, дефолтный стандарт для mp3 - S16
                // F32 по сути не должен встретиться, но на всякий случай
                // этот кейс отрабатывается конвертацией
                AudioBufferRef::S16(b) => b.chan(0).into_iter().map(|p| *p).collect(),
                AudioBufferRef::F32(buf) => buf
                    .chan(0)
                    .iter()
                    .map(|v| (v * i16::MAX as f32) as i16)
                    .collect(),
                _ => return Err("Unsupported format!".into()),
            };

            let compressed = match self.config.precision {
                Precision::Max => packets,
                Precision::High => compress(packets, Precision::High),
                Precision::Medium => compress(packets, Precision::Medium),
                Precision::Low => compress(packets, Precision::Low),
                Precision::Ultralow => compress(packets, Precision::Ultralow),
            };

            samples.extend(compressed);

            packet_count += 1;

            if packet_count % 100 == 0 {
                println!("Analyzed {} packets", &packet_count);
            }

            if self.config.packets_limit > 100000 {
                // Файл может быть битым - не содержать корректного EOF
                // На этот случай нужна эта отсечка - в случае
                // беды, просто крашим луп.
                break;
            }
        }

        Ok(samples)
    }
}

/// Compresses Vec<i16> by Precision value
pub fn compress(vec: Vec<i16>, step: Precision) -> Vec<i16> {
    vec.iter().step_by(step.value()).copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonexistent_file() {
        let decoder = Decoder::new("audio.wav", DecoderConfig::default());
        let result = decoder.decode();
        assert!(result.is_err());
    }

    #[test]
    fn wrong_format() {
        let decoder = Decoder::new("audio.wav", DecoderConfig::default());
        let result = decoder.decode();
        assert!(result.is_err());
    }

    #[test]
    fn correct_case() {
        let decoder = Decoder::new("audio.mp3", DecoderConfig::default());
        let result = decoder.decode();
        assert!(result.is_ok());
    }
}
