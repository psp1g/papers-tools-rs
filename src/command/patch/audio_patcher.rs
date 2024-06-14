use std::{fs, io};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use anyhow::Context;
use binrw::io::BufReader;
use serde::{Deserialize, Serialize};

use crate::command::unpack::RepackInfo;
use crate::unity::audio::{AudioClip, AudioCompressionFormat, StreamedResource};
use crate::unity::util::{AlignedString, U8Bool};

type AudioPatchList = Vec<AudioPatch>;

/// If you rename this for some reason, make sure it has a length between 21 and 24 characters
/// (inclusive) to avoid changing the size of the audio clips in the assets file.
const MODDED_RESOURCES_FILE: &str = "modded_assets0.resource";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPatch {
    pub object_name: String,
    pub patched_path: PathBuf,
    pub load_type: i32,
    pub channels: i32,
    pub frequency: i32,
    pub bits_per_sample: i32,
    pub length: f32,
    pub is_tracker_format: bool,
    pub subsound_index: i32,
    pub preload_audio_data: bool,
    pub load_in_background: bool,
    pub legacy3d: bool,
    pub compression_format: AudioCompressionFormat,
}

pub fn patch_audio(audio_patches_path: &PathBuf, game_dir: &PathBuf, repack_info: &mut RepackInfo) -> anyhow::Result<()> {
    let audio_patches = fs::read_to_string(audio_patches_path)
        .context("Failed to read audio patches file")?;
    let audio_patches: AudioPatchList = serde_json::from_str(&audio_patches)
        .context("Failed to parse audio patches file")?;

    let by_object_name = repack_info.audio_assets.iter()
        .map(|(path_id, clip)| (clip.object_name.as_str(), (path_id.clone(), clip)))
        .collect::<HashMap<_, _>>();

    let mut patched_clips: HashMap<i64, AudioClip> = HashMap::new();
    let mut modded_audio_writer = BufWriter::new(File::create(
        game_dir.join(MODDED_RESOURCES_FILE)
    ).context("Failed to create modded audio file")?);
    let patches_dir = audio_patches_path.parent()
        .context("Failed to get parent directory of audio patches file")?;

    let mut offset = 0u64;
    for patch in &audio_patches {
        if patch.patched_path.extension() != Some(OsStr::new("fsb")) {
            anyhow::bail!("Only FSB files are supported for audio patches.");
        }

        if let Some((path_id, clip)) = by_object_name.get(patch.object_name.as_str()) {
            let mut reader = BufReader::new(File::open(patches_dir.join(&patch.patched_path))
                .context("Failed to open patched audio file")?);
            let written = io::copy(&mut reader, &mut modded_audio_writer)
                .context("Failed to copy patched audio file to modded audio file")?;

            let new_clip = AudioClip {
                object_name: clip.object_name.clone(),
                resource: StreamedResource {
                    source: AlignedString(MODDED_RESOURCES_FILE.to_string()),
                    offset: offset as i64,
                    size: written as i64,
                },
                load_type: patch.load_type,
                channels: patch.channels,
                frequency: patch.frequency,
                bits_per_sample: patch.bits_per_sample,
                length: patch.length,
                is_tracker_format: U8Bool(patch.is_tracker_format),
                subsound_index: patch.subsound_index,
                preload_audio_data: U8Bool(patch.preload_audio_data),
                load_in_background: U8Bool(patch.load_in_background),
                legacy3d: U8Bool(patch.legacy3d),
                compression_format: patch.compression_format.clone(),
            };
            patched_clips.insert(*path_id, new_clip);

            offset += written;
        } else {
            let mut available = by_object_name.keys().map(|s| s.to_string()).collect::<Vec<_>>();
            available.sort();
            anyhow::bail!("Audio name {} in audio patches does not exist in the assets file. Available audio names:\n{}",
                patch.object_name,
                available.join(", ")
            );
        }
    }

    repack_info.audio_assets = patched_clips;

    Ok(())
}