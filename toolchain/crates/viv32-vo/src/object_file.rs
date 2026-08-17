use std::io::{Read, Write};
use std::str::FromStr;

use crate::{Bss, Relocation, Symbol, VoError};

const MAGIC: &str = "VIVO";
const VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectFile {
    pub bss: Vec<Bss>,
    pub symbols: Vec<Symbol>,
    pub relocations: Vec<Relocation>,
    pub bytes: Vec<u8>,
}

impl ObjectFile {
    pub fn new() -> Self {
        Self {
            bss: Vec::new(),
            symbols: Vec::new(),
            relocations: Vec::new(),
            bytes: Vec::new(),
        }
    }

    pub fn write<W: Write>(&self, mut writer: W) -> Result<(), VoError> {
        writeln!(writer, "{MAGIC} {VERSION}")?;
        writeln!(writer)?;

        writeln!(writer, "BSS {}", self.bss.len())?;
        for bss in &self.bss {
            writeln!(writer, "{bss}")?;
        }
        writeln!(writer)?;

        writeln!(writer, "SYMBOLS {}", self.symbols.len())?;
        for symbol in &self.symbols {
            writeln!(writer, "{symbol}")?;
        }
        writeln!(writer)?;

        writeln!(writer, "RELOCATIONS {}", self.relocations.len())?;
        for relocation in &self.relocations {
            writeln!(writer, "{relocation}")?;
        }
        writeln!(writer)?;

        writeln!(writer, "BYTES {:08X}", self.bytes.len())?;
        writer.write_all(&self.bytes)?;

        Ok(())
    }

    pub fn read<R: Read>(mut reader: R) -> Result<Self, VoError> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        let bytes_marker = find_bytes_marker(&data)
            .ok_or_else(|| VoError::InvalidObject("missing BYTES marker".to_owned()))?;

        let metadata = std::str::from_utf8(&data[..bytes_marker])
            .map_err(|err| VoError::InvalidObject(format!("metadata is not UTF-8: {err}")))?;

        let byte_start = find_next_line_start(&data, bytes_marker).ok_or_else(|| {
            VoError::InvalidObject("missing bytecode after BYTES marker".to_owned())
        })?;

        let mut lines = metadata.lines().filter(|line| !line.trim().is_empty());

        let header = lines
            .next()
            .ok_or_else(|| VoError::InvalidObject("missing object header".to_owned()))?;

        if header != format!("{MAGIC} {VERSION}") {
            return Err(VoError::InvalidObject(format!(
                "invalid object header `{header}`"
            )));
        }

        let bss_header = lines
            .next()
            .ok_or_else(|| VoError::InvalidObject("missing BSS header".to_owned()))?;

        let bss_count = parse_count_header(bss_header, "BSS")?;

        let mut bss = Vec::with_capacity(bss_count);

        for _ in 0..bss_count {
            let line = lines
                .next()
                .ok_or_else(|| VoError::InvalidObject("missing bss row".to_owned()))?;

            bss.push(Bss::from_str(line)?);
        }

        let symbols_header = lines
            .next()
            .ok_or_else(|| VoError::InvalidObject("missing SYMBOLS header".to_owned()))?;

        let symbol_count = parse_count_header(symbols_header, "SYMBOLS")?;

        let mut symbols = Vec::with_capacity(symbol_count);

        for _ in 0..symbol_count {
            let line = lines
                .next()
                .ok_or_else(|| VoError::InvalidObject("missing symbol row".to_owned()))?;

            symbols.push(Symbol::from_str(line)?);
        }

        let relocations_header = lines
            .next()
            .ok_or_else(|| VoError::InvalidObject("missing RELOCATIONS header".to_owned()))?;

        let relocation_count = parse_count_header(relocations_header, "RELOCATIONS")?;

        let mut relocations = Vec::with_capacity(relocation_count);

        for _ in 0..relocation_count {
            let line = lines
                .next()
                .ok_or_else(|| VoError::InvalidObject("missing relocation row".to_owned()))?;

            relocations.push(Relocation::from_str(line)?);
        }

        let bytes_header = std::str::from_utf8(&data[bytes_marker..byte_start.saturating_sub(1)])
            .map_err(|err| {
            VoError::InvalidObject(format!("BYTES header is not UTF-8: {err}"))
        })?;

        let byte_count = parse_byte_count(bytes_header.trim())?;
        let bytes = data[byte_start..].to_vec();

        if bytes.len() != byte_count {
            return Err(VoError::InvalidObject(format!(
                "byte count mismatch: expected {byte_count}, found {}",
                bytes.len()
            )));
        }

        if lines.next().is_some() {
            return Err(VoError::InvalidObject(
                "unexpected metadata after relocation table".to_owned(),
            ));
        }

        Ok(Self {
            bss,
            symbols,
            relocations,
            bytes,
        })
    }

    pub fn contains_label(&self, label: &str) -> bool {
        self.symbols.iter().any(|s| s.name == label) || self.bss.iter().any(|r| r.name == label)
    }

    pub fn get_symbol_by_name(&self, symbol_name: &str) -> Option<Symbol> {
        self.symbols.iter().find(|s| s.name == symbol_name).cloned()
    }
}

fn parse_count_header(line: &str, expected_name: &str) -> Result<usize, VoError> {
    let fields: Vec<&str> = line.split_whitespace().collect();

    if fields.len() != 2 || fields[0] != expected_name {
        return Err(VoError::InvalidObject(format!(
            "expected `{expected_name} <count>`, found `{line}`"
        )));
    }

    fields[1].parse::<usize>().map_err(|err| {
        VoError::InvalidObject(format!(
            "invalid {expected_name} count `{}`: {err}",
            fields[1]
        ))
    })
}

fn parse_byte_count(line: &str) -> Result<usize, VoError> {
    let fields: Vec<&str> = line.split_whitespace().collect();

    if fields.len() != 2 || fields[0] != "BYTES" {
        return Err(VoError::InvalidObject(format!(
            "expected `BYTES <byte_count>`, found `{line}`"
        )));
    }

    usize::from_str_radix(fields[1], 16)
        .map_err(|err| VoError::InvalidObject(format!("invalid byte count `{}`: {err}", fields[1])))
}

fn find_bytes_marker(data: &[u8]) -> Option<usize> {
    let marker = b"\nBYTES ";
    data.windows(marker.len())
        .position(|window| window == marker)
        .map(|index| index + 1)
        .or_else(|| {
            let marker = b"BYTES ";
            data.starts_with(marker).then_some(0)
        })
}

fn find_next_line_start(data: &[u8], from: usize) -> Option<usize> {
    data[from..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|offset| from + offset + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Relocation, RelocationBase, RelocationSign, Symbol};

    fn round_trip(object: ObjectFile) {
        let mut encoded = Vec::new();
        object.write(&mut encoded).unwrap();

        let decoded = ObjectFile::read(encoded.as_slice()).unwrap();

        assert_eq!(decoded, object);
    }

    #[test]
    fn object_file_round_trip() {
        let object = ObjectFile {
            bss: vec![Bss {
                alignment: 0x4,
                size: 0x100,
                name: "zerod".to_owned(),
            }],
            symbols: vec![Symbol {
                offset: 0x100,
                name: "message".to_owned(),
            }],
            relocations: vec![Relocation {
                patch_offset: 0x80,
                symbol: "_start".to_owned(),
                addend: -4,
                base: RelocationBase::Relative,
                sign: RelocationSign::Signed,
                value_shift: 2,
                width: 26,
                field_shift: 0,
            }],
            bytes: vec![0x00, 0x00, 0x00, 0x00, b'H', b'i', 0],
        };

        let mut encoded = Vec::new();
        object.write(&mut encoded).unwrap();

        let decoded = ObjectFile::read(encoded.as_slice()).unwrap();

        assert_eq!(decoded, object);
    }

    #[test]
    fn empty_object_round_trips() {
        round_trip(ObjectFile {
            bss: Vec::new(),
            symbols: Vec::new(),
            relocations: Vec::new(),
            bytes: Vec::new(),
        });
    }

    #[test]
    fn object_with_no_symbols_round_trips() {
        round_trip(ObjectFile {
            bss: Vec::new(),
            symbols: Vec::new(),
            relocations: vec![Relocation {
                patch_offset: 0x0000_0080,
                symbol: "_start".to_owned(),
                addend: -4,
                base: RelocationBase::Relative,
                sign: RelocationSign::Signed,
                value_shift: 2,
                width: 26,
                field_shift: 0,
            }],
            bytes: vec![0x00, 0x00, 0x00, 0x00],
        });
    }

    #[test]
    fn object_with_no_relocations_round_trips() {
        round_trip(ObjectFile {
            bss: Vec::new(),
            symbols: vec![Symbol {
                offset: 0x0000_0100,
                name: "message".to_owned(),
            }],
            relocations: Vec::new(),
            bytes: vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00],
        });
    }

    #[test]
    fn arbitrary_byte_values_round_trip() {
        round_trip(ObjectFile {
            bss: vec![Bss {
                alignment: 0x2,
                size: 0x12,
                name: "array".to_owned(),
            }],
            symbols: vec![Symbol {
                offset: 0x0000_0000,
                name: "blob".to_owned(),
            }],
            relocations: Vec::new(),
            bytes: vec![
                0x00, 0x01, 0x02, 0x03, 0x0A, 0x0D, 0x20, 0x7F, 0x80, 0xFE, 0xFF, b'V', b'I', b'V',
                b'O', b'\n', b'B', b'Y', b'T', b'E', b'S', b' ',
            ],
        });
    }

    #[test]
    fn invalid_header_is_rejected() {
        let data = b"NOPE 1\n\nSYMBOLS 0\n\nRELOCATIONS 0\n\nBYTES 00000000\n";

        let err = ObjectFile::read(&data[..]).unwrap_err();

        assert!(matches!(err, VoError::InvalidObject(_)));
    }

    #[test]
    fn byte_count_mismatch_is_rejected() {
        let data = b"VIVO 1\n\nSYMBOLS 0\n\nRELOCATIONS 0\n\nBYTES 00000004\nABC";

        let err = ObjectFile::read(&data[..]).unwrap_err();

        assert!(matches!(err, VoError::InvalidObject(_)));
    }
}
