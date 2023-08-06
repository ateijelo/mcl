use anyhow::{bail, Result};
use fastanvil::{Block, BlockData, Region};
use fastnbt::{from_bytes, to_bytes, Value};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs::File,
    io::Seek,
};

#[derive(Deserialize, Debug)]
struct Section {
    #[serde(rename = "Y")]
    y: i8,

    block_states: BlockData<Block>,
}

#[derive(Deserialize, Debug)]
struct Chunk {
    #[serde(rename = "InhabitedTime")]
    inhabited_time: i64,
    sections: Vec<Section>,
}

fn dump_nbt(data: &[u8]) -> Result<()> {
    let compound: HashMap<String, Value> = from_bytes(data)?;
    println!("{:#?}", compound);
    Ok(())
}

fn reset_lighting(mut reg: Region<File>) -> Result<()> {
    let mut new_chunks = vec![];

    for raw_chunk in reg.iter() {
        let raw_chunk = raw_chunk?;
        let x = raw_chunk.x;
        let z = raw_chunk.z;

        let mut full_chunk: HashMap<String, Value> = from_bytes(raw_chunk.data.as_slice())?;

        if full_chunk.remove("isLightOn").is_some() {
            println!("removed isLightOn from chunk x={x} z={z}");
        }

        let Some(Value::List(sections)) = full_chunk.get_mut("sections") else {
            continue;
        };

        for section in sections.iter_mut() {
            let Value::Compound(section) = section else {
                continue;
            };
            if section.remove("BlockLight").is_some() {
                println!("removed block light from chunk x={x} z={z}");
            };
            if section.remove("SkyLight").is_some() {
                println!("removed sky light from chunk x={x} z={z}");
            };
        }

        new_chunks.push((raw_chunk.x, raw_chunk.z, full_chunk));
    }

    for (x, z, full_chunk) in new_chunks {
        reg.write_chunk(x, z, to_bytes(&full_chunk)?.as_slice())?;
    }

    let mut file = reg.into_inner()?;
    let len = file.stream_position()?;
    file.set_len(len)?;

    Ok(())
}

fn blocks(mut reg: Region<File>, pattern: &str) -> Result<()> {
    for raw_chunk in reg.iter().take(1) {
        let raw_chunk = raw_chunk?;
        // let x = raw_chunk.x;
        // let z = raw_chunk.z;

        // dump_nbt(raw_chunk.data.as_slice())?;

        let chunk: Chunk = from_bytes(raw_chunk.data.as_slice())?;
        for section in chunk.sections {
            // println!("{:#?}", section);
            let mut ids: HashSet<usize> = HashSet::new();
            for (id, block) in section.block_states.palette().iter().enumerate() {
                if block.name().contains(pattern) {
                    ids.insert(id);
                }
            }

            let indices = section.block_states.try_iter_indices();
            if indices.is_none() {
                continue;
            }

            for (i, palette_index) in indices.unwrap().enumerate() {
                if !ids.contains(&palette_index) {
                    continue;
                }
                let x = (raw_chunk.x << 4) + (i & 0x000F);
                let y = ((section.y as isize) << 4) + ((i as isize & 0x0F00) >> 8);
                let z = (raw_chunk.z << 4) + ((i & 0x00F0) >> 4);
                let block = &section.block_states.palette()[palette_index];
                println!("{x} {y} {z} {:#?}", block.name());
            }
        }
    }
    Ok(())
}

fn remove_chunks(mut reg: Region<File>) -> Result<()> {
    let mut pruned = vec![];

    for raw_chunk in reg.iter().take(1) {
        let raw_chunk = raw_chunk?;
        let x = raw_chunk.x;
        let z = raw_chunk.z;

        dump_nbt(raw_chunk.data.as_slice())?;

        let chunk: Chunk = from_bytes(raw_chunk.data.as_slice())?;

        // 60 seconds * 20 ticks per second
        if chunk.inhabited_time < 60 * 20 {
            pruned.push((x, z));
        }
    }

    println!("Removing {} chunks...", pruned.len());
    for (x, z) in pruned {
        reg.remove_chunk(x, z)?;
    }
    let mut file = reg.into_inner()?;
    let len = file.stream_position()?;
    file.set_len(len)?;
    println!("done.");

    Ok(())
}

fn main() -> Result<()> {
    let action = args().nth(1).unwrap();
    let filename = args().nth(2).unwrap();
    let file = File::options().read(true).write(true).open(filename)?;

    let reg = Region::from_stream(file)?;

    match action.as_str() {
        "prune" => {
            remove_chunks(reg)?;
        }
        "blocks" => {
            blocks(reg, "diamond")?;
        }
        "reset-lighting" => {
            reset_lighting(reg)?;
        }
        _ => {
            bail!("unknown action: {action}")
        }
    }

    Ok(())
}
