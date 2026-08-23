use anyhow::Result;
use clap::Parser;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use viv32_asm::State;

#[derive(Debug, Parser)]
#[command(
    name = "viv32-asm",
    version,
    about = "Assemble VIV-32 source into a relocatable object for linking"
)]
struct Args {
    /// The input file
    input: PathBuf,

    /// Output VIV32 object file
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let output = args
        .output
        .unwrap_or_else(|| default_output_path(&args.input));
    let input_file = File::open(&args.input)?;

    let constants = HashMap::from([
        ("%zero".to_owned(), "0".to_owned()),
        ("%fp".to_owned(), "13".to_owned()),
        ("%sp".to_owned(), "14".to_owned()),
        ("%lr".to_owned(), "15".to_owned()),
    ]);

    let mut state = State::new(args.input.to_string_lossy().to_string(), constants);

    state.assemble(input_file)?;
    let object_file = state.build_object_file();

    let output_file = File::create(&output)?;
    object_file.write(output_file)?;

    Ok(())
}

fn default_output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default();
    PathBuf::from(stem).with_extension("vo")
}
