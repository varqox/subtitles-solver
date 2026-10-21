use anyhow::Context;

use log::debug;
use regex::Regex;

use crate::{Subtitle, Subtitles, decoders::Decoder};

pub struct Sub {}

impl Decoder for Sub {
    fn load_from_bytes(bytes: Vec<u8>) -> anyhow::Result<crate::Subtitles> {
        // Remove UTF-8 BOM (byte order mark)
        let bytes = if bytes.starts_with(b"\xef\xbb\xbf") {
            &bytes[3..]
        } else {
            &bytes
        };
        let mut subtitles = Vec::new();
        let regex = Regex::new(r#"^\{(\d+)\}\{(\d+)\}(.*)$"#)?;
        let mut frame_rate: f64 = 23.976;
        for (line_num, line) in bytes.split(|&b| b == b'\n').enumerate() {
            let line = line.trim_ascii();
            if line.is_empty() {
                continue;
            }

            let line_as_str = str::from_utf8(line).with_context(|| {
                format!(
                    "Failed to parse subtitle text line {} as UTF-8: {line:?}",
                    line_num + 1
                )
            })?;
            let caps = regex.captures(line_as_str).with_context(|| {
                format!("Line {} has invalid format: {line_as_str}", line_num + 1)
            })?;
            let start_frame: u64 = caps.get(1).unwrap().as_str().parse().with_context(|| {
                format!(
                    "Failed to parse start_frame number at line {}",
                    line_num + 1
                )
            })?;
            let end_frame: u64 = caps.get(2).unwrap().as_str().parse().with_context(|| {
                format!("Failed to parse end_frame number at line {}", line_num + 1)
            })?;
            let text = caps.get(3).unwrap().as_str();
            if line_num == 0 {
                if start_frame == end_frame && start_frame <= 1 {
                    frame_rate = text
                        .parse()
                        .with_context(|| format!("Failed to parse the frame rate: {text}"))?;
                    debug!("frame_rate: {frame_rate}");
                    continue;
                }
                debug!("frame_rate: {frame_rate}");
            }

            subtitles.push(Subtitle {
                beg: start_frame as f64 / frame_rate,
                end: end_frame as f64 / frame_rate,
                text: text.replace('|', "\n"),
            });
        }
        Ok(Subtitles(subtitles))
    }
}
