use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Write, path::PathBuf};

use viv32_vo::{ObjectFile, RelocationBase, RelocationSign, Symbol};

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

#[derive(Debug)]
enum LinkerError {
    DuplicateLabel(String),
    UnknownLabel(String),
    InvalidRange(String),
}

impl std::fmt::Display for LinkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkerError::DuplicateLabel(message)
            | LinkerError::UnknownLabel(message)
            | LinkerError::InvalidRange(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for LinkerError {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut object_file = ObjectFile::new();
    for input in &args.inputs {
        let file = File::open(input)?;
        let current = ObjectFile::read(file)?;
        let offset = object_file.bytes.len();

        // Add bytes and pad if necessary
        object_file.bytes.extend_from_slice(&current.bytes);
        let ragged = object_file.bytes.len() % 4;
        if ragged != 0 {
            let target = object_file.bytes.len() + (4 - ragged);
            object_file.bytes.resize(target, 0);
        }

        // Add bss.
        for bss in current.bss {
            if object_file.contains_label(&bss.name) {
                return Err(Box::new(LinkerError::DuplicateLabel(format!(
                    "Duplicated Symbol: {}|{}",
                    input.to_string_lossy().to_owned(),
                    bss.name
                ))));
            }
            object_file.bss.push(bss.clone());
        }

        // Add adjusted symbols.
        for symbol in current.symbols {
            if object_file.contains_label(&symbol.name) {
                return Err(Box::new(LinkerError::DuplicateLabel(format!(
                    "Duplicated Symbol: {}|{}",
                    input.to_string_lossy().to_owned(),
                    symbol.name
                ))));
            }
            let mut symbol = symbol.clone();
            symbol.offset += offset as u32;
            object_file.symbols.push(symbol);
        }

        // Add adjusted relocations.
        for relocation in current.relocations {
            let mut relocation = relocation.clone();
            relocation.patch_offset += offset as u32;
            object_file.relocations.push(relocation);
        }
    }

    // Apply BSS, ensuring alignment contract is met.
    let mut offset = object_file.bytes.len() as u32;
    for bss in &object_file.bss {
        let ragged = offset % bss.alignment;
        if ragged != 0 {
            offset = offset + (bss.alignment - ragged);
        }
        object_file.symbols.push(Symbol {
            offset,
            name: bss.name.to_owned(),
        });
        offset += bss.size;
    }

    // Apply relocations
    for relocation in &object_file.relocations {
        if let Some(symbol) = object_file.get_symbol_by_name(&relocation.symbol) {
            let client = relocation.patch_offset as i64;
            let mut target = symbol.offset as i64 + relocation.addend as i64;
            if relocation.base == RelocationBase::Relative {
                target = target - client;
            }
            target = target >> relocation.value_shift;
            if relocation.bounds_check != 0 {
                let (min, max) = match relocation.sign {
                    RelocationSign::Unsigned => {
                        if relocation.width == 32 {
                            (0_i64, u32::MAX as i64)
                        } else {
                            (0_i64, (1_i64 << relocation.width) - 1)
                        }
                    }

                    RelocationSign::Signed => (
                        -(1_i64 << (relocation.width - 1)),
                        (1_i64 << (relocation.width - 1)) - 1,
                    ),
                };
                if target < min || target > max {
                    return Err(Box::new(LinkerError::InvalidRange(format!(
                        "Invalid range for width {} detected: {} vs {}..{}",
                        relocation.width, target, min, max
                    ))));
                }
            }
            let mask: i64 = if relocation.width == 32 {
                u32::MAX as i64
            } else {
                (1_i64 << relocation.width) - 1
            };
            let relocated = ((target & mask) << relocation.field_shift) as u32;
            let replacing = u32::from_be_bytes(
                object_file.bytes[client as usize..((client as usize) + 4)]
                    .try_into()
                    .expect("This should have existed"),
            );
            let field_mask = (mask as u32) << relocation.field_shift;
            let replaced = (replacing & !field_mask) | relocated;
            // println!(
            //     "Replacing {:08X} with {:08X} at {:08X}",
            //     replacing, replaced, client
            // );
            object_file.bytes[client as usize..(client as usize) + 4]
                .copy_from_slice(&replaced.to_be_bytes());
        } else {
            return Err(Box::new(LinkerError::UnknownLabel(format!(
                "Unable to link undeclared label: {}",
                relocation.symbol.to_owned()
            ))));
        };
    }

    let mut output_file = File::create(&args.output)?;
    output_file.write_all(&object_file.bytes)?;

    Ok(())
}
