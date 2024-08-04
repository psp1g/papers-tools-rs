use std::fs::File;
use std::io::{Read, Seek};
use std::slice;
use tracing::info;
use crate::command::DATA_FOLDER_NAME;
use crate::Args;

const KEY_OFFSET: usize = 0x39420;

pub fn to_key_array(key: &str) -> Vec<u32> {
    md5::compute(key)
        .0
        .chunks(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
        .collect()
}

// Below implementation is a very slightly modified version of
// https://github.com/mgottschlag/xxtea-nostd
// Modified to accept a u32 slice key directly

pub fn encrypt(key: &[u32], block: &mut [u8]) {
    let block = as_u32_slice_mut(block);

    let rounds = 6 + 52 / block.len();
    let n = block.len() - 1;

    let mut sum = 0u32;
    let mut z = block[n]; // left neighbour for the first round
    for _ in 0..rounds {
        // cycle
        sum = sum.wrapping_add(0x9e3779b9);
        let e = sum >> 2;
        for r in 0..block.len() {
            // round
            let y = block[(r + 1) % block.len()]; // right neighbour
            block[r] = block[r].wrapping_add(
                (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                    ^ ((sum ^ y).wrapping_add(key[(r ^ e as usize) & 3] ^ z)),
            );
            z = block[r]; // left neighbour for the next round
        }
    }
}

pub fn decrypt(key: &[u32], block: &mut [u8]) {
    let block = as_u32_slice_mut(block);

    let rounds = 6 + 52 / block.len();

    let mut sum = (rounds as u32).wrapping_mul(0x9e3779b9);
    let mut y = block[0];
    for _ in 0..rounds {
        // cycle
        let e = sum >> 2;
        for r in (0..block.len()).rev() {
            // round
            let z = block[(r + block.len() - 1) % block.len()];
            block[r] = block[r].wrapping_sub(
                (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                    ^ ((sum ^ y).wrapping_add(key[(r ^ e as usize) & 3] ^ z)),
            );
            y = block[r];
        }
        sum = sum.wrapping_sub(0x9e3779b9);
    }
}

fn as_u32_slice_mut(x: &mut [u8]) -> &mut [u32] {
    unsafe { slice::from_raw_parts_mut(x.as_mut_ptr() as *mut u32, x.len() / 4) }
}

pub fn extract_key(args: &Args) -> anyhow::Result<String> {
    let mut game_dir = args.game_dir.clone();
    if !game_dir.exists() || !game_dir.is_dir() {
        anyhow::bail!("Game directory not found: {}", game_dir.display());
    }

    if !game_dir.ends_with(DATA_FOLDER_NAME) {
        game_dir.push(DATA_FOLDER_NAME);
    }

    let global_metadata = game_dir
        .join("il2cpp_data")
        .join("Metadata")
        .join("global-metadata.dat");

    if !global_metadata.exists() {
        anyhow::bail!("Global metadata file not found: {}", global_metadata.display());
    }

    let mut file = File::open(global_metadata)?;
    file.seek(std::io::SeekFrom::Start(KEY_OFFSET as u64))?;
    let mut key = [0; 16];
    file.read_exact(&mut key)?;
    let key = String::from_utf8(key.to_vec())?;
    info!("Extracted Art.dat decryption key from global metadata");

    Ok(key)
}