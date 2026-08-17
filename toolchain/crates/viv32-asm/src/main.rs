use anyhow::Result;
use clap::Parser;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use viv32_isa::{Creg, EncodeError, Instruction, encode};
use viv32_vo::{Bss, ObjectFile, Relocation, RelocationBase, RelocationSign, Symbol, VoError};

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Modes {
    Text,
    Data,
    RoData,
    Bss,
}

#[derive(Debug)]
enum ParseError {
    InvalidInstruction(String),
    InvalidName(String),
    DuplicateLabel(String),
    ObjectError(VoError),
    IOError(std::io::Error),
    DataAlignmentError(String),
    InvalidNumber(String),
    EncodeError(EncodeError),
}

impl From<VoError> for ParseError {
    fn from(err: VoError) -> Self {
        Self::ObjectError(err)
    }
}

impl From<EncodeError> for ParseError {
    fn from(err: EncodeError) -> Self {
        Self::EncodeError(err)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidInstruction(message) => write!(f, "{}", message),
            ParseError::InvalidName(name) => write!(f, "Invalid symbol name: {}", name),
            ParseError::DuplicateLabel(message) => write!(f, "{}", message),
            ParseError::ObjectError(err) => write!(f, "{}", err),
            ParseError::IOError(err) => write!(f, "{}", err),
            ParseError::DataAlignmentError(message) => write!(f, "{}", message),
            ParseError::InvalidNumber(message) => write!(f, "{}", message),
            ParseError::EncodeError(err) => write!(f, "{:?}", err),
        }
    }
}

impl std::error::Error for ParseError {}

type ParseResult = Result<(), ParseError>;

#[derive(Debug)]
struct State {
    mode: Modes,
    bytes: Vec<u8>,
    symbols: HashMap<String, Symbol>,
    bss: HashMap<String, Bss>,
    relocations: Vec<Relocation>,
    constants: HashMap<String, String>,
    mangles: HashMap<String, String>,
    filename: String,
    line_no: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let output = args
        .output
        .unwrap_or_else(|| default_output_path(&args.input));
    let input_file = File::open(&args.input)?;

    let constants = HashMap::from([
        ("%pc".to_owned(), Creg::PC.to_string()),
        ("%pc".to_owned(), Creg::SR.to_string()),
        ("%pc".to_owned(), Creg::EPC.to_string()),
        ("%pc".to_owned(), Creg::ESR.to_string()),
        ("%pc".to_owned(), Creg::ECause.to_string()),
        ("%pc".to_owned(), Creg::EData.to_string()),
        ("%pc".to_owned(), Creg::EvBase.to_string()),
    ]);

    let mut state = State {
        mode: Modes::Text,
        bytes: Vec::new(),
        symbols: HashMap::new(),
        bss: HashMap::new(),
        relocations: Vec::new(),
        constants,
        mangles: HashMap::new(),
        filename: args.input.to_string_lossy().to_string(),
        line_no: 0,
    };
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

impl State {
    fn build_object_file(self) -> ObjectFile {
        let mut bss: Vec<Bss> = self.bss.into_values().collect();
        bss.sort_by(|a, b| a.name.cmp(&b.name));
        let mut symbols: Vec<Symbol> = self.symbols.into_values().collect();
        symbols.sort_by_key(|s| s.offset);

        ObjectFile {
            bss,
            symbols,
            relocations: self.relocations,
            bytes: self.bytes,
        }
    }

    #[rustfmt::skip]
    fn assemble(&mut self, input_file: File) -> ParseResult {
        let reader = BufReader::new(input_file);

        for line in reader.lines() {
            self.line_no += 1;
            let line = line?;
            if let Some(line) = strip_spaces_comments(&line) {
                let parts = self.tokenize(&line);

                match (
                    self.mode,
                    parts
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .as_slice(),
                ) {
                    // Directives
                    (_, [".text", ..]) => self.mode = Modes::Text,
                    (_, [".data", ..]) => self.mode = Modes::Data,
                    (_, [".rodata", ..]) => self.mode = Modes::RoData,
                    (_, [".bss", ..]) => self.mode = Modes::Bss,
                    (_, [".const", name, value]) => self.add_constant(name, value)?,
                    (_, [".org", location]) => self.assemble_org(location)?,
                    (_, [".nomangle", label]) => self.assemble_nomangle(label)?,

                    // Labels
                    (Modes::Bss,    [label, ":", ".space", alignment, size]) => self.assemble_bss(label, alignment, size)?,
                    (Modes::Data,   [label, ":", format, data, data2]) => self.assemble_data(label, format, data, data2)?,
                    (Modes::RoData, [label, ":", format, data, data2]) => self.assemble_data(label, format, data, data2)?,
                    (Modes::Data,   [label, ":", format, data]) => self.assemble_data(label, format, data, "0")?,
                    (Modes::RoData, [label, ":", format, data]) => self.assemble_data(label, format, data, "0")?,
                    (Modes::Text,   [label, ":"]) => self.assemble_label(label)?,

                    // Control instructions
                    (Modes::Text, ["nop"]) => self.bytes.extend_from_slice(&(encode(Instruction::Nop)?).to_be_bytes()),
                    (Modes::Text, ["halt"]) => self.bytes.extend_from_slice(&(encode(Instruction::Halt)?).to_be_bytes()),

                    _ => {
                        println!("Unknown Instruction: {}", line);
                    }
                }
            }
        }

        Ok(())
    }

    fn add_constant(&mut self, name: &str, value: &str) -> ParseResult {
        self.validate_symbol_name(name)?;
        if self.constants.contains_key(name) {
            return Err(ParseError::DuplicateLabel(format!(
                "Constant {} already defined. [{}:{}]",
                name, self.filename, self.line_no
            )));
        }
        self.constants.insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn assemble_label(&mut self, label: &str) -> ParseResult {
        self.check_label_already_known(label)?;
        self.validate_symbol_name(label)?;
        self.mangle_name(label)?;
        let mangled = self.mangles.get(label).cloned().unwrap();
        self.symbols.insert(
            label.to_owned(),
            Symbol {
                offset: self.bytes.len() as u32,
                name: mangled,
            },
        );
        Ok(())
    }

    fn assemble_data(&mut self, label: &str, format: &str, data: &str, data2: &str) -> ParseResult {
        self.check_label_already_known(label)?;
        self.validate_symbol_name(label)?;
        self.mangle_name(label)?;

        let mangled = self.mangles.get(label).cloned().unwrap();
        match format {
            ".byte" => self.append_signed(label, mangled, data, 8)?,
            ".ubyte" => self.append_unsigned(label, mangled, data, 8)?,
            ".half" => self.append_signed(label, mangled, data, 16)?,
            ".uhalf" => self.append_signed(label, mangled, data, 16)?,
            ".word" => self.append_signed(label, mangled, data, 32)?,
            ".uword" => self.append_unsigned(label, mangled, data, 32)?,
            ".ascii" => self.append_string(label, mangled, data, false)?,
            ".asciz" => self.append_string(label, mangled, data, true)?,
            ".space" => self.append_space(label, mangled, data, data2)?,
            _ => {
                return Err(ParseError::InvalidInstruction(format!(
                    "Invalid instruction at {}:{}",
                    self.filename, self.line_no
                )));
            }
        };
        Ok(())
    }

    fn append_signed(&mut self, label: &str, mangled: String, data: &str, bits: u8) -> ParseResult {
        self.ensure_alignment(bits >> 3)?;
        let address = self.bytes.len() as u32;
        let data = parse_signed_number(data)?;
        self.assert_range(
            data as i64,
            -(1_i64 << (bits - 1)),
            (1_i64 << (bits - 1)) - 1,
        )?;
        match bits {
            8 => self.bytes.push(data as u8),
            16 => self.bytes.extend_from_slice(&(data as u16).to_be_bytes()),
            32 => self.bytes.extend_from_slice(&(data as u32).to_be_bytes()),
            _ => unreachable!("Fixed arguments above"),
        }

        self.symbols.insert(
            label.to_owned(),
            Symbol {
                offset: address,
                name: mangled,
            },
        );
        Ok(())
    }

    fn append_unsigned(
        &mut self,
        label: &str,
        mangled: String,
        data: &str,
        bits: u8,
    ) -> ParseResult {
        self.ensure_alignment(bits >> 3)?;
        let address = self.bytes.len() as u32;
        let data = if data.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && data.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.mangle_name(data)?;
            let target_mangled = self.mangles.get(data).cloned().unwrap();
            self.relocations.push(Relocation {
                patch_offset: self.bytes.len() as u32,
                symbol: target_mangled,
                addend: 0,
                base: RelocationBase::Absolute,
                sign: RelocationSign::Unsigned,
                value_shift: 0,
                width: 32,
                field_shift: 0,
            });
            0
        } else {
            parse_unsigned_number(data)?
        };
        self.assert_range(data as i64, 0, (1_i64 << bits) - 1)?;
        match bits {
            8 => self.bytes.push(data as u8),
            16 => self.bytes.extend_from_slice(&(data as u16).to_be_bytes()),
            32 => self.bytes.extend_from_slice(&(data as u32).to_be_bytes()),
            _ => unreachable!("Fixed arguments above"),
        }

        self.symbols.insert(
            label.to_owned(),
            Symbol {
                offset: address,
                name: mangled,
            },
        );
        Ok(())
    }

    fn append_string(
        &mut self,
        label: &str,
        mangled: String,
        data: &str,
        zero_term: bool,
    ) -> ParseResult {
        let mut target = self.bytes.len() + data.len();
        if zero_term {
            target += 1;
        }
        if target > u32::MAX as usize {
            return Err(ParseError::DataAlignmentError(format!(
                "Calculated data size is too large: {}[{}:{}]",
                target, self.filename, self.line_no
            )));
        }
        let address = self.bytes.len() as u32;
        self.bytes.extend_from_slice(data.as_bytes());
        if zero_term {
            self.bytes.push(0);
        }
        self.symbols.insert(
            label.to_owned(),
            Symbol {
                offset: address,
                name: mangled,
            },
        );
        Ok(())
    }

    fn append_space(
        &mut self,
        label: &str,
        mangled: String,
        data: &str,
        data2: &str,
    ) -> ParseResult {
        let alignment = parse_unsigned_number(data)? as u8;
        self.ensure_alignment(alignment)?;
        let address = self.bytes.len() as u32;
        let size = parse_unsigned_number(data2)? as usize;
        let target = self.bytes.len() + size;
        if target > u32::MAX as usize {
            return Err(ParseError::DataAlignmentError(format!(
                "Calculated data size is too large: {}[{}:{}]",
                target, self.filename, self.line_no
            )));
        }
        self.bytes.resize(target, 0);
        self.symbols.insert(
            label.to_owned(),
            Symbol {
                offset: address,
                name: mangled,
            },
        );
        Ok(())
    }

    fn assert_range(&mut self, number: i64, min: i64, max: i64) -> ParseResult {
        if number < min || number > max {
            return Err(ParseError::InvalidNumber(format!(
                "Number outside of data range: {}[{}:{}]",
                number, self.filename, self.line_no
            )));
        };
        Ok(())
    }

    fn ensure_alignment(&mut self, alignment: u8) -> ParseResult {
        let diff = self.bytes.len() % alignment as usize;
        if diff != 0 {
            let target = self.bytes.len() + (alignment as usize - diff);
            self.bytes.resize(target, 0);
        }
        Ok(())
    }

    fn assemble_bss(&mut self, label: &str, alignment: &str, size: &str) -> ParseResult {
        self.check_label_already_known(label)?;
        self.validate_symbol_name(label)?;
        self.mangle_name(label)?;

        let alignment = parse_unsigned_number(alignment)?;
        let size = parse_unsigned_number(size)?;
        let mangled = self.mangles.get(label).cloned().unwrap();

        self.bss.insert(
            label.to_owned(),
            Bss {
                alignment,
                size,
                name: mangled,
            },
        );

        Ok(())
    }

    fn assemble_nomangle(&mut self, label: &str) -> ParseResult {
        self.check_label_already_known(label)?;
        self.validate_symbol_name(label)?;
        if self.mangles.contains_key(label) {
            return Err(ParseError::InvalidInstruction(format!(
                ".nomangle after usage: {}[{}:{}]",
                label, self.filename, self.line_no
            )));
        }
        self.mangles.insert(label.to_owned(), label.to_owned());

        Ok(())
    }

    fn mangle_name(&mut self, label: &str) -> ParseResult {
        if self.mangles.contains_key(label) {
            return Ok(());
        }

        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, self.filename.as_bytes());
        let mangled = format!("{}_{}", uuid.simple(), label);
        self.mangles.insert(label.to_owned(), mangled);

        Ok(())
    }

    fn assemble_org(&mut self, location: &str) -> ParseResult {
        if self.mode == Modes::Bss {
            return Err(ParseError::InvalidInstruction(format!(
                "Cannot place items in .bss section: {}:{}",
                self.filename, self.line_no
            )));
        }
        let size = parse_unsigned_number(location)? as usize;
        if self.bytes.len() < size {
            self.bytes.resize(size, 0);
        }
        Ok(())
    }

    fn validate_symbol_name(&mut self, name: &str) -> ParseResult {
        if name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Ok(());
        }
        Err(ParseError::InvalidName(format!(
            "Invalid name: {}[{}:{}]",
            name, self.filename, self.line_no
        )))
    }

    // This function is used whenever we need to add a new label, to
    // ensure we are not adding it twice. This checks symbols and bss.
    fn check_label_already_known(&mut self, label: &str) -> ParseResult {
        if self.symbols.contains_key(label) || self.bss.contains_key(label) {
            return Err(ParseError::DuplicateLabel(format!(
                "Duplicate label: {}[{}:{}]",
                label, self.filename, self.line_no
            )));
        }
        Ok(())
    }
    fn tokenize(&mut self, line: &str) -> Vec<String> {
        let line_len = line.len();
        let (prefix, string_token) = match line.find('"') {
            Some(index) => (
                &line[..index],
                Some(line[(index + 1)..(line_len - 1)].trim()), // TODO: use a validator instead..
            ),
            None => (line, None),
        };

        let mut normalized = String::with_capacity(prefix.len());

        for ch in prefix.chars() {
            match ch {
                ',' | '$' | '[' | ']' => normalized.push(' '),
                ':' => {
                    normalized.push(' ');
                    normalized.push(':');
                    normalized.push(' ');
                }
                _ => normalized.push(ch),
            }
        }

        let mut parts: Vec<String> = normalized
            .split_whitespace()
            .map(str::to_owned)
            .map(|a| {
                if self.constants.contains_key(&a) {
                    self.constants.get(&a).cloned().unwrap()
                } else {
                    a
                }
            })
            .collect();

        if let Some(string_token) = string_token {
            parts.push(string_token.to_owned());
        }

        parts
    }
}

fn parse_signed_number(text: &str) -> Result<i32, ParseError> {
    let text = text.trim();

    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i32::from_str_radix(hex, 16)
    } else {
        text.parse::<i32>()
    }
    .map_err(|_| ParseError::InvalidNumber(text.to_owned()))
}

fn parse_unsigned_number(text: &str) -> Result<u32, ParseError> {
    let text = text.trim();

    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        text.parse::<u32>()
    }
    .map_err(|_| ParseError::InvalidNumber(text.to_owned()))
}

fn strip_spaces_comments(line: &str) -> Option<String> {
    let mut in_string = false;
    let mut escaped = false;

    let mut end = line.len();
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                escaped = true;
            }
            '"' => {
                in_string = !in_string;
            }
            ';' if !in_string => {
                end = index;
                break;
            }
            _ => {}
        }
    }

    let line = line[..end].trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}
