use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Context;
use walkdir::WalkDir;

use crate::command::AssetMetadata;
use crate::crypto;

pub fn pack(art_key: &String, input: &Option<PathBuf>, output: &PathBuf) -> anyhow::Result<()> {
    let input = find_input(input);
    if let Err(e) = input {
        anyhow::bail!("Error while finding input: {}", e);
    }

    let extension = output.extension();
    match extension {
        Some(ext) => {
            if ext == OsStr::new("dat") || ext == OsStr::new("txt") {
                pack_dat(art_key, &input.unwrap(), output)
            } else {
                anyhow::bail!("Output file has an invalid extension. (Use .dat or .txt)");
            }
        }
        None => {
            anyhow::bail!("Output file has no extension. (Use .dat or .txt)");
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

fn pack_dat(art_key: &String, input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    println!("Packing assets...");
    let mut assets: Vec<AssetMetadata> = Vec::new();
    let mut asset_bytes: Vec<u8> = Vec::new();
    let mut count = 0;

    for file in WalkDir::new(input) {
        let file = file.unwrap();
        if file.file_type().is_dir() {
            continue;
        }

        let path = file.path();
        let mut name = path.strip_prefix(input)?.to_str()
            .context("Failed to convert path to string")?
            .to_string();
        let size = path.metadata()?.len() as usize;
        if !name.starts_with("assets/") {
            name = format!("assets/{}", name.replace("\\", "/"));
        }

        assets.push(AssetMetadata { name, size });

        asset_bytes.extend_from_slice(&std::fs::read(path)?);
        count += 1;
    }

    let header = haxeformat::to_string(&assets)?;
    let mut header = header.into_bytes();
    let mut out = Vec::new();
    out.extend_from_slice((header.len() as u16).to_le_bytes().as_ref());
    out.append(&mut header);
    out.append(&mut asset_bytes);

    println!("Encrypting assets...");
    let key = art_key.clone();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key = enc_key.as_slice();
    crypto::encrypt(enc_key, out.as_mut_slice());

    std::fs::write(output, out)?;
    println!("Packed {} assets", count);

    Ok(())
}
