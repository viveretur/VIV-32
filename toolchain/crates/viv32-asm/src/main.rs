use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg()]
    input: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("{args:?}");

    Ok(())
}
