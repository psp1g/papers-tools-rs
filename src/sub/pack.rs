use std::ffi::OsStr;
use std::fs::File;
use std::path::PathBuf;

use walkdir::WalkDir;
use zip::ZipArchive;

use crate::{crypto, I18nCompatMode, NewArgs};
use crate::sub::AssetMetadata;

pub fn pack(args: &NewArgs, input: &Option<PathBuf>, output: &PathBuf, locale_mode: &I18nCompatMode) -> anyhow::Result<()> {
    let input = find_input(input);
    if let Err(e) = input {
        anyhow::bail!("Error while finding input: {}", e);
    }

    let extension = output.extension();
    match extension {
        Some(ext) => {
            if ext == OsStr::new("dat") || ext == OsStr::new("txt") {
                pack_dat(args, &input.unwrap(), output, locale_mode)
            } else {
                anyhow::bail!("Output file has an invalid extension. (Use .dat or .txt)");
            }
        }
        None => {
            anyhow::bail!("Output file has no extension. (Use .dat or .assets)");
        }
    }
}

fn find_input(input: &Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match input {
        // Check if an input path was provided
        Some(path) => {
            if !path.is_dir() {
                anyhow::bail!("Input path is not a directory");
            }
            // Is the given path already a direct path to the "assets" directory?
            if path.file_name() == Some(OsStr::new("assets")) {
                return Ok(path.clone());
            }
            // Check if the "assets" directory is a subdirectory of the given path
            let assets = path.join("assets");
            if assets.is_dir() {
                return Ok(assets);
            }
            anyhow::bail!("Input path does not contain an 'assets' directory (or is too nested)");
        }
        None => {
            // Check if the current directory contains an "assets" directory
            let assets = std::env::current_dir()?.join("assets");
            if assets.is_dir() {
                return Ok(assets);
            }
            // Check if an "out" directory exists in the current directory
            let out = std::env::current_dir()?.join("out").join("assets");
            if out.is_dir() {
                return Ok(out);
            }
            anyhow::bail!("Current directory does not contain an 'assets' or 'out/assets' directory. Please provide an input path.");
        }
    }
}

fn pack_dat(args: &NewArgs, input: &PathBuf, output: &PathBuf, locale_mode: &I18nCompatMode) -> anyhow::Result<()> {
    let mut assets: Vec<AssetMetadata> = Vec::new();
    let mut asset_bytes: Vec<u8> = Vec::new();

    // let localized_assets = match locale_mode {
    //     I18nCompatMode::None => Vec::new(),
    //     _ => find_localized_assets(&args.game).unwrap_or_else(|e| {
    //         eprintln!("Error while finding localized assets: {}. Disabling localization mode.", e);
    //         Vec::new()
    //     }),
    // };

    for file in WalkDir::new(input) {
        let file = file.unwrap();
        if file.file_type().is_dir() {
            continue;
        }

        let path = file.path();
        let name = path.strip_prefix(input)?.to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
            .to_string();
        let size = path.metadata()?.len() as usize;

        println!("Packing asset: {} ({} bytes)", name, size);
        assets.push(AssetMetadata { name, size });

        asset_bytes.extend_from_slice(&std::fs::read(path)?);
    }

    let header = haxeformat::to_string(&assets)?;
    let mut header = header.into_bytes();
    let mut out = Vec::new();
    out.extend_from_slice((header.len() as u16).to_le_bytes().as_ref());
    out.append(&mut header);
    out.append(&mut asset_bytes);

    println!("Encrypting assets...");
    let key = args.art_key.clone().unwrap();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key = enc_key.as_slice();
    crypto::encrypt(enc_key, out.as_mut_slice());

    println!("Packing assets to: {}...", output.display());
    std::fs::write(output, out)?;

    Ok(())
}

fn find_localized_assets(game_dir: &PathBuf) -> anyhow::Result<Vec<String>> {
    let english_loc = game_dir
        .join("PapersPlease_Data")
        .join("StreamingAssets")
        .join("loc")
        .join("en.zip");

    if !english_loc.exists() {
        anyhow::bail!("English localization not found at: {}", english_loc.display());
    }
    let mut assets: Vec<String> = Vec::new();
    let zip_handle = File::open(english_loc)?;
    let mut zip = ZipArchive::new(zip_handle)?;
    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        if file.is_dir() {
            continue;
        }

        // TODO: bail or skip?
        assets.push(format!("assets/{}", file
            .enclosed_name().ok_or_else(|| anyhow::anyhow!("Failed to get zip entry name"))?
            .to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert file name to string"))?
        ));
    }

    Ok(assets)
}
