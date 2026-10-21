use std::{fs::File, io::Read, path::Path};

use anyhow::Context;

use crate::Subtitles;

pub mod srt;
pub mod sub;

pub trait Decoder {
    fn load_from(path: impl AsRef<Path>) -> anyhow::Result<Subtitles> {
        Self::load_from_reader(
            File::open(&path)
                .with_context(|| format!("Failed to load subtitles from {:?}", path.as_ref()))?,
        )
    }

    fn load_from_reader(mut reader: impl Read) -> anyhow::Result<Subtitles> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Self::load_from_bytes(buf)
    }

    fn load_from_bytes(bytes: Vec<u8>) -> anyhow::Result<Subtitles>;
}

pub fn decode_subtitles_file(path: impl AsRef<Path>) -> anyhow::Result<Subtitles> {
    let path = path.as_ref();
    match path.extension() {
        None => Err(anyhow::Error::msg(format!(
            "path {path:?} misses extension"
        ))),
        Some(ext) if ext == "srt" => srt::Srt::load_from(path),
        Some(ext) if ext == "sub" => sub::Sub::load_from(path),
        Some(ext) => Err(anyhow::Error::msg(format!(
            "path {path:?} has unsupported extension {ext:?}"
        ))),
    }
    .with_context(|| format!("Parsing subtitle file: {path:?}"))
}
