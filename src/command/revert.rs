use std::path::PathBuf;

pub fn revert(game_dir: &PathBuf) -> anyhow::Result<()> {
    // if game_dir is not already PapersPlease_Data, append it
    let game_dir = if game_dir.ends_with("PapersPlease_Data") {
        game_dir.clone()
    } else {
        game_dir.join("PapersPlease_Data")
    };

    if !game_dir.is_dir() {
        anyhow::bail!("Game directory {:?} does not exist", game_dir);
    }
    
    let assets = game_dir.join("sharedassets0.assets");
    let assets_bak = game_dir.join("sharedassets0.assets-bak");
    copy_backup(&assets, &assets_bak)?;

    let resources = game_dir.join("sharedassets0.resource");
    let resources_bak = game_dir.join("sharedassets0.resource-bak");
    copy_backup(&resources, &resources_bak)?;

    let locale = game_dir.join("StreamingAssets/loc/en.zip");
    let locale_bak = game_dir.join("StreamingAssets/loc/en.zip-bak");
    copy_backup(&locale, &locale_bak)?;
    
    println!("Reverted game files in: {:?} to vanilla state", game_dir);

    Ok(())
}

fn copy_backup(file: &PathBuf, backup: &PathBuf) -> anyhow::Result<()> {
    if !backup.exists() {
        anyhow::bail!("Couldn't find {} in game directory {:?}. You'll to verify game integrity in steam to revert changes.", backup.display(), backup.parent());
    }
    std::fs::copy(backup, file)?;
    Ok(())
}