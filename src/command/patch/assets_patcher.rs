use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use anyhow::Context;
use walkdir::WalkDir;
use crate::command::patch::xml_patcher;

pub fn patch_assets(patch: &PathBuf, unpacked: &PathBuf, temp_dir: &PathBuf) -> anyhow::Result<PathBuf> {
    println!("Patching assets..");
    let patched_assets = temp_dir.join("patched");
    std::fs::create_dir_all(&patched_assets)
        .context("Failed to create patched assets directory")?;

    // copy over original files and if they have a patch, apply the patch
    for file in WalkDir::new(unpacked) {
        let file = file.map_err(|e| anyhow::anyhow!("Failed to walk directory: {}", e))?;
        let rel_path = file.path().strip_prefix(unpacked)
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
            println!("Copying patch file for: {}", rel_path.display());
            copy_file(&patch_file.as_path(), rel_path, &patched_assets)?;
        } else if ext == OsStr::new("xml") || ext == OsStr::new("fnt") {
            println!("Patching xml file: {}", rel_path.display());
            patch_xml(&file, &patch_file, rel_path, &patched_assets)?;
        } else {
            anyhow::bail!("Unsupported file type: {}", patch_file.display());
        }

    }

    // Loop over any files newly added with the patch
    for file in WalkDir::new(patch) {
        let file = file.map_err(|e| anyhow::anyhow!("Failed to walk directory: {}", e))?;
        let rel_path = file.path().strip_prefix(patch)
            .context("Failed to strip prefix")?;
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
            println!("Adding new file: {}", rel_path.display());
            copy_file(&file.path(), rel_path, &patched_assets)?;
        }
    }

    return Ok(patched_assets);
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
fn patch_xml(original: &walkdir::DirEntry, patch_file: &PathBuf, rel_path: &Path, patched_assets: &PathBuf) -> anyhow::Result<()> {
    let output = patched_assets.join(rel_path);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create directory")?;
    }
    xml_patcher::patch(original.path(), patch_file, &output)
}