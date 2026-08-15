use super::{
    Creg, Instruction,
    spec::{self, opcode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    IllegalInstruction(u32),
    IllegalPayload(u32),
}

pub fn decode(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{architecture, opcode};

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

fn decode_r(opcode: u32, raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{
        format::r,
        func::{register_alu, register_shift},
    };

    let func = (raw & r::FUNC_MASK) >> r::FUNC_SHIFT;
    let rd = ((raw & r::RD_MASK) >> r::RD_SHIFT) as u8;
    let ra = ((raw & r::RA_MASK) >> r::RA_SHIFT) as u8;
    let rb = ((raw & r::RB_MASK) >> r::RB_SHIFT) as u8;

    match (opcode, func) {
        (opcode::REGISTER_ALU, register_alu::ADD) => Ok(Instruction::Add { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::SUB) => Ok(Instruction::Sub { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::AND) => Ok(Instruction::And { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::OR) => Ok(Instruction::Or { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::XOR) => Ok(Instruction::Xor { rd, ra, rb }),
        (opcode::REGISTER_ALU, register_alu::NOT) => Ok(Instruction::Not { rd, ra }),
        (opcode::REGISTER_ALU, register_alu::NEG) => Ok(Instruction::Neg { rd, ra }),
        (opcode::REGISTER_ALU, register_alu::CMP) => Ok(Instruction::Cmp { ra, rb }),

        (opcode::REGISTER_SHIFT, register_shift::SHL) => Ok(Instruction::Shl { rd, ra, rb }),
        (opcode::REGISTER_SHIFT, register_shift::SHR) => Ok(Instruction::Shr { rd, ra, rb }),
        (opcode::REGISTER_SHIFT, register_shift::SAR) => Ok(Instruction::Sar { rd, ra, rb }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

#[rustfmt::skip]
fn decode_i(opcode: u32, raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{
        format::i,
        mode::{bit_immediate, immediate_arithmetic_compare, immediate_logical, immediate_shift},
    };

    let mode = (raw & i::MODE_MASK) >> i::MODE_SHIFT;
    let imm16 = (raw & i::IMM16_MASK) >> i::IMM16_SHIFT;
    let imm = ((imm16 as i16) as i32) as u32;
    let ra = ((raw & i::RA_MASK) >> i::RA_SHIFT) as u8;
    let rd = ((raw & i::RD_MASK) >> i::RD_SHIFT) as u8;

    match (opcode, mode) {
        (opcode::IMMEDIATE_ARITHMETIC_COMPARE, immediate_arithmetic_compare::ADDI) => {
            Ok(Instruction::Addi { rd, ra, imm })
        }
        (opcode::IMMEDIATE_ARITHMETIC_COMPARE, immediate_arithmetic_compare::SUBI) => {
            Ok(Instruction::Subi { rd, ra, imm })
        }
        (opcode::IMMEDIATE_ARITHMETIC_COMPARE, immediate_arithmetic_compare::CMPI) => {
            Ok(Instruction::Cmpi { ra, imm })
        }

        (opcode::IMMEDIATE_LOGICAL, immediate_logical::ANDI) => {
            Ok(Instruction::Andi { rd, ra, imm: imm16 })
        }
        (opcode::IMMEDIATE_LOGICAL, immediate_logical::ORI) => {
            Ok(Instruction::Ori { rd, ra, imm: imm16 })
        }
        (opcode::IMMEDIATE_LOGICAL, immediate_logical::XORI) => {
            Ok(Instruction::Xori { rd, ra, imm: imm16 })
        }
        
        (opcode::IMMEDIATE_SHIFT, immediate_shift::SHLI) => {
            Ok(Instruction::Shli { rd, ra, imm: imm16 as u8 })
        }
        (opcode::IMMEDIATE_SHIFT, immediate_shift::SHRI) => {
            Ok(Instruction::Shri { rd, ra, imm: imm16 as u8 })
        }
        (opcode::IMMEDIATE_SHIFT, immediate_shift::SARI) => {
            Ok(Instruction::Sari { rd, ra, imm: imm16 as u8 })
        }

        (opcode::BIT_IMMEDIATE, bit_immediate::BTST) => {
            Ok(Instruction::Btst { ra, imm: imm16 as u8 })
        }
        (opcode::BIT_IMMEDIATE, bit_immediate::BSET) => {
            Ok(Instruction::Bset { rd, ra, imm: imm16 as u8 })
        }
        (opcode::BIT_IMMEDIATE, bit_immediate::BCLR) => {
            Ok(Instruction::Bclr { rd, ra, imm: imm16 as u8 })
        }
        (opcode::BIT_IMMEDIATE, bit_immediate::BTGL) => {
            Ok(Instruction::Btgl { rd, ra, imm: imm16 as u8 })
        }

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_r2(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{format::r2, func::multiply_divide};

    let func = (raw & r2::FUNC_MASK) >> r2::FUNC_SHIFT;
    let rd0 = ((raw & r2::RD0_MASK) >> r2::RD0_SHIFT) as u8;
    let rd1 = ((raw & r2::RD1_MASK) >> r2::RD1_SHIFT) as u8;
    let ra = ((raw & r2::RA_MASK) >> r2::RA_SHIFT) as u8;
    let rb = ((raw & r2::RB_MASK) >> r2::RB_SHIFT) as u8;

    match func {
        multiply_divide::MUL => Ok(Instruction::Mul { rd0, rd1, ra, rb }),
        multiply_divide::MULU => Ok(Instruction::Mulu { rd0, rd1, ra, rb }),
        multiply_divide::DIV => Ok(Instruction::Div { rd0, rd1, ra, rb }),
        multiply_divide::DIVU => Ok(Instruction::Divu { rd0, rd1, ra, rb }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_u(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{format::u, mode::constant_construction};

    let mode = (raw & u::MODE_MASK) >> u::MODE_SHIFT;
    let imm16 = ((raw & u::IMM16_MASK) >> u::IMM16_SHIFT) as u16;
    let rd = ((raw & u::RD_MASK) >> u::RD_SHIFT) as u8;

    match mode {
        constant_construction::LUI => Ok(Instruction::Lui { rd, imm16 }),
        constant_construction::LLI => Ok(Instruction::Lli { rd, imm16 }),
        constant_construction::LHI => Ok(Instruction::Lhi { rd, imm16 }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

#[rustfmt::skip]
fn decode_m(opcode: u32, raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{format::m, memory_size};

    let sx = (raw & m::SX_MASK) >> m::SX_SHIFT;
    let size = (raw & m::SIZE_MASK) >> m::SIZE_SHIFT;
    let mut offset = ((raw & m::OFFSET15_MASK) >> m::OFFSET15_SHIFT) as i32;
    let ss = 32 - m::OFFSET15_WIDTH;
    offset = (offset << ss) >> ss;
    let r = ((raw & m::RD_RS_MASK) >> m::RD_RS_SHIFT) as u8;
    let base = ((raw & m::BASE_MASK) >> m::BASE_SHIFT) as u8;

    match (opcode, sx, size) {
        (opcode::LOAD, 0, memory_size::BYTE) => Ok(Instruction::Lbu { rd: r, base, offset }),
        (opcode::LOAD, 1, memory_size::BYTE) => Ok(Instruction::Lb { rd: r, base, offset }),
        (opcode::LOAD, 0, memory_size::HALFWORD) => Ok(Instruction::Lhu { rd: r, base, offset }),
        (opcode::LOAD, 1, memory_size::HALFWORD) => Ok(Instruction::Lh { rd: r, base, offset }),
        (opcode::LOAD, _, memory_size::WORD) => Ok(Instruction::Lw { rd: r, base, offset }),
        (opcode::STORE, _, memory_size::BYTE) => Ok(Instruction::Sb { rs: r, base, offset }),
        (opcode::STORE, _, memory_size::HALFWORD) => Ok(Instruction::Sh { rs: r, base, offset }),
        (opcode::STORE, _, memory_size::WORD) => Ok(Instruction::Sw { rs: r, base, offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
    
}

fn decode_bf(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{condition, format::bf};

    let cond = (raw & bf::COND_MASK) >> bf::COND_SHIFT;
    let mut offset = ((raw & bf::OFFSET22_MASK) >> bf::OFFSET22_SHIFT) as i32;
    let ss = 32 - bf::OFFSET22_WIDTH;
    offset = (offset << ss) >> (ss - 2); // All branch offsets are SHL2.

    match cond {
        condition::EQ => Ok(Instruction::BfEq { offset }),
        condition::NE => Ok(Instruction::BfNe { offset }),
        condition::LT => Ok(Instruction::BfLt { offset }),
        condition::LE => Ok(Instruction::BfLe { offset }),
        condition::GT => Ok(Instruction::BfGt { offset }),
        condition::GE => Ok(Instruction::BfGe { offset }),
        condition::LTU => Ok(Instruction::BfLtu { offset }),
        condition::LEU => Ok(Instruction::BfLeu { offset }),
        condition::GTU => Ok(Instruction::BfGtu { offset }),
        condition::GEU => Ok(Instruction::BfGeu { offset }),
        condition::CS => Ok(Instruction::BfCs { offset }),
        condition::CC => Ok(Instruction::BfCc { offset }),
        condition::VS => Ok(Instruction::BfVs { offset }),
        condition::VC => Ok(Instruction::BfVc { offset }),
        condition::ES => Ok(Instruction::BfEs { offset }),
        condition::EC => Ok(Instruction::BfEc { offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_bc(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{condition, format::bc};

    let cond = (raw & bc::COND_MASK) >> bc::COND_SHIFT;
    let mut offset = ((raw & bc::OFFSET14_MASK) >> bc::OFFSET14_SHIFT) as i32;
    let ss = 32 - bc::OFFSET14_WIDTH;
    offset = (offset << ss) >> (ss - 2); // All branch offsets are SHL2.
    let ra = ((raw & bc::RA_MASK) >> bc::RA_SHIFT) as u8;
    let rb = ((raw & bc::RB_MASK) >> bc::RB_SHIFT) as u8;

    match cond {
        condition::EQ => Ok(Instruction::BEq { ra, rb, offset }),
        condition::NE => Ok(Instruction::BNe { ra, rb, offset }),
        condition::LT => Ok(Instruction::BLt { ra, rb, offset }),
        condition::LE => Ok(Instruction::BLe { ra, rb, offset }),
        condition::GT => Ok(Instruction::BGt { ra, rb, offset }),
        condition::GE => Ok(Instruction::BGe { ra, rb, offset }),
        condition::LTU => Ok(Instruction::BLtu { ra, rb, offset }),
        condition::LEU => Ok(Instruction::BLeu { ra, rb, offset }),
        condition::GTU => Ok(Instruction::BGtu { ra, rb, offset }),
        condition::GEU => Ok(Instruction::BGeu { ra, rb, offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_j(opcode: u32, raw: u32) -> Result<Instruction, DecodeError> {
    use spec::format::j;

    let mut offset = ((raw & j::OFFSET26_MASK) >> j::OFFSET26_SHIFT) as i32;
    let shift = 32 - j::OFFSET26_WIDTH;
    offset = (offset << shift) >> (shift - 2); // All branch offsets are SHL2.

    match opcode {
        opcode::PC_RELATIVE_JUMP => Ok(Instruction::Jmp { offset }),
        opcode::PC_RELATIVE_CALL => Ok(Instruction::Call { offset }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_jr(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{format::jr, func::register_jump_call};

    let func = (raw & jr::FUNC_MASK) >> jr::FUNC_SHIFT;
    let rd = ((raw & jr::RD_MASK) >> jr::RD_SHIFT) as u8;
    let target = ((raw & jr::TARGET_MASK) >> jr::TARGET_SHIFT) as u8;

    match func {
        register_jump_call::JR => Ok(Instruction::Jr { target }),
        register_jump_call::JALR => Ok(Instruction::Jalr { rd, target }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_x(raw: u32) -> Result<Instruction, DecodeError> {
    use spec::{format::x, sysop};

    let sysop = (raw & x::SYSOP_MASK) >> x::SYSOP_SHIFT;
    let payload = (raw & x::PAYLOAD_MASK) >> x::PAYLOAD_SHIFT;
    let rd = (raw & x::RD_MASK) >> x::RD_SHIFT;
    let rs = (raw & x::RS_MASK) >> x::RS_SHIFT;

    match sysop {
        sysop::NOP => Ok(Instruction::Nop),
        sysop::HALT => Ok(Instruction::Halt),
        sysop::TRAP => Ok(Instruction::SoftwareTrap { imm: payload }),
        sysop::SYSCALL => Ok(Instruction::SystemCall),
        sysop::IRET => Ok(Instruction::IRet),
        sysop::EI => Ok(Instruction::EI),
        sysop::DI => Ok(Instruction::DI),
        sysop::RDPC => Ok(Instruction::RdPc { rd: rd as u8 }),
        sysop::MRS => Ok(Instruction::Mrs {
            creg4: decode_creg(payload)?,
            rd: rd as u8,
        }),
        sysop::MSR => Ok(Instruction::Msr {
            creg4: decode_creg(payload)?,
            rs: rs as u8,
        }),

        _ => Err(DecodeError::IllegalInstruction(raw)),
    }
}

fn decode_creg(payload: u32) -> Result<Creg, DecodeError> {
    use spec::creg;

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
        use spec::architecture;

        (value & architecture::OPCODE_MASK) << architecture::OPCODE_SHIFT
    }

    fn field(mask: u32, shift: u32, value: u32) -> u32 {
        (value << shift) & mask
    }

    fn signed_field(value: i32, width: u32) -> u32 {
        value as u32 & ((1u32 << width) - 1)
    }

    fn enc_r(op: u32, func: u32) -> u32 {
        use spec::format::r;

        opcode(op)
            | field(r::FUNC_MASK, r::FUNC_SHIFT, func)
            | field(r::RD_MASK, r::RD_SHIFT, 3)
            | field(r::RA_MASK, r::RA_SHIFT, 4)
            | field(r::RB_MASK, r::RB_SHIFT, 5)
    }

    fn enc_i(op: u32, mode: u32, imm16: u16) -> u32 {
        use spec::format::i;

        opcode(op)
            | field(i::MODE_MASK, i::MODE_SHIFT, mode)
            | field(i::RD_MASK, i::RD_SHIFT, 3)
            | field(i::RA_MASK, i::RA_SHIFT, 4)
            | field(i::IMM16_MASK, i::IMM16_SHIFT, imm16 as u32)
    }

    fn enc_r2(func: u32) -> u32 {
        use spec::format::r2;

        opcode(spec::opcode::MULTIPLY_DIVIDE)
            | field(r2::FUNC_MASK, r2::FUNC_SHIFT, func)
            | field(r2::RD0_MASK, r2::RD0_SHIFT, 3)
            | field(r2::RD1_MASK, r2::RD1_SHIFT, 4)
            | field(r2::RA_MASK, r2::RA_SHIFT, 5)
            | field(r2::RB_MASK, r2::RB_SHIFT, 6)
    }

    fn enc_u(mode: u32, imm16: u16) -> u32 {
        use spec::format::u;

        opcode(spec::opcode::CONSTANT_CONSTRUCTION)
            | field(u::MODE_MASK, u::MODE_SHIFT, mode)
            | field(u::RD_MASK, u::RD_SHIFT, 3)
            | field(u::IMM16_MASK, u::IMM16_SHIFT, imm16 as u32)
    }

    fn enc_m(op: u32, sx: u32, size: u32, offset: i16) -> u32 {
        use spec::format::m;

        let offset15 = signed_field(offset as i32, m::OFFSET15_WIDTH);

        opcode(op)
            | field(m::SX_MASK, m::SX_SHIFT, sx)
            | field(m::SIZE_MASK, m::SIZE_SHIFT, size)
            | field(m::RD_RS_MASK, m::RD_RS_SHIFT, 3)
            | field(m::BASE_MASK, m::BASE_SHIFT, 4)
            | field(m::OFFSET15_MASK, m::OFFSET15_SHIFT, offset15)
    }

    fn enc_bf(cond: u32, offset: i32) -> u32 {
        use spec::format::bf;

        opcode(spec::opcode::FLAG_BRANCH)
            | field(bf::COND_MASK, bf::COND_SHIFT, cond)
            | field(
                bf::OFFSET22_MASK,
                bf::OFFSET22_SHIFT,
                signed_field(offset, bf::OFFSET22_WIDTH),
            )
    }

    fn enc_bc(cond: u32, offset: i32) -> u32 {
        use spec::format::bc;

        opcode(spec::opcode::REGISTER_BRANCH)
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
        use spec::format::j;

        opcode(op)
            | field(
                j::OFFSET26_MASK,
                j::OFFSET26_SHIFT,
                signed_field(offset, j::OFFSET26_WIDTH),
            )
    }

    fn enc_jr(func: u32) -> u32 {
        use spec::format::jr;

        opcode(spec::opcode::REGISTER_JUMP_CALL)
            | field(jr::FUNC_MASK, jr::FUNC_SHIFT, func)
            | field(jr::RD_MASK, jr::RD_SHIFT, 3)
            | field(jr::TARGET_MASK, jr::TARGET_SHIFT, 4)
    }

    fn enc_x(sysop: u32, payload: u32, rd: u32, rs: u32) -> u32 {
        use spec::format::x;

        opcode(spec::opcode::SYSTEM_CONTROL)
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
        enc_x(spec::sysop::NOP, 0, 0, 0),
        Instruction::Nop
    );
    decode_case!(
        decodes_halt,
        enc_x(spec::sysop::HALT, 0, 0, 0),
        Instruction::Halt
    );

    decode_case!(
        decodes_software_trap,
        enc_x(spec::sysop::TRAP, 0x7A9, 0, 0),
        Instruction::SoftwareTrap { imm: 0x7A9 }
    );

    decode_case!(
        decodes_system_call,
        enc_x(spec::sysop::SYSCALL, 0, 0, 0),
        Instruction::SystemCall
    );

    decode_case!(
        decodes_iret,
        enc_x(spec::sysop::IRET, 0, 0, 0),
        Instruction::IRet
    );
    decode_case!(decodes_ei, enc_x(spec::sysop::EI, 0, 0, 0), Instruction::EI);
    decode_case!(decodes_di, enc_x(spec::sysop::DI, 0, 0, 0), Instruction::DI);

    decode_case!(
        decodes_rdpc,
        enc_x(spec::sysop::RDPC, 0, 3, 0),
        Instruction::RdPc { rd: 3 }
    );

    decode_case!(
        decodes_mrs_pc,
        enc_x(spec::sysop::MRS, spec::creg::PC as u32, 3, 0),
        Instruction::Mrs {
            creg4: Creg::PC,
            rd: 3,
        }
    );

    decode_case!(
        decodes_msr_sr,
        enc_x(spec::sysop::MSR, spec::creg::SR as u32, 0, 4),
        Instruction::Msr {
            creg4: Creg::SR,
            rs: 4,
        }
    );

    // ---------------------------------------------------------------------
    // R-type / register ALU
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_add,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::ADD),
        Instruction::Add {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_sub,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::SUB),
        Instruction::Sub {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_and,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::AND),
        Instruction::And {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_or,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::OR),
        Instruction::Or {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_xor,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::XOR),
        Instruction::Xor {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_not,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::NOT),
        Instruction::Not { rd: 3, ra: 4 }
    );

    decode_case!(
        decodes_neg,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::NEG),
        Instruction::Neg { rd: 3, ra: 4 }
    );

    decode_case!(
        decodes_cmp,
        enc_r(spec::opcode::REGISTER_ALU, spec::func::register_alu::CMP),
        Instruction::Cmp { ra: 4, rb: 5 }
    );

    // ---------------------------------------------------------------------
    // R-type / register shifts
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_shl,
        enc_r(
            spec::opcode::REGISTER_SHIFT,
            spec::func::register_shift::SHL
        ),
        Instruction::Shl {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_shr,
        enc_r(
            spec::opcode::REGISTER_SHIFT,
            spec::func::register_shift::SHR
        ),
        Instruction::Shr {
            rd: 3,
            ra: 4,
            rb: 5
        }
    );

    decode_case!(
        decodes_sar,
        enc_r(
            spec::opcode::REGISTER_SHIFT,
            spec::func::register_shift::SAR
        ),
        Instruction::Sar {
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
            spec::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            spec::mode::immediate_arithmetic_compare::ADDI,
            0x1234,
        ),
        Instruction::Addi {
            rd: 3,
            ra: 4,
            imm: 0x1234
        }
    );

    decode_case!(
        decodes_addi_negative,
        enc_i(
            spec::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            spec::mode::immediate_arithmetic_compare::ADDI,
            0xFFFE,
        ),
        Instruction::Addi {
            rd: 3,
            ra: 4,
            imm: -2i32 as u32
        }
    );

    decode_case!(
        decodes_subi_negative,
        enc_i(
            spec::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            spec::mode::immediate_arithmetic_compare::SUBI,
            0x8000,
        ),
        Instruction::Subi {
            rd: 3,
            ra: 4,
            imm: -32768i32 as u32
        }
    );

    decode_case!(
        decodes_cmpi_negative,
        enc_i(
            spec::opcode::IMMEDIATE_ARITHMETIC_COMPARE,
            spec::mode::immediate_arithmetic_compare::CMPI,
            0xFFFF,
        ),
        Instruction::Cmpi {
            ra: 4,
            imm: -1i32 as u32
        }
    );

    // ---------------------------------------------------------------------
    // I-type / logical immediates
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_andi,
        enc_i(
            spec::opcode::IMMEDIATE_LOGICAL,
            spec::mode::immediate_logical::ANDI,
            0xF0F0,
        ),
        Instruction::Andi {
            rd: 3,
            ra: 4,
            imm: 0xF0F0
        }
    );

    decode_case!(
        decodes_ori,
        enc_i(
            spec::opcode::IMMEDIATE_LOGICAL,
            spec::mode::immediate_logical::ORI,
            0x00FF,
        ),
        Instruction::Ori {
            rd: 3,
            ra: 4,
            imm: 0x00FF
        }
    );

    decode_case!(
        decodes_xori,
        enc_i(
            spec::opcode::IMMEDIATE_LOGICAL,
            spec::mode::immediate_logical::XORI,
            0xAAAA,
        ),
        Instruction::Xori {
            rd: 3,
            ra: 4,
            imm: 0xAAAA
        }
    );

    // ---------------------------------------------------------------------
    // I-type / immediate shifts
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_shli,
        enc_i(
            spec::opcode::IMMEDIATE_SHIFT,
            spec::mode::immediate_shift::SHLI,
            7,
        ),
        Instruction::Shli {
            rd: 3,
            ra: 4,
            imm: 7
        }
    );

    decode_case!(
        decodes_shri,
        enc_i(
            spec::opcode::IMMEDIATE_SHIFT,
            spec::mode::immediate_shift::SHRI,
            8,
        ),
        Instruction::Shri {
            rd: 3,
            ra: 4,
            imm: 8
        }
    );

    decode_case!(
        decodes_sari,
        enc_i(
            spec::opcode::IMMEDIATE_SHIFT,
            spec::mode::immediate_shift::SARI,
            9,
        ),
        Instruction::Sari {
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
            spec::opcode::BIT_IMMEDIATE,
            spec::mode::bit_immediate::BTST,
            1
        ),
        Instruction::Btst { ra: 4, imm: 1 }
    );

    decode_case!(
        decodes_bset,
        enc_i(
            spec::opcode::BIT_IMMEDIATE,
            spec::mode::bit_immediate::BSET,
            2
        ),
        Instruction::Bset {
            rd: 3,
            ra: 4,
            imm: 2
        }
    );

    decode_case!(
        decodes_bclr,
        enc_i(
            spec::opcode::BIT_IMMEDIATE,
            spec::mode::bit_immediate::BCLR,
            3
        ),
        Instruction::Bclr {
            rd: 3,
            ra: 4,
            imm: 3
        }
    );

    decode_case!(
        decodes_btgl,
        enc_i(
            spec::opcode::BIT_IMMEDIATE,
            spec::mode::bit_immediate::BTGL,
            4
        ),
        Instruction::Btgl {
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
        enc_r2(spec::func::multiply_divide::MUL),
        Instruction::Mul {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    decode_case!(
        decodes_mulu,
        enc_r2(spec::func::multiply_divide::MULU),
        Instruction::Mulu {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    decode_case!(
        decodes_div,
        enc_r2(spec::func::multiply_divide::DIV),
        Instruction::Div {
            rd0: 3,
            rd1: 4,
            ra: 5,
            rb: 6
        }
    );

    decode_case!(
        decodes_divu,
        enc_r2(spec::func::multiply_divide::DIVU),
        Instruction::Divu {
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
        enc_u(spec::mode::constant_construction::LUI, 0x1234),
        Instruction::Lui {
            rd: 3,
            imm16: 0x1234
        }
    );

    decode_case!(
        decodes_lli,
        enc_u(spec::mode::constant_construction::LLI, 0x5678),
        Instruction::Lli {
            rd: 3,
            imm16: 0x5678
        }
    );

    decode_case!(
        decodes_lhi,
        enc_u(spec::mode::constant_construction::LHI, 0x9ABC),
        Instruction::Lhi {
            rd: 3,
            imm16: 0x9ABC
        }
    );

    // ---------------------------------------------------------------------
    // M-type / load store, including positive and negative offsets
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_lbu_positive_offset,
        enc_m(spec::opcode::LOAD, 0, spec::memory_size::BYTE, 12),
        Instruction::Lbu {
            rd: 3,
            base: 4,
            offset: 12
        }
    );

    decode_case!(
        decodes_lb_negative_offset,
        enc_m(spec::opcode::LOAD, 1, spec::memory_size::BYTE, -1),
        Instruction::Lb {
            rd: 3,
            base: 4,
            offset: -1
        }
    );

    decode_case!(
        decodes_lhu_positive_offset,
        enc_m(spec::opcode::LOAD, 0, spec::memory_size::HALFWORD, 14),
        Instruction::Lhu {
            rd: 3,
            base: 4,
            offset: 14
        }
    );

    decode_case!(
        decodes_lh_negative_offset,
        enc_m(spec::opcode::LOAD, 1, spec::memory_size::HALFWORD, -2),
        Instruction::Lh {
            rd: 3,
            base: 4,
            offset: -2
        }
    );

    decode_case!(
        decodes_lw_negative_offset,
        enc_m(spec::opcode::LOAD, 0, spec::memory_size::WORD, -4),
        Instruction::Lw {
            rd: 3,
            base: 4,
            offset: -4
        }
    );

    decode_case!(
        decodes_sb_positive_offset,
        enc_m(spec::opcode::STORE, 0, spec::memory_size::BYTE, 16),
        Instruction::Sb {
            rs: 3,
            base: 4,
            offset: 16
        }
    );

    decode_case!(
        decodes_sh_negative_offset,
        enc_m(spec::opcode::STORE, 0, spec::memory_size::HALFWORD, -8),
        Instruction::Sh {
            rs: 3,
            base: 4,
            offset: -8
        }
    );

    decode_case!(
        decodes_sw_negative_offset,
        enc_m(spec::opcode::STORE, 0, spec::memory_size::WORD, -12),
        Instruction::Sw {
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
        enc_bf(spec::condition::EQ, 4),
        Instruction::BfEq { offset: 16 }
    );
    decode_case!(
        decodes_bf_ne_negative,
        enc_bf(spec::condition::NE, -4),
        Instruction::BfNe { offset: -16 }
    );
    decode_case!(
        decodes_bf_lt_negative,
        enc_bf(spec::condition::LT, -8),
        Instruction::BfLt { offset: -32 }
    );
    decode_case!(
        decodes_bf_le_negative,
        enc_bf(spec::condition::LE, -12),
        Instruction::BfLe { offset: -48 }
    );
    decode_case!(
        decodes_bf_gt_positive,
        enc_bf(spec::condition::GT, 16),
        Instruction::BfGt { offset: 64 }
    );
    decode_case!(
        decodes_bf_ge_positive,
        enc_bf(spec::condition::GE, 20),
        Instruction::BfGe { offset: 80 }
    );
    decode_case!(
        decodes_bf_ltu_negative,
        enc_bf(spec::condition::LTU, -16),
        Instruction::BfLtu { offset: -64 }
    );
    decode_case!(
        decodes_bf_leu_negative,
        enc_bf(spec::condition::LEU, -20),
        Instruction::BfLeu { offset: -80 }
    );
    decode_case!(
        decodes_bf_gtu_positive,
        enc_bf(spec::condition::GTU, 24),
        Instruction::BfGtu { offset: 96 }
    );
    decode_case!(
        decodes_bf_geu_positive,
        enc_bf(spec::condition::GEU, 28),
        Instruction::BfGeu { offset: 112 }
    );
    decode_case!(
        decodes_bf_cs_negative,
        enc_bf(spec::condition::CS, -24),
        Instruction::BfCs { offset: -96 }
    );
    decode_case!(
        decodes_bf_cc_negative,
        enc_bf(spec::condition::CC, -28),
        Instruction::BfCc { offset: -112 }
    );
    decode_case!(
        decodes_bf_vs_positive,
        enc_bf(spec::condition::VS, 32),
        Instruction::BfVs { offset: 128 }
    );
    decode_case!(
        decodes_bf_vc_positive,
        enc_bf(spec::condition::VC, 36),
        Instruction::BfVc { offset: 144 }
    );
    decode_case!(
        decodes_bf_es_negative,
        enc_bf(spec::condition::ES, -32),
        Instruction::BfEs { offset: -128 }
    );
    decode_case!(
        decodes_bf_ec_negative,
        enc_bf(spec::condition::EC, -36),
        Instruction::BfEc { offset: -144 }
    );

    // ---------------------------------------------------------------------
    // BC-type / register branches, including positive and negative offsets
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_beq_positive,
        enc_bc(spec::condition::EQ, 4),
        Instruction::BEq {
            ra: 3,
            rb: 4,
            offset: 16
        }
    );

    decode_case!(
        decodes_bne_negative,
        enc_bc(spec::condition::NE, -4),
        Instruction::BNe {
            ra: 3,
            rb: 4,
            offset: -16
        }
    );

    decode_case!(
        decodes_blt_negative,
        enc_bc(spec::condition::LT, -8),
        Instruction::BLt {
            ra: 3,
            rb: 4,
            offset: -32
        }
    );

    decode_case!(
        decodes_ble_negative,
        enc_bc(spec::condition::LE, -12),
        Instruction::BLe {
            ra: 3,
            rb: 4,
            offset: -48
        }
    );

    decode_case!(
        decodes_bgt_positive,
        enc_bc(spec::condition::GT, 16),
        Instruction::BGt {
            ra: 3,
            rb: 4,
            offset: 64
        }
    );

    decode_case!(
        decodes_bge_positive,
        enc_bc(spec::condition::GE, 20),
        Instruction::BGe {
            ra: 3,
            rb: 4,
            offset: 80
        }
    );

    decode_case!(
        decodes_bltu_negative,
        enc_bc(spec::condition::LTU, -16),
        Instruction::BLtu {
            ra: 3,
            rb: 4,
            offset: -64
        }
    );

    decode_case!(
        decodes_bleu_negative,
        enc_bc(spec::condition::LEU, -20),
        Instruction::BLeu {
            ra: 3,
            rb: 4,
            offset: -80
        }
    );

    decode_case!(
        decodes_bgtu_positive,
        enc_bc(spec::condition::GTU, 24),
        Instruction::BGtu {
            ra: 3,
            rb: 4,
            offset: 96
        }
    );

    decode_case!(
        decodes_bgeu_positive,
        enc_bc(spec::condition::GEU, 28),
        Instruction::BGeu {
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
        enc_j(spec::opcode::PC_RELATIVE_JUMP, 1024),
        Instruction::Jmp { offset: 4096 }
    );

    decode_case!(
        decodes_jmp_negative,
        enc_j(spec::opcode::PC_RELATIVE_JUMP, -1024),
        Instruction::Jmp { offset: -4096 }
    );

    decode_case!(
        decodes_call_positive,
        enc_j(spec::opcode::PC_RELATIVE_CALL, 2048),
        Instruction::Call { offset: 8192 }
    );

    decode_case!(
        decodes_call_negative,
        enc_j(spec::opcode::PC_RELATIVE_CALL, -2048),
        Instruction::Call { offset: -8192 }
    );

    // ---------------------------------------------------------------------
    // JR-type / register jump and call
    // ---------------------------------------------------------------------

    decode_case!(
        decodes_jr,
        enc_jr(spec::func::register_jump_call::JR),
        Instruction::Jr { target: 4 }
    );

    decode_case!(
        decodes_jalr,
        enc_jr(spec::func::register_jump_call::JALR),
        Instruction::Jalr { rd: 3, target: 4 }
    );
}
