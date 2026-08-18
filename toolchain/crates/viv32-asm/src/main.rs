use anyhow::Result;
use clap::Parser;
use nofmt::pls;
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
    InvalidStringLiteral(String),
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
            ParseError::InvalidStringLiteral(message) => write!(f, "{}", message),
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
        ("%zero".to_owned(), "0".to_owned()),
        ("%fp".to_owned(), "13".to_owned()),
        ("%sp".to_owned(), "14".to_owned()),
        ("%lr".to_owned(), "15".to_owned()),
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

    fn assemble(&mut self, input_file: File) -> ParseResult {
        let reader = BufReader::new(input_file);

        for line in reader.lines() {
            self.line_no += 1;
            let Some(line) = strip_spaces_comments(&line?) else {
                continue;
            };
            let parts = self.tokenize(&line);

            pls! {
                match (
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
                    (Modes::Data, [label, ":", format, data]) => self.assemble_data(label, format, data, "0")?,
                    (Modes::RoData, [label, ":", format, data]) => self.assemble_data(label, format, data, "0")?,
                    (Modes::Text, [label, ":"]) => self.assemble_label(label)?,

                    // Control
                    (Modes::Text, ["nop"]) => self.append_instruction(Instruction::Nop)?,
                    (Modes::Text, ["halt"]) => self.append_instruction(Instruction::Halt)?,
                    (Modes::Text, ["trap", imm]) => self.assemble_trap(imm)?,
                    (Modes::Text, ["syscall"]) => self.append_instruction(Instruction::SystemCall)?,
                    (Modes::Text, ["iret"]) => self.append_instruction(Instruction::IRet)?,
                    (Modes::Text, ["ei"]) => self.append_instruction(Instruction::EI)?,
                    (Modes::Text, ["di"]) => self.append_instruction(Instruction::DI)?,
                    (Modes::Text, ["rdpc", rd]) => self.assemble_rdpc(rd)?,
                    (Modes::Text, ["mrs", rd, cr]) => self.assemble_mrs(rd, cr)?,
                    (Modes::Text, ["msr", cr, rs]) => self.assemble_msr(cr, rs)?,

                    // Arithmetic
                    (Modes::Text, ["add", rd, ra, rb]) => self.assemble_alu("add", rd, ra, rb)?,
                    (Modes::Text, ["sub", rd, ra, rb]) => self.assemble_alu("sub", rd, ra, rb)?,
                    (Modes::Text, ["and", rd, ra, rb]) => self.assemble_alu("and", rd, ra, rb)?,
                    (Modes::Text, ["or", rd, ra, rb]) => self.assemble_alu("or", rd, ra, rb)?,
                    (Modes::Text, ["xor", rd, ra, rb]) => self.assemble_alu("xor", rd, ra, rb)?,
                    (Modes::Text, ["shl", rd, ra, rb]) => self.assemble_alu("shl", rd, ra, rb)?,
                    (Modes::Text, ["shr", rd, ra, rb]) => self.assemble_alu("shr", rd, ra, rb)?,
                    (Modes::Text, ["sar", rd, ra, rb]) => self.assemble_alu("sar", rd, ra, rb)?,
                    (Modes::Text, ["neg", rd, ra]) => self.assemble_alu("neg", rd, ra, "0")?,
                    (Modes::Text, ["not", rd, ra]) => self.assemble_alu("not", rd, ra, "0")?,
                    (Modes::Text, ["cmp", ra, rb]) => self.assemble_alu("cmp", "0", ra, rb)?,
                    (Modes::Text, ["addi", rd, ra, imm]) => self.assemble_alui("addi", rd, ra, imm)?,
                    (Modes::Text, ["subi", rd, ra, imm]) => self.assemble_alui("subi", rd, ra, imm)?,
                    (Modes::Text, ["andi", rd, ra, imm]) => self.assemble_alui("andi", rd, ra, imm)?,
                    (Modes::Text, ["ori", rd, ra, imm]) => self.assemble_alui("ori", rd, ra, imm)?,
                    (Modes::Text, ["xori", rd, ra, imm]) => self.assemble_alui("xori", rd, ra, imm)?,
                    (Modes::Text, ["shli", rd, ra, imm]) => self.assemble_alui("shli", rd, ra, imm)?,
                    (Modes::Text, ["shri", rd, ra, imm]) => self.assemble_alui("shri", rd, ra, imm)?,
                    (Modes::Text, ["sari", rd, ra, imm]) => self.assemble_alui("sari", rd, ra, imm)?,
                    (Modes::Text, ["cmpi", ra, imm]) => self.assemble_alui("xori", "0", ra, imm)?,

                    // Bitwise
                    (Modes::Text, ["btst", ra, imm]) => self.assemble_bits("btst", "0", ra, imm)?,
                    (Modes::Text, ["bset", rd, ra, imm]) => self.assemble_bits("bset", rd, ra, imm)?,
                    (Modes::Text, ["bclr", rd, ra, imm]) => self.assemble_bits("bclr", rd, ra, imm)?,
                    (Modes::Text, ["btgl", rd, ra, imm]) => self.assemble_bits("btgl", rd, ra, imm)?,

                    // Multiply/Divide
                    (Modes::Text, ["mul", rd0, rd1, ra, rb]) => self.assemble_mul("mul", rd0, rd1, ra, rb)?,
                    (Modes::Text, ["mulu", rd0, rd1, ra, rb]) => self.assemble_mul("mulu", rd0, rd1, ra, rb)?,
                    (Modes::Text, ["div", rd0, rd1, ra, rb]) => self.assemble_mul("div", rd0, rd1, ra, rb)?,
                    (Modes::Text, ["divu", rd0, rd1, ra, rb]) => self.assemble_mul("divu", rd0, rd1, ra, rb)?,

                    // Load and Store
                    (Modes::Text, ["lb",  rd, base, offset]) => self.assemble_m("lb", rd, base, offset)?,
                    (Modes::Text, ["lbu", rd, base, offset]) => self.assemble_m("lbu",rd, base, offset)?,
                    (Modes::Text, ["lh",  rd, base, offset]) => self.assemble_m("lh", rd, base, offset)?,
                    (Modes::Text, ["lhu", rd, base, offset]) => self.assemble_m("lhu",rd, base, offset)?,
                    (Modes::Text, ["lw",  rd, base, offset]) => self.assemble_m("lw", rd, base, offset)?,
                    (Modes::Text, ["sb",  rd, base, offset]) => self.assemble_m("sb", rd, base, offset)?,
                    (Modes::Text, ["sh",  rd, base, offset]) => self.assemble_m("sh", rd, base, offset)?,
                    (Modes::Text, ["sw",  rd, base, offset]) => self.assemble_m("sw", rd, base, offset)?,

                    // Constants
                    (Modes::Text, ["lui", rd, imm]) => self.assemble_constant("lui", rd, imm)?,
                    (Modes::Text, ["lli", rd, imm]) => self.assemble_constant("lli", rd, imm)?,
                    (Modes::Text, ["lhi", rd, imm]) => self.assemble_constant("lhi", rd, imm)?,

                    // Jump/Call
                    (Modes::Text, ["jmp", label]) => self.assemble_jmp(label)?,
                    (Modes::Text, ["call", label]) => self.assemble_call(label)?,
                    (Modes::Text, ["jr", target]) => self.assemble_jr(target)?,
                    (Modes::Text, ["jalr", rd, target]) => self.assemble_jalr(rd, target)?,

                    // Branches
                    (Modes::Text, [b, ra, rb, label]) if b.starts_with("b.") => self.assemble_b(b, ra, rb, label)?,
                    (Modes::Text, [b, label]) if b.starts_with("bf.") => self.assemble_bf(b, label)?,

                    // Pseudo-instructions
                    (Modes::Text, ["clr", rd]) => self.assemble_alu("and", rd, "0", "0")?,
                    (Modes::Text, ["mov", rd, rs]) => self.assemble_alu("or", rd, rs, "0")?,
                    (Modes::Text, ["inc", rd]) => self.assemble_alui("addi", rd, rd, "1")?,
                    (Modes::Text, ["dec", rd]) => self.assemble_alui("subi", rd, rd, "1")?,
                    (Modes::Text, ["ret"]) => self.assemble_jr("15")?,
                    (Modes::Text, ["li", rd, imm]) => self.assemble_li(rd, imm)?,
                    (Modes::Text, ["la", rd, label]) => self.assemble_la(rd, label)?,
                    (Modes::Text, ["push", regs @ ..]) => self.assemble_push(regs)?,
                    (Modes::Text, ["pop", regs @ ..]) => self.assemble_pop(regs)?,

                    _ => {
                        println!("Unknown Instruction: {}", line);
                        self.bytes.extend_from_slice(&[0u8; 4]);
                    }
                }
            }
        }

        Ok(())
    }

    fn assemble_bits(&mut self, cmd: &str, rd: &str, ra: &str, imm: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        let ra = self.reg(ra)?;
        let imm = parse_unsigned_number(imm)?;
        self.assert_range(imm as i64, 0, 31)?;
        let imm = imm as u8;
        let instruction = match cmd {
            "btst" => Instruction::Btst { ra, imm },
            "bset" => Instruction::Bset { rd, ra, imm },
            "bclr" => Instruction::Bclr { rd, ra, imm },
            "btgl" => Instruction::Btgl { rd, ra, imm },
            _ => unreachable!(),
        };
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_mul(&mut self, cmd: &str, rd0: &str, rd1: &str, ra: &str, rb: &str) -> ParseResult {
        let rd0 = self.reg(rd0)?;
        let rd1 = self.reg(rd1)?;
        let ra = self.reg(ra)?;
        let rb = self.reg(rb)?;
        let instruction = match cmd {
            "mul" => Instruction::Mul { rd0, rd1, ra, rb },
            "mulu" => Instruction::Mulu { rd0, rd1, ra, rb },
            "div" => Instruction::Div { rd0, rd1, ra, rb },
            "divu" => Instruction::Divu { rd0, rd1, ra, rb },
            _ => unreachable!(),
        };
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_push(&mut self, regs: &[&str]) -> ParseResult {
        for reg in regs {
            let rs = self.reg(reg)?;
            // %sp == r14
            // subi %sp, %sp, 4
            // sw $rs, [%sp, 0]
            pls! {
                self.append_instruction(Instruction::Subi { rd: 14, ra: 14, imm: 4 })?;
                self.append_instruction(Instruction::Sw { rs, base: 14, offset: 0 })?;
            }
        }
        Ok(())
    }

    fn assemble_pop(&mut self, regs: &[&str]) -> ParseResult {
        for reg in regs {
            let rd = self.reg(reg)?;
            // %sp == r14
            // lw $rs, [%sp, 0]
            // addi %sp, %sp 4
            pls! {
                self.append_instruction(Instruction::Lw { rd, base: 14, offset: 0 })?;
                self.append_instruction(Instruction::Addi { rd: 14, ra: 14, imm: 4 })?;
            }
        }
        Ok(())
    }

    fn assemble_m(&mut self, m: &str, rds: &str, base: &str, offset: &str) -> ParseResult {
        let rds = self.reg(rds)?;
        let base = self.reg(base)?;
        let offset = parse_signed_number(offset)?;
        self.assert_range(offset as i64, -(1i64 << 14), (1i64 << 14) - 1)?;
        pls! {
            let instruction = match m {
                "lb" => Instruction::Lb { rd: rds, base, offset },
                "lbu" => Instruction::Lbu { rd: rds, base, offset },
                "lh" => Instruction::Lh { rd: rds, base, offset },
                "lhu" => Instruction::Lhu { rd: rds, base, offset },
                "lw" => Instruction::Lw { rd: rds, base, offset },
                "sb" => Instruction::Sb { rs: rds, base, offset },
                "sh" => Instruction::Sh { rs: rds, base, offset },
                "sw" => Instruction::Sw { rs: rds, base, offset },
                _ => unreachable!(),
            };
        }
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_constant(&mut self, ld: &str, rd: &str, imm: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        let imm = parse_unsigned_number(imm)?;
        if imm > u16::MAX as u32 {
            return Err(ParseError::InvalidNumber(format!(
                "{} too large for 16-bit field [{}:{}]",
                imm, self.filename, self.line_no
            )));
        }
        pls! {
            let instruction = match ld {
                "lui" => Instruction::Lui { rd, imm16: imm as u16 },
                "lli" => Instruction::Lli { rd, imm16: imm as u16 },
                "lhi" => Instruction::Lhi { rd, imm16: imm as u16 },
                _ => unreachable!(),
            };
        }
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_li(&mut self, rd: &str, imm: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        let imm = parse_unsigned_number(imm)?;
        pls! {
            self.append_instruction(Instruction::Lli { rd, imm16: (imm & 0xFFFF) as u16 })?;
            self.append_instruction(Instruction::Lhi { rd, imm16: (imm >> 16) as u16 })?;
        }
        Ok(())
    }

    fn assemble_la(&mut self, rd: &str, label: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        self.mangle_name(label)?;
        let mangled = self.mangles.get(label).cloned().unwrap();
        self.relocations.push(Relocation {
            patch_offset: self.bytes.len() as u32,
            symbol: mangled.clone(),
            addend: 0,
            base: RelocationBase::Absolute,
            sign: RelocationSign::Unsigned,
            value_shift: 0,
            width: 16,
            field_shift: 4,
            bounds_check: 0,
        });
        self.append_instruction(Instruction::Lli { rd, imm16: 0 })?;
        self.relocations.push(Relocation {
            patch_offset: self.bytes.len() as u32,
            symbol: mangled,
            addend: 0,
            base: RelocationBase::Absolute,
            sign: RelocationSign::Unsigned,
            value_shift: 16,
            width: 16,
            field_shift: 4,
            bounds_check: 0,
        });
        self.append_instruction(Instruction::Lhi { rd, imm16: 0 })?;
        Ok(())
    }

    fn assemble_b(&mut self, b: &str, ra: &str, rb: &str, label: &str) -> ParseResult {
        let ra = self.reg(ra)?;
        let rb = self.reg(rb)?;
        self.mangle_name(label)?;
        let mangled = self.mangles.get(label).cloned().unwrap();
        let instruction = match b {
            "b.eq" => Instruction::BEq { ra, rb, offset: 0 },
            "b.ne" => Instruction::BNe { ra, rb, offset: 0 },
            "b.lt" => Instruction::BLt { ra, rb, offset: 0 },
            "b.le" => Instruction::BLe { ra, rb, offset: 0 },
            "b.gt" => Instruction::BGt { ra, rb, offset: 0 },
            "b.ge" => Instruction::BGe { ra, rb, offset: 0 },
            "b.ltu" => Instruction::BLtu { ra, rb, offset: 0 },
            "b.leu" => Instruction::BLeu { ra, rb, offset: 0 },
            "b.gtu" => Instruction::BGtu { ra, rb, offset: 0 },
            "b.geu" => Instruction::BGeu { ra, rb, offset: 0 },
            _ => {
                return Err(ParseError::InvalidInstruction(format!(
                    "Instruction unknown: {} [{}:{}]",
                    b, self.filename, self.line_no
                )));
            }
        };
        self.relocations.push(Relocation {
            patch_offset: self.bytes.len() as u32,
            symbol: mangled,
            addend: -4,
            base: RelocationBase::Relative,
            sign: RelocationSign::Signed,
            value_shift: 2,
            width: 14,
            field_shift: 8,
            bounds_check: 1,
        });
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_bf(&mut self, b: &str, label: &str) -> ParseResult {
        self.mangle_name(label)?;
        let mangled = self.mangles.get(label).cloned().unwrap();
        let instruction = match b {
            "bf.eq" => Instruction::BfEq { offset: 0 },
            "bf.ne" => Instruction::BfNe { offset: 0 },
            "bf.lt" => Instruction::BfLt { offset: 0 },
            "bf.le" => Instruction::BfLe { offset: 0 },
            "bf.gt" => Instruction::BfGt { offset: 0 },
            "bf.ge" => Instruction::BfGe { offset: 0 },
            "bf.ltu" => Instruction::BfLtu { offset: 0 },
            "bf.leu" => Instruction::BfLeu { offset: 0 },
            "bf.gtu" => Instruction::BfGtu { offset: 0 },
            "bf.geu" => Instruction::BfGeu { offset: 0 },
            "bf.cs" => Instruction::BfCs { offset: 0 },
            "bf.cc" => Instruction::BfCc { offset: 0 },
            "bf.vs" => Instruction::BfVs { offset: 0 },
            "bf.vc" => Instruction::BfVc { offset: 0 },
            "bf.es" => Instruction::BfEs { offset: 0 },
            "bf.ec" => Instruction::BfEc { offset: 0 },
            _ => {
                return Err(ParseError::InvalidInstruction(format!(
                    "Instruction unknown: {} [{}:{}]",
                    b, self.filename, self.line_no
                )));
            }
        };
        self.relocations.push(Relocation {
            patch_offset: self.bytes.len() as u32,
            symbol: mangled,
            addend: -4,
            base: RelocationBase::Relative,
            sign: RelocationSign::Signed,
            value_shift: 2,
            width: 22,
            field_shift: 0,
            bounds_check: 1,
        });
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_jmp(&mut self, label: &str) -> ParseResult {
        self.mangle_name(label)?;
        let mangled = self.mangles.get(label).cloned().unwrap();
        self.relocations.push(Relocation {
            patch_offset: self.bytes.len() as u32,
            symbol: mangled,
            addend: -4,
            base: RelocationBase::Relative,
            sign: RelocationSign::Signed,
            value_shift: 2,
            width: 26,
            field_shift: 0,
            bounds_check: 1,
        });
        self.append_instruction(Instruction::Jmp { offset: 0 })?;
        Ok(())
    }

    fn assemble_call(&mut self, label: &str) -> ParseResult {
        self.mangle_name(label)?;
        let mangled = self.mangles.get(label).cloned().unwrap();
        self.relocations.push(Relocation {
            patch_offset: self.bytes.len() as u32,
            symbol: mangled,
            addend: -4,
            base: RelocationBase::Relative,
            sign: RelocationSign::Signed,
            value_shift: 2,
            width: 26,
            field_shift: 0,
            bounds_check: 1,
        });
        self.append_instruction(Instruction::Call { offset: 0 })?;
        Ok(())
    }

    fn assemble_jr(&mut self, rd: &str) -> ParseResult {
        let target = self.reg(rd)?;
        self.append_instruction(Instruction::Jr { target })?;
        Ok(())
    }

    fn assemble_jalr(&mut self, rd: &str, target: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        let target = self.reg(target)?;
        self.append_instruction(Instruction::Jalr { rd, target })?;
        Ok(())
    }

    fn assemble_trap(&mut self, imm: &str) -> ParseResult {
        let imm = parse_unsigned_number(imm)?;
        self.assert_range(imm as i64, 0, (0x1 << 12) - 1)?;
        self.append_instruction(Instruction::SoftwareTrap { imm })?;
        Ok(())
    }

    fn assemble_rdpc(&mut self, rd: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        self.append_instruction(Instruction::RdPc { rd })?;
        Ok(())
    }

    fn assemble_mrs(&mut self, rd: &str, cr: &str) -> ParseResult {
        let cr = match cr {
            "%pc" => Creg::PC,
            "%sr" => Creg::SR,
            "%epc" => Creg::EPC,
            "%esr" => Creg::ESR,
            "%ecause" => Creg::ECause,
            "%edata" => Creg::EData,
            "%evbase" => Creg::EvBase,
            _ => {
                return Err(ParseError::InvalidName(format!(
                    "Invalid creg: {} [{}:{}]",
                    cr, self.filename, self.line_no
                )));
            }
        };
        let rd = self.reg(rd)?;
        self.append_instruction(Instruction::Mrs { rd: rd, creg4: cr })?;
        Ok(())
    }

    fn assemble_msr(&mut self, cr: &str, rs: &str) -> ParseResult {
        let cr = match cr {
            "%pc" => Creg::PC,
            "%sr" => Creg::SR,
            "%epc" => Creg::EPC,
            "%esr" => Creg::ESR,
            "%ecause" => Creg::ECause,
            "%edata" => Creg::EData,
            "%evbase" => Creg::EvBase,
            _ => {
                return Err(ParseError::InvalidName(format!(
                    "Invalid creg: {} [{}:{}]",
                    cr, self.filename, self.line_no
                )));
            }
        };
        let rs = self.reg(rs)?;
        self.append_instruction(Instruction::Msr { creg4: cr, rs: rs })?;
        Ok(())
    }

    fn assemble_alu(&mut self, cmd: &str, rd: &str, ra: &str, rb: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        let ra = self.reg(ra)?;
        let rb = self.reg(rb)?;
        let instruction = match cmd {
            "add" => Instruction::Add { rd, ra, rb },
            "sub" => Instruction::Sub { rd, ra, rb },
            "and" => Instruction::And { rd, ra, rb },
            "or" => Instruction::Or { rd, ra, rb },
            "xor" => Instruction::Xor { rd, ra, rb },
            "shl" => Instruction::Shl { rd, ra, rb },
            "shr" => Instruction::Shr { rd, ra, rb },
            "sar" => Instruction::Sar { rd, ra, rb },
            "neg" => Instruction::Neg { rd, ra },
            "not" => Instruction::Not { rd, ra },
            "cmp" => Instruction::Cmp { ra, rb },
            _ => unreachable!(),
        };
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn assemble_alui(&mut self, cmd: &str, rd: &str, ra: &str, imm: &str) -> ParseResult {
        let rd = self.reg(rd)?;
        let ra = self.reg(ra)?;
        let imm = parse_signed_number(imm)?;
        self.assert_range(imm as i64, i16::MIN as i64, i16::MAX as i64)?;
        let imm = imm as u32;
        pls! {
            let instruction = match cmd {
                "addi" => Instruction::Addi { rd, ra, imm },
                "subi" => Instruction::Subi { rd, ra, imm },
                "andi" => Instruction::Andi { rd, ra, imm },
                "ori" => Instruction::Ori { rd, ra, imm },
                "xori" => Instruction::Xori { rd, ra, imm },
                "shli" => Instruction::Shli { rd, ra, imm: imm as u8 },
                "shri" => Instruction::Shri { rd, ra, imm: imm as u8 },
                "sari" => Instruction::Sari { rd, ra, imm: imm as u8 },
                "cmpi" => Instruction::Cmpi { ra, imm },
                _ => unreachable!(),
            };
        }
        self.append_instruction(instruction)?;
        Ok(())
    }

    fn append_instruction(&mut self, instruction: Instruction) -> ParseResult {
        // println!("{:08X}: {}", self.bytes.len(), instruction);
        self.bytes
            .extend_from_slice(&encode(instruction)?.to_be_bytes());
        Ok(())
    }

    fn reg(&mut self, value: &str) -> Result<u8, ParseError> {
        let reg = parse_unsigned_number(value)?;
        if reg > 0xF {
            Err(ParseError::InvalidNumber(format!(
                "Invalid register: {} [{}:{}]",
                value, self.filename, self.line_no
            )))
        } else {
            Ok(reg as u8)
        }
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
                bounds_check: 0,
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
        let data = parse_string_literal(data)?;
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
