pub mod assets_patcher;
pub mod xml_patcher;

use std::env::temp_dir;
use std::path::PathBuf;
use anyhow::Context;

use rand::random;

use crate::{I18nCompatMode, NewArgs};
use crate::command::patch::assets_patcher::patch_assets;
use crate::command::unpack;

pub fn patch(args: &NewArgs, patch: &PathBuf, locale_mode: &I18nCompatMode) -> anyhow::Result<()> {
    println!("Patching assets with {:?} with locale mode {:?}", patch, locale_mode);

    if !patch.is_dir() {
        anyhow::bail!("Patch directory {:?} does not exist", patch);
    }

    let game_files = prepare_game_files(&patch)?;

    let temp_dir = create_temp_dir();
    let temp_unpacked = temp_dir.join("unpacked");
    std::fs::create_dir_all(&temp_unpacked)
        .context("Failed to create temp directory")?;

    unpack::unpack_dat(args, &game_files.assets, &temp_unpacked)?;

    patch_assets(patch, &temp_unpacked, &temp_dir)?;

    unimplemented!()
}


//<editor-fold desc="Filesystem preparations" defaultstate="collapsed">
pub struct GameFiles {
    pub assets: PathBuf,
    pub resources: PathBuf,
    pub locale: PathBuf,
}

fn prepare_game_files(game_dir: &PathBuf) -> anyhow::Result<GameFiles> {
    if !game_dir.is_dir() {
        anyhow::bail!("Game directory {:?} does not exist", game_dir);
    }

    let assets = prepare_file(game_dir, "sharedassets0.assets")?;
    let resources = prepare_file(game_dir, "sharedassets0.resource")?;
    let locale = prepare_file(game_dir, "StreamingAssets/loc/en.zip")?;

    Ok(GameFiles { assets, resources, locale })
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

    std::fs::copy(&file, &copy_file)
        .map_err(|e| anyhow::anyhow!("Failed to create backup of {}: {}", name, e))?;

    Ok(copy_file)
}

fn create_temp_dir() -> PathBuf {
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("papers_please_assets_{}", random::<u64>()));
    temp_dir
}
//</editor-fold>