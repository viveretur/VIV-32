use super::Creg;

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Instruction {
    // Control/X-type
    Nop,
    Halt,
    SoftwareTrap { imm: i32 },
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
