use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use viv32_isa::decode;

#[derive(Debug, Parser)]
#[command(
    name = "viv32-dis",
    version,
    about = "Dissemble VIV-32 source from a compatible binary"
)]
struct Args {
    /// The input file
    #[arg()]
    input: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let bytes = std::fs::read(&args.input)?;

    for (index, chunk) in bytes.chunks(4).enumerate() {
        let address = index * 4;
        print!("{:08X}: ", address);

        for &byte in chunk {
            if (0x20..=0x7E).contains(&byte) {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }
        print!(" ");
        if chunk.len() == 4 {
            let word = u32::from_be_bytes(chunk.try_into().unwrap());
            print!("{:08X} ", word);
            let instruction = decode(word).ok();
            if instruction.is_none() {
                println!();
                continue;
            }
            println!("{}", instruction.unwrap());
        } else {
            for byte in chunk {
                print!("{:02X}", byte);
            }
            println!();
        }
    }

    Ok(())
}
