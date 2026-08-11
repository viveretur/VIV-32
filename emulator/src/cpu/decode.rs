use super::creg_file::Creg;

use crate::isa::generated::{self, opcode};

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DecodedInstruction {
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
    Addi { rd: u8, ra: u8, sext32: i32 },
    Subi { rd: u8, ra: u8, sext32: i32 },
    Cmpi { ra: u8, sext32: i32 },

    // Immediate logical/I-type
    Andi { rd: u8, ra: u8, imm32: u32 },
    Ori { rd: u8, ra: u8, imm32: u32 },
    Xori { rd: u8, ra: u8, imm32: u32 },

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    IllegalInstruction(u32),
    IllegalPayload(u32),
}

pub fn decode(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{architecture, opcode};

    let opcode = (raw >> architecture::OPCODE_SHIFT) & architecture::OPCODE_MASK;

    match opcode {
        opcode::REGISTER_ALU | opcode::REGISTER_SHIFT => decode_r(opcode, raw),
        opcode::IMMEDIATE_ARITHMETIC_COMPARE
        | opcode::IMMEDIATE_LOGICAL
        | opcode::IMMEDIATE_SHIFT
        | opcode::BIT_IMMEDIATE => decode_i(opcode, raw),
        opcode::MULTIPLY_DIVIDE => decode_r2(raw),
        opcode::CONSTANT_CONSTRUCTION => decode_u(raw),
        opcode::LOAD | opcode::STORE => decode_m(opcode, raw),
        opcode::FLAG_BRANCH => decode_bf(raw),
        opcode::REGISTER_BRANCH => decode_bc(raw),
        opcode::PC_RELATIVE_JUMP | opcode::PC_RELATIVE_CALL => decode_j(opcode, raw),
        opcode::REGISTER_JUMP_CALL => decode_jr(raw),
        opcode::SYSTEM_CONTROL => decode_x(raw),
        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_r(opcode: u32, raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{
        format::r,
        func::{register_alu, register_shift},
    };

    let func = (raw & r::FUNC_MASK) >> r::FUNC_SHIFT;
    let rd = ((raw & r::RD_MASK) >> r::RD_SHIFT) as u8;
    let ra = ((raw & r::RA_MASK) >> r::RA_SHIFT) as u8;
    let rb = ((raw & r::RB_MASK) >> r::RB_SHIFT) as u8;

    match (opcode, func) {
        (opcode::REGISTER_ALU, register_alu::ADD) => Ok(DecodedInstruction::Add { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::SUB) => Ok(DecodedInstruction::Sub { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::AND) => Ok(DecodedInstruction::And { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::OR) => Ok(DecodedInstruction::Or { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::XOR) => Ok(DecodedInstruction::Xor { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::NOT) => Ok(DecodedInstruction::Not { rd, ra }),
        (opcode::REGISTER_ALU, register_alu::NEG) => Ok(DecodedInstruction::Neg { rd, ra }),
        (opcode::REGISTER_ALU, register_alu::CMP) => Ok(DecodedInstruction::Cmp { ra, rb }),

        (opcode::REGISTER_SHIFT, register_shift::SHL) => Ok(DecodedInstruction::Shl { rd, ra, rb }),
        (opcode::REGISTER_SHIFT, register_shift::SHR) => Ok(DecodedInstruction::Shr { rd, ra, rb }),
        (opcode::REGISTER_SHIFT, register_shift::SAR) => Ok(DecodedInstruction::Sar { rd, ra, rb }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

#[rustfmt::skip]
fn decode_i(opcode: u32, raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{
        format::i,
        mode::{bit_immediate, immediate_arithmetic_compare, immediate_logical, immediate_shift},
    };

    let mode = (raw & i::MODE_MASK) >> i::MODE_SHIFT;
    let imm16 = (raw & i::IMM16_MASK) >> i::IMM16_SHIFT;
    let sext32 = (((imm16 as i32) << 16) >> 16) as i32;
    let ra = ((raw & i::RA_MASK) >> i::RA_SHIFT) as u8;
    let rd = ((raw & i::RD_MASK) >> i::RD_SHIFT) as u8;

    match (opcode, mode) {
        (opcode::IMMEDIATE_ARITHMETIC_COMPARE, immediate_arithmetic_compare::ADDI) => {
            Ok(DecodedInstruction::Addi { rd, ra, sext32 })
        }
        (opcode::IMMEDIATE_ARITHMETIC_COMPARE, immediate_arithmetic_compare::SUBI) => {
            Ok(DecodedInstruction::Subi { rd, ra, sext32 })
        }
        (opcode::IMMEDIATE_ARITHMETIC_COMPARE, immediate_arithmetic_compare::CMPI) => {
            Ok(DecodedInstruction::Cmpi { ra, sext32 })
        }

        (opcode::IMMEDIATE_LOGICAL, immediate_logical::ANDI) => {
            Ok(DecodedInstruction::Andi { rd, ra, imm32: imm16 })
        }
        (opcode::IMMEDIATE_LOGICAL, immediate_logical::ORI) => {
            Ok(DecodedInstruction::Ori { rd, ra, imm32: imm16 })
        }
        (opcode::IMMEDIATE_LOGICAL, immediate_logical::XORI) => {
            Ok(DecodedInstruction::Xori { rd, ra, imm32: imm16 })
        }
        
        (opcode::IMMEDIATE_SHIFT, immediate_shift::SHLI) => {
            Ok(DecodedInstruction::Shli { rd, ra, imm: imm16 as u8 })
        }
        (opcode::IMMEDIATE_SHIFT, immediate_shift::SHRI) => {
            Ok(DecodedInstruction::Shri { rd, ra, imm: imm16 as u8 })
        }
        (opcode::IMMEDIATE_SHIFT, immediate_shift::SARI) => {
            Ok(DecodedInstruction::Sari { rd, ra, imm: imm16 as u8 })
        }

        (opcode::BIT_IMMEDIATE, bit_immediate::BTST) => {
            Ok(DecodedInstruction::Btst { ra, imm: imm16 as u8 })
        }
        (opcode::BIT_IMMEDIATE, bit_immediate::BSET) => {
            Ok(DecodedInstruction::Bset { rd, ra, imm: imm16 as u8 })
        }
        (opcode::BIT_IMMEDIATE, bit_immediate::BCLR) => {
            Ok(DecodedInstruction::Bclr { rd, ra, imm: imm16 as u8 })
        }
        (opcode::BIT_IMMEDIATE, bit_immediate::BTGL) => {
            Ok(DecodedInstruction::Btgl { rd, ra, imm: imm16 as u8 })
        }

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_r2(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{format::r2, func::multiply_divide};

    let func = (raw & r2::FUNC_MASK) >> r2::FUNC_SHIFT;
    let rd0 = ((raw & r2::RD0_MASK) >> r2::RD0_SHIFT) as u8;
    let rd1 = ((raw & r2::RD1_MASK) >> r2::RD1_SHIFT) as u8;
    let ra = ((raw & r2::RA_MASK) >> r2::RA_SHIFT) as u8;
    let rb = ((raw & r2::RB_MASK) >> r2::RB_SHIFT) as u8;

    match func {
        multiply_divide::MUL => Ok(DecodedInstruction::Mul { rd0, rd1, ra, rb }),
        multiply_divide::MULU => Ok(DecodedInstruction::Mulu { rd0, rd1, ra, rb }),
        multiply_divide::DIV => Ok(DecodedInstruction::Div { rd0, rd1, ra, rb }),
        multiply_divide::DIVU => Ok(DecodedInstruction::Divu { rd0, rd1, ra, rb }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_u(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{format::u, mode::constant_construction};

    let mode = (raw & u::MODE_MASK) >> u::MODE_SHIFT;
    let imm16 = ((raw & u::IMM16_MASK) >> u::IMM16_SHIFT) as u16;
    let rd = ((raw & u::RD_MASK) >> u::RD_SHIFT) as u8;

    match mode {
        constant_construction::LUI => Ok(DecodedInstruction::Lui { rd, imm16 }),
        constant_construction::LLI => Ok(DecodedInstruction::Lli { rd, imm16 }),
        constant_construction::LHI => Ok(DecodedInstruction::Lhi { rd, imm16 }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

#[rustfmt::skip]
fn decode_m(opcode: u32, raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{format::m, memory_size};

    let sx = (raw & m::SX_MASK) >> m::SX_SHIFT;
    let size = (raw & m::SIZE_MASK) >> m::SIZE_SHIFT;
    let mut offset = ((raw & m::OFFSET15_MASK) >> m::OFFSET15_SHIFT) as i32;
    let ss = 32 - m::OFFSET15_WIDTH;
    offset = (offset << ss) >> ss;
    let r = ((raw & m::RD_RS_MASK) >> m::RD_RS_SHIFT) as u8;
    let base = ((raw & m::BASE_MASK) >> m::BASE_SHIFT) as u8;

    match (opcode, sx, size) {
        (opcode::LOAD, 0, memory_size::BYTE) => Ok(DecodedInstruction::Lbu { rd: r, base, offset }),
        (opcode::LOAD, 1, memory_size::BYTE) => Ok(DecodedInstruction::Lb { rd: r, base, offset }),
        (opcode::LOAD, 0, memory_size::HALFWORD) => Ok(DecodedInstruction::Lhu { rd: r, base, offset }),
        (opcode::LOAD, 1, memory_size::HALFWORD) => Ok(DecodedInstruction::Lh { rd: r, base, offset }),
        (opcode::LOAD, _, memory_size::WORD) => Ok(DecodedInstruction::Lw { rd: r, base, offset }),
        (opcode::STORE, _, memory_size::BYTE) => Ok(DecodedInstruction::Sb { rs: r, base, offset }),
        (opcode::STORE, _, memory_size::HALFWORD) => Ok(DecodedInstruction::Sh { rs: r, base, offset }),
        (opcode::STORE, _, memory_size::WORD) => Ok(DecodedInstruction::Sw { rs: r, base, offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
    
}

fn decode_bf(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{condition, format::bf};

    let cond = (raw & bf::COND_MASK) >> bf::COND_SHIFT;
    let mut offset = ((raw & bf::OFFSET22_MASK) >> bf::OFFSET22_SHIFT) as i32;
    let ss = 32 - bf::OFFSET22_WIDTH;
    offset = (offset << ss) >> (ss - 2); // All branch offsets are SHL2.

    match cond {
        condition::EQ => Ok(DecodedInstruction::BfEq { offset }),
        condition::NE => Ok(DecodedInstruction::BfNe { offset }),
        condition::LT => Ok(DecodedInstruction::BfLt { offset }),
        condition::LE => Ok(DecodedInstruction::BfLe { offset }),
        condition::GT => Ok(DecodedInstruction::BfGt { offset }),
        condition::GE => Ok(DecodedInstruction::BfGe { offset }),
        condition::LTU => Ok(DecodedInstruction::BfLtu { offset }),
        condition::LEU => Ok(DecodedInstruction::BfLeu { offset }),
        condition::GTU => Ok(DecodedInstruction::BfGtu { offset }),
        condition::GEU => Ok(DecodedInstruction::BfGeu { offset }),
        condition::CS => Ok(DecodedInstruction::BfCs { offset }),
        condition::CC => Ok(DecodedInstruction::BfCc { offset }),
        condition::VS => Ok(DecodedInstruction::BfVs { offset }),
        condition::VC => Ok(DecodedInstruction::BfVc { offset }),
        condition::ES => Ok(DecodedInstruction::BfEs { offset }),
        condition::EC => Ok(DecodedInstruction::BfEc { offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_bc(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{condition, format::bc};

    let cond = (raw & bc::COND_MASK) >> bc::COND_SHIFT;
    let mut offset = ((raw & bc::OFFSET14_MASK) >> bc::OFFSET14_SHIFT) as i32;
    let ss = 32 - bc::OFFSET14_WIDTH;
    offset = (offset << ss) >> (ss - 2); // All branch offsets are SHL2.
    let ra = ((raw & bc::RA_MASK) >> bc::RA_SHIFT) as u8;
    let rb = ((raw & bc::RB_MASK) >> bc::RB_SHIFT) as u8;

    match cond {
        condition::EQ => Ok(DecodedInstruction::BEq { ra, rb, offset }),
        condition::NE => Ok(DecodedInstruction::BNe { ra, rb, offset }),
        condition::LT => Ok(DecodedInstruction::BLt { ra, rb, offset }),
        condition::LE => Ok(DecodedInstruction::BLe { ra, rb, offset }),
        condition::GT => Ok(DecodedInstruction::BGt { ra, rb, offset }),
        condition::GE => Ok(DecodedInstruction::BGe { ra, rb, offset }),
        condition::LTU => Ok(DecodedInstruction::BLtu { ra, rb, offset }),
        condition::LEU => Ok(DecodedInstruction::BLeu { ra, rb, offset }),
        condition::GTU => Ok(DecodedInstruction::BGtu { ra, rb, offset }),
        condition::GEU => Ok(DecodedInstruction::BGeu { ra, rb, offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_j(opcode: u32, raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::format::j;

    let mut offset = ((raw & j::OFFSET26_MASK) >> j::OFFSET26_SHIFT) as i32;
    let shift = 32 - j::OFFSET26_WIDTH;
    offset = (offset << shift) >> (shift - 2); // All branch offsets are SHL2.

    match opcode {
        opcode::PC_RELATIVE_JUMP => Ok(DecodedInstruction::Jmp { offset }),
        opcode::PC_RELATIVE_CALL => Ok(DecodedInstruction::Call { offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_jr(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{format::jr, func::register_jump_call};

    let func = (raw & jr::FUNC_MASK) >> jr::FUNC_SHIFT;
    let rd = ((raw & jr::RD_MASK) >> jr::RD_SHIFT) as u8;
    let target = ((raw & jr::TARGET_MASK) >> jr::TARGET_SHIFT) as u8;

    match func {
        register_jump_call::JR => Ok(DecodedInstruction::Jr { target }),
        register_jump_call::JALR => Ok(DecodedInstruction::Jalr { rd, target }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_x(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    use generated::{format::x, sysop};

    let sysop = (raw & x::SYSOP_MASK) >> x::SYSOP_SHIFT;
    let payload = (raw & x::PAYLOAD_MASK) >> x::PAYLOAD_SHIFT;
    let rd = (raw & x::RD_MASK) >> x::RD_SHIFT;
    let rs = (raw & x::RS_MASK) >> x::RS_SHIFT;

    match sysop {
        sysop::NOP => Ok(DecodedInstruction::Nop),
        sysop::HALT => Ok(DecodedInstruction::Halt),
        sysop::TRAP => {
            let ss = 32 - x::PAYLOAD_WIDTH;
            let offset = ((payload as i32) << ss) >> (ss - 2); // Branch offsets are SHL2.
            Ok(DecodedInstruction::SoftwareTrap { imm: offset })
        }
        sysop::SYSCALL => Ok(DecodedInstruction::SystemCall),
        sysop::IRET => Ok(DecodedInstruction::IRet),
        sysop::EI => Ok(DecodedInstruction::EI),
        sysop::DI => Ok(DecodedInstruction::DI),
        sysop::RDPC => Ok(DecodedInstruction::RdPc { rd: rd as u8 }),
        sysop::MRS => Ok(DecodedInstruction::Mrs {
            creg4: decode_creg(payload)?,
            rd: rd as u8,
        }),
        sysop::MSR => Ok(DecodedInstruction::Msr {
            creg4: decode_creg(payload)?,
            rs: rs as u8,
        }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_creg(payload: u32) -> Result<Creg, DecodeError> {
    use generated::creg;

    match payload as usize {
        creg::PC => Ok(Creg::PC),
        creg::SR => Ok(Creg::SR),
        creg::EPC => Ok(Creg::EPC),
        creg::ESR => Ok(Creg::ESR),
        creg::ECAUSE => Ok(Creg::ECause),
        creg::EDATA => Ok(Creg::EData),
        creg::EVBASE => Ok(Creg::EvBase),

        _ => Err(DecodeError::IllegalPayload(payload)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opcode(value: u32) -> u32 {
        use generated::architecture;

        (value & architecture::OPCODE_MASK) << architecture::OPCODE_SHIFT
    }

    fn field(mask: u32, shift: u32, value: u32) -> u32 {
        (value << shift) & mask
    }

    fn signed_field(value: i32, width: u32) -> u32 {
        value as u32 & ((1u32 << width) - 1)
    }

    fn enc_r(op: u32, func: u32) -> u32 {
        use generated::format::r;

        opcode(op)
            | field(r::FUNC_MASK, r::FUNC_SHIFT, func)
            | field(r::RD_MASK, r::RD_SHIFT, 3)
            | field(r::RA_MASK, r::RA_SHIFT, 4)
            | field(r::RB_MASK, r::RB_SHIFT, 5)
    }

    fn enc_i(op: u32, mode: u32, imm16: u16) -> u32 {
        use generated::format::i;

        opcode(op)
            | field(i::MODE_MASK, i::MODE_SHIFT, mode)
            | field(i::RD_MASK, i::RD_SHIFT, 3)
            | field(i::RA_MASK, i::RA_SHIFT, 4)
            | field(i::IMM16_MASK, i::IMM16_SHIFT, imm16 as u32)
    }

    fn enc_r2(func: u32) -> u32 {
        use generated::format::r2;

        opcode(generated::opcode::MULTIPLY_DIVIDE)
            | field(r2::FUNC_MASK, r2::FUNC_SHIFT, func)
            | field(r2::RD0_MASK, r2::RD0_SHIFT, 3)
            | field(r2::RD1_MASK, r2::RD1_SHIFT, 4)
            | field(r2::RA_MASK, r2::RA_SHIFT, 5)
            | field(r2::RB_MASK, r2::RB_SHIFT, 6)
    }

    fn enc_u(mode: u32, imm16: u16) -> u32 {
        use generated::format::u;

        opcode(generated::opcode::CONSTANT_CONSTRUCTION)
            | field(u::MODE_MASK, u::MODE_SHIFT, mode)
            | field(u::RD_MASK, u::RD_SHIFT, 3)
            | field(u::IMM16_MASK, u::IMM16_SHIFT, imm16 as u32)
    }

    fn enc_m(op: u32, sx: u32, size: u32, offset: i16) -> u32 {
        use generated::format::m;

        let offset15 = signed_field(offset as i32, m::OFFSET15_WIDTH);

        opcode(op)
            | field(m::SX_MASK, m::SX_SHIFT, sx)
            | field(m::SIZE_MASK, m::SIZE_SHIFT, size)
            | field(m::RD_RS_MASK, m::RD_RS_SHIFT, 3)
            | field(m::BASE_MASK, m::BASE_SHIFT, 4)
            | field(m::OFFSET15_MASK, m::OFFSET15_SHIFT, offset15)
    }

    fn enc_bf(cond: u32, offset: i32) -> u32 {
        use generated::format::bf;

        opcode(generated::opcode::FLAG_BRANCH)
            | field(bf::COND_MASK, bf::COND_SHIFT, cond)
            | field(
                bf::OFFSET22_MASK,
                bf::OFFSET22_SHIFT,
                signed_field(offset, bf::OFFSET22_WIDTH),
            )
    }

    fn enc_bc(cond: u32, offset: i32) -> u32 {
        use generated::format::bc;

        opcode(generated::opcode::REGISTER_BRANCH)
            | field(bc::COND_MASK, bc::COND_SHIFT, cond)
            | field(bc::RA_MASK, bc::RA_SHIFT, 3)
            | field(bc::RB_MASK, bc::RB_SHIFT, 4)
            | field(
                bc::OFFSET14_MASK,
                bc::OFFSET14_SHIFT,
                signed_field(offset, bc::OFFSET14_WIDTH),
            )
    }

    fn enc_j(op: u32, offset: i32) -> u32 {
        use generated::format::j;

        opcode(op)
            | field(
                j::OFFSET26_MASK,
                j::OFFSET26_SHIFT,
                signed_field(offset, j::OFFSET26_WIDTH),
            )
    }

    fn enc_jr(func: u32) -> u32 {
        use generated::format::jr;

        opcode(generated::opcode::REGISTER_JUMP_CALL)
            | field(jr::FUNC_MASK, jr::FUNC_SHIFT, func)
            | field(jr::RD_MASK, jr::RD_SHIFT, 3)
            | field(jr::TARGET_MASK, jr::TARGET_SHIFT, 4)
    }

    fn enc_x(sysop: u32, payload: u32, rd: u32, rs: u32) -> u32 {
        use generated::format::x;

        opcode(generated::opcode::SYSTEM_CONTROL)
            | field(x::SYSOP_MASK, x::SYSOP_SHIFT, sysop)
            | field(x::PAYLOAD_MASK, x::PAYLOAD_SHIFT, payload)
            | field(x::RD_MASK, x::RD_SHIFT, rd)
            | field(x::RS_MASK, x::RS_SHIFT, rs)
    }

    macro_rules! decode_case {
        ($name:ident, $raw:expr, $expected:expr) => {
            #[test]
            fn $name() {
                assert_eq!(decode($raw), Ok($expected));
            }
        };
    }

    // ---------------------------------------------------------------------
    // X-type / system control
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_nop,
        enc_x(generated::sysop::NOP, 0, 0, 0),
        DecodedInstruction::Nop
    );
    decode_case!(
        decodes_halt,
        enc_x(generated::sysop::HALT, 0, 0, 0),
        DecodedInstruction::Halt
    );

    decode_case!(
        decodes_software_trap,
        enc_x(generated::sysop::TRAP, 0x7A9, 0, 0),
        DecodedInstruction::SoftwareTrap { imm: 0x1EA4 }
    );

    decode_case!(
        decodes_system_call,
        enc_x(generated::sysop::SYSCALL, 0, 0, 0),
        DecodedInstruction::SystemCall
    );

    decode_case!(
        decodes_iret,
        enc_x(generated::sysop::IRET, 0, 0, 0),
        DecodedInstruction::IRet
    );
    decode_case!(
        decodes_ei,
        enc_x(generated::sysop::EI, 0, 0, 0),
        DecodedInstruction::EI
    );
    decode_case!(
        decodes_di,
        enc_x(generated::sysop::DI, 0, 0, 0),
        DecodedInstruction::DI
    );

    decode_case!(
        decodes_rdpc,
        enc_x(generated::sysop::RDPC, 0, 3, 0),
        DecodedInstruction::RdPc { rd: 3 }
    );

    decode_case!(
        decodes_mrs_pc,
        enc_x(generated::sysop::MRS, generated::creg::PC as u32, 3, 0),
        DecodedInstruction::Mrs {
            creg4: Creg::PC,
            rd: 3,
        }
    );

    decode_case!(
        decodes_msr_sr,
        enc_x(generated::sysop::MSR, generated::creg::SR as u32, 0, 4),
        DecodedInstruction::Msr {
            creg4: Creg::SR,
            rs: 4,
        }
    );

    // ---------------------------------------------------------------------
    // R-type / register ALU
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_add,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::ADD
        ),
        DecodedInstruction::Add {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_sub,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::SUB
        ),
        DecodedInstruction::Sub {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_and,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::AND
        ),
        DecodedInstruction::And {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_or,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::OR
        ),
        DecodedInstruction::Or {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_xor,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::XOR
        ),
        DecodedInstruction::Xor {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_not,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::NOT
        ),
        DecodedInstruction::Not { rd: 3, ra: 4 }
    );

    decode_case!(
        decodes_neg,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::NEG
        ),
        DecodedInstruction::Neg { rd: 3, ra: 4 }
    );

    decode_case!(
        decodes_cmp,
        enc_r(
            generated::opcode::REGISTER_ALU,
            generated::func::register_alu::CMP
        ),
        DecodedInstruction::Cmp { ra: 4, rb: 5 }
    );

    // ---------------------------------------------------------------------
    // R-type / register shifts
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_shl,
        enc_r(
            generated::opcode::REGISTER_SHIFT,
            generated::func::register_shift::SHL
        ),
        DecodedInstruction::Shl {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_shr,
        enc_r(
            generated::opcode::REGISTER_SHIFT,
            generated::func::register_shift::SHR
        ),
        DecodedInstruction::Shr {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_sar,
        enc_r(
            generated::opcode::REGISTER_SHIFT,
            generated::func::register_shift::SAR
        ),
        DecodedInstruction::Sar {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    // ---------------------------------------------------------------------
    // I-type / signed immediate arithmetic and compare
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_addi_positive,
        enc_i(
            generated::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            generated::mode::immediate_arithmetic_compare::ADDI,
            0x1234,
        ),
        DecodedInstruction::Addi {
            rd: 3,
            ra: 4,
            sext32: 0x1234
        }
    );

    decode_case!(
        decodes_addi_negative,
        enc_i(
            generated::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            generated::mode::immediate_arithmetic_compare::ADDI,
            0xFFFE,
        ),
        DecodedInstruction::Addi {
            rd: 3,
            ra: 4,
            sext32: -2
        }
    );

    decode_case!(
        decodes_subi_negative,
        enc_i(
            generated::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            generated::mode::immediate_arithmetic_compare::SUBI,
            0x8000,
        ),
        DecodedInstruction::Subi {
            rd: 3,
            ra: 4,
            sext32: -32768
        }
    );

    decode_case!(
        decodes_cmpi_negative,
        enc_i(
            generated::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            generated::mode::immediate_arithmetic_compare::CMPI,
            0xFFFF,
        ),
        DecodedInstruction::Cmpi { ra: 4, sext32: -1 }
    );

    // ---------------------------------------------------------------------
    // I-type / logical immediates
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_andi,
        enc_i(
            generated::opcode::IMMEDIATE_LOGICAL,
            generated::mode::immediate_logical::ANDI,
            0xF0F0,
        ),
        DecodedInstruction::Andi {
            rd: 3,
            ra: 4,
            imm32: 0xF0F0
        }
    );

    decode_case!(
        decodes_ori,
        enc_i(
            generated::opcode::IMMEDIATE_LOGICAL,
            generated::mode::immediate_logical::ORI,
            0x00FF,
        ),
        DecodedInstruction::Ori {
            rd: 3,
            ra: 4,
            imm32: 0x00FF
        }
    );

    decode_case!(
        decodes_xori,
        enc_i(
            generated::opcode::IMMEDIATE_LOGICAL,
            generated::mode::immediate_logical::XORI,
            0xAAAA,
        ),
        DecodedInstruction::Xori {
            rd: 3,
            ra: 4,
            imm32: 0xAAAA
        }
    );

    // ---------------------------------------------------------------------
    // I-type / immediate shifts
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_shli,
        enc_i(
            generated::opcode::IMMEDIATE_SHIFT,
            generated::mode::immediate_shift::SHLI,
            7,
        ),
        DecodedInstruction::Shli {
            rd: 3,
            ra: 4,
            imm: 7
        }
    );

    decode_case!(
        decodes_shri,
        enc_i(
            generated::opcode::IMMEDIATE_SHIFT,
            generated::mode::immediate_shift::SHRI,
            8,
        ),
        DecodedInstruction::Shri {
            rd: 3,
            ra: 4,
            imm: 8
        }
    );

    decode_case!(
        decodes_sari,
        enc_i(
            generated::opcode::IMMEDIATE_SHIFT,
            generated::mode::immediate_shift::SARI,
            9,
        ),
        DecodedInstruction::Sari {
            rd: 3,
            ra: 4,
            imm: 9
        }
    );

    // ---------------------------------------------------------------------
    // I-type / bit immediates
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_btst,
        enc_i(
            generated::opcode::BIT_IMMEDIATE,
            generated::mode::bit_immediate::BTST,
            1
        ),
        DecodedInstruction::Btst { ra: 4, imm: 1 }
    );

    decode_case!(
        decodes_bset,
        enc_i(
            generated::opcode::BIT_IMMEDIATE,
            generated::mode::bit_immediate::BSET,
            2
        ),
        DecodedInstruction::Bset {
            rd: 3,
            ra: 4,
            imm: 2
        }
    );

    decode_case!(
        decodes_bclr,
        enc_i(
            generated::opcode::BIT_IMMEDIATE,
            generated::mode::bit_immediate::BCLR,
            3
        ),
        DecodedInstruction::Bclr {
            rd: 3,
            ra: 4,
            imm: 3
        }
    );

    decode_case!(
        decodes_btgl,
        enc_i(
            generated::opcode::BIT_IMMEDIATE,
            generated::mode::bit_immediate::BTGL,
            4
        ),
        DecodedInstruction::Btgl {
            rd: 3,
            ra: 4,
            imm: 4
        }
    );

    // ---------------------------------------------------------------------
    // R2-type / multiply divide
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_mul,
        enc_r2(generated::func::multiply_divide::MUL),
        DecodedInstruction::Mul {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    decode_case!(
        decodes_mulu,
        enc_r2(generated::func::multiply_divide::MULU),
        DecodedInstruction::Mulu {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    decode_case!(
        decodes_div,
        enc_r2(generated::func::multiply_divide::DIV),
        DecodedInstruction::Div {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    decode_case!(
        decodes_divu,
        enc_r2(generated::func::multiply_divide::DIVU),
        DecodedInstruction::Divu {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    // ---------------------------------------------------------------------
    // U-type / constant construction
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_lui,
        enc_u(generated::mode::constant_construction::LUI, 0x1234),
        DecodedInstruction::Lui {
            rd: 3,
            imm16: 0x1234
        }
    );

    decode_case!(
        decodes_lli,
        enc_u(generated::mode::constant_construction::LLI, 0x5678),
        DecodedInstruction::Lli {
            rd: 3,
            imm16: 0x5678
        }
    );

    decode_case!(
        decodes_lhi,
        enc_u(generated::mode::constant_construction::LHI, 0x9ABC),
        DecodedInstruction::Lhi {
            rd: 3,
            imm16: 0x9ABC
        }
    );

    // ---------------------------------------------------------------------
    // M-type / load store, including positive and negative offsets
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_lbu_positive_offset,
        enc_m(generated::opcode::LOAD, 0, generated::memory_size::BYTE, 12),
        DecodedInstruction::Lbu {
            rd: 3,
            base: 4,
            offset: 12
        }
    );

    decode_case!(
        decodes_lb_negative_offset,
        enc_m(generated::opcode::LOAD, 1, generated::memory_size::BYTE, -1),
        DecodedInstruction::Lb {
            rd: 3,
            base: 4,
            offset: -1
        }
    );

    decode_case!(
        decodes_lhu_positive_offset,
        enc_m(
            generated::opcode::LOAD,
            0,
            generated::memory_size::HALFWORD,
            14
        ),
        DecodedInstruction::Lhu {
            rd: 3,
            base: 4,
            offset: 14
        }
    );

    decode_case!(
        decodes_lh_negative_offset,
        enc_m(
            generated::opcode::LOAD,
            1,
            generated::memory_size::HALFWORD,
            -2
        ),
        DecodedInstruction::Lh {
            rd: 3,
            base: 4,
            offset: -2
        }
    );

    decode_case!(
        decodes_lw_negative_offset,
        enc_m(generated::opcode::LOAD, 0, generated::memory_size::WORD, -4),
        DecodedInstruction::Lw {
            rd: 3,
            base: 4,
            offset: -4
        }
    );

    decode_case!(
        decodes_sb_positive_offset,
        enc_m(
            generated::opcode::STORE,
            0,
            generated::memory_size::BYTE,
            16
        ),
        DecodedInstruction::Sb {
            rs: 3,
            base: 4,
            offset: 16
        }
    );

    decode_case!(
        decodes_sh_negative_offset,
        enc_m(
            generated::opcode::STORE,
            0,
            generated::memory_size::HALFWORD,
            -8
        ),
        DecodedInstruction::Sh {
            rs: 3,
            base: 4,
            offset: -8
        }
    );

    decode_case!(
        decodes_sw_negative_offset,
        enc_m(
            generated::opcode::STORE,
            0,
            generated::memory_size::WORD,
            -12
        ),
        DecodedInstruction::Sw {
            rs: 3,
            base: 4,
            offset: -12
        }
    );

    // ---------------------------------------------------------------------
    // BF-type / flag branches, including positive and negative offsets
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_bf_eq_positive,
        enc_bf(generated::condition::EQ, 4),
        DecodedInstruction::BfEq { offset: 16 }
    );
    decode_case!(
        decodes_bf_ne_negative,
        enc_bf(generated::condition::NE, -4),
        DecodedInstruction::BfNe { offset: -16 }
    );
    decode_case!(
        decodes_bf_lt_negative,
        enc_bf(generated::condition::LT, -8),
        DecodedInstruction::BfLt { offset: -32 }
    );
    decode_case!(
        decodes_bf_le_negative,
        enc_bf(generated::condition::LE, -12),
        DecodedInstruction::BfLe { offset: -48 }
    );
    decode_case!(
        decodes_bf_gt_positive,
        enc_bf(generated::condition::GT, 16),
        DecodedInstruction::BfGt { offset: 64 }
    );
    decode_case!(
        decodes_bf_ge_positive,
        enc_bf(generated::condition::GE, 20),
        DecodedInstruction::BfGe { offset: 80 }
    );
    decode_case!(
        decodes_bf_ltu_negative,
        enc_bf(generated::condition::LTU, -16),
        DecodedInstruction::BfLtu { offset: -64 }
    );
    decode_case!(
        decodes_bf_leu_negative,
        enc_bf(generated::condition::LEU, -20),
        DecodedInstruction::BfLeu { offset: -80 }
    );
    decode_case!(
        decodes_bf_gtu_positive,
        enc_bf(generated::condition::GTU, 24),
        DecodedInstruction::BfGtu { offset: 96 }
    );
    decode_case!(
        decodes_bf_geu_positive,
        enc_bf(generated::condition::GEU, 28),
        DecodedInstruction::BfGeu { offset: 112 }
    );
    decode_case!(
        decodes_bf_cs_negative,
        enc_bf(generated::condition::CS, -24),
        DecodedInstruction::BfCs { offset: -96 }
    );
    decode_case!(
        decodes_bf_cc_negative,
        enc_bf(generated::condition::CC, -28),
        DecodedInstruction::BfCc { offset: -112 }
    );
    decode_case!(
        decodes_bf_vs_positive,
        enc_bf(generated::condition::VS, 32),
        DecodedInstruction::BfVs { offset: 128 }
    );
    decode_case!(
        decodes_bf_vc_positive,
        enc_bf(generated::condition::VC, 36),
        DecodedInstruction::BfVc { offset: 144 }
    );
    decode_case!(
        decodes_bf_es_negative,
        enc_bf(generated::condition::ES, -32),
        DecodedInstruction::BfEs { offset: -128 }
    );
    decode_case!(
        decodes_bf_ec_negative,
        enc_bf(generated::condition::EC, -36),
        DecodedInstruction::BfEc { offset: -144 }
    );

    // ---------------------------------------------------------------------
    // BC-type / register branches, including positive and negative offsets
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_beq_positive,
        enc_bc(generated::condition::EQ, 4),
        DecodedInstruction::BEq {
            ra: 3,
            rb: 4,
            offset: 16
        }
    );

    decode_case!(
        decodes_bne_negative,
        enc_bc(generated::condition::NE, -4),
        DecodedInstruction::BNe {
            ra: 3,
            rb: 4,
            offset: -16
        }
    );

    decode_case!(
        decodes_blt_negative,
        enc_bc(generated::condition::LT, -8),
        DecodedInstruction::BLt {
            ra: 3,
            rb: 4,
            offset: -32
        }
    );

    decode_case!(
        decodes_ble_negative,
        enc_bc(generated::condition::LE, -12),
        DecodedInstruction::BLe {
            ra: 3,
            rb: 4,
            offset: -48
        }
    );

    decode_case!(
        decodes_bgt_positive,
        enc_bc(generated::condition::GT, 16),
        DecodedInstruction::BGt {
            ra: 3,
            rb: 4,
            offset: 64
        }
    );

    decode_case!(
        decodes_bge_positive,
        enc_bc(generated::condition::GE, 20),
        DecodedInstruction::BGe {
            ra: 3,
            rb: 4,
            offset: 80
        }
    );

    decode_case!(
        decodes_bltu_negative,
        enc_bc(generated::condition::LTU, -16),
        DecodedInstruction::BLtu {
            ra: 3,
            rb: 4,
            offset: -64
        }
    );

    decode_case!(
        decodes_bleu_negative,
        enc_bc(generated::condition::LEU, -20),
        DecodedInstruction::BLeu {
            ra: 3,
            rb: 4,
            offset: -80
        }
    );

    decode_case!(
        decodes_bgtu_positive,
        enc_bc(generated::condition::GTU, 24),
        DecodedInstruction::BGtu {
            ra: 3,
            rb: 4,
            offset: 96
        }
    );

    decode_case!(
        decodes_bgeu_positive,
        enc_bc(generated::condition::GEU, 28),
        DecodedInstruction::BGeu {
            ra: 3,
            rb: 4,
            offset: 112
        }
    );

    // ---------------------------------------------------------------------
    // J-type / PC-relative jump and call
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_jmp_positive,
        enc_j(generated::opcode::PC_RELATIVE_JUMP, 1024),
        DecodedInstruction::Jmp { offset: 4096 }
    );

    decode_case!(
        decodes_jmp_negative,
        enc_j(generated::opcode::PC_RELATIVE_JUMP, -1024),
        DecodedInstruction::Jmp { offset: -4096 }
    );

    decode_case!(
        decodes_call_positive,
        enc_j(generated::opcode::PC_RELATIVE_CALL, 2048),
        DecodedInstruction::Call { offset: 8192 }
    );

    decode_case!(
        decodes_call_negative,
        enc_j(generated::opcode::PC_RELATIVE_CALL, -2048),
        DecodedInstruction::Call { offset: -8192 }
    );

    // ---------------------------------------------------------------------
    // JR-type / register jump and call
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_jr,
        enc_jr(generated::func::register_jump_call::JR),
        DecodedInstruction::Jr { target: 4 }
    );

    decode_case!(
        decodes_jalr,
        enc_jr(generated::func::register_jump_call::JALR),
        DecodedInstruction::Jalr { rd: 3, target: 4 }
    );
}
