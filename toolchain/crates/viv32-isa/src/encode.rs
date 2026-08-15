use super::{
    Instruction,
    spec::{self, architecture, opcode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    InvalidImmediate { size: u8, value: u32 },
    InvalidPayload(u32),
    InvalidRegister(u8),
}

pub fn encode(inst: Instruction) -> Result<u32, EncodeError> {
    use opcode::{
        BIT_IMMEDIATE, IMMEDIATE_ARITHMETIC_COMPARE, IMMEDIATE_LOGICAL, IMMEDIATE_SHIFT, LOAD,
        REGISTER_ALU, REGISTER_SHIFT, STORE,
    };
    use spec::{
        condition,
        func::{
            multiply_divide as md, register_alu as r_alu, register_jump_call as r_jc,
            register_shift as r_shift,
        },
        memory_size as ms,
        mode::{
            bit_immediate, constant_construction as cc, immediate_arithmetic_compare as i_alu,
            immediate_logical as i_logical, immediate_shift as i_shift,
        },
        sysop,
    };

    match inst {
        // Control/X-type
        Instruction::Nop => encode_x(sysop::NOP, 0, 0, 0),
        Instruction::Halt => encode_x(sysop::HALT, 0, 0, 0),
        Instruction::SoftwareTrap { imm } => encode_x(sysop::TRAP, imm, 0, 0),
        Instruction::SystemCall => encode_x(sysop::SYSCALL, 0, 0, 0),
        Instruction::IRet => encode_x(sysop::IRET, 0, 0, 0),
        Instruction::EI => encode_x(sysop::EI, 0, 0, 0),
        Instruction::DI => encode_x(sysop::DI, 0, 0, 0),
        Instruction::RdPc { rd } => encode_x(sysop::RDPC, 0, rd, 0),
        Instruction::Mrs { creg4, rd } => encode_x(sysop::MRS, creg4 as u32, rd, 0),
        Instruction::Msr { creg4, rs } => encode_x(sysop::MSR, creg4 as u32, 0, rs),

        // Register ALU/R-type
        Instruction::Add { rd, ra, rb } => encode_r(REGISTER_ALU, r_alu::ADD, rd, ra, rb),
        Instruction::Sub { rd, ra, rb } => encode_r(REGISTER_ALU, r_alu::SUB, rd, ra, rb),
        Instruction::And { rd, ra, rb } => encode_r(REGISTER_ALU, r_alu::AND, rd, ra, rb),
        Instruction::Or { rd, ra, rb } => encode_r(REGISTER_ALU, r_alu::OR, rd, ra, rb),
        Instruction::Xor { rd, ra, rb } => encode_r(REGISTER_ALU, r_alu::XOR, rd, ra, rb),
        Instruction::Not { rd, ra } => encode_r(REGISTER_ALU, r_alu::NOT, rd, ra, 0),
        Instruction::Neg { rd, ra } => encode_r(REGISTER_ALU, r_alu::NEG, rd, ra, 0),
        Instruction::Cmp { ra, rb } => encode_r(REGISTER_ALU, r_alu::CMP, 0, ra, rb),

        // Immediate Arithmetic and Compare/I-type
        Instruction::Addi { rd, ra, imm } => {
            encode_i(IMMEDIATE_ARITHMETIC_COMPARE, i_alu::ADDI, rd, ra, imm)
        }
        Instruction::Subi { rd, ra, imm } => {
            encode_i(IMMEDIATE_ARITHMETIC_COMPARE, i_alu::SUBI, rd, ra, imm)
        }
        Instruction::Cmpi { ra, imm } => {
            encode_i(IMMEDIATE_ARITHMETIC_COMPARE, i_alu::CMPI, 0, ra, imm)
        }

        // Immediate logical/I-type
        Instruction::Andi { rd, ra, imm } => {
            encode_i(IMMEDIATE_LOGICAL, i_logical::ANDI, rd, ra, imm)
        }
        Instruction::Ori { rd, ra, imm } => {
            encode_i(IMMEDIATE_LOGICAL, i_logical::ORI, rd, ra, imm)
        }
        Instruction::Xori { rd, ra, imm } => {
            encode_i(IMMEDIATE_LOGICAL, i_logical::XORI, rd, ra, imm)
        }

        // Register shifts/R-type
        Instruction::Shl { rd, ra, rb } => encode_r(REGISTER_SHIFT, r_shift::SHL, rd, ra, rb),
        Instruction::Shr { rd, ra, rb } => encode_r(REGISTER_SHIFT, r_shift::SHR, rd, ra, rb),
        Instruction::Sar { rd, ra, rb } => encode_r(REGISTER_SHIFT, r_shift::SAR, rd, ra, rb),

        // Immediate shifts/I-type
        Instruction::Shli { rd, ra, imm } => {
            encode_i(IMMEDIATE_SHIFT, i_shift::SHLI, rd, ra, imm as u32)
        }
        Instruction::Shri { rd, ra, imm } => {
            encode_i(IMMEDIATE_SHIFT, i_shift::SHRI, rd, ra, imm as u32)
        }
        Instruction::Sari { rd, ra, imm } => {
            encode_i(IMMEDIATE_SHIFT, i_shift::SARI, rd, ra, imm as u32)
        }

        // Bit immediate/I-type
        Instruction::Btst { ra, imm } => {
            encode_i(BIT_IMMEDIATE, bit_immediate::BTST, 0, ra, imm as u32)
        }
        Instruction::Bset { rd, ra, imm } => {
            encode_i(BIT_IMMEDIATE, bit_immediate::BSET, rd, ra, imm as u32)
        }
        Instruction::Bclr { rd, ra, imm } => {
            encode_i(BIT_IMMEDIATE, bit_immediate::BCLR, rd, ra, imm as u32)
        }
        Instruction::Btgl { rd, ra, imm } => {
            encode_i(BIT_IMMEDIATE, bit_immediate::BTGL, rd, ra, imm as u32)
        }

        // Multiply/Divide/R2-type
        Instruction::Mul { rd0, rd1, ra, rb } => encode_r2(md::MUL, rd0, rd1, ra, rb),
        Instruction::Mulu { rd0, rd1, ra, rb } => encode_r2(md::MULU, rd0, rd1, ra, rb),
        Instruction::Div { rd0, rd1, ra, rb } => encode_r2(md::DIV, rd0, rd1, ra, rb),
        Instruction::Divu { rd0, rd1, ra, rb } => encode_r2(md::DIVU, rd0, rd1, ra, rb),

        // Constant construction/U-type
        Instruction::Lui { rd, imm16 } => encode_u(cc::LUI, rd, imm16),
        Instruction::Lli { rd, imm16 } => encode_u(cc::LLI, rd, imm16),
        Instruction::Lhi { rd, imm16 } => encode_u(cc::LHI, rd, imm16),

        // Load/Store/M-type
        Instruction::Lb { rd, base, offset } => encode_m(LOAD, 1, ms::BYTE, rd, base, offset),
        Instruction::Lbu { rd, base, offset } => encode_m(LOAD, 0, ms::BYTE, rd, base, offset),
        Instruction::Lh { rd, base, offset } => encode_m(LOAD, 1, ms::HALFWORD, rd, base, offset),
        Instruction::Lhu { rd, base, offset } => encode_m(LOAD, 0, ms::HALFWORD, rd, base, offset),
        Instruction::Lw { rd, base, offset } => encode_m(LOAD, 0, ms::WORD, rd, base, offset),
        Instruction::Sb { rs, base, offset } => encode_m(STORE, 0, ms::BYTE, rs, base, offset),
        Instruction::Sh { rs, base, offset } => encode_m(STORE, 0, ms::HALFWORD, rs, base, offset),
        Instruction::Sw { rs, base, offset } => encode_m(STORE, 0, ms::WORD, rs, base, offset),

        // Flag branch/BF-type
        Instruction::BfEq { offset } => encode_bf(condition::EQ, offset),
        Instruction::BfNe { offset } => encode_bf(condition::NE, offset),
        Instruction::BfLt { offset } => encode_bf(condition::LT, offset),
        Instruction::BfLe { offset } => encode_bf(condition::LE, offset),
        Instruction::BfGt { offset } => encode_bf(condition::GT, offset),
        Instruction::BfGe { offset } => encode_bf(condition::GE, offset),
        Instruction::BfLtu { offset } => encode_bf(condition::LTU, offset),
        Instruction::BfLeu { offset } => encode_bf(condition::LEU, offset),
        Instruction::BfGtu { offset } => encode_bf(condition::GTU, offset),
        Instruction::BfGeu { offset } => encode_bf(condition::GEU, offset),
        Instruction::BfCs { offset } => encode_bf(condition::CS, offset),
        Instruction::BfCc { offset } => encode_bf(condition::CC, offset),
        Instruction::BfVs { offset } => encode_bf(condition::VS, offset),
        Instruction::BfVc { offset } => encode_bf(condition::VC, offset),
        Instruction::BfEs { offset } => encode_bf(condition::ES, offset),
        Instruction::BfEc { offset } => encode_bf(condition::EC, offset),

        // Register branch/BC-type
        Instruction::BEq { ra, rb, offset } => encode_bc(condition::EQ, ra, rb, offset),
        Instruction::BNe { ra, rb, offset } => encode_bc(condition::NE, ra, rb, offset),
        Instruction::BLt { ra, rb, offset } => encode_bc(condition::LT, ra, rb, offset),
        Instruction::BLe { ra, rb, offset } => encode_bc(condition::LE, ra, rb, offset),
        Instruction::BGt { ra, rb, offset } => encode_bc(condition::GT, ra, rb, offset),
        Instruction::BGe { ra, rb, offset } => encode_bc(condition::GE, ra, rb, offset),
        Instruction::BLtu { ra, rb, offset } => encode_bc(condition::LTU, ra, rb, offset),
        Instruction::BLeu { ra, rb, offset } => encode_bc(condition::LEU, ra, rb, offset),
        Instruction::BGtu { ra, rb, offset } => encode_bc(condition::GTU, ra, rb, offset),
        Instruction::BGeu { ra, rb, offset } => encode_bc(condition::GEU, ra, rb, offset),

        // PC-relative jump/call/J-type
        Instruction::Jmp { offset } => encode_j(opcode::PC_RELATIVE_JUMP, offset),
        Instruction::Call { offset } => encode_j(opcode::PC_RELATIVE_CALL, offset),

        // Register jump/call/JR-type
        Instruction::Jr { target } => encode_jr(r_jc::JR, 0, target),
        Instruction::Jalr { rd, target } => encode_jr(r_jc::JALR, rd, target),
    }
}

fn encode_r(opcode: u32, alu: u32, rd: u8, ra: u8, rb: u8) -> Result<u32, EncodeError> {
    use spec::format::r;
    let rd = validate_register(rd)?;
    let ra = validate_register(ra)?;
    let rb = validate_register(rb)?;

    Ok(opcode << architecture::OPCODE_SHIFT
        | alu << r::FUNC_SHIFT
        | rd << r::RD_SHIFT
        | ra << r::RA_SHIFT
        | rb << r::RB_SHIFT)
}

fn encode_i(opcode: u32, mode: u32, rd: u8, ra: u8, imm: u32) -> Result<u32, EncodeError> {
    use spec::format::i;
    let rd = validate_register(rd)?;
    let ra = validate_register(ra)?;

    if imm > 0xFFFF {
        return Err(EncodeError::InvalidImmediate {
            size: 16,
            value: imm,
        });
    }

    Ok(opcode << architecture::OPCODE_SHIFT
        | mode << i::MODE_SHIFT
        | imm << i::IMM16_SHIFT
        | rd << i::RD_SHIFT
        | ra << i::RA_SHIFT)
}

fn encode_r2(func: u32, rd0: u8, rd1: u8, ra: u8, rb: u8) -> Result<u32, EncodeError> {
    use spec::format::r2;
    let rd0 = validate_register(rd0)?;
    let rd1 = validate_register(rd1)?;
    let ra = validate_register(ra)?;
    let rb = validate_register(rb)?;

    Ok(opcode::MULTIPLY_DIVIDE << architecture::OPCODE_SHIFT
        | func << r2::FUNC_SHIFT
        | rd0 << r2::RD0_SHIFT
        | rd1 << r2::RD1_SHIFT
        | ra << r2::RA_SHIFT
        | rb << r2::RB_SHIFT)
}

fn encode_u(mode: u32, rd: u8, imm: u16) -> Result<u32, EncodeError> {
    use spec::format::u;
    let rd = validate_register(rd)?;

    Ok(opcode::CONSTANT_CONSTRUCTION << architecture::OPCODE_SHIFT
        | mode << u::MODE_SHIFT
        | (imm as u32) << u::IMM16_SHIFT
        | rd << u::RD_SHIFT)
}

fn encode_m(
    opcode: u32,
    sx: u8,
    size: u32,
    rd: u8,
    base: u8,
    offset: i32,
) -> Result<u32, EncodeError> {
    use spec::format::m;
    let rd = validate_register(rd)?;
    let base = validate_register(base)?;
    let offset = encode_signed_field(offset, 15)?;

    if sx > 1 {
        return Err(EncodeError::InvalidImmediate {
            size: 1,
            value: sx as u32,
        });
    }

    Ok(opcode << architecture::OPCODE_SHIFT
        | (sx as u32) << m::SX_SHIFT
        | (size as u32) << m::SIZE_SHIFT
        | offset << m::OFFSET15_SHIFT
        | rd << m::RD_RS_SHIFT
        | base << m::BASE_SHIFT)
}

fn encode_bf(cond: u32, offset: i32) -> Result<u32, EncodeError> {
    use spec::format::bf;
    let offset = encode_word_offset(offset, 22)?;

    Ok(opcode::FLAG_BRANCH << architecture::OPCODE_SHIFT
        | cond << bf::COND_SHIFT
        | offset << bf::OFFSET22_SHIFT)
}

fn encode_bc(cond: u32, ra: u8, rb: u8, offset: i32) -> Result<u32, EncodeError> {
    use spec::format::bc;
    let ra = validate_register(ra)?;
    let rb = validate_register(rb)?;
    let offset = encode_word_offset(offset, 14)?;

    Ok(opcode::REGISTER_BRANCH << architecture::OPCODE_SHIFT
        | cond << bc::COND_SHIFT
        | offset << bc::OFFSET14_SHIFT
        | ra << bc::RA_SHIFT
        | rb << bc::RB_SHIFT)
}

fn encode_j(opcode: u32, offset: i32) -> Result<u32, EncodeError> {
    use spec::format::j;
    let offset = encode_word_offset(offset, 26)?;

    Ok(opcode << architecture::OPCODE_SHIFT | offset << j::OFFSET26_SHIFT)
}

fn encode_jr(func: u32, rd: u8, target: u8) -> Result<u32, EncodeError> {
    use spec::format::jr;
    let rd = validate_register(rd)?;
    let target = validate_register(target)?;
    Ok(opcode::REGISTER_JUMP_CALL << architecture::OPCODE_SHIFT
        | func << jr::FUNC_SHIFT
        | rd << jr::RD_SHIFT
        | target << jr::TARGET_SHIFT)
}
fn encode_x(sysop: u32, payload: u32, rd: u8, rs: u8) -> Result<u32, EncodeError> {
    use spec::format::x;
    let rd = validate_register(rd)?;
    let rs = validate_register(rs)?;

    if payload > 0xFFF {
        return Err(EncodeError::InvalidPayload(payload));
    }

    Ok(opcode::SYSTEM_CONTROL << architecture::OPCODE_SHIFT
        | sysop << x::SYSOP_SHIFT
        | payload << x::PAYLOAD_SHIFT
        | rd << x::RD_SHIFT
        | rs << x::RS_SHIFT)
}

fn validate_register(reg: u8) -> Result<u32, EncodeError> {
    if reg > 0xF {
        Err(EncodeError::InvalidRegister(reg))
    } else {
        Ok(reg as u32)
    }
}

fn encode_signed_field(value: i32, bits: u8) -> Result<u32, EncodeError> {
    debug_assert!(bits > 0 && bits < 32);

    let min = -(1i32 << (bits - 1));
    let max = (1i32 << (bits - 1)) - 1;

    if value < min || value > max {
        return Err(EncodeError::InvalidImmediate {
            size: bits,
            value: value as u32,
        });
    }

    Ok((value as u32) & ((1u32 << bits) - 1))
}

fn encode_word_offset(byte_offset: i32, bits: u8) -> Result<u32, EncodeError> {
    if byte_offset & 0b11 != 0 {
        return Err(EncodeError::InvalidImmediate {
            size: bits,
            value: byte_offset as u32,
        });
    }

    encode_signed_field(byte_offset >> 2, bits)
}
