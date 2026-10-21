use log::LevelFilter;

mod decoders;
mod encoders;

#[derive(Debug)]
struct Subtitles(Vec<Subtitle>);

#[derive(Debug)]
struct Subtitle {
    beg: f64,
    end: f64,
    text: String,
}

pub mod cli {
    use std::path::PathBuf;

    use clap::{Parser, Subcommand};

    #[derive(Parser, Debug)]
    #[command(
        name = "subtitles-solver",
        version,
        about = "Synchronize subtitles to match times of another subtitles with math and algorithms"
    )]
    pub struct Args {
        /// Print debug logs
        #[arg(short, long)]
        pub debug: bool,
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Synchronize subtitles to another subtitles
        Sync {
            /// Subtitles file to synchronize with
            #[arg(long)]
            to: PathBuf,
            /// Subtitles that will be synchronized
            input: PathBuf,
            /// Output subtitle file
            out: PathBuf,
        },
    }
}

fn init_logging(debug: bool) {
    let mut logger = pretty_env_logger::formatted_builder();
    logger.parse_default_env();
    if debug {
        logger.filter_level(LevelFilter::Debug);
    }
    logger.init();
}

fn main() -> anyhow::Result<()> {
    let args = <cli::Args as clap::Parser>::parse();
    init_logging(args.debug);

    let cli::Commands::Sync { to, input, out } = args.command;
    let to_subtitles = decoders::decode_subtitles_file(to)?;
    let input_subtitles = decoders::decode_subtitles_file(input)?;
    dbg!(&input_subtitles);
    dbg!(&to_subtitles);
    encoders::encode_subtitles_to_file(&input_subtitles, out)?;

    Ok(())
}
