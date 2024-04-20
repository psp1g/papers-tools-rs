use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use anyhow::Context;

use binrw::BinRead;
use binrw::io::BufReader;

use crate::{crypto, NewArgs};
use crate::command::ArtHeader;
use crate::read_ext::ReadExt;
use crate::unity::AssetsFile;

pub fn unpack(args: &NewArgs, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let extension = input.extension();
    match extension {
        Some(ext) => {
            if ext == OsStr::new("dat") || ext ==  OsStr::new("txt") {
                unpack_dat(args, input, output)
            } else if ext == OsStr::new("assets") {
                unpack_assets(args, input, output)
            } else {
                anyhow::bail!("Input file has an invalid extension. (Supported: .dat, .assets)");
            }
        }
        None => {
            anyhow::bail!("Input file has no extension. (Supported: .dat, .assets)");
        }
    }
}

pub fn unpack_dat(args: &NewArgs, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let mut data = std::fs::read(input)
        .context("Failed to read input file")?;
    println!("Unpacking assets from: {}", input.display());

    // key can be unwrapped safely here
    let key = args.art_key.clone().unwrap();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key = enc_key.as_slice();
    crypto::decrypt(enc_key, data.as_mut_slice());

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
    for asset in assets {
        println!("Unpacking asset: {} ({} bytes)", asset.name, asset.size);
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

    Ok(())
}

pub fn unpack_assets(args: &NewArgs, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let input = File::open(input)
        .context("Failed to open input file")?;
    let mut input = BufReader::new(input);
    let assets = AssetsFile::read(&mut input)
        .context("Failed to read assets file")?;
    let objects = assets.resolve_object_classes()
        .context("Failed to resolve object classes")?;

    let mut art_file: Option<PathBuf> = None;
    for obj in objects {
        if obj.class_id == 49 { // text asset
            input.seek(SeekFrom::Start(assets.header.offset_first_file + obj.byte_start))
                .context("Failed to seek to object")?;
            let name = input.read_dyn_string(&assets.header.endianness, i32::BITS)
                .context("Failed to read object name")?;

            if name == "Art.dat" {
                let temp = PathBuf::from("./temp-art.dat");
                println!("Found Art.dat in unity assets. Temporarily saving to: {}", temp.display());

                let temp_writer = File::create(&temp)
                    .context("Failed to create temporary file")?;
                let mut temp_writer = BufWriter::new(temp_writer);

                // skip field index byte
                input.seek(SeekFrom::Current(1))
                    .context("Failed to seek to object data")?;
                let to_copy = input.read_u32_order(&assets.header.endianness)
                    .context("Failed to read asset length")?;
                let mut temp_reader = input.by_ref().take(to_copy as u64);

                std::io::copy(&mut temp_reader, &mut temp_writer)
                    .context("Failed to copy object data")?;
                
                art_file = Some(temp);
                break;
            }
        }
    }

    if let Some(art_file) = art_file {
        unpack_dat(args, &art_file, output)?;
        println!("Removing temporary file: {}", art_file.display());
        if let Err(e) = std::fs::remove_file(art_file) {
            eprintln!("Failed to remove temporary file: {}", e);
        }
    } else {
        anyhow::bail!("Failed to find Art.dat object in assets file");
    }

    Ok(())
}