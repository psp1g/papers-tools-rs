use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub name: String,
    pub size: usize,
}

pub type ArtHeader = Vec<AssetMetadata>;

pub const DATA_FOLDER_NAME: &str = "PapersPlease_Data";

pub mod pack;
pub mod unpack;
pub mod patch;
pub mod revert;