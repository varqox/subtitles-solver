use std::{io::Write, path::Path};

use anyhow::Context;

use crate::Subtitles;

pub mod srt;

pub trait Encoder {
    fn encode_to_writer(subtitles: &Subtitles, writer: &mut impl Write) -> anyhow::Result<()>;

    fn encode_to_file(subtitles: &Subtitles, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let mut file = std::fs::File::create(path)
            .with_context(|| format!("Failed to create file {path:?}"))?;
        Self::encode_to_writer(subtitles, &mut file)
    }
}

pub fn encode_subtitles_to_file(
    subtitles: &Subtitles,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    match path.extension() {
        None => Err(anyhow::Error::msg(format!(
            "path {path:?} misses extension"
        ))),
        Some(ext) if ext == "srt" => srt::Srt::encode_to_file(subtitles, path),
        Some(ext) => Err(anyhow::Error::msg(format!(
            "path {path:?} has unsupported extension {ext:?}"
        ))),
    }
    .with_context(|| format!("Parsing subtitle file: {path:?}"))
}
