use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::Context;
use binrw::BinRead;
use binrw::io::BufReader;

use crate::{crypto, Args, unity};
use crate::command::{ArtHeader, DATA_FOLDER_NAME};
use crate::unity::AssetsFile;
use crate::unity::audio::AudioClip;
use crate::unity::util::{AlignedString, AlignmentArgs};

pub fn unpack(args: &Args, input: &Option<PathBuf>, output: &PathBuf) -> anyhow::Result<()> {
    let input = &find_input(args, input)?;
    let extension = input.extension();
    match extension {
        Some(ext) => {
            if ext == OsStr::new("dat") || ext == OsStr::new("txt") {
                unpack_dat(args, input, output)?;
            } else if ext == OsStr::new("assets") {
                unpack_assets(args, input, output, false)?;
            } else {
                anyhow::bail!("Input file has an invalid extension. (Supported: .dat, .assets)");
            }
        }
        None => {
            anyhow::bail!("Input file has no extension. (Supported: .dat, .assets)");
        }
    }

    Ok(())
}

fn find_input(args: &Args, input: &Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match input {
        // Check if an input path was provided
        Some(path) => {
            if !path.is_file() {
                anyhow::bail!("Input path is not a file");
            }
            Ok(path.clone())
        }
        None => {
            let mut assets = args.game_dir.clone();
            if !assets.ends_with(DATA_FOLDER_NAME) {
                assets.push(DATA_FOLDER_NAME);
            }
            assets.push("sharedassets0.assets");

            if assets.is_file() {
                Ok(assets)
            } else {
                anyhow::bail!("No input file provided and no sharedassets0.assets file found in game directory");
            }
        }
    }
}

pub fn unpack_dat(args: &Args, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let mut data = std::fs::read(input)
        .context("Failed to read input file")?;
    println!("Unpacking assets from: {}", input.display());

    // key can be unwrapped safely here
    let key = args.art_key.clone().unwrap();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key_slice = enc_key.as_slice();
    crypto::decrypt(enc_key_slice, data.as_mut_slice());

    // Read header string
    let len = u16::from_le_bytes([data[0], data[1]]) as usize;
    let header = String::from_utf8(data[2..len + 2].to_vec())
        .context("Failed to read header string")?;

    let assets = haxeformat::from_str::<ArtHeader>(header.as_str())
        .context("Failed to parse header string")?;

    // Create output directory
    std::fs::create_dir_all(&output)?;
    let abs_output = Path::new(output).canonicalize()?;

    // Loop through assets in the data and write them to the output directory
    let mut index = len + 2;
    for asset in &assets {
        let asset_bytes = &data[index..index + asset.size];
        index += asset.size;

        let path = abs_output.join(&asset.name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            if !parent.canonicalize()?.starts_with(&abs_output) {
                eprintln!("Skipping asset: {} (Tried escaping output directory)", asset.name);
                continue;
            }
        }

        std::fs::write(path, asset_bytes)
            .context(format!("Failed to write asset {} to file", asset.name))?;
    }

    println!("Unpacked {} assets", assets.len());

    Ok(())
}

pub struct RepackInfo {
    pub assets: AssetsFile,
    pub audio_assets: HashMap<i64, AudioClip>,
    pub art_key: String,
    pub art_path_id: i64,
    pub original_assets: PathBuf,
}

pub fn unpack_assets(args: &Args, input_path: &PathBuf, output: &PathBuf, process_audio: bool) -> anyhow::Result<RepackInfo> {
    let input = File::open(input_path)
        .context("Failed to open input file")?;
    let mut input = BufReader::new(input);
    let assets = AssetsFile::read(&mut input)
        .context("Failed to read assets file")?;
    let objects = assets.resolve_object_classes()
        .context("Failed to resolve object classes")?;

    let mut art_file: Option<PathBuf> = None;
    let mut art_path_id: Option<i64> = None;
    let mut audio_assets = HashMap::new();
    for obj in objects {
        if obj.class_id == unity::TEXT_ASSET_CLASS { // text asset
            input.seek(SeekFrom::Start(assets.header.offset_first_file + obj.byte_start))
                .context("Failed to seek to object")?;
            let name = AlignedString::read_options(&mut input, assets.endian(), AlignmentArgs::new(4))
                .context("Failed to read object name")?.0;

            if name == "Art.dat" {
                let temp = PathBuf::from("./temp-art.dat");
                println!("Found Art.dat in unity assets. Temporarily saving to: {}", temp.display());

                let temp_writer = File::create(&temp)
                    .context("Failed to create temporary file")?;
                let mut temp_writer = BufWriter::new(temp_writer);

                let to_copy = u32::read_options(&mut input, assets.endian(), ())
                    .context("Failed to read asset length")?;
                let mut temp_reader = input.by_ref().take(to_copy as u64);

                std::io::copy(&mut temp_reader, &mut temp_writer)
                    .context("Failed to copy object data")?;

                art_file = Some(temp);
                art_path_id = Some(obj.path_id);
                if !process_audio {
                    break;
                }
            }
        } else if process_audio && obj.class_id == unity::AUDIO_CLIP_CLASS {
            input.seek(SeekFrom::Start(assets.header.offset_first_file + obj.byte_start))
                .context("Failed to seek to object")?;

            let audio_clip = AudioClip::read_options(&mut input, assets.endian(), ())
                .context("Failed to read AudioClip object")?;
            audio_assets.insert(obj.path_id, audio_clip);
        }
    }

    if let Some(art_file) = art_file {
        unpack_dat(args, &art_file, output)?;
        println!("Removing temporary file: {}", art_file.display());
        if let Err(e) = std::fs::remove_file(art_file) {
            eprintln!("Failed to remove temporary file: {}", e);
        }
        // Any unwraps here are safe because None values would've resulted in earlier bail
        Ok(RepackInfo {
            assets,
            audio_assets,
            art_path_id: art_path_id.unwrap(),
            art_key: args.art_key.clone().unwrap(),
            original_assets: input_path.clone(),
        })
    } else {
        anyhow::bail!("Failed to find Art.dat object in assets file");
    }
}