use crate::cli::Dimension;
use crate::nbt::load_chunk;

use anyhow::{bail, Context, Result};
use fastanvil::Region;
use kiddo::{distance::squared_euclidean, float::kdtree::KdTree};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{
    fs::{read_dir, File},
    io::Seek,
};

pub(crate) type ChunkAges = HashMap<(i32, i32), u64>;

pub(crate) fn list_region_files(region_dir: &PathBuf) -> Result<Vec<PathBuf>, anyhow::Error> {
    if !region_dir.is_dir() {
        bail!("{} is not a directory", region_dir.display());
    }
    log::info!("Reading chunks...");
    let region_files: Vec<PathBuf> = read_dir(region_dir)?
        .filter_map(|entry| {
            let Ok(entry) = entry else {
                return None;
            };
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            if !path.to_string_lossy().ends_with(".mca") {
                return None;
            }
            Some(path)
        })
        .collect();
    Ok(region_files)
}

pub(crate) fn read_inhabited_time(
    region_files: &[PathBuf],
) -> Result<HashMap<(i32, i32), u64>, anyhow::Error> {
    let chunk_age = region_files
        .par_iter()
        .map(|path: &PathBuf| -> Result<ChunkAges> {
            let mut chunk_age: ChunkAges = HashMap::new();
            let file = File::options().read(true).write(false).open(path)?;
            let mut reg = Region::from_stream(file)?;

            let stem = path.file_stem().context("reading file stem")?;

            let stem = stem.to_string_lossy();

            let mut parts = stem.split('.').skip(1);
            let reg_x: i32 = parts.next().context("parsing filename")?.parse()?;
            let reg_z: i32 = parts.next().context("parsing filename")?.parse()?;

            for raw_chunk in reg.iter() {
                let raw_chunk = raw_chunk?;
                let x = reg_x * 32 + raw_chunk.x as i32;
                let z = reg_z * 32 + raw_chunk.z as i32;

                let chunk = match load_chunk(raw_chunk.data.as_slice()) {
                    Ok(c) => c,
                    Err(e) => {
                        log::debug!("reading chunk {x} {z}: {:?}", e);
                        continue;
                    }
                };

                chunk_age.insert((x, z), chunk.inhabited_time());
            }
            log::debug!("region {} has {} chunks", &path.display(), chunk_age.len());
            Ok(chunk_age)
        })
        .filter_map(|x| x.ok())
        .reduce(HashMap::new, |mut a, b| {
            a.extend(b);
            a
        });
    log::info!("{} chunks read.", chunk_age.len());
    Ok(chunk_age)
}

pub(crate) fn compute_boundary(
    chunk_ages: &HashMap<(i32, i32), u64>,
    inhabited_under: u64,
) -> Result<Vec<[f64; 2]>> {
    log::info!("Computing boundary...");
    let old = |x: i32, z: i32| -> bool {
        let Some(t) = chunk_ages.get(&(x, z)) else {
            return false;
        };
        *t >= inhabited_under
    };
    let mut boundary = vec![];
    for (x, z) in chunk_ages.keys() {
        if !old(*x, *z) {
            continue;
        }
        let mut old_neighbors = 0;
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                if old(x + dx, z + dz) {
                    old_neighbors += 1;
                }
            }
        }
        if old_neighbors != 8 {
            boundary.push([*x as f64, *z as f64]);
        }
    }
    log::info!("{} chunks in boundary", boundary.len());
    Ok(boundary)
}

pub(crate) fn remove_chunks(
    mut reg: Region<File>,
    reg_x: i32,
    reg_z: i32,
    chunks_kept: &HashSet<(i32, i32)>,
) -> Result<usize> {
    let mut pruned = vec![];

    for raw_chunk in reg.iter() {
        let raw_chunk = raw_chunk?;
        let x = reg_x * 32 + raw_chunk.x as i32;
        let z = reg_z * 32 + raw_chunk.z as i32;

        let r = load_chunk(raw_chunk.data.as_slice());
        if r.is_err() {
            // log::debug!(
            //     "error reading chunk {x} {z}: {:?}; not pruning it",
            //     r.unwrap_err()
            // );
            continue;
        };

        if !chunks_kept.contains(&(x, z)) {
            pruned.push((raw_chunk.x, raw_chunk.z));
        }
    }

    if pruned.is_empty() {
        return Ok(0);
    }

    log::debug!(
        "Removing {} chunks from region ({},{})...",
        pruned.len(),
        reg_x,
        reg_z
    );
    for (x, z) in pruned.iter() {
        reg.remove_chunk(*x, *z)?;
    }
    let mut file = reg.into_inner()?;
    let len = file.stream_position()?;
    file.set_len(len)?;

    Ok(pruned.len())
}

pub(crate) fn prune(
    world: &Path,
    dimension: Dimension,
    inhabited_under: u64,
    buffer: f64,
) -> Result<()> {
    let region_dir = match dimension {
        Dimension::Overworld => world.join("region"),
        Dimension::Nether => world.join("DIM-1/region"),
        Dimension::End => world.join("DIM1/region"),
    };
    let region_files = list_region_files(&region_dir)?;
    let chunk_ages = read_inhabited_time(&region_files)?;
    let boundary = compute_boundary(&chunk_ages, inhabited_under)?;

    log::info!("Building KDTree...");
    let boundary_kd: KdTree<f64, usize, 2, 256, u32> = (&boundary).into();
    log::info!("done.");
    //pub type KdTree<A, const K: usize> = float::kdtree::KdTree<A, usize, K, 32, u32> // size = 64 (0x40), align = 0x8

    log::info!("Creating buffer zone...");
    let mut chunks_kept = HashSet::new();
    for ((x, z), t) in chunk_ages {
        if t >= inhabited_under {
            chunks_kept.insert((x, z));
            continue;
        }
        let point = [x as f64, z as f64];
        let (distance, _) = boundary_kd.nearest_one(&point, &squared_euclidean);
        if distance <= buffer.powi(2) {
            chunks_kept.insert((x, z));
        }
    }
    log::info!("{} chunks will be kept.", chunks_kept.len());

    log::info!("Pruning...");
    let pruned: Result<Vec<usize>> = region_files
        .par_iter()
        .map(|path: &PathBuf| -> Result<usize> {
            // let mut chunk_age: ChunkAges = HashMap::new();
            let file = File::options().read(true).write(true).open(path)?;

            let Ok(reg) = Region::from_stream(file) else {
                // if reading the chunk fails, we count 0 chunks pruned and continue
                return Ok(0);
            };

            let stem = path.file_stem().context("reading file stem")?;
            let stem = stem.to_string_lossy();

            let mut parts = stem.split('.').skip(1);
            let reg_x: i32 = parts.next().context("parsing filename")?.parse()?;
            let reg_z: i32 = parts.next().context("parsing filename")?.parse()?;

            remove_chunks(reg, reg_x, reg_z, &chunks_kept)
        })
        .collect();

    log::info!("{} chunks pruned.", pruned?.iter().sum::<usize>());

    Ok(())
}
