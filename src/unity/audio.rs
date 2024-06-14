use binrw::binrw;
use serde::{Deserialize, Serialize};
use crate::unity::util::{AlignedString, U8Bool};

#[binrw]
#[derive(Debug, Clone)]
pub struct AudioClip {
    pub object_name: AlignedString,
    pub load_type: i32,
    pub channels: i32,
    pub frequency: i32,
    pub bits_per_sample: i32,
    pub length: f32,
    pub is_tracker_format: U8Bool,
    #[brw(align_before= 4)]
    pub subsound_index: i32,
    pub preload_audio_data: U8Bool,
    pub load_in_background: U8Bool,
    pub legacy3d: U8Bool,
    #[brw(align_before = 4)]
    pub resource: StreamedResource,
    pub compression_format: AudioCompressionFormat,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct StreamedResource {
    pub source: AlignedString,
    pub offset: i64,
    pub size: i64,
}

#[binrw]
#[brw(repr = u32)]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[repr(u32)]
#[serde(rename_all = "lowercase")]
pub enum AudioCompressionFormat {
    PCM = 0,
    Vorbis = 1,
    ADPCM = 2,
    MP3 = 3,
    PSMVAG = 4,
    HEVAG = 5,
    XMA = 6,
    AAC = 7,
    GCADPCM = 8,
    ATRAC9 = 9,
}