use crate::AssemblerError;
use crate::error::ParseError;
use nofmt::pls;
use uuid::Uuid;
use viv32_isa::{Creg, Instruction, encode};
use viv32_vo::{
    Bss, ObjectFile, Relocation,
    RelocationBase::{Absolute, Relative},
    RelocationSign::{Signed, Unsigned},
    Symbol,
};

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

pub type AssemblerResult = Result<(), AssemblerError>;
type ParseResult = Result<(), ParseError>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Modes {
    Text,
    Data,
    RoData,
    Bss,
}

#[derive(Debug)]
pub struct State {
    mode: Modes,
    bytes: Vec<u8>,
    symbols: HashMap<String, Symbol>,
    bss: HashMap<String, Bss>,
    relocations: Vec<Relocation>,
    constants: HashMap<String, String>,
    mangles: HashMap<String, String>,
    filename: String,
}

impl State {
    pub fn new(filename: String, constants: HashMap<String, String>) -> Self {
        Self {
            mode: Modes::Text,
            bytes: Vec::new(),
            symbols: HashMap::new(),
            bss: HashMap::new(),
            relocations: Vec::new(),
            constants,
            mangles: HashMap::new(),
            filename,
        }
    }

    pub fn assemble(&mut self, input_file: File) -> AssemblerResult {
        let reader = BufReader::new(input_file);

        for (line_no, line) in reader.lines().enumerate() {
            let line = line.map_err(|err| AssemblerError {
                err: ParseError::IOError(err),
                filename: self.filename.to_owned(),
                line: line_no,
            });
            let Some(line) = strip_spaces_comments(&line?) else {
                continue;
            };
            self.process_instruction(&line)
                .map_err(|err| AssemblerError {
                    err,
                    filename: self.filename.to_owned(),
                    line: line_no,
                })?;
        }

        Ok(())
    }

    pub fn build_object_file(self) -> ObjectFile {
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

    fn process_instruction(&mut self, line: &str) -> ParseResult {
        use Instruction as I;

        const MAX_U12: i64 = (1i64 << 12) - 1;
        const MIN_I14: i64 = -(1i64 << 14);
        const MAX_I14: i64 = (1i64 << 14) - 1;
        const MIN_I16: i64 = i16::MIN as i64;
        const MAX_I16: i64 = i16::MAX as i64;
        const MAX_U16: i64 = i16::MAX as i64;
        const MIN_I24: i64 = -(1i64 << 24);
        const MAX_I24: i64 = (1i64 << 24) - 1;
        const MIN_I28: i64 = -(1i64 << 28);
        const MAX_I28: i64 = (1i64 << 28) - 1;
        const MAX_U32: i64 = u32::MAX as i64;

        let parts = self.tokenize(&line);

        pls! { match (
            self.mode,
            parts?
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
            (Modes::Bss, [label, ":", ".space", alignment, size]) => self.assemble_bss(label, alignment, size)?,
            (Modes::Data, [label, ":", format, data, data2]) => self.assemble_data(label, format, data, data2)?,
            (Modes::RoData, [label, ":", format, data, data2]) => self.assemble_data(label, format, data, data2)?,
            (Modes::Data, [label, ":", ".bin", bin_name]) => {
                self.ensure_alignment(4)?;
                self.assemble_label(label)?;
                self.insert_data(bin_name)?;
            }
            (Modes::RoData, [label, ":", ".bin", bin_name]) => {
                self.ensure_alignment(4)?;
                self.assemble_label(label)?;
                self.insert_data(bin_name)?;
            }
            (Modes::Data, [label, ":", format, data]) => self.assemble_data(label, format, data, "0")?,
            (Modes::RoData, [label, ":", format, data]) => self.assemble_data(label, format, data, "0")?,
            (Modes::Text, [label, ":"]) => self.assemble_label(label)?,

            // Control
            (Modes::Text, ["nop"]) => self.append(I::Nop)?,
            (Modes::Text, ["halt"]) => self.append(I::Halt)?,
            (Modes::Text, ["trap", imm]) =>  {
                let offset = self.rimmi(imm, 0, MAX_U12, Relocation::new(self.bytes.len() as u32, String::new(), 0, Relative, Signed, 2, 12, 0, 1))?;
                self.append(I::SoftwareTrap { imm: offset as u32 })?;
            }
            (Modes::Text, ["syscall"]) => self.append(I::SystemCall)?,
            (Modes::Text, ["iret"]) => self.append(I::IRet)?,
            (Modes::Text, ["ei"]) => self.append(I::EI)?,
            (Modes::Text, ["di"]) => self.append(I::DI)?,
            (Modes::Text, ["rdpc", rd]) => self.append(I::RdPc { rd: reg(rd)? })?,
            (Modes::Text, ["mrs", rd, cr]) => self.append(I::Mrs { rd: reg(rd)?, creg4: creg(cr)? })?,
            (Modes::Text, ["msr", cr, rs]) => self.append(I::Msr { creg4: creg(cr)?, rs: reg(rs)? })?,

            // Arithmetic
            (Modes::Text, ["add", rd, ra, rb]) => self.append(I::Add { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["sub", rd, ra, rb]) => self.append(I::Sub { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["and", rd, ra, rb]) => self.append(I::And { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["or",  rd, ra, rb]) => self.append(I::Or  { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["xor", rd, ra, rb]) => self.append(I::Xor { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["shl", rd, ra, rb]) => self.append(I::Shl { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["shr", rd, ra, rb]) => self.append(I::Shr { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["sar", rd, ra, rb]) => self.append(I::Sar { rd: reg(rd)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["neg", rd, ra]) => self.append(I::Neg { rd: reg(rd)?, ra: reg(ra)? })?,
            (Modes::Text, ["not", rd, ra]) => self.append(I::Not { rd: reg(rd)?, ra: reg(ra)? })?,
            (Modes::Text, ["cmp", ra, rb]) => self.append(I::Cmp { ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["addi", rd, ra, imm]) => self.append(I::Addi { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, MIN_I16, MAX_I16)? as u32 })?,
            (Modes::Text, ["subi", rd, ra, imm]) => self.append(I::Subi { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, MIN_I16, MAX_I16)? as u32 })?,
            (Modes::Text, ["andi", rd, ra, imm]) => self.append(I::Andi { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, MIN_I16, MAX_I16)? as u32 })?,
            (Modes::Text, ["ori",  rd, ra, imm]) => self.append(I::Ori  { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, MIN_I16, MAX_I16)? as u32 })?,
            (Modes::Text, ["xori", rd, ra, imm]) => self.append(I::Xori { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, MIN_I16, MAX_I16)? as u32 })?,
            (Modes::Text, ["shli", rd, ra, imm]) => self.append(I::Shli { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, 0, 32)? as u8 })?,
            (Modes::Text, ["shri", rd, ra, imm]) => self.append(I::Shri { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, 0, 32)? as u8 })?,
            (Modes::Text, ["sari", rd, ra, imm]) => self.append(I::Sari { rd: reg(rd)?, ra: reg(ra)?, imm: immi(imm, 0, 32)? as u8 })?,
            (Modes::Text, ["cmpi", ra, imm]) => self.append(I::Cmpi { ra: reg(ra)?, imm: immi(imm, MIN_I16, MAX_I16)? as u32 })?,

            // Bitwise
            (Modes::Text, ["btst", ra, imm]) => self.append(I::Btst { ra: reg(ra)?, imm: immu(imm, 0, 32)? as u8 })?,
            (Modes::Text, ["bset", rd, ra, imm]) => self.append(I::Bset { rd: reg(rd)?, ra: reg(ra)?, imm: immu(imm, 0, 32)? as u8 })?,
            (Modes::Text, ["bclr", rd, ra, imm]) => self.append(I::Bclr { rd: reg(rd)?, ra: reg(ra)?, imm: immu(imm, 0, 32)? as u8})?,
            (Modes::Text, ["btgl", rd, ra, imm]) => self.append(I::Btgl { rd: reg(rd)?, ra: reg(ra)?, imm: immu(imm, 0, 32)? as u8})?,

            // Multiply/Divide
            (Modes::Text, ["mul",  rd0, rd1, ra, rb]) => self.append(I::Mul  { rd0: reg(rd0)?, rd1: reg(rd1)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["mulu", rd0, rd1, ra, rb]) => self.append(I::Mulu { rd0: reg(rd0)?, rd1: reg(rd1)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["div",  rd0, rd1, ra, rb]) => self.append(I::Div  { rd0: reg(rd0)?, rd1: reg(rd1)?, ra: reg(ra)?, rb: reg(rb)? })?,
            (Modes::Text, ["divu", rd0, rd1, ra, rb]) => self.append(I::Divu { rd0: reg(rd0)?, rd1: reg(rd1)?, ra: reg(ra)?, rb: reg(rb)? })?,

            // Load and Store
            (Modes::Text, ["lb",  rd, base, offset]) => self.append(I::Lb  { rd: reg(rd)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["lbu", rd, base, offset]) => self.append(I::Lbu { rd: reg(rd)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["lh",  rd, base, offset]) => self.append(I::Lh  { rd: reg(rd)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["lhu", rd, base, offset]) => self.append(I::Lhu { rd: reg(rd)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["lw",  rd, base, offset]) => self.append(I::Lw  { rd: reg(rd)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["sb",  rs, base, offset]) => self.append(I::Sb  { rs: reg(rs)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["sh",  rs, base, offset]) => self.append(I::Sh  { rs: reg(rs)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,
            (Modes::Text, ["sw",  rs, base, offset]) => self.append(I::Sw  { rs: reg(rs)?, base: reg(base)?, offset: immi(offset, MIN_I14, MAX_I14)? })?,

            // Constants
            (Modes::Text, ["lui", rd, imm]) => self.append(I::Lui { rd: reg(rd)?, imm16: immu(imm, 0, MAX_U16)? as u16 })?,
            (Modes::Text, ["lli", rd, imm]) => self.append(I::Lli { rd: reg(rd)?, imm16: immu(imm, 0, MAX_U16)? as u16 })?,
            (Modes::Text, ["lhi", rd, imm]) => self.append(I::Lhi { rd: reg(rd)?, imm16: immu(imm, 0, MAX_U16)? as u16 })?,

            // Jump/Call
            (Modes::Text, ["jmp", target]) => {
                let offset = self.rimmi(target, MIN_I28, MAX_I28, Relocation::new(self.bytes.len() as u32, String::new(), -4, Relative, Signed, 2, 26, 0, 1))?;
                self.append(I::Jmp { offset })?;
            }
            (Modes::Text, ["call", target]) => {
                let offset = self.rimmi(target, MIN_I28, MAX_I28, Relocation::new(self.bytes.len() as u32, String::new(), -4, Relative, Signed, 2, 26, 0, 1))?;
                self.append(I::Call { offset })?;
            }
            (Modes::Text, ["jr", target]) => self.append(I::Jr { target: reg(target)? })?,
            (Modes::Text, ["jalr", rd, target]) => self.append(I::Jalr { rd: reg(rd)?, target: reg(target)? })?,

            // Branches
            (Modes::Text, [b, ra, rb, target]) if b.starts_with("b.") => {
                let ra = reg(ra)?;
                let rb = reg(rb)?;
                let offset = self.rimmi(target, MIN_I16, MAX_I16, Relocation::new(self.bytes.len() as u32, String::new(), -4, Relative, Signed, 2, 14, 8, 1))?;
                let instruction = match *b {
                    "b.eq" => Instruction::BEq { ra, rb, offset },
                    "b.ne" => Instruction::BNe { ra, rb, offset },
                    "b.lt" => Instruction::BLt { ra, rb, offset },
                    "b.le" => Instruction::BLe { ra, rb, offset },
                    "b.gt" => Instruction::BGt { ra, rb, offset },
                    "b.ge" => Instruction::BGe { ra, rb, offset },
                    "b.ltu" => Instruction::BLtu { ra, rb, offset },
                    "b.leu" => Instruction::BLeu { ra, rb, offset },
                    "b.gtu" => Instruction::BGtu { ra, rb, offset },
                    "b.geu" => Instruction::BGeu { ra, rb, offset },
                    _ => return Err(ParseError::InvalidInstruction(format!("Instruction unknown: {}", b))),
                };
                self.append(instruction)?;
            }
            (Modes::Text, [b, target]) if b.starts_with("bf.") => {
                let offset = self.rimmi(target, MIN_I24, MAX_I24, Relocation::new(self.bytes.len() as u32, String::new(), -4, Relative, Signed, 2, 22, 0, 1))?;
                let instruction = match *b {
                    "bf.eq" => Instruction::BfEq { offset },
                    "bf.ne" => Instruction::BfNe { offset },
                    "bf.lt" => Instruction::BfLt { offset },
                    "bf.le" => Instruction::BfLe { offset },
                    "bf.gt" => Instruction::BfGt { offset },
                    "bf.ge" => Instruction::BfGe { offset },
                    "bf.ltu" => Instruction::BfLtu { offset },
                    "bf.leu" => Instruction::BfLeu { offset },
                    "bf.gtu" => Instruction::BfGtu { offset },
                    "bf.geu" => Instruction::BfGeu { offset },
                    "bf.cs" => Instruction::BfCs { offset },
                    "bf.cc" => Instruction::BfCc { offset },
                    "bf.vs" => Instruction::BfVs { offset },
                    "bf.vc" => Instruction::BfVc { offset },
                    "bf.es" => Instruction::BfEs { offset },
                    "bf.ec" => Instruction::BfEc { offset },
                    _ => return Err(ParseError::InvalidInstruction(format!("Instruction unknown: {}", b))),
                };
                self.append(instruction)?;
            }

            // Pseudo-instructions
            (Modes::Text, ["clr", rd]) => self.append(I::And { rd: reg(rd)?, ra: 0, rb: 0 })?,
            (Modes::Text, ["mov", rd, rs]) => self.append(I::Or { rd: reg(rd)?, ra: reg(rs)?, rb: 0 })?,
            (Modes::Text, ["inc", rd]) => self.append(I::Addi { rd: reg(rd)?, ra: reg(rd)?, imm: 1 })?,
            (Modes::Text, ["dec", rd]) => self.append(I::Subi { rd: reg(rd)?, ra: reg(rd)?, imm: 1 })?,
            (Modes::Text, ["ret"]) => self.append(I::Jr { target: 15 })?,
            (Modes::Text, ["li", rd, imm]) => {
                let rd = reg(rd)?;
                let imm = immu(imm, 0, MAX_U32)?;
                self.append(I::Lli { rd, imm16: (imm & 0xFFFF) as u16 })?;
                self.append(I::Lhi { rd, imm16: (imm >> 16) as u16 })?;
            }
            (Modes::Text, ["la", rd, label]) => {
                let rd = reg(rd)?;
                self.mangle_name(label)?;
                let mangled = self.mangles.get(*label).cloned().unwrap();
                self.relocations.push(Relocation::new(self.bytes.len() as u32, mangled.clone(), 0, Absolute, Unsigned, 0, 16, 4, 0));
                self.append(Instruction::Lli { rd, imm16: 0 })?;
                self.relocations.push(Relocation::new(self.bytes.len() as u32, mangled, 0, Absolute, Unsigned, 16, 16, 4, 0));
                self.append(Instruction::Lhi { rd, imm16: 0 })?;
            }
            (Modes::Text, ["push", regs @ ..]) => {
                for r in regs {
                    let rs = reg(r)?;
                    self.append(Instruction::Subi { rd: 14, ra: 14, imm: 4 })?;
                    self.append(Instruction::Sw { rs, base: 14, offset: 0 })?;
                }
            }
            (Modes::Text, ["pop", regs @ ..]) => {
                for r in regs {
                    let rd = reg(r)?;
                    self.append(Instruction::Lw { rd, base: 14, offset: 0 })?;
                    self.append(Instruction::Addi { rd: 14, ra: 14, imm: 4 })?;
                }
            }

            _ => {
                println!("Unknown Instruction: {}", line);
                self.bytes.extend_from_slice(&[0u8; 4]);
            }
        }}
        Ok(())
    }

    fn insert_data(&mut self, filename: &str) -> ParseResult {
        let bytes: Vec<u8> = std::fs::read(filename)?;
        self.bytes.extend_from_slice(&bytes);
        Ok(())
    }

    fn append(&mut self, instruction: Instruction) -> ParseResult {
        // println!("{:08X}: {}", self.bytes.len(), instruction);
        self.bytes
            .extend_from_slice(&encode(instruction)?.to_be_bytes());
        Ok(())
    }

    fn add_constant(&mut self, name: &str, value: &str) -> ParseResult {
        validate_symbol_name(name)?;
        if self.constants.contains_key(name) {
            return Err(ParseError::DuplicateLabel(format!(
                "Constant {} already defined.",
                name
            )));
        }
        self.constants.insert(name.to_owned(), value.to_owned());
        Ok(())
    }

    fn assemble_label(&mut self, label: &str) -> ParseResult {
        self.check_label_already_known(label)?;
        validate_symbol_name(label)?;
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
        validate_symbol_name(label)?;
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
            f => {
                return Err(ParseError::InvalidInstruction(format!(
                    "Invalid instruction: {}",
                    f
                )));
            }
        };
        Ok(())
    }

    fn append_signed(&mut self, label: &str, mangled: String, data: &str, bits: u8) -> ParseResult {
        self.ensure_alignment(bits >> 3)?;
        let address = self.bytes.len() as u32;
        let data = immi(data, -(1_i64 << (bits - 1)), (1_i64 << (bits - 1)) - 1)?;
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
                base: Absolute,
                sign: Unsigned,
                value_shift: 0,
                width: 32,
                field_shift: 0,
                bounds_check: 0,
            });
            0
        } else {
            immu(data, 0, (1i64 << bits) - 1)?
        };
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
        let data = parse_string_literal(data)?;
        let mut target = self.bytes.len() + data.len();
        if zero_term {
            target += 1;
        }
        if target > u32::MAX as usize {
            return Err(ParseError::DataAlignmentError(format!(
                "Calculated data size is too large: {}",
                target
            )));
        }
        let address = self.bytes.len() as u32;
        self.bytes.extend_from_slice(&data);
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
        let alignment = immu(data, 0, 0)? as u8;
        self.ensure_alignment(alignment)?;
        let address = self.bytes.len() as u32;
        let size = immu(data2, 0, 0)? as usize;
        let target = self.bytes.len() + size;
        if target > u32::MAX as usize {
            return Err(ParseError::DataAlignmentError(format!(
                "Calculated data size is too large: {}",
                target
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
        validate_symbol_name(label)?;
        self.mangle_name(label)?;

        let alignment = immu(alignment, 0, 0)?;
        let size = immu(size, 0, 0)?;
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
        validate_symbol_name(label)?;
        if self.mangles.contains_key(label) {
            return Err(ParseError::InvalidInstruction(format!(
                ".nomangle after usage: {}",
                label
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
                "Cannot place items in .bss section",
            )));
        }
        let size = immu(location, 0, 0)? as usize;
        if self.bytes.len() < size {
            self.bytes.resize(size, 0);
        }
        Ok(())
    }

    // This function is used whenever we need to add a new label, to
    // ensure we are not adding it twice. This checks symbols and bss.
    fn check_label_already_known(&mut self, label: &str) -> ParseResult {
        if self.symbols.contains_key(label) || self.bss.contains_key(label) {
            return Err(ParseError::DuplicateLabel(format!(
                "Duplicate label: '{}'",
                label
            )));
        }
        Ok(())
    }

    fn tokenize(&mut self, line: &str) -> Result<Vec<String>, ParseError> {
        let line_len = line.len();
        let (prefix, string_token) = match line.find('"') {
            Some(index) => (&line[..index], Some(line[index..line_len].trim())),
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

        Ok(parts)
    }

    fn rimmi(
        &mut self,
        imm: &str,
        min: i64,
        max: i64,
        mut rel: Relocation,
    ) -> Result<i32, ParseError> {
        if is_valid_symbol_name(imm) {
            self.mangle_name(imm)?;
            rel.symbol = self.mangles.get(imm).cloned().unwrap();
            self.relocations.push(rel);
            Ok(0)
        } else {
            immi(imm, min, max)
        }
    }
}

fn creg(cr: &str) -> Result<Creg, ParseError> {
    let r = match cr {
        "%pc" => Creg::PC,
        "%sr" => Creg::SR,
        "%epc" => Creg::EPC,
        "%esr" => Creg::ESR,
        "%ecause" => Creg::ECause,
        "%edata" => Creg::EData,
        "%evbase" => Creg::EvBase,
        _ => {
            return Err(ParseError::InvalidName(format!("Invalid creg: {}", cr)));
        }
    };
    Ok(r)
}

fn reg(value: &str) -> Result<u8, ParseError> {
    let reg = immu(value, 0, 0xF)?;
    if reg > 0xF {
        Err(ParseError::InvalidNumber(format!(
            "Invalid register: {}",
            value
        )))
    } else {
        Ok(reg as u8)
    }
}

fn parse_string_literal(value: &str) -> Result<Vec<u8>, ParseError> {
    if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
        return Err(ParseError::InvalidStringLiteral(value.to_owned()));
    }

    let inner = &value[1..value.len() - 1];

    let mut output = Vec::new();
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut buffer = [0; 4];
            output.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            continue;
        }

        let escaped = chars
            .next()
            .ok_or_else(|| ParseError::InvalidStringLiteral("\\".to_owned()))?;

        match escaped {
            'n' => output.push(b'\n'),
            'r' => output.push(b'\r'),
            't' => output.push(b'\t'),
            '0' => output.push(0),
            '"' => output.push(b'"'),
            '\\' => output.push(b'\\'),

            'x' => {
                let hi = chars
                    .next()
                    .ok_or_else(|| ParseError::InvalidStringLiteral("\\x".to_owned()))?;

                let lo = chars
                    .next()
                    .ok_or_else(|| ParseError::InvalidStringLiteral(format!("\\x{hi}")))?;

                let hex = format!("{hi}{lo}");

                let byte = u8::from_str_radix(&hex, 16)
                    .map_err(|_| ParseError::InvalidStringLiteral(format!("\\x{hex}")))?;

                output.push(byte);
            }

            other => {
                return Err(ParseError::InvalidStringLiteral(format!("\\{other}")));
            }
        }
    }

    Ok(output)
}

fn is_valid_symbol_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn validate_symbol_name(name: &str) -> ParseResult {
    if is_valid_symbol_name(name) {
        return Ok(());
    }
    Err(ParseError::InvalidName(format!("Invalid name: '{}'", name)))
}

fn immi(text: &str, min: i64, max: i64) -> Result<i32, ParseError> {
    let text = text.trim();

    let num = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i32::from_str_radix(hex, 16)
    } else {
        text.parse::<i32>()
    }
    .map_err(|_| ParseError::InvalidNumber(text.to_owned()))?;
    assert_range(num as i64, min, max)?;
    Ok(num)
}

fn immu(text: &str, min: i64, max: i64) -> Result<u32, ParseError> {
    let text = text.trim();

    let num = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        text.parse::<u32>()
    }
    .map_err(|_| ParseError::InvalidNumber(text.to_owned()))?;
    assert_range(num as i64, min, max)?;
    Ok(num)
}

fn assert_range(number: i64, min: i64, max: i64) -> ParseResult {
    // Skip test if min == max
    if min != max && (number < min || number > max) {
        return Err(ParseError::InvalidNumber(format!(
            "Number outside of data range: {}",
            number
        )));
    };
    Ok(())
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
