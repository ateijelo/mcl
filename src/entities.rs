use super::rect_intersects_bounds;
use super::within_bounds;
use crate::cli::Coords;
use crate::cli::Dimension;
use crate::nbt::load_chunk;
use crate::nbt::BlockEntity;
use crate::prune::list_region_files;
use anyhow::{Context, Result};
use fastanvil::Region;
use fastnbt::from_bytes;
use fastnbt::from_value;
use fastnbt::Value;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

pub(crate) fn entities(
    world: &Path,
    dimension: Dimension,
    from: Option<Coords>,
    to: Option<Coords>,
    json: bool,
) -> Result<()> {
    let entities_dir = match dimension {
        Dimension::Overworld => world.join("entities"),
        Dimension::Nether => world.join("DIM-1/entities"),
        Dimension::End => world.join("DIM1/entities"),
    };
    let region_files = list_region_files(&entities_dir)?;
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

            let compound: HashMap<String, Value> = from_bytes(raw_chunk.data.as_slice())?;
            if json {
                println!("{}", serde_json::to_string(&compound)?);
            } else {
                println!("{:#?}", compound);
            }

            // let chunk = match load_chunk(raw_chunk.data.as_slice()) {
            //     Ok(c) => c,
            //     Err(e) => {
            //         // if reading the chunk fails, we count 0 chunks pruned and continue
            //         log::debug!("error reading chunk {x} {z}: {}", e);
            //         continue;
            //     }
            // };
            // for entity in chunk.block_entities() {
            //     let pos: BlockEntity = from_value(entity)?;
            //     let pos = (pos.x, pos.y, pos.z);
            //     if !within_bounds(&pos, from.as_ref(), to.as_ref()) {
            //         log::debug!("entity {reg_x} {reg_z} doesn't intersect bounds, skipping");
            //         continue;
            //     }
            //     if json {
            //         println!("{}", serde_json::to_string(&entity)?);
            //     } else {
            //         println!("{:?}", entity);
            //     }
            // }
        }
    }
    Ok(())
}
