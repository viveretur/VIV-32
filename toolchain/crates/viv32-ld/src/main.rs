use anyhow::Result;
use clap::Parser;
use std::{fs::File, path::PathBuf};

use viv32_ld::Linker;

#[derive(Debug, Parser)]
#[command(
    name = "viv32-ld",
    version,
    about = "Link VIV-32 objects (.vo) into a VIV-32 binary (.bin)"
)]
struct Args {
    /// The input files
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,

    /// Output VIV32 object file
    #[arg(short, long, default_value = "a.bin")]
    pub output: PathBuf,

    /// Optional linker map output
    #[arg(long)]
    pub map: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut linker = Linker::new();

    for input in &args.inputs {
        let file = File::open(input)?;
        linker.add(file)?;
    }

    linker.link(&args.output)?;

    Ok(())
}
