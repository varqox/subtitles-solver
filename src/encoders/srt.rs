use std::io::Write;

use crate::{Subtitle, Subtitles, encoders::Encoder};

pub struct Srt {}

impl Encoder for Srt {
    fn encode_to_writer(subtitles: &Subtitles, writer: &mut impl Write) -> anyhow::Result<()> {
        let format_time = |time: f64| {
            let mut millis = (time * 1000.0).round() as u64;
            let hours = millis / 3600000;
            millis %= 3600000;
            let minutes = millis / 60000;
            millis %= 60000;
            let seconds = millis / 1000;
            millis %= 1000;
            format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
        };
        for (idx, Subtitle { beg, end, text }) in subtitles.0.iter().enumerate() {
            write!(
                writer,
                "{}\n{} --> {}\n{text}\n\n",
                idx + 1,
                format_time(*beg),
                format_time(*end)
            )?;
        }
        Ok(())
    }
}
