use std::mem::take;

use anyhow::Context;

use crate::{Subtitle, Subtitles, decoders::Decoder};

pub struct Srt {}

impl Srt {
    fn parse_time(time_str: &str) -> anyhow::Result<f64> {
        let mut parts = time_str.split(':');
        let hours: f64 = parts
            .next()
            .with_context(|| format!("Invalid time format (expected HH:MM:SS,sss) in {time_str}"))?
            .parse()
            .with_context(|| format!("Failed to parse hours in {time_str}"))?;
        let minutes: f64 = parts
            .next()
            .with_context(|| format!("Invalid time format (expected HH:MM:SS,sss) in {time_str}"))?
            .parse()
            .with_context(|| format!("Failed to parse minutes in {time_str}"))?;
        let sec_subsec = parts.next().with_context(|| {
            format!("Invalid time format (expected HH:MM:SS,sss) in {time_str}")
        })?;
        let None = parts.next() else {
            return Err(anyhow::Error::msg(format!(
                "Invalid time format (expected HH:MM:SS,sss) in {time_str}"
            )));
        };
        let seconds: f64 = sec_subsec
            .replace(',', ".")
            .parse()
            .context("Failed to parse seconds")?;
        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    }
}

impl Decoder for Srt {
    fn load_from_bytes(bytes: Vec<u8>) -> anyhow::Result<crate::Subtitles> {
        // Remove UTF-8 BOM (byte order mark)
        let bytes = if bytes.starts_with(b"\xef\xbb\xbf") {
            &bytes[3..]
        } else {
            &bytes
        };
        let mut subtitles = Vec::new();
        let mut append_sub = |span: &[u8], text: String| -> anyhow::Result<()> {
            let mut parts = str::from_utf8(span)
                .context("Line is not valid UTF-8 string")?
                .split("-->");
            let start_str = parts.next().context("Missing start time")?.trim();
            let end_str = parts.next().context("Missing end time")?.trim();
            subtitles.push(Subtitle {
                beg: Self::parse_time(start_str)?,
                end: Self::parse_time(end_str)?,
                text,
            });
            Ok(())
        };
        let mut num = None;
        let mut span = None;
        let mut span_line_num = 0;
        let mut text = String::new();
        for (line_num, line) in bytes.split(|&b| b == b'\n').enumerate() {
            let line = line.trim_ascii();
            if line.is_empty() {
                if let Some(span) = span
                    && !text.is_empty()
                {
                    append_sub(span, take(&mut text))
                        .with_context(|| format!("At line {span_line_num}"))?;
                }
                num = None;
                span = None;
                text.clear();
            } else if num.is_none() {
                num = Some(line);
            } else if span.is_none() {
                span = Some(line);
                span_line_num = line_num + 1;
            } else {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(str::from_utf8(line).with_context(|| {
                    format!(
                        "Failed to parse subtitle text line {} as UTF-8: {line:?}",
                        line_num + 1
                    )
                })?);
            }
        }
        if let Some(span) = span
            && !text.is_empty()
        {
            append_sub(span, text).with_context(|| format!("At line {span_line_num}"))?;
        }
        Ok(Subtitles(subtitles))
    }
}
