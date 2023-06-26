use anyhow::Result;
use fastanvil::Region;
use fastnbt::from_bytes;
use serde::Deserialize;
use std::{env::args, fs::File, io::Seek};

#[derive(Deserialize, Debug)]
struct Chunk {
    #[serde(rename = "InhabitedTime")]
    inhabited_time: i64,
}

fn main() -> Result<()> {
    let filename = args().nth(1).unwrap();
    let file = File::options().read(true).write(true).open(filename)?;

    let mut reg = Region::from_stream(file)?;
    let mut pruned = vec![];

    for raw_chunk in reg.iter().take(1) {
        let raw_chunk = raw_chunk?;
        let x = raw_chunk.x;
        let z = raw_chunk.z;

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
