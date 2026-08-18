use super::Creg;

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Instruction {
    // Control/X-type
    Nop,
    Halt,
    SoftwareTrap { imm: u32 },
    SystemCall,
    IRet,
    EI,
    DI,
    RdPc { rd: u8 },
    Mrs { creg4: Creg, rd: u8 },
    Msr { creg4: Creg, rs: u8 },

    // Register ALU/R-type
    Add { rd: u8, ra: u8, rb: u8 },
    Sub { rd: u8, ra: u8, rb: u8 },
    And { rd: u8, ra: u8, rb: u8 },
    Or { rd: u8, ra: u8, rb: u8 },
    Xor { rd: u8, ra: u8, rb: u8 },
    Not { rd: u8, ra: u8 },
    Neg { rd: u8, ra: u8 },
    Cmp { ra: u8, rb: u8 },

    // Immediate Arithmetic and Compare/I-type
    Addi { rd: u8, ra: u8, imm: u32 },
    Subi { rd: u8, ra: u8, imm: u32 },
    Cmpi { ra: u8, imm: u32 },

    // Immediate logical/I-type
    Andi { rd: u8, ra: u8, imm: u32 },
    Ori { rd: u8, ra: u8, imm: u32 },
    Xori { rd: u8, ra: u8, imm: u32 },

    // Register shifts/R-type
    Shl { rd: u8, ra: u8, rb: u8 },
    Shr { rd: u8, ra: u8, rb: u8 },
    Sar { rd: u8, ra: u8, rb: u8 },

    // Immediate shifts/I-type
    Shli { rd: u8, ra: u8, imm: u8 },
    Shri { rd: u8, ra: u8, imm: u8 },
    Sari { rd: u8, ra: u8, imm: u8 },

    // Bit immediate/I-type
    Btst { ra: u8, imm: u8 },
    Bset { rd: u8, ra: u8, imm: u8 },
    Bclr { rd: u8, ra: u8, imm: u8 },
    Btgl { rd: u8, ra: u8, imm: u8 },
    
    // Multiply/Divide/R2-type
    Mul { rd0: u8, rd1: u8, ra: u8, rb: u8 },
    Mulu { rd0: u8, rd1: u8, ra: u8, rb: u8 },
    Div { rd0: u8, rd1: u8, ra: u8, rb: u8 },
    Divu { rd0: u8, rd1: u8, ra: u8, rb: u8 },

    // Constant construction/U-type
    Lui { rd: u8, imm16: u16 },
    Lli { rd: u8, imm16: u16 },
    Lhi { rd: u8, imm16: u16 },

    // Load/Store/M-type
    Lb { rd: u8, base: u8, offset: i32 },
    Lbu { rd: u8, base: u8, offset: i32 },
    Lh { rd: u8, base: u8, offset: i32 },
    Lhu { rd: u8, base: u8, offset: i32 },
    Lw { rd: u8, base: u8, offset: i32 },
    Sb { rs: u8, base: u8, offset: i32 },
    Sh { rs: u8, base: u8, offset: i32 },
    Sw { rs: u8, base: u8, offset: i32 },

    // Flag branch/BF-type
    BfEq { offset: i32 },
    BfNe { offset: i32 },
    BfLt { offset: i32 },
    BfLe { offset: i32 },
    BfGt { offset: i32 },
    BfGe { offset: i32 },
    BfLtu { offset: i32 },
    BfLeu { offset: i32 },
    BfGtu { offset: i32 },
    BfGeu { offset: i32 },
    BfCs { offset: i32 },
    BfCc { offset: i32 },
    BfVs { offset: i32 },
    BfVc { offset: i32 },
    BfEs { offset: i32 },
    BfEc { offset: i32 },

    // Register branch/BC-type
    BEq { ra: u8, rb: u8, offset: i32 },
    BNe { ra: u8, rb: u8, offset: i32 },
    BLt { ra: u8, rb: u8, offset: i32 },
    BLe { ra: u8, rb: u8, offset: i32 },
    BGt { ra: u8, rb: u8, offset: i32 },
    BGe { ra: u8, rb: u8, offset: i32 },
    BLtu { ra: u8, rb: u8, offset: i32 },
    BLeu { ra: u8, rb: u8, offset: i32 },
    BGtu { ra: u8, rb: u8, offset: i32 },
    BGeu { ra: u8, rb: u8, offset: i32 },

    // PC-relative jump/call/J-type
    Jmp { offset: i32 },
    Call { offset: i32 },

    // Register jump/call/JR-type
    Jr { target: u8 },
    Jalr { rd: u8, target: u8 },
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! out {
            ($($arg:tt)*) => {
                write!(f, $($arg)*)
            };
        }

        match self {
            // Control/X-type
            Self::Nop => out!("nop"),
            Self::Halt => out!("halt"),
            Self::SoftwareTrap { imm } => out!("{:<8}{:03X}", "trap", imm),
            Self::SystemCall => out!("syscall"),
            Self::IRet => out!("iret"),
            Self::EI => out!("ei"),
            Self::DI => out!("di"),
            Self::RdPc { rd } => out!("{:<8}${}", "rdpc", rd),
            Self::Mrs { creg4, rd } => out!("{:<8}{}, ${}", "mrs", creg4, rd),
            Self::Msr { creg4, rs } => out!("{:<8}{}, ${}", "msr", creg4, rs),

            // Register ALU/R-type
            Self::Add { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "add", rd, ra, rb),
            Self::Sub { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "sub", rd, ra, rb),
            Self::And { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "and", rd, ra, rb),
            Self::Or { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "or", rd, ra, rb),
            Self::Xor { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "xor", rd, ra, rb),
            Self::Not { rd, ra } => out!("{:<8}${}, ${}", "not", rd, ra),
            Self::Neg { rd, ra } => out!("{:<8}${}, ${}", "neg", rd, ra),
            Self::Cmp { ra, rb } => out!("{:<8}${}, ${}", "cmp", ra, rb),

            // Immediate Arithmetic and Compare/I-type
            Self::Addi { rd, ra, imm } => out!("{:<8}${}, ${}, {:04X}", "addi", rd, ra, imm),
            Self::Subi { rd, ra, imm } => out!("{:<8}${}, ${}, {:04X}", "subi", rd, ra, imm),
            Self::Cmpi { ra, imm } => out!("{:<8}${}, {:04X}", "cmpi", ra, imm),

            // Immediate logical/I-type
            Self::Andi { rd, ra, imm } => out!("{:<8}${}, ${}, {:04X}", "andi", rd, ra, imm),
            Self::Ori { rd, ra, imm } => out!("{:<8}${}, ${}, {:04X}", "ori", rd, ra, imm),
            Self::Xori { rd, ra, imm } => out!("{:<8}${}, ${}, {:04X}", "xori", rd, ra, imm),

            // Register shifts/R-type
            Self::Shl { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "shl", rd, ra, rb),
            Self::Shr { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "shr", rd, ra, rb),
            Self::Sar { rd, ra, rb } => out!("{:<8}${}, ${}, ${}", "sar", rd, ra, rb),

            // Immediate shifts/I-type
            Self::Shli { rd, ra, imm } => out!("{:<8}${}, ${}, {}", "shli", rd, ra, imm),
            Self::Shri { rd, ra, imm } => out!("{:<8}${}, ${}, {}", "shri", rd, ra, imm),
            Self::Sari { rd, ra, imm } => out!("{:<8}${}, ${}, {}", "sari", rd, ra, imm),

            // Bit immediate/I-type
            Self::Btst { ra, imm } => out!("{:<8}${}, {}", "btst", ra, imm),
            Self::Bset { rd, ra, imm } => out!("{:<8}${}, ${}, {}", "bset", rd, ra, imm),
            Self::Bclr { rd, ra, imm } => out!("{:<8}${}, ${}, {}", "bclr", rd, ra, imm),
            Self::Btgl { rd, ra, imm } => out!("{:<8}${}, ${}, {}", "btgl", rd, ra, imm),

            // Multiply/Divide/R2-type
            Self::Mul { rd0, rd1, ra, rb } => {
                out!("{:<8}${}, ${}, ${}, ${}", "mul", rd0, rd1, ra, rb)
            }
            Self::Mulu { rd0, rd1, ra, rb } => {
                out!("{:<8}${}, ${}, ${}, ${}", "mulu", rd0, rd1, ra, rb)
            }
            Self::Div { rd0, rd1, ra, rb } => {
                out!("{:<8}${}, ${}, ${}, ${}", "div", rd0, rd1, ra, rb)
            }
            Self::Divu { rd0, rd1, ra, rb } => {
                out!("{:<8}${}, ${}, ${}, ${}", "divu", rd0, rd1, ra, rb)
            }

            // Constant construction/U-type
            Self::Lui { rd, imm16 } => out!("{:<8}${}, {:04X}", "lui", rd, imm16),
            Self::Lli { rd, imm16 } => out!("{:<8}${}, {:04X}", "lli", rd, imm16),
            Self::Lhi { rd, imm16 } => out!("{:<8}${}, {:04X}", "lhi", rd, imm16),

            // Load/Store/M-type
            Self::Lb { rd, base, offset } => out!("{:<8}${}, [${}, {}]", "lb", rd, base, offset),
            Self::Lbu { rd, base, offset } => out!("{:<8}${}, [${}, {}]", "lbu", rd, base, offset),
            Self::Lh { rd, base, offset } => out!("{:<8}${}, [${}, {}]", "lh", rd, base, offset),
            Self::Lhu { rd, base, offset } => out!("{:<8}${}, [${}, {}]", "lhu", rd, base, offset),
            Self::Lw { rd, base, offset } => out!("{:<8}${}, [${}, {}]", "lb", rd, base, offset),
            Self::Sb { rs, base, offset } => out!("{:<8}${}, [${}, {}]", "sb", rs, base, offset),
            Self::Sh { rs, base, offset } => out!("{:<8}${}, [${}, {}]", "sh", rs, base, offset),
            Self::Sw { rs, base, offset } => out!("{:<8}${}, [${}, {}]", "sw", rs, base, offset),

            // Flag branch/BF-type
            Self::BfEq { offset } => out!("{:<8}{}", "bf.eq", offset),
            Self::BfNe { offset } => out!("{:<8}{}", "bf.ne", offset),
            Self::BfLt { offset } => out!("{:<8}{}", "bf.lt", offset),
            Self::BfLe { offset } => out!("{:<8}{}", "bf.le", offset),
            Self::BfGt { offset } => out!("{:<8}{}", "bf.gt", offset),
            Self::BfGe { offset } => out!("{:<8}{}", "bf.gte", offset),
            Self::BfLtu { offset } => out!("{:<8}{}", "bf.ltu", offset),
            Self::BfLeu { offset } => out!("{:<8}{}", "bf.leu", offset),
            Self::BfGtu { offset } => out!("{:<8}{}", "bf.gtu", offset),
            Self::BfGeu { offset } => out!("{:<8}{}", "bf.geu", offset),
            Self::BfCs { offset } => out!("{:<8}{}", "bf.cs", offset),
            Self::BfCc { offset } => out!("{:<8}{}", "bf.cc", offset),
            Self::BfVs { offset } => out!("{:<8}{}", "bf.vs", offset),
            Self::BfVc { offset } => out!("{:<8}{}", "bf.vc", offset),
            Self::BfEs { offset } => out!("{:<8}{}", "bf.es", offset),
            Self::BfEc { offset } => out!("{:<8}{}", "bf.ec", offset),

            // Register branch/BC-type
            Self::BEq { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.eq", ra, rb, offset),
            Self::BNe { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.ne", ra, rb, offset),
            Self::BLt { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.lt", ra, rb, offset),
            Self::BLe { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.le", ra, rb, offset),
            Self::BGt { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.gt", ra, rb, offset),
            Self::BGe { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.ge", ra, rb, offset),
            Self::BLtu { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.ltu", ra, rb, offset),
            Self::BLeu { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.leu", ra, rb, offset),
            Self::BGtu { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.gtu", ra, rb, offset),
            Self::BGeu { ra, rb, offset } => out!("{:<8}${}, ${}, {}", "b.geu", ra, rb, offset),

            // PC-relative jump/call/J-type
            Self::Jmp { offset } => out!("{:<8}{}", "jmp", offset),
            Self::Call { offset } => out!("{:<8}{}", "call", offset),

            // Register jump/call/JR-type
            Self::Jr { target } => out!("{:<8}${}", "jr", target),
            Self::Jalr { rd, target } => out!("{:<8}${}, ${}", "jalr", rd, target),
        }
    }
}
