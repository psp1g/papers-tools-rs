use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use anyhow::Context;
use binrw::io::BufReader;
use zip::write::FileOptions;
use zip::ZipArchive;

pub fn patch_locale(patched: &PathBuf, game_dir: &PathBuf) -> anyhow::Result<()> {
    let patched = patched.join("assets");
    println!("Patching en.zip...");

    let input = BufReader::new(File::open(game_dir.join("StreamingAssets/loc/en.zip-bak"))
        .context("Failed to open en.zip-bak")?
    );
    let output = BufWriter::new(File::create(game_dir.join("StreamingAssets/loc/en.zip"))
        .context("Failed to create en.zip")?
    );

    let mut zip = ZipArchive::new(input)
        .context("Failed to read en.zip-bak")?;
    let mut writer = zip::ZipWriter::new(output);

    for i in 0..zip.len() {
        let entry = zip.by_index(i).context("Failed to read entry")?;
        let name = entry.name();
        let patch_file = patched.join(&name[1..]); // Remove leading slash

        if patch_file.exists() {
            // General assets, just copy the patched file
            writer.start_file(name, FileOptions::default()).context("Failed to start file")?;
            let mut patch_file = BufReader::new(File::open(patch_file)
                .context("Failed to open patch file")?
            );
            std::io::copy(&mut patch_file, &mut writer).context("Failed to copy patch file")?;
        } else {
            // Locale specific assets, just copy the original file
            writer.raw_copy_file(entry).context("Failed to copy entry")?;
        }
    }

    writer.finish().context("Failed to finish writing")?;

    println!("Patched en.zip locale with {} entries", zip.len());

    Ok(())
}