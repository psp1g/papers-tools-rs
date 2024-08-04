use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use rand::random;
use tracing::info;
use unpack::unpack_assets;

use crate::{I18nCompatMode, Args};
use crate::command::patch::assets_patcher::patch_assets;
use crate::command::patch::audio_patcher::patch_audio;
use crate::command::patch::locale_patcher::patch_locale;
use crate::command::{DATA_FOLDER_NAME, unpack};

mod assets_patcher;
mod xml_patcher;
mod locale_patcher;
pub mod audio_patcher;

pub fn patch(args: &Args, patch: &PathBuf, locale_mode: &I18nCompatMode) -> anyhow::Result<()> {
    info!("Patching assets with {:?} with locale mode {:?}", patch, locale_mode);

    if !patch.is_dir() {
        anyhow::bail!("Patch directory {:?} does not exist", patch);
    }

    let game_files = prepare_game_files(&args.game_dir)?;

    let temp_dir = create_temp_dir();
    let temp_unpacked = temp_dir.join("unpacked");
    fs::create_dir_all(&temp_unpacked)
        .context("Failed to create temp directory")?;
    let audio_patches = patch.join("audio_patches.json");
    let process_audio =  audio_patches.is_file();

    let mut repack_info = unpack_assets(args, &game_files.assets, &temp_unpacked, process_audio)?;
    if process_audio {
        patch_audio(&audio_patches, &game_files.game_dir, &mut repack_info)?;
    }
    let patched_dir = patch_assets(patch, &temp_dir, &game_files.game_dir, repack_info)?;

    if locale_mode == &I18nCompatMode::Normal {
        patch_locale(&patched_dir, &game_files.game_dir)?;
    }

    info!("Cleaning up...");
    fs::remove_dir_all(&temp_dir).context("Failed to remove temp directory")?;

    Ok(())
}

//<editor-fold desc="Filesystem preparations" defaultstate="collapsed">
pub struct GameFiles {
    pub game_dir: PathBuf,
    pub assets: PathBuf,
}

fn prepare_game_files(game_dir: &PathBuf) -> anyhow::Result<GameFiles> {
    // if game_dir is not already PapersPlease_Data, append it
    let game_dir = if game_dir.ends_with(DATA_FOLDER_NAME) {
        game_dir.clone()
    } else {
        game_dir.join(DATA_FOLDER_NAME)
    };

    if !game_dir.is_dir() {
        anyhow::bail!("Game directory {:?} does not exist", game_dir);
    }

    let assets = prepare_file(&game_dir, "sharedassets0.assets")?;
    let _ = prepare_file(&game_dir, "StreamingAssets/loc/en.zip")?;

    Ok(GameFiles { game_dir, assets})
}

fn prepare_file(game_dir: &PathBuf, name: &str) -> anyhow::Result<PathBuf> {

    // check if backup file of original file already exists
    let copy_file = game_dir.join(format!("{}-bak", name));
    if copy_file.exists() {
        return Ok(copy_file);
    }

    // check if original file exists and create a backup
    let file = game_dir.join(name);
    if !file.exists() {
        anyhow::bail!("Couldn't find {} in game directory {:?}", name, game_dir);
    }

    fs::copy(&file, &copy_file)
        .map_err(|e| anyhow::anyhow!("Failed to create backup of {}: {}", name, e))?;

    Ok(copy_file)
}

fn create_temp_dir() -> PathBuf {
    let mut temp_dir = temp_dir();
    temp_dir.push("papers-tools");
    temp_dir.push(format!("papers_please_assets_{}", random::<u64>()));
    temp_dir
}
//</editor-fold>