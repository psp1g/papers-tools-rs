use std::path::PathBuf;

use clap::Parser;
use clap_derive::{Parser, Subcommand, ValueEnum};
use tracing::{error, info};
use tracing_subscriber::fmt::time::OffsetTime;
use crate::command::{pack, patch, revert, unpack};

mod crypto;
mod command;
mod unity;

#[derive(Debug, Parser)]
#[command(version, about = "Cli tool to work with Papers, Please data files", long_about = None)]
struct Args {
    /// Subcommand to run
    #[command(subcommand)]
    command: Command,

    /// Path to the Papers, Please game directory
    #[arg(short, long)]
    game_dir: PathBuf,

    /// Optional encryption key to use for Art.dat. If none is provided it will be extracted from the global-metadata.dat file.
    #[arg(short, long)]
    art_key: Option<String>,

}

#[derive(Debug, Subcommand)]
enum Command {
    /// Pack assets into an Art.dat (For asset bundles, use the patch command)
    Pack {
        /// Input file. If none is provided, the tool will check for an "assets" and "out" directory in the current working directory.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output file. Make sure to use the .dat or .txt extension.
        #[arg(short, long, default_value = "Art-modded.dat")]
        output: PathBuf,
    },
    /// Unpack assets from an Art.dat or unity asset bundle.
    Unpack {
        /// Input file. Can either be an Art.dat file or a unity asset bundle. Make sure to either use the .dat or .assets extension. Defaults to the sharedassets0.assets in the game directory.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output directory.
        #[arg(short, long, default_value = "./out")]
        output: PathBuf,
    },
    /// Patch the game files with new/replaced assets from a directory.
    Patch {
        /// Directory containing assets to insert/replace.
        #[arg(short, long, default_value = "./patch")]
        patch: PathBuf,

        /// How should the tool handle localized assets.
        #[arg(long, default_value = "normal")]
        i18n: I18nCompatMode,
    },
    /// Reverts the game files to their original state.
    Revert,
}

impl Command {
    fn needs_key(&self) -> bool {
        match self {
            Command::Revert => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, ValueEnum)]
enum I18nCompatMode {
    /// Everything is packed into the Art.dat file. Localized assets are ignored.
    None,
    /// the en.zip i18n file is also processed to have the same localized assets as Art.dat.
    Normal,
}


fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_timer(OffsetTime::new(
            time::UtcOffset::current_local_offset().unwrap_or_else(|_| time::UtcOffset::UTC),
            time::format_description::parse("[hour]:[minute]:[second]").unwrap(),
        ))
        .init();
    let mut args = Args::parse();
    info!("papers-tools v{} by {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"));
    if args.art_key.is_none() && args.command.needs_key() {
        let res = crypto::extract_key(&args);
        if let Err(err) = res {
            error!("Failed to extract key: {}", err);
            return;
        }
        args.art_key = Some(res.unwrap());
    }

    let res = match &args.command {
        Command::Pack { input, output } => {
            // unwrap is safe here
            pack::pack(&args.art_key.unwrap(), input, output)
        }
        Command::Unpack { input, output } => {
            unpack::unpack(&args, input, output)
        }
        Command::Patch { patch, i18n } => {
            patch::patch(&args, patch, i18n)
        }
        Command::Revert => {
            revert::revert(&args.game_dir)
        }
    };

    if let Err(err) = res {
        error!("An error occurred while running the command:");
        error!("{err}");
    }
}