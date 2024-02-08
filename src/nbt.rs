use fastanvil::Block;
use fastanvil::BlockData;
use fastnbt::error::Result;
use fastnbt::{from_bytes, Value};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BlockEntity {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
}

#[derive(Deserialize, Debug)]
pub struct Section {
    #[serde(rename = "Y")]
    pub(crate) y: i8,
    pub(crate) block_states: Option<BlockData<Block>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Chunk118 {
    #[serde(rename = "InhabitedTime")]
    inhabited_time: u64,
    sections: Vec<Section>,
    #[serde(default)]
    block_entities: Vec<Value>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Level {
    #[serde(rename = "InhabitedTime")]
    inhabited_time: u64,
    #[serde(rename = "Sections")]
    sections: Vec<Section>,
    #[serde(rename = "BlockEntities", default)]
    block_entities: Vec<Value>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Chunk117 {
    #[serde(rename = "Level")]
    level: Level,
}

pub trait Chunk {
    fn inhabited_time(&self) -> u64;
    fn sections(&self) -> &Vec<Section>;
    fn block_entities(&self) -> &Vec<Value>;
}

impl Chunk for Chunk118 {
    fn inhabited_time(&self) -> u64 {
        self.inhabited_time
    }

    fn sections(&self) -> &Vec<Section> {
        &self.sections
    }

    fn block_entities(&self) -> &Vec<Value> {
        &self.block_entities
    }
}

impl Chunk for Chunk117 {
    fn inhabited_time(&self) -> u64 {
        self.level.inhabited_time
    }

    fn sections(&self) -> &Vec<Section> {
        &self.level.sections
    }

    fn block_entities(&self) -> &Vec<Value> {
        &self.level.block_entities
    }
}

#[derive(Deserialize, Debug)]
struct DataVersionChunk {
    #[serde(rename = "DataVersion")]
    pub(crate) data_version: u32,
}

pub fn load_chunk(input: &[u8]) -> Result<Box<dyn Chunk>> {
    let data_version;
    {
        let dv_chunk: DataVersionChunk = from_bytes(input)?;
        data_version = dv_chunk.data_version;
    }
    match data_version {
        v if v >= 2825 => Ok(Box::new(from_bytes::<Chunk118>(input)?)),
        _ => Ok(Box::new(from_bytes::<Chunk117>(input)?)),
    }
}


