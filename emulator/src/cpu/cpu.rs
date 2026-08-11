use super::{Creg, CregFile, DecodedInstruction, ExceptionCause, GprFile, decode};

use crate::{
    lifecycle::{Init, Reset, Tick},
    platform::{SystemBus, SystemBusError},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CpuState {
    Running,
    Halted,
}

#[derive(Debug)]
pub struct Cpu {
    gpr: GprFile,
    creg: CregFile,
    bus: SystemBus,
    state: CpuState,
}

impl Cpu {
    pub fn new(bus: SystemBus) -> Self {
        Self {
            gpr: GprFile::new(),
            creg: CregFile::new(),
            bus,
            state: CpuState::Halted,
        }
    }

    fn read_gpr(&self, index: u8) -> u32 {
        self.gpr.read(index as usize)
    }

    fn write_gpr(&mut self, index: u8, value: u32) {
        self.gpr.write(index as usize, value);
    }

    pub fn reset(&mut self) {
        // Reset is vector cause 0. With the default evbase of 0, reset begins at
        // physical address 0, where the reset stub is expected to jump to boot code.
        self.gpr.reset();
        self.creg.reset();
        self.state = CpuState::Running;
        self.bus.reset();
    }

    pub fn halt(&mut self) {
        self.state = CpuState::Halted;
    }

    pub fn is_halted(&self) -> bool {
        self.state == CpuState::Halted
    }

    fn tick_with_bus(&mut self) {
        if self.is_halted() {
            return;
        }

        let fetch_pc = self.creg.read_register(Creg::PC);

        let instruction = match self.bus.read32(fetch_pc) {
            Ok(instruction) => instruction,

            // The bus reports data-word misalignment generically. During instruction fetch,
            // that maps to the architectural MisalignedInstruction cause.
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedInstructionFetch, fetch_pc);
                self.bus.tick();
                return;
            }

            // Unsupported/unmapped fetch has no dedicated architectural cause in the current
            // 8-entry exception table, so the reference machine halts and preserves state
            // for debugger/test inspection.
            Err(
                SystemBusError::AddressUnmapped { .. } | SystemBusError::UnsupportedAccess { .. },
            ) => {
                self.creg.write_register(Creg::EData, fetch_pc);
                self.halt();
                return;
            }
        };

        self.creg.advance_pc_word();
        match decode(instruction) {
            Ok(decoded) => self.execute(decoded),
            Err(_) => self
                .creg
                .raise_exception(ExceptionCause::IllegalInstruction, fetch_pc),
        }
        self.bus.tick();
    }

    #[rustfmt::skip]
    fn execute(&mut self, instruction: DecodedInstruction) {
        match instruction {
            // Control Instructions. These don't set flags, so handled inline here.
            DecodedInstruction::Nop => (),
            DecodedInstruction::Halt => self.halt(),
            DecodedInstruction::SoftwareTrap { imm } => self.creg.raise_exception(ExceptionCause::SoftwareTrap, imm as u32),
            DecodedInstruction::SystemCall => self.creg.raise_exception(ExceptionCause::SystemCall, self.creg.read_register(Creg::PC)),
            DecodedInstruction::IRet => self.creg.iret(),
            DecodedInstruction::EI => self.creg.ei(),
            DecodedInstruction::DI => self.creg.di(),
            DecodedInstruction::RdPc { rd } => self.write_gpr(rd, self.creg.read_register(Creg::PC)),
            DecodedInstruction::Mrs { creg4, rd } => self.write_gpr(rd, self.creg.read_register(creg4)),
            DecodedInstruction::Msr { creg4, rs } => self.creg.write_register(creg4, self.read_gpr(rs)),

            // Register ALU
            DecodedInstruction::Add { rd, ra, rb } => self.execute_add(rd, ra, rb),
            DecodedInstruction::Sub { rd, ra, rb } => self.execute_sub(rd, ra, rb),
            DecodedInstruction::And { rd, ra, rb } => self.execute_and(rd, ra, rb),
            DecodedInstruction::Or { rd, ra, rb } => self.execute_or(rd, ra, rb),
            DecodedInstruction::Xor { rd, ra, rb } => self.execute_xor(rd, ra, rb),
            DecodedInstruction::Not { rd, ra } => self.execute_not(rd, ra),
            DecodedInstruction::Neg { rd, ra } => self.execute_neg(rd, ra),
            DecodedInstruction::Cmp { ra, rb } => self.execute_cmp(ra, rb),

            // Immediate Arithmetic and Compare
            DecodedInstruction::Addi { rd, ra, sext32 } => self.execute_addi(rd, ra, sext32),
            DecodedInstruction::Subi { rd, ra, sext32 } => self.execute_subi(rd, ra, sext32),
            DecodedInstruction::Cmpi { ra, sext32 } => self.execute_cmpi(ra, sext32),

            // Immediate Logical
            DecodedInstruction::Andi { rd, ra, imm32 } => self.execute_andi(rd, ra, imm32),
            DecodedInstruction::Ori { rd, ra, imm32 } => self.execute_ori(rd, ra, imm32),
            DecodedInstruction::Xori { rd, ra, imm32 } => self.execute_xori(rd, ra, imm32),

            // Register Shifts
            DecodedInstruction::Shl { rd, ra, rb } => self.execute_shl(rd, ra, rb),
            DecodedInstruction::Shr { rd, ra, rb } => self.execute_shr(rd, ra, rb),
            DecodedInstruction::Sar { rd, ra, rb } => self.execute_sar(rd, ra, rb),

            // Immediate Shifts
            DecodedInstruction::Shli { rd, ra, imm } => self.execute_shli(rd, ra, imm),
            DecodedInstruction::Shri { rd, ra, imm } => self.execute_shri(rd, ra, imm),
            DecodedInstruction::Sari { rd, ra, imm } => self.execute_sari(rd, ra, imm),

            // Bit Immediate
            DecodedInstruction::Btst { ra, imm } => self.execute_btst(ra, imm),
            DecodedInstruction::Bset { rd, ra, imm } => self.execute_bset(rd, ra, imm),
            DecodedInstruction::Bclr { rd, ra, imm } => self.execute_bclr(rd, ra, imm),
            DecodedInstruction::Btgl { rd, ra, imm } => self.execute_btgl(rd, ra, imm),

            // Multiply and Divide
            DecodedInstruction::Mul { rd0, rd1, ra, rb } => self.execute_mul(rd0, rd1, ra, rb),
            DecodedInstruction::Mulu { rd0, rd1, ra, rb } => self.execute_mulu(rd0, rd1, ra, rb),
            DecodedInstruction::Div { rd0, rd1, ra, rb } => self.execute_div(rd0, rd1, ra, rb),
            DecodedInstruction::Divu { rd0, rd1, ra, rb } => self.execute_divu(rd0, rd1, ra, rb),

            // Constant Construction
            DecodedInstruction::Lui { rd, imm16 } => self.execute_lui(rd, imm16),
            DecodedInstruction::Lli { rd, imm16 } => self.execute_lli(rd, imm16),
            DecodedInstruction::Lhi { rd, imm16 } => self.execute_lhi(rd, imm16),

            // Load and Store
            DecodedInstruction::Lb { rd, base, offset } => self.execute_lb(rd, base, offset),
            DecodedInstruction::Lbu { rd, base, offset } => self.execute_lbu(rd, base, offset),
            DecodedInstruction::Lh { rd, base, offset } => self.execute_lh(rd, base, offset),
            DecodedInstruction::Lhu { rd, base, offset } => self.execute_lhu(rd, base, offset),
            DecodedInstruction::Lw { rd, base, offset } => self.execute_lw(rd, base, offset),
            DecodedInstruction::Sb { rs, base, offset } => self.execute_sb(rs, base, offset),
            DecodedInstruction::Sh { rs, base, offset } => self.execute_sh(rs, base, offset),
            DecodedInstruction::Sw { rs, base, offset } => self.execute_sw(rs, base, offset),

            // Branch on Flag
            DecodedInstruction::BfEq { offset } => self.execute_bf_eq(offset),
            DecodedInstruction::BfNe { offset } => self.execute_bf_ne(offset),
            DecodedInstruction::BfLt { offset } => self.execute_bf_lt(offset),
            DecodedInstruction::BfLe { offset } => self.execute_bf_le(offset),
            DecodedInstruction::BfGt { offset } => self.execute_bf_gt(offset),
            DecodedInstruction::BfGe { offset } => self.execute_bf_ge(offset),
            DecodedInstruction::BfLtu { offset } => self.execute_bf_ltu(offset),
            DecodedInstruction::BfLeu { offset } => self.execute_bf_leu(offset),
            DecodedInstruction::BfGtu { offset } => self.execute_bf_gtu(offset),
            DecodedInstruction::BfGeu { offset } => self.execute_bf_geu(offset),
            DecodedInstruction::BfCs { offset } => self.execute_bf_cs(offset),
            DecodedInstruction::BfCc { offset } => self.execute_bf_cc(offset),
            DecodedInstruction::BfVs { offset } => self.execute_bf_vs(offset),
            DecodedInstruction::BfVc { offset } => self.execute_bf_vc(offset),
            DecodedInstruction::BfEs { offset } => self.execute_bf_es(offset),
            DecodedInstruction::BfEc { offset } => self.execute_bf_ec(offset),

            // Branch on Register Comparison
            DecodedInstruction::BEq { ra, rb, offset } => self.execute_b_eq(ra, rb, offset),
            DecodedInstruction::BNe { ra, rb, offset } => self.execute_b_ne(ra, rb, offset),
            DecodedInstruction::BLt { ra, rb, offset } => self.execute_b_lt(ra, rb, offset),
            DecodedInstruction::BLe { ra, rb, offset } => self.execute_b_le(ra, rb, offset),
            DecodedInstruction::BGt { ra, rb, offset } => self.execute_b_gt(ra, rb, offset),
            DecodedInstruction::BGe { ra, rb, offset } => self.execute_b_ge(ra, rb, offset),
            DecodedInstruction::BLtu { ra, rb, offset } => self.execute_b_ltu(ra, rb, offset),
            DecodedInstruction::BLeu { ra, rb, offset } => self.execute_b_leu(ra, rb, offset),
            DecodedInstruction::BGtu { ra, rb, offset } => self.execute_b_gtu(ra, rb, offset),
            DecodedInstruction::BGeu { ra, rb, offset } => self.execute_b_geu(ra, rb, offset),

            // Jump/Call Immediate
            DecodedInstruction::Jmp { offset } => self.execute_jmp(offset),
            DecodedInstruction::Call { offset } => self.execute_call(offset),

            // Jump/Call Register
            DecodedInstruction::Jr { target } => self.execute_jr(target),
            DecodedInstruction::Jalr { rd, target } => self.execute_jalr(rd, target),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Register ALU
    //
    // Add, Sub, And, Or, Xor, Not, Neg
    fn execute_add(&mut self, rd: u8, ra: u8, rb: u8) {
        let a = self.read_gpr(ra);
        let b = self.read_gpr(rb);

        let (result, carry) = a.overflowing_add(b);
        let (_, overflow) = (a as i32).overflowing_add(b as i32);

        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, carry, overflow);
    }

    fn execute_sub(&mut self, rd: u8, ra: u8, rb: u8) {
        let a = self.read_gpr(ra);
        let b = self.read_gpr(rb);

        let (result, borrow) = a.overflowing_sub(b);
        let (_, overflow) = (a as i32).overflowing_sub(b as i32);

        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, !borrow, overflow);
    }

    fn execute_and(&mut self, rd: u8, ra: u8, rb: u8) {
        let value = self.read_gpr(ra) & self.read_gpr(rb);
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_or(&mut self, rd: u8, ra: u8, rb: u8) {
        let value = self.read_gpr(ra) | self.read_gpr(rb);
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_xor(&mut self, rd: u8, ra: u8, rb: u8) {
        let value = self.read_gpr(ra) ^ self.read_gpr(rb);
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_not(&mut self, rd: u8, ra: u8) {
        let value = !self.read_gpr(ra);
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_neg(&mut self, rd: u8, ra: u8) {
        let value = -(self.read_gpr(ra) as i32);
        self.write_gpr(rd, value as u32);
        self.creg
            .update_sr_flags(false, value == 0, value < 0, false, false);
    }

    fn execute_cmp(&mut self, ra: u8, rb: u8) {
        let a = self.read_gpr(ra);
        let b = self.read_gpr(rb);
        let result = a.wrapping_sub(b);

        let no_unsigned_borrow = a >= b;
        let (_, overflow) = (a as i32).overflowing_sub(b as i32);

        self.creg.update_sr_flags(
            false,
            result == 0,
            (result as i32) < 0,
            no_unsigned_borrow,
            overflow,
        );
    }

    ///////////////////////////////////////////////////////////////////////////
    // Immediate Arithmetic and Compare
    fn execute_addi(&mut self, rd: u8, ra: u8, sext32: i32) {
        let a = self.read_gpr(ra);
        let b = sext32 as u32;

        let (result, carry) = a.overflowing_add(b);
        let (_, overflow) = (a as i32).overflowing_add(sext32);

        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, carry, overflow);
    }

    fn execute_subi(&mut self, rd: u8, ra: u8, sext32: i32) {
        let a = self.read_gpr(ra);
        let b = sext32 as u32;

        let (result, borrow) = a.overflowing_sub(b);
        let (_, overflow) = (a as i32).overflowing_sub(sext32);

        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, !borrow, overflow);
    }

    fn execute_cmpi(&mut self, ra: u8, sext32: i32) {
        let a = self.read_gpr(ra);
        let b_bits = sext32 as u32;
        let (_, overflow) = (a as i32).overflowing_sub(sext32);

        self.creg.update_sr_flags(
            false,
            a == b_bits,
            (a.wrapping_sub(b_bits) as i32) < 0,
            a >= b_bits,
            overflow,
        );
    }

    ///////////////////////////////////////////////////////////////////////////
    // Immediate Logical
    fn execute_andi(&mut self, rd: u8, ra: u8, imm32: u32) {
        let value = self.read_gpr(ra) & imm32;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_ori(&mut self, rd: u8, ra: u8, imm32: u32) {
        let value = self.read_gpr(ra) | imm32;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_xori(&mut self, rd: u8, ra: u8, imm32: u32) {
        let value = self.read_gpr(ra) ^ imm32;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Register Shifts
    fn execute_shl(&mut self, rd: u8, ra: u8, rb: u8) {
        let value = (self.read_gpr(ra) as u64) << self.read_gpr(rb) as u8;
        let carry = (value >> 32) & 0x01;
        self.write_gpr(rd, value as u32);
        self.creg.sr().clear_condition_flags();
        self.creg.sr().set_carry(carry != 0);
    }

    fn execute_shr(&mut self, rd: u8, ra: u8, rb: u8) {
        self.creg.sr().clear_condition_flags();
        let mut value = self.read_gpr(ra);
        let imm = self.read_gpr(rb) as u8;
        if imm != 0 {
            value >>= imm - 1;
            let carry = value & 0x01;
            value >>= 1;
            self.creg.sr().set_carry(carry != 0);
        }
        self.write_gpr(rd, value);
    }

    fn execute_sar(&mut self, rd: u8, ra: u8, rb: u8) {
        self.creg.sr().clear_condition_flags();
        let mut value = self.read_gpr(ra) as i32;
        let imm = self.read_gpr(rb) as u8;
        if imm != 0 {
            value >>= imm - 1;
            let carry = value & 0x01;
            value >>= 1;
            self.creg.sr().set_carry(carry != 0);
        }
        self.write_gpr(rd, value as u32);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Immediate Shift
    fn execute_shli(&mut self, rd: u8, ra: u8, imm: u8) {
        let value = (self.read_gpr(ra) as u64) << imm;
        let carry = (value >> 32) & 0x01;
        self.write_gpr(rd, value as u32);
        self.creg.sr().clear_condition_flags();
        self.creg.sr().set_carry(carry != 0);
    }

    fn execute_shri(&mut self, rd: u8, ra: u8, imm: u8) {
        self.creg.sr().clear_condition_flags();
        let mut value = self.read_gpr(ra);
        if imm != 0 {
            value >>= imm - 1;
            let carry = value & 0x01;
            value >>= 1;
            self.creg.sr().set_carry(carry != 0);
        }
        self.write_gpr(rd, value);
    }

    fn execute_sari(&mut self, rd: u8, ra: u8, imm: u8) {
        self.creg.sr().clear_condition_flags();
        let mut value = self.read_gpr(ra) as i32;
        if imm != 0 {
            value >>= imm - 1;
            let carry = value & 0x01;
            value >>= 1;
            self.creg.sr().set_carry(carry != 0);
        }
        self.write_gpr(rd, value as u32);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Bit Immediate
    fn execute_btst(&mut self, ra: u8, imm: u8) {
        let mask = 0x01u32 << imm;
        let value = self.read_gpr(ra) & mask;
        self.creg.sr().clear_condition_flags();
        self.creg.sr().set_zero(value == 0);
    }

    fn execute_bset(&mut self, rd: u8, ra: u8, imm: u8) {
        let mask = 0x01u32 << imm;
        let value = self.read_gpr(ra) | mask;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_bclr(&mut self, rd: u8, ra: u8, imm: u8) {
        let mask = !(0x01u32 << imm);
        let value = self.read_gpr(ra) & mask;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_btgl(&mut self, rd: u8, ra: u8, imm: u8) {
        let mask = 0x01u32 << imm;
        let value = self.read_gpr(ra) ^ mask;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Multiply and Divide
    fn execute_mul(&mut self, rd0: u8, rd1: u8, ra: u8, rb: u8) {
        let result = self.read_gpr(ra) as i64 * self.read_gpr(rb) as i64;
        self.write_gpr(rd0, result as u32);
        self.write_gpr(rd1, (result >> 32) as u32);
        self.creg
            .update_sr_flags(false, result == 0, result < 0, false, false);
    }

    fn execute_mulu(&mut self, rd0: u8, rd1: u8, ra: u8, rb: u8) {
        let result = self.read_gpr(ra) as u64 * self.read_gpr(rb) as u64;
        self.write_gpr(rd0, result as u32);
        self.write_gpr(rd1, (result >> 32) as u32);
        self.creg
            .update_sr_flags(false, result == 0, (result as i64) < 0, false, false);
    }

    fn execute_div(&mut self, rd0: u8, rd1: u8, ra: u8, rb: u8) {
        let dividend = self.read_gpr(ra) as i32;
        let divisor = self.read_gpr(rb) as i32;

        if divisor == 0 {
            self.creg.update_sr_flags(true, false, false, false, false);
            return;
        }

        let Some(quotient) = dividend.checked_div(divisor) else {
            self.creg.update_sr_flags(true, false, false, false, true);
            return;
        };

        let remainder = dividend % divisor;

        self.write_gpr(rd0, quotient as u32);
        self.write_gpr(rd1, remainder as u32);

        self.creg
            .update_sr_flags(false, quotient == 0, quotient < 0, false, false);
    }

    fn execute_divu(&mut self, rd0: u8, rd1: u8, ra: u8, rb: u8) {
        let dividend = self.read_gpr(ra);
        let divisor = self.read_gpr(rb);

        if divisor == 0 {
            self.creg.update_sr_flags(true, false, false, false, false);
            return;
        }

        let quotient = dividend / divisor;
        let remainder = dividend % divisor;

        self.write_gpr(rd0, quotient);
        self.write_gpr(rd1, remainder);

        self.creg
            .update_sr_flags(false, quotient == 0, (quotient as i32) < 0, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Constant Construction
    fn execute_lui(&mut self, rd: u8, imm16: u16) {
        self.write_gpr(rd, (imm16 as u32) << 16);
        self.creg
            .update_sr_flags(false, imm16 == 0, (imm16 as i32) < 0, false, false);
    }

    fn execute_lli(&mut self, rd: u8, imm16: u16) {
        self.write_gpr(rd, imm16 as u32);
        self.creg
            .update_sr_flags(false, imm16 == 0, (imm16 as i32) < 0, false, false);
    }

    fn execute_lhi(&mut self, rd: u8, imm16: u16) {
        let value = ((imm16 as u32) << 16) | self.read_gpr(rd);
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, imm16 == 0, (imm16 as i32) < 0, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Load and Store
    fn execute_lb(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read8(address) {
            Ok(data) => self.write_gpr(rd, data as i8 as i32 as u32),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_lbu(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read8(address) {
            Ok(data) => self.write_gpr(rd, data as u32),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_lh(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read16(address) {
            Ok(data) => self.write_gpr(rd, data as i16 as i32 as u32),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_lhu(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read16(address) {
            Ok(data) => self.write_gpr(rd, data as u32),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_lw(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read32(address) {
            Ok(data) => self.write_gpr(rd, data),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_sb(&mut self, rs: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.write8(address, self.read_gpr(rs) as u8) {
            Ok(_) => (),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_sh(&mut self, rs: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.write16(address, self.read_gpr(rs) as u16) {
            Ok(_) => (),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_sw(&mut self, rs: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.write32(address, self.read_gpr(rs)) {
            Ok(_) => (),
            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            Err(SystemBusError::AddressUnmapped { .. })
            | Err(SystemBusError::UnsupportedAccess { .. }) => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Branch on Flag
    fn execute_bf_eq(&mut self, offset: i32) {
        if self.creg.sr().zero() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_ne(&mut self, offset: i32) {
        if !self.creg.sr().zero() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_lt(&mut self, offset: i32) {
        if self.creg.sr().negative() != self.creg.sr().overflow() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_le(&mut self, offset: i32) {
        if self.creg.sr().zero() || self.creg.sr().negative() != self.creg.sr().overflow() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_gt(&mut self, offset: i32) {
        if !self.creg.sr().zero() && self.creg.sr().negative() == self.creg.sr().overflow() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_ge(&mut self, offset: i32) {
        if self.creg.sr().negative() == self.creg.sr().overflow() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_ltu(&mut self, offset: i32) {
        if !self.creg.sr().carry() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_leu(&mut self, offset: i32) {
        if !self.creg.sr().carry() || self.creg.sr().zero() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_gtu(&mut self, offset: i32) {
        if self.creg.sr().carry() && !self.creg.sr().zero() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_geu(&mut self, offset: i32) {
        if self.creg.sr().carry() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_cs(&mut self, offset: i32) {
        if self.creg.sr().carry() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_cc(&mut self, offset: i32) {
        if !self.creg.sr().carry() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_vs(&mut self, offset: i32) {
        if self.creg.sr().overflow() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_vc(&mut self, offset: i32) {
        if !self.creg.sr().overflow() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_es(&mut self, offset: i32) {
        if self.creg.sr().arithmetic_error() {
            self.execute_jmp(offset);
        }
    }

    fn execute_bf_ec(&mut self, offset: i32) {
        if !self.creg.sr().arithmetic_error() {
            self.execute_jmp(offset);
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Branch on Register Comparison
    fn execute_b_eq(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) == self.read_gpr(rb) {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_ne(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) != self.read_gpr(rb) {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_lt(&mut self, ra: u8, rb: u8, offset: i32) {
        if (self.read_gpr(ra) as i32) < (self.read_gpr(rb) as i32) {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_le(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) as i32 <= self.read_gpr(rb) as i32 {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_gt(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) as i32 > self.read_gpr(rb) as i32 {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_ge(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) as i32 >= self.read_gpr(rb) as i32 {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_ltu(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) < self.read_gpr(rb) {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_leu(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) <= self.read_gpr(rb) {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_gtu(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) > self.read_gpr(rb) {
            self.execute_jmp(offset);
        }
    }

    fn execute_b_geu(&mut self, ra: u8, rb: u8, offset: i32) {
        if self.read_gpr(ra) >= self.read_gpr(rb) {
            self.execute_jmp(offset);
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Jump/Call Immediate
    fn execute_jmp(&mut self, offset: i32) {
        self.creg.write_register(
            Creg::PC,
            self.creg
                .read_register(Creg::PC)
                .wrapping_add_signed(offset),
        );
    }

    fn execute_call(&mut self, offset: i32) {
        use crate::isa::generated::gpr;

        // Note that PC is updated between fetch and execute, so points to
        // the correct return address already.
        self.write_gpr(gpr::R15 as u8, self.creg.read_register(Creg::PC));
        self.execute_jmp(offset);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Jump/Call Register
    fn execute_jr(&mut self, target: u8) {
        self.creg.write_register(Creg::PC, self.read_gpr(target));
    }

    fn execute_jalr(&mut self, rd: u8, target: u8) {
        self.write_gpr(rd, self.creg.read_register(Creg::PC));
        self.creg.write_register(Creg::PC, self.read_gpr(target));
    }
}

impl Init for Cpu {
    fn init(&mut self) {
        self.reset();
    }
}

impl Reset for Cpu {
    fn reset(&mut self) {
        self.gpr.reset();
        self.creg.reset();
    }
}

impl Tick for Cpu {
    fn tick(&mut self) {
        self.tick_with_bus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{ProgramCounter, StatusRegister};
    use crate::isa::generated::gpr;

    #[test]
    fn new_cpu_starts_reset() {
        let cpu = Cpu::new(SystemBus::new(1024));

        assert_eq!(
            cpu.creg.read_register(Creg::PC),
            ProgramCounter::RESET_VALUE
        );
        assert_eq!(
            cpu.creg.read_register(Creg::SR),
            StatusRegister::RESET_VALUE
        );

        for index in 0..GprFile::COUNT {
            assert_eq!(cpu.read_gpr(index as u8), 0);
        }
    }

    #[test]
    fn reset_clears_cpu_state() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.creg.write_register(Creg::PC, 0x1234_5678);
        cpu.creg.write_register(Creg::SR, 0x0000_00FF);
        cpu.write_gpr(gpr::R1 as u8, 0xAAAA_BBBB);

        cpu.reset();

        assert_eq!(
            cpu.creg.read_register(Creg::PC),
            ProgramCounter::RESET_VALUE
        );
        assert_eq!(
            cpu.creg.read_register(Creg::SR),
            StatusRegister::RESET_VALUE
        );
        assert_eq!(cpu.read_gpr(gpr::R1 as u8), 0);
    }

    #[test]
    fn init_resets_cpu_state() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.creg.write_register(Creg::PC, 0x1234_5678);
        cpu.creg.write_register(Creg::SR, 0x0000_00FF);
        cpu.write_gpr(gpr::R1 as u8, 0xAAAA_BBBB);

        cpu.init();

        assert_eq!(
            cpu.creg.read_register(Creg::PC),
            ProgramCounter::RESET_VALUE
        );
        assert_eq!(
            cpu.creg.read_register(Creg::SR),
            StatusRegister::RESET_VALUE
        );
        assert_eq!(cpu.read_gpr(gpr::R1 as u8), 0);
    }

    #[test]
    fn r0_always_reads_as_zero() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.write_gpr(gpr::R0 as u8, 0xFFFF_FFFF);

        assert_eq!(cpu.read_gpr(gpr::R0 as u8), 0);
    }

    #[test]
    fn nonzero_gprs_round_trip() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.write_gpr(gpr::R1 as u8, 0x1234_5678);
        cpu.write_gpr(gpr::R15 as u8, 0xCAFE_BABE);

        assert_eq!(cpu.read_gpr(gpr::R1 as u8), 0x1234_5678);
        assert_eq!(cpu.read_gpr(gpr::R15 as u8), 0xCAFE_BABE);
    }

    #[test]
    fn tick_is_currently_a_noop() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.tick();

        assert_eq!(
            cpu.creg.read_register(Creg::PC),
            ProgramCounter::RESET_VALUE
        );
        assert_eq!(
            cpu.creg.read_register(Creg::SR),
            StatusRegister::RESET_VALUE
        );
    }

    #[test]
    fn set_sr_masks_reserved_bits() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.creg.write_register(Creg::SR, u32::MAX);

        assert_eq!(cpu.creg.read_register(Creg::SR), StatusRegister::VALID_MASK);
    }

    #[test]
    fn tick_incrememts_pc() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.reset();
        cpu.tick();

        assert_eq!(cpu.creg.read_register(Creg::PC), 4);
    }
}
