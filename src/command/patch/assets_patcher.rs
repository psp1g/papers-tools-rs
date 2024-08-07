use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use binrw::__private::write_zeroes;
use binrw::BinWrite;
use binrw::io::BufReader;
use tracing::info;
use walkdir::WalkDir;

use crate::command::pack;
use crate::command::patch::xml_patcher;
use crate::command::unpack::RepackInfo;
use crate::unity::{AssetsFile, AssetsFileContent, AssetsFileHeader, ObjectInfo};
use crate::unity::util::{AlignedString, AlignmentArgs};

/// Length of the header of the Art.dat object.
/// The header consists of:
/// - 4 bytes for object name length
/// - 7 bytes for object name (Art.dat)
/// - 1 byte for alignment to next 4-byte block
/// - 4 bytes for data length
const ART_OBJ_HEADER_LEN: u64 = 4 + 7 + 1 + 4;

pub fn patch_assets(
    patch: &PathBuf,
    temp_dir: &PathBuf,
    game_dir: &PathBuf,
    repack_info: RepackInfo,
) -> anyhow::Result<PathBuf> { // patched assets directory
    info!("Patching assets..");
    let patched_assets = temp_dir.join("patched");
    let unpacked = temp_dir.join("unpacked");
    std::fs::create_dir_all(&patched_assets)
        .context("Failed to create patched assets directory")?;

    // copy over original files and if they have a patch, apply the patch
    for file in WalkDir::new(&unpacked) {
        let file = file.map_err(|e| anyhow::anyhow!("Failed to walk directory: {}", e))?;
        let rel_path = file.path().strip_prefix(&unpacked)
            .context("Failed to strip prefix")?;
        let file_type = file.file_type();

        // if file is a directory, just create it in the patched assets directory
        if file_type.is_dir() {
            let target = patched_assets.join(rel_path);
            std::fs::create_dir_all(&target).context("Failed to create directory")?;
            continue;
        }

        // skip symlinks etc.
        if !file_type.is_file() {
            continue;
        }

        // check if file exists in patch directory
        let patch_file = patch.join(rel_path);
        if !patch_file.exists() { // patch file doesn't exist, so copy over the original
            copy_file(&file.path(), rel_path, &patched_assets)?;
            continue;
        }

        let ext = patch_file.extension()
            .with_context(|| format!("Failed to get extension of {}", patch_file.display()))?;

        // copy over the patch file if it's a png, csv or txt file
        // TODO: csv and txt patching
        if ext == OsStr::new("png") || ext == OsStr::new("csv") || ext == OsStr::new("txt") {
            info!("Copying patch file for: {}", rel_path.display());
            copy_file(&patch_file.as_path(), rel_path, &patched_assets)?;
        } else if ext == OsStr::new("xml") || ext == OsStr::new("fnt") {
            info!("Patching xml file: {}", rel_path.display());
            patch_xml(&file.path(), &patch_file, rel_path, &patched_assets)?;
        } else {
            anyhow::bail!("Unsupported file type: {}", patch_file.display());
        }
    }

    // Loop over any files newly added with the patch
    for file in WalkDir::new(patch) {
        let file = file.map_err(|e| anyhow::anyhow!("Failed to walk directory: {}", e))?;
        let rel_path = file.path().strip_prefix(patch)
            .context("Failed to strip prefix")?;
        if !rel_path.starts_with("assets") {
            continue;
        }

        let target = patched_assets.join(rel_path);
        let file_type = file.file_type();

        // if file is a directory, just create it in the patched assets directory
        if file_type.is_dir() && !target.exists() {
            std::fs::create_dir_all(&target).context("Failed to create directory")?;
            continue;
        }

        // skip symlinks etc.
        if !file_type.is_file() {
            continue;
        }

        // copy over the file if it doesn't exist already
        if !target.exists() {
            info!("Adding new file: {}", rel_path.display());
            copy_file(&file.path(), rel_path, &patched_assets)?;
        }
    }

    pack_to_assets(temp_dir, &game_dir, repack_info)?;

    Ok(patched_assets)
}

/// Copies a file from one of the input directories to the patched assets directory and makes sure
/// the directory structure is created
fn copy_file(file: &Path, rel_path: &Path, patched_assets: &PathBuf) -> anyhow::Result<()> {
    let output = patched_assets.join(rel_path);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).context("Failed to create directory")?;
    }
    std::fs::copy(file, &output).context("Failed to copy file")?;
    Ok(())
}

/// Patches an XML file using the given patch file and writes the output to the patched assets directory
fn patch_xml(original: &Path, patch_file: &PathBuf, rel_path: &Path, patched_assets: &PathBuf) -> anyhow::Result<()> {
    let output = patched_assets.join(rel_path);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create directory")?;
    }
    xml_patcher::patch(original, patch_file, &output)
}

fn pack_to_assets(temp_dir: &PathBuf, game_dir: &PathBuf, repack: RepackInfo) -> anyhow::Result<()> {
    let output = game_dir.join("sharedassets0.assets");
    let patched = temp_dir.join("patched");
    let temp_art = temp_dir.join("patched-art.dat");
    pack::pack(&repack.art_key, &Some(patched.clone()), &temp_art)?;
    let assets = repack.assets;
    let new_art_len = std::fs::metadata(&temp_art)
        .context("Failed to get metadata of temp art file")?
        .len();

    // header
    let mut header = AssetsFileHeader { file_size: 0, ..assets.header };

    // content
    let mut objects = Vec::new();
    let mut current_offset = 0;
    for obj in &assets.content.objects {
        let mut new_object = ObjectInfo {
            path_id: obj.path_id,
            byte_start: current_offset,
            byte_size: 0,
            type_id: obj.type_id,
        };
        if obj.path_id == repack.art_path_id {
            new_object.byte_size = (new_art_len + ART_OBJ_HEADER_LEN) as u32;
        } else {
            new_object.byte_size = obj.byte_size;
        }
        current_offset += new_object.byte_size as u64;

        // When writing the object data, the start of object data is always aligned to 8 bytes, and
        // the end of object data is always aligned to 4 bytes. The bytes used to pad the end of the
        // object data to the next 4-byte boundary are included in the object's byte size. The bytes
        // used to align the start of the object data to 8 bytes are not included in any size.
        if current_offset % 8 != 0 {
            let padding = 8 - (current_offset % 8);
            new_object.byte_size += (padding % 4) as u32;
            current_offset += padding;
        }

        objects.push(new_object);
    }
    header.file_size = header.offset_first_file + current_offset;
    let content = AssetsFileContent { objects, ..assets.content };
    let new_assets = AssetsFile { header, content };

    let mut writer = BufWriter::new(File::create(&output)
        .context("Failed to create output file")?);
    new_assets.write(&mut writer)
        .context("Failed to write assets file header")?;

    // pad with zeroes until first file offset is reached (yes this is also what Unity does)
    let pad = assets.header.offset_first_file - writer.stream_position()
        .context("Failed to get current position in output file")?;
    write_zeroes(&mut writer, pad)?;

    // write the actual object data
    let mut original = BufReader::new(File::open(&repack.original_assets)
        .context("Failed to open original assets file")?);
    let original_file_offset = &assets.header.offset_first_file;
    for (obj, old_obj) in new_assets.content.objects.iter().zip(assets.content.objects) {
        let pos = writer.stream_position()
            .context("Failed to get current position in output file")?;
        if pos != obj.byte_start + original_file_offset {
            // pad with zeroes until the object's start offset is reached
            let pad = obj.byte_start + original_file_offset - pos;
            write_zeroes(&mut writer, pad).context("Failed to write padding zeroes")?;
        }

        if obj.path_id == repack.art_path_id {
            AlignedString("Art.dat".to_string()).write_options(&mut writer, new_assets.endian(), AlignmentArgs::new(4))
                .context("Failed to write object name")?;
            (new_art_len as u32).write_options(&mut writer, new_assets.endian(), ())
                .context("Failed to write object data length")?;
            // copy over the new art file
            let mut art_file = BufReader::new(File::open(&temp_art)
                .context("Failed to open temp art file")?);
            std::io::copy(&mut art_file, &mut writer)
                .context("Failed to copy new art file to assets file")?;
        } else if let Some(audio) = repack.audio_assets.get(&obj.path_id) {
            audio.write_options(&mut writer, new_assets.endian(), ())
                .context("Failed to write audio object")?;
        } else {
            original.seek(SeekFrom::Start(original_file_offset + old_obj.byte_start))
                .context("Failed to seek to object in original assets file")?;
            let mut data = vec![0; obj.byte_size as usize];
            original.read_exact(&mut data)
                .context("Failed to read object data from original assets file")?;
            writer.write_all(&data)?;
        }
    }

    info!("Packed {} objects", new_assets.content.objects.len());
    Ok(())
}