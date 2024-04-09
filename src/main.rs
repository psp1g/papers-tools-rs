use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use clap::Parser;
use clap_derive::Parser;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::read_ext::ReadExt;

mod crypto;
mod read_ext;

const KEY_OFFSET: usize = 0x39420;

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Asset {
    name: String,
    size: usize,
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value = "Art.dat")]
    input: String,

    #[arg(short, long)]
    game: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(short, long)]
    key: Option<String>,
}

fn main() {
    let mut args = Args::parse();
    let file = Path::new(args.input.as_str());

    if args.key.is_none() {
        args.key = Some(extract_key(&args).expect("Failed to extract key from game metadata"));
    }

    if file.is_file() {
        extract(args);
    } else {
        pack(args);
    }
}

fn extract_key(args: &Args) -> Option<String> {
    let game_dir = Path::new(&args.game);
    if !game_dir.exists() || !game_dir.is_dir() {
        eprintln!("Game directory does not exist: {}", game_dir.display());
        return None;
    }
    let global_metadata = game_dir.join("PapersPlease_Data")
        .join("il2cpp_data")
        .join("Metadata")
        .join("global-metadata.dat");
    
    if !global_metadata.exists() {
        eprintln!("Global metadata file does not exist: {}", global_metadata.display());
        return None;
    }

    let mut file = File::open(global_metadata)
        .expect("Failed to open global metadata file");
    file.seek(std::io::SeekFrom::Start(KEY_OFFSET as u64))
        .expect("Failed to seek to key offset");
    let mut key = [0; 16];
    file.read_exact(&mut key)
        .expect("Failed to read key from file");
    let key = String::from_utf8(key.to_vec()).expect("Failed to convert key to string");
    println!("Extracted decryption key from global metadata: {}", key);
    Some(key)
}

fn extract(args: Args) {
    let mut data = std::fs::read(&args.input)
        .expect("Failed to read input file");
    println!("Extracting assets from: {}", args.input);

    let key = args.key.unwrap();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key = enc_key.as_slice();
    crypto::decrypt(enc_key, data.as_mut_slice());

    let mut cursor = Cursor::new(data);
    let str = cursor.read_string();

    let assets = haxeformat::from_str::<Vec<Asset>>(str.as_str())
        .expect("Failed to parse assets");

    let output = args.output.unwrap_or("./out".to_string());
    std::fs::create_dir_all(&output)
        .expect("Failed to create output directory");
    let abs_output = Path::new(output.as_str()).canonicalize()
        .expect("Failed to canonicalize output path");
    for asset in assets {
        println!("Extracting asset: {} ({} bytes)", asset.name, asset.size);
        let mut asset_bytes = vec![0; asset.size];
        cursor.read_exact(asset_bytes.as_mut_slice())
            .expect("Failed to read asset bytes");

        let path = abs_output.join(&asset.name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
            if !parent.canonicalize().unwrap().starts_with(&abs_output) {
                eprintln!("Skipping asset: {} (Tried escaping output directory)", asset.name);
                continue;
            }
        }
        std::fs::write(path, asset_bytes)
            .expect("Failed to write asset to file");
    }
}

fn pack(args: Args) {
    let input = Path::new(&args.input);
    let output = args.output.unwrap_or("Art-modded.dat".to_string());
    println!("Packing assets from: {} to: {}", input.display(), output);

    let mut assets: Vec<Asset> = Vec::new();
    let mut asset_bytes: Vec<u8> = Vec::new();
    for file in WalkDir::new(input) {
        let file = file.unwrap();
        if file.file_type().is_dir() {
            continue;
        }

        let path = file.path();
        let name = path.strip_prefix(input)
            .expect("Failed to strip prefix")
            .to_str()
            .expect("Failed to convert path to string")
            .to_string();
        let size = path.metadata()
            .expect("Failed to get file metadata")
            .len() as usize;
        println!("Packing asset: {} ({} bytes)", name, size);
        assets.push(Asset { name, size });

        asset_bytes.extend_from_slice(&std::fs::read(path)
            .expect("Failed to read file"));
    }

    let header = haxeformat::to_string(&assets)
        .expect("Failed to serialize assets");
    let mut header = header.into_bytes();
    let mut out = Vec::new();
    out.extend_from_slice((header.len() as u16).to_le_bytes().as_ref());
    out.append(&mut header);
    out.append(&mut asset_bytes);

    println!("Encrypting assets...");
    let key = args.key.unwrap();
    let enc_key = crypto::to_key_array(key.as_str());
    let enc_key = enc_key.as_slice();
    crypto::encrypt(enc_key, out.as_mut_slice());

    println!("Packing assets to: {}...", output);
    std::fs::write(output, out)
        .expect("Failed to write output file");
    println!("Done! You can now use a tool like UABE to replace the Art.dat file in sharedassets0.assets")
}