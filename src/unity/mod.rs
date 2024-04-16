use binrw::{binrw, NullString};

use crate::unity::util::{Endian, U8Bool};

pub mod util;

#[binrw]
#[brw(big)]
#[derive(Debug, PartialEq)]
pub struct AssetsFile {
    #[brw(big)]
    pub header: AssetFileHeader,
    #[brw(is_little = header.endianness == Endian::Little)]
    pub content: AssetsFileContent,
}

impl AssetsFile {

    pub fn resolve_object_classes(&self) -> anyhow::Result<Vec<ResolvedObjectInfo>> {
        let types = &self.content.types;
        let mut out = Vec::new();
        for obj in &self.content.objects {
            let class_id = types.get(obj.type_id as usize).map(|t| t.class_id)
                .ok_or_else(|| anyhow::anyhow!("Failed to resolve class id for object with type id {}", obj.type_id))?;
            out.push(ResolvedObjectInfo {
                path_id: obj.path_id,
                byte_start: obj.byte_start,
                byte_size: obj.byte_size,
                class_id,
            });
        }

        return Ok(out);
    }

}

#[binrw]
#[brw(big)]
#[derive(Debug, PartialEq)]
pub struct AssetFileHeader {
    pub _unknown: u64,
    pub version: u32,
    pub _padding: u32,
    pub metadata_size: u64,
    pub file_size: u64,
    pub offset_first_file: u64,
    pub endianness: Endian,
    pub _unknown2: [u8; 7],
}

#[binrw]
#[derive(Debug, PartialEq)]
pub struct AssetsFileContent {
    pub unity_version: NullString,
    pub target: u32,
    #[br(assert(enable_type_tree == U8Bool(false)))]
    pub enable_type_tree: U8Bool,
    #[bw(calc = types.len() as i32)]
    type_count: i32,
    #[br(count = type_count)]
    pub types: Vec<SerializedType>,
    #[bw(calc = objects.len() as i32)]
    object_count: i32,
    #[br(count = object_count)]
    pub objects: Vec<ObjectInfo>,
    #[bw(calc = script_types.len() as i32)]
    script_count: i32,
    #[br(count = script_count)]
    pub script_types: Vec<ScriptType>,
    #[bw(calc = externals.len() as i32)]
    externals_count: i32,
    #[br(count = externals_count)]
    pub externals: Vec<FileIdentifier>,
    #[bw(calc = ref_types.len() as i32)]
    ref_type_count: i32,
    #[br(count = ref_type_count)]
    pub ref_types: Vec<SerializedType>,
    pub user_information: NullString,
}

#[binrw]
#[derive(Debug, PartialEq)]
pub struct SerializedType {
    pub class_id: i32,
    pub is_stripped_type: U8Bool,
    pub script_type_index: u16,
    #[brw(if(class_id.clone() == 114))]
    pub script_id: Option<[u8; 16]>,
    pub old_type_hash: [u8; 16],
}

#[binrw]
#[derive(Debug, PartialEq)]
pub struct ScriptType {
    local_serialized_file_index: i32,
    #[br(align_before(4))]
    local_identifier_in_file: i64,
}

#[binrw]
#[derive(Debug, PartialEq, Clone)]
pub struct FileIdentifier {
    pub temp_empty: NullString,
    pub guid: [u8; 16],
    pub r#type: i32,
    pub path: NullString,
}

#[binrw]
#[derive(Debug, PartialEq)]
pub struct ObjectInfo {
    #[br(align_before(4))]
    pub path_id: i64,
    pub byte_start: u64,
    pub byte_size: u32,
    pub type_id: i32,
}

#[derive(Debug, PartialEq)]
pub struct ResolvedObjectInfo {
    pub path_id: i64,
    pub byte_start: u64,
    pub byte_size: u32,
    pub class_id: i32,
}