use anyhow::{Context, Result};
use clap::Parser;
use cli::{Coords, Dimension, MclArgs};
use fastanvil::Region;
use fastnbt::{from_bytes, from_value, to_bytes, Value};
use nbt::load_chunk;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Seek,
    path::Path,
};

use crate::nbt::BlockEntity;

mod cli;
mod entities;
mod nbt;
mod prune;

// fn dump_nbt(data: &[u8]) -> Result<()> {
//     let compound: HashMap<String, Value> = from_bytes(data)?;
//     println!("{:#?}", compound);
//     Ok(())
// }

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

        for section in sections {
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
    for raw_chunk in reg.iter() {
        let raw_chunk = raw_chunk?;
        // let x = raw_chunk.x;
        // let z = raw_chunk.z;

        // dump_nbt(raw_chunk.data.as_slice())?;

        let chunk = load_chunk(raw_chunk.data.as_slice())?;
        for section in chunk.sections() {
            // println!("{:#?}", section);
            let mut ids: HashSet<usize> = HashSet::new();
            let Some(ref block_states) = section.block_states else {
                continue;
            };
            for (id, block) in block_states.palette().iter().enumerate() {
                if block.name() == "minecraft:air" {
                    continue;
                }
                if block.name().contains(pattern) {
                    ids.insert(id);
                }
            }

            let indices = block_states.try_iter_indices();
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
                let block = &block_states.palette()[palette_index];
                println!("{x} {y} {z} {:#?}", block.name());
            }
        }
    }
    Ok(())
}

fn within_bounds(point: &Coords, from: Option<&Coords>, to: Option<&Coords>) -> bool {
    let p = point;
    // if both from and to are provided, check that p is inside,
    if let (Some(from), Some(to)) = (from, to) {
        let f = (
            i32::min(from.0, to.0),
            i32::min(from.1, to.1),
            i32::min(from.2, to.2),
        );
        let t = (
            i32::max(from.0, to.0),
            i32::max(from.1, to.1),
            i32::max(from.2, to.2),
        );
        return (f.0 <= p.0 && p.0 <= t.0)
            && (f.1 <= p.1 && p.1 <= t.1)
            && (f.2 <= p.2 && p.2 <= t.2);
    }
    // if only from is provided, the range extends towards positive infinity
    if let Some(f) = from {
        return (f.0 <= p.0) && (f.1 <= p.1) && (f.2 <= p.2);
    }
    // if only to is provided, the range extends towards negative infinity
    if let Some(t) = to {
        return (p.0 <= t.0) && (p.1 <= t.1) && (p.2 <= t.2);
    }
    // if no bounds are provided, p is always within
    true
}

//   rf------------rt
//            bf----------bt
fn rect_intersects_bounds(
    rf: (i32, i32),
    rt: (i32, i32),
    from: Option<&Coords>,
    to: Option<&Coords>,
) -> bool {
    // rf -> region from
    // rt -> region to
    if let (Some(from), Some(to)) = (from, to) {
        // bf -> bounds from
        // bt -> bounds to
        let bf = (i32::min(from.0, to.0), i32::min(from.2, to.2));
        let bt = (i32::max(from.0, to.0), i32::max(from.2, to.2));

        if (bf.0 <= rt.0 && rf.0 <= bt.0) && (bf.1 <= rt.1 && rf.1 <= bt.1) {
            return true;
        }
        return false;
    }
    if let Some(from) = from {
        let bf = (from.0, from.2);
        return rt.0 >= bf.0 && rt.1 >= bf.1;
    }
    if let Some(to) = to {
        let bt = (to.0, to.2);
        return rf.0 <= bt.0 && rf.1 <= bt.1;
    }

    // if no bounds are provided, everything is within
    true
}

fn block_entities(
    world: &Path,
    dimension: Dimension,
    from: Option<Coords>,
    to: Option<Coords>,
    json: bool,
) -> Result<()> {
    let region_dir = match dimension {
        Dimension::Overworld => world.join("region"),
        Dimension::Nether => world.join("DIM-1/region"),
        Dimension::End => world.join("DIM1/region"),
    };
    let region_files = prune::list_region_files(&region_dir)?;
    for reg_file in region_files {
        let file = File::options().read(true).write(false).open(&reg_file)?;

        let stem = reg_file.file_stem().context("reading file stem")?;
        let stem = stem.to_string_lossy();

        let mut parts = stem.split('.').skip(1);
        let reg_x: i32 = parts.next().context("parsing filename")?.parse()?;
        let reg_z: i32 = parts.next().context("parsing filename")?.parse()?;

        let rf = (reg_x * 512, reg_z * 512);
        let rt = (reg_x * 512 + 511, reg_z * 512 + 511);
        if !rect_intersects_bounds(rf, rt, from.as_ref(), to.as_ref()) {
            log::debug!("region {reg_x} {reg_z} doesn't intersect bounds, skipping");
            continue;
        }

        log::debug!("reading region {}", reg_file.display());
        let mut reg = match Region::from_stream(file) {
            Ok(reg) => reg,
            Err(e) => {
                // if reading the chunk fails, we count 0 chunks pruned and continue
                log::debug!("error reading region {}: {}", reg_file.display(), e);
                continue;
            }
        };

        for raw_chunk in reg.iter() {
            let raw_chunk = raw_chunk?;
            let x = reg_x * 32 + raw_chunk.x as i32;
            let z = reg_z * 32 + raw_chunk.z as i32;

            let cf = (x * 16, z * 16);
            let ct = (x * 16 + 15, z * 16 + 15);
            if !rect_intersects_bounds(cf, ct, from.as_ref(), to.as_ref()) {
                log::debug!("chunk {x} {z} doesn't intersect bounds, skipping");
                continue;
            }

            let chunk = match load_chunk(raw_chunk.data.as_slice()) {
                Ok(c) => c,
                Err(e) => {
                    // if reading the chunk fails, we count 0 chunks pruned and continue
                    log::debug!("error reading chunk {x} {z}: {}", e);
                    continue;
                }
            };
            for entity in chunk.block_entities() {
                let pos: BlockEntity = from_value(&entity)?;
                let pos = (pos.x, pos.y, pos.z);
                if !within_bounds(&pos, from.as_ref(), to.as_ref()) {
                    log::debug!("entity {reg_x} {reg_z} doesn't intersect bounds, skipping");
                    continue;
                }
                if json {
                    println!("{}", serde_json::to_string(&entity)?);
                } else {
                    println!("{:?}", entity);
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::builder().format_timestamp_millis().init();

    let args = MclArgs::parse();

    if let Some(action) = args.action {
        use cli::Action;
        match action {
            Action::Prune(prune_args) => {
                prune::prune(
                    &prune_args.world,
                    prune_args.dimension,
                    prune_args.inhabited_under,
                    prune_args.buffer,
                )?;
            }
            // "blocks" => {
            //     blocks(reg, "diamond")?;
            // }
            // _ => {
            //     bail!("unknown action: {action}")
            // }
            Action::ResetLighting(rl_args) => {
                let file = File::options()
                    .read(true)
                    .write(true)
                    .open(rl_args.region)?;
                let reg = Region::from_stream(file)?;
                reset_lighting(reg)?;
            }
            Action::Blocks(block_args) => {
                let file = File::options()
                    .read(true)
                    .write(false)
                    .open(&block_args.world)
                    .with_context(|| format!("Failed to open `{}`", block_args.world.display()))?;
                let reg = Region::from_stream(file)?;
                blocks(reg, &block_args.pattern)?;
            }
            Action::BlockEntities(storage_args) => {
                block_entities(
                    &storage_args.world,
                    storage_args.dimension,
                    storage_args.from,
                    storage_args.to,
                    storage_args.json,
                )?;
            }
            Action::Entities(storage_args) => {
                entities::entities(
                    &storage_args.world,
                    storage_args.dimension,
                    storage_args.from,
                    storage_args.to,
                    storage_args.json,
                )?;
            }
        }
    }

    Ok(())
}
