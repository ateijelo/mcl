use fastanvil::Block;
use fastanvil::BlockData;
use fastnbt::Value;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BlockEntity {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Section {
    #[serde(rename = "Y")]
    pub(crate) y: i8,
    pub(crate) block_states: Option<BlockData<Block>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Chunk {
    #[serde(rename = "InhabitedTime")]
    pub(crate) inhabited_time: u64,
    pub(crate) sections: Vec<Section>,
    #[serde(default)]
    pub(crate) block_entities: Vec<Value>,
}

// pub(crate) trait Chunk {
//     fn inhabited_time(&self) -> u32 where Self: Sized{
//         0
//     }
// }
//
// impl Chunk for Chunk118 {}
//

// #[derive(Deserialize, Debug)]
// struct VersionedChunk {
//     #[serde(rename = "DataVersion")]
//     pub(crate) data_version: u32,
// }
