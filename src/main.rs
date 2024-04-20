use std::path::PathBuf;

use clap::Parser;
use clap_derive::{Parser, Subcommand, ValueEnum};

use crate::command::{pack, patch, unpack};

mod crypto;
mod read_ext;
mod command;
mod unity;

#[derive(Debug, Parser)]
#[command(version, about = "Cli tool to work with Paper, please data files", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[arg(short, long, default_value = "Art.dat")]
    input: String,

    #[arg(short, long)]
    game: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(short, long)]
    key: Option<String>,
}

#[derive(Debug, Parser)]
#[command(version, about = "Cli tool to work with Papers, Please data files", long_about = None)]
struct NewArgs {
    /// Subcommand to run
    #[command(subcommand)]
    command: Command,

    /// Path to the Papers, Please game directory
    #[arg(short, long)]
    game: PathBuf,

    /// Optional encryption key to use for Art.dat. If none is provided it will be extracted from the global-metadata.dat file.
    #[arg(short, long)]
    art_key: Option<String>,

}

#[derive(Debug, Subcommand)]
enum Command {
    /// Pack assets into an Art.dat (For asset bundles, use the patch command)
    Pack {
        /// Input file. If none is provided, the tool will check for a "assets" and "out" directory in the current working directory.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output file. Make sure to use the .dat or .txt extension.
        #[arg(short, long, default_value = "Art-modded.dat")]
        output: PathBuf,

        /// How should the tool handle localized assets.
        #[arg(long, default_value = "None")]
        i18n: I18nCompatMode,
    },
    /// Unpack assets from an Art.dat or unity asset bundle.
    Unpack {
        /// Input file. Can either be a Art.dat file or a unity asset bundle. Make sure to either use the .dat or .assets extension.
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory.
        #[arg(short, long, default_value = "./out")]
        output: PathBuf,
    },
    /// Patch an Art.dat or unity asset bundle with new/replaced assets from a directory.
    Patch {
        /// Directory containing assets to insert/replace.
        #[arg(short, long)]
        patch: PathBuf,

        /// How should the tool handle localized assets.
        #[arg(long, default_value = "None")]
        i18n: I18nCompatMode,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum I18nCompatMode {
    /// Everything is packed into the Art.dat file. Localized assets are ignored.
    None,
    /// All i18n zip files get the same localized assets.
    Normal,
    /// The tool finds the delta between localized assets and overlays them onto the packed assets.
    Smart,
}


fn main() {
    let mut args = NewArgs::parse();
    println!("papers-tools v{} by {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"));
    if args.art_key.is_none() {
        let res = crypto::extract_key(&args);
        if let Err(err) = res {
            eprintln!("Failed to extract key: {}", err);
            return;
        }
        args.art_key = Some(res.unwrap());
    }

    let res = match &args.command {
        Command::Pack { input, output, i18n } => {
            pack::pack(&args, input, output, i18n)
        }
        Command::Unpack { input, output } => {
            unpack::unpack(&args, input, output)
        }
        Command::Patch { patch, i18n } => {
            patch::patch(&args, patch, i18n)
        }
    };

    if let Err(err) = res {
        eprintln!("An error occurred while running the command:");
        eprintln!("{err}");
    }
}