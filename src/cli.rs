use std::num::ParseIntError;

use anyhow::bail;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
pub struct MclArgs {
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    #[arg(short, long)]
    pub world: String,

    #[command(subcommand)]
    pub action: Option<Action>,
}

#[derive(Subcommand, Debug)]
pub enum Action {
    ResetLighting(ResetLightingArgs),
    Prune(PruneArgs),
    Blocks(BlockArgs),
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct PruneArgs {
    #[arg(short, long)]
    pub inhabited_under: String,

    #[arg()]
    pub region: String,
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct BlockArgs {
    #[arg(short, long)]
    pub pattern: String,

    #[arg(short, long)]
    pub from: String,

    #[arg(short, long, value_parser=parse_coords)]
    pub to: (i32, i32, i32),
}

fn parse_coords(coords: &str) -> Result<(i32, i32, i32)> {
    let v: Result<Vec<i32>, ParseIntError> =
        coords.splitn(3, ',').map(|s| s.parse::<i32>()).collect();
    let v = v?;
    if v.len() == 3 {
        return Ok((v[0], v[1], v[2]));
    }
    bail!(format!("Failed to parse coordinates {}", coords))
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct ResetLightingArgs {
    #[arg()]
    pub region: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_check() {
        MclArgs::command().debug_assert();
    }
}
