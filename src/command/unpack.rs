use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use binrw::BinRead;
use binrw::io::BufReader;

use crate::{crypto, NewArgs};
use crate::read_ext::ReadExt;
use crate::command::ArtHeader;
use crate::unity::AssetsFile;

pub fn unpack(args: &NewArgs, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let extension = input.extension();
    match extension {
        Some(ext) => {
            if ext == OsStr::new("dat") {
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
        .map_err(|e| anyhow::anyhow!("Failed to read input file: {}", e))?;
    println!("Unpacking assets from: {}", input.display());

    // key can be unwrapped safely here
    let key = args.art_key.clone().unwrap();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key = enc_key.as_slice();
    crypto::decrypt(enc_key, data.as_mut_slice());

    // Read header string
    let len = u16::from_le_bytes([data[0], data[1]]) as usize;
    let header = String::from_utf8(data[2..len + 2].to_vec())
        .map_err(|e| anyhow::anyhow!("Failed to read header string: {}", e))?;

    let assets = haxeformat::from_str::<ArtHeader>(header.as_str())
        .map_err(|e| anyhow::anyhow!("Failed to parse header string: {}", e))?;

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
            .map_err(|e| anyhow::anyhow!("Failed to write asset {} to file: {}", asset.name, e))?;
    }

    Ok(())
}

pub fn unpack_assets(args: &NewArgs, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let input = File::open(input)
        .map_err(|e| anyhow::anyhow!("Failed to open input file: {}", e))?;
    let mut input = BufReader::new(input);
    let assets = AssetsFile::read(&mut input)
        .map_err(|e| anyhow::anyhow!("Failed to read assets file: {}", e))?;
    let objects = assets.resolve_object_classes()
        .map_err(|e| anyhow::anyhow!("Failed to resolve object classes: {}", e))?;

    let mut art_file: Option<PathBuf> = None;
    for obj in objects {
        if obj.class_id == 49 { // text asset
            input.seek(SeekFrom::Start(assets.header.offset_first_file + obj.byte_start))
                .map_err(|e| anyhow::anyhow!("Failed to seek to object: {}", e))?;
            let name = input.read_dyn_string(&assets.header.endianness, i32::BITS)
                .map_err(|e| anyhow::anyhow!("Failed to read object name: {}", e))?;

            if name == "Art.dat" {
                let temp = PathBuf::from("./temp-art.dat");
                println!("Found Art.dat in unity assets. Temporarily saving to: {}", temp.display());

                let temp_writer = File::create(&temp)
                    .map_err(|e| anyhow::anyhow!("Failed to create temporary file: {}", e))?;
                let mut temp_writer = BufWriter::new(temp_writer);

                // skip 5 unknown bytes
                input.seek(SeekFrom::Current(5))
                    .map_err(|e| anyhow::anyhow!("Failed to seek to object data: {}", e))?;
                let to_copy = obj.byte_size - (u32::BITS / 8 + name.len() as u32 + 5);
                let mut temp_reader = input.by_ref().take(to_copy as u64);

                std::io::copy(&mut temp_reader, &mut temp_writer)
                    .map_err(|e| anyhow::anyhow!("Failed to copy object data: {}", e))?;
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