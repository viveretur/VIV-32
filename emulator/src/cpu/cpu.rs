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

    fn reset_internal(&mut self) {
        // Reset is vector cause 0. With the default evbase of 0, reset begins at
        // physical address 0, where the reset stub is expected to jump to boot code.
        self.gpr.reset();
        self.creg.reset();
        self.state = CpuState::Running;
        self.bus.reset();
    }

    pub fn reset(&mut self) {
        self.reset_internal();
    }

    pub fn halt(&mut self) {
        self.state = CpuState::Halted;
    }

    pub fn is_halted(&self) -> bool {
        self.state == CpuState::Halted
    }

    fn check_interrupts(&mut self) {
        use crate::platform::PendingInterrupt;

        if !self.creg.sr().interrupt_enable() {
            return;
        }

        match self.bus.pending_interrupt() {
            Some(PendingInterrupt::Timer) => {
                self.creg.raise_exception(ExceptionCause::TimerInterrupt, 0);
            }

            Some(PendingInterrupt::External { source }) => {
                self.creg
                    .raise_exception(ExceptionCause::ExternalInterrupt, source);
            }

            None => {}
        }
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

            // Raise an error if we simply couldn't read the location. This could happen
            // because the data is read-only, or its simply beyond the installed RAM location,
            // or a device doesn't map that area.
            Err(
                SystemBusError::AddressUnmapped { .. } | SystemBusError::UnsupportedAccess { .. },
            ) => {
                self.creg
                    .raise_exception(ExceptionCause::BusError, fetch_pc);
                self.bus.tick();
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
            DecodedInstruction::SystemCall => self.creg.raise_exception(ExceptionCause::SystemCall, 0),
            DecodedInstruction::IRet => self.creg.iret(),
            DecodedInstruction::EI => self.creg.ei(),
            DecodedInstruction::DI => self.creg.di(),
            DecodedInstruction::RdPc { rd } => self.write_gpr(rd, self.creg.read_register(Creg::PC)),
            DecodedInstruction::Mrs { creg4, rd } => self.write_gpr(rd, self.creg.read_register(creg4)),
            DecodedInstruction::Msr { creg4, rs } => self.creg.write_register(creg4, self.read_gpr(rs)),

            // Register ALU
            DecodedInstruction::Add { rd, ra, rb } => self.execute_add(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::Sub { rd, ra, rb } => self.execute_sub(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::And { rd, ra, rb } => self.execute_and(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::Or { rd, ra, rb } => self.execute_or(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::Xor { rd, ra, rb } => self.execute_xor(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::Not { rd, ra } => self.execute_not(rd, ra),
            DecodedInstruction::Neg { rd, ra } => self.execute_neg(rd, ra),
            DecodedInstruction::Cmp { ra, rb } => self.execute_cmp(ra, self.read_gpr(rb)),

            // Immediate Arithmetic and Compare
            DecodedInstruction::Addi { rd, ra, imm } => self.execute_add(rd, ra, imm),
            DecodedInstruction::Subi { rd, ra, imm } => self.execute_sub(rd, ra, imm),
            DecodedInstruction::Cmpi { ra, imm } => self.execute_cmp(ra, imm),

            // Immediate Logical
            DecodedInstruction::Andi { rd, ra, imm } => self.execute_and(rd, ra, imm),
            DecodedInstruction::Ori { rd, ra, imm } => self.execute_or(rd, ra, imm),
            DecodedInstruction::Xori { rd, ra, imm } => self.execute_xor(rd, ra, imm),

            // Register Shifts
            DecodedInstruction::Shl { rd, ra, rb } => self.execute_shl(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::Shr { rd, ra, rb } => self.execute_shr(rd, ra, self.read_gpr(rb)),
            DecodedInstruction::Sar { rd, ra, rb } => self.execute_sar(rd, ra, self.read_gpr(rb)),

            // Immediate Shifts
            DecodedInstruction::Shli { rd, ra, imm } => self.execute_shl(rd, ra, imm as u32),
            DecodedInstruction::Shri { rd, ra, imm } => self.execute_shr(rd, ra, imm as u32),
            DecodedInstruction::Sari { rd, ra, imm } => self.execute_sar(rd, ra, imm as u32),

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
            DecodedInstruction::BfEq { offset } => self.execute_jmp(offset, self.creg.sr().zero()),
            DecodedInstruction::BfNe { offset } => self.execute_jmp(offset, !self.creg.sr().zero()),
            DecodedInstruction::BfLt { offset } => self.execute_jmp(offset, self.creg.sr().negative() != self.creg.sr().overflow()),
            DecodedInstruction::BfLe { offset } => self.execute_jmp(offset, self.creg.sr().zero() || self.creg.sr().negative() != self.creg.sr().overflow()),
            DecodedInstruction::BfGt { offset } => self.execute_jmp(offset, !self.creg.sr().zero() && self.creg.sr().negative() == self.creg.sr().overflow()),
            DecodedInstruction::BfGe { offset } => self.execute_jmp(offset, self.creg.sr().negative() == self.creg.sr().overflow()),
            DecodedInstruction::BfLtu { offset } => self.execute_jmp(offset, !self.creg.sr().carry()),
            DecodedInstruction::BfLeu { offset } => self.execute_jmp(offset, !self.creg.sr().carry() || self.creg.sr().zero()),
            DecodedInstruction::BfGtu { offset } => self.execute_jmp(offset, self.creg.sr().carry() && !self.creg.sr().zero()),
            DecodedInstruction::BfGeu { offset } => self.execute_jmp(offset, self.creg.sr().carry()),
            DecodedInstruction::BfCs { offset } => self.execute_jmp(offset, self.creg.sr().carry()),
            DecodedInstruction::BfCc { offset } => self.execute_jmp(offset, !self.creg.sr().carry()),
            DecodedInstruction::BfVs { offset } => self.execute_jmp(offset, self.creg.sr().overflow()),
            DecodedInstruction::BfVc { offset } => self.execute_jmp(offset, !self.creg.sr().overflow()),
            DecodedInstruction::BfEs { offset } => self.execute_jmp(offset, self.creg.sr().arithmetic_error()),
            DecodedInstruction::BfEc { offset } => self.execute_jmp(offset, !self.creg.sr().arithmetic_error()),

            // Branch on Register Comparison
            DecodedInstruction::BEq { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) == self.read_gpr(rb)),
            DecodedInstruction::BNe { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) != self.read_gpr(rb)),
            DecodedInstruction::BLt { ra, rb, offset } => self.execute_jmp(offset, (self.read_gpr(ra) as i32) < self.read_gpr(rb) as i32),
            DecodedInstruction::BLe { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) as i32 <= self.read_gpr(rb) as i32),
            DecodedInstruction::BGt { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) as i32 > self.read_gpr(rb) as i32),
            DecodedInstruction::BGe { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) as i32 >= self.read_gpr(rb) as i32),
            DecodedInstruction::BLtu { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) < self.read_gpr(rb)), 
            DecodedInstruction::BLeu { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) <= self.read_gpr(rb)),
            DecodedInstruction::BGtu { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) > self.read_gpr(rb)),
            DecodedInstruction::BGeu { ra, rb, offset } => self.execute_jmp(offset, self.read_gpr(ra) >= self.read_gpr(rb)),

            // Jump/Call Immediate
            DecodedInstruction::Jmp { offset } => self.execute_jmp(offset, true),
            DecodedInstruction::Call { offset } => self.execute_call(offset),

            // Jump/Call Register
            DecodedInstruction::Jr { target } => self.execute_jr(target),
            DecodedInstruction::Jalr { rd, target } => self.execute_jalr(rd, target),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // ALU
    //
    // add, addi, sub, subi, and, andi, or, ori, xor, xori, not, neg, cmp, cmpi
    fn execute_add(&mut self, rd: u8, ra: u8, imm: u32) {
        let a = self.read_gpr(ra);

        let (result, carry) = a.overflowing_add(imm);
        let (_, overflow) = (a as i32).overflowing_add(imm as i32);

        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, carry, overflow);
    }

    fn execute_sub(&mut self, rd: u8, ra: u8, imm: u32) {
        let a = self.read_gpr(ra);

        let (result, borrow) = a.overflowing_sub(imm);
        let (_, overflow) = (a as i32).overflowing_sub(imm as i32);

        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, !borrow, overflow);
    }

    fn execute_and(&mut self, rd: u8, ra: u8, imm32: u32) {
        let value = self.read_gpr(ra) & imm32;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_or(&mut self, rd: u8, ra: u8, imm32: u32) {
        let value = self.read_gpr(ra) | imm32;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_xor(&mut self, rd: u8, ra: u8, imm32: u32) {
        let value = self.read_gpr(ra) ^ imm32;
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
        let a = self.read_gpr(ra);
        let result = 0u32.wrapping_sub(a);
        let overflow = (a as i32) == i32::MIN;
        let carry = a == 0;
        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, carry, overflow);
    }

    fn execute_cmp(&mut self, ra: u8, imm: u32) {
        let a = self.read_gpr(ra);
        let (_, overflow) = (a as i32).overflowing_sub(imm as i32);

        self.creg.update_sr_flags(
            false,
            a == imm,
            (a.wrapping_sub(imm) as i32) < 0,
            a >= imm,
            overflow,
        );
    }

    ///////////////////////////////////////////////////////////////////////////
    // Shifts
    //
    // shl, shli, shr, shri, sar, sari
    fn shift_guard(&mut self, count: u32) -> Option<u8> {
        if count > 31 {
            self.creg.update_sr_flags(true, false, false, false, false);
            None
        } else {
            Some(count as u8)
        }
    }

    fn execute_shl(&mut self, rd: u8, ra: u8, imm: u32) {
        let Some(shift) = self.shift_guard(imm) else {
            return;
        };
        let value = self.read_gpr(ra);
        let carry = shift != 0 && ((value >> (32 - shift)) & 1) != 0;
        let result = value << shift;
        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, carry, false);
    }

    fn execute_shr(&mut self, rd: u8, ra: u8, imm: u32) {
        let Some(shift) = self.shift_guard(imm) else {
            return;
        };
        let original = self.read_gpr(ra);
        let carry = shift != 0 && ((original >> (shift - 1)) & 1) != 0;
        let result = original >> shift;
        self.write_gpr(rd, result);
        self.creg
            .update_sr_flags(false, result == 0, (result as i32) < 0, carry, false);
    }

    fn execute_sar(&mut self, rd: u8, ra: u8, imm: u32) {
        let Some(shift) = self.shift_guard(imm) else {
            return;
        };
        let original = self.read_gpr(ra) as i32;
        let carry = shift != 0 && (((original as u32) >> (shift - 1)) & 1) != 0;
        let result = original >> shift;
        self.write_gpr(rd, result as u32);
        self.creg
            .update_sr_flags(false, result == 0, result < 0, carry, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Bit Immediate
    //
    // btst, bset, bclr, btgl
    fn execute_btst(&mut self, ra: u8, imm: u8) {
        let mask = 0x01u32 << imm;
        let value = self.read_gpr(ra) & mask;
        self.creg
            .update_sr_flags(false, value == 0, false, false, false);
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
    //
    // mul, mulu, div, divu
    fn execute_mul(&mut self, rd0: u8, rd1: u8, ra: u8, rb: u8) {
        let result = self.read_gpr(ra) as i32 as i64 * self.read_gpr(rb) as i32 as i64;
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
            .update_sr_flags(false, result == 0, false, false, false);
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
            .update_sr_flags(false, quotient == 0, false, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Constant Construction
    //
    // lui, lli, lhi
    fn execute_lui(&mut self, rd: u8, imm16: u16) {
        let value = (imm16 as u32) << 16;
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    fn execute_lli(&mut self, rd: u8, imm16: u16) {
        self.write_gpr(rd, imm16 as u32);
        self.creg
            .update_sr_flags(false, imm16 == 0, false, false, false);
    }

    fn execute_lhi(&mut self, rd: u8, imm16: u16) {
        let value = ((imm16 as u32) << 16) | self.read_gpr(rd);
        self.write_gpr(rd, value);
        self.creg
            .update_sr_flags(false, value == 0, (value as i32) < 0, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Load and Store
    //
    // lb, lbu, lh, lhu, lw, sb, sh, sw
    fn load_store_bus_error(&mut self, error: SystemBusError, address: u32) {
        match error {
            SystemBusError::MisalignedAccess { .. } => {
                self.creg
                    .raise_exception(ExceptionCause::MisalignedDataAccess, address);
            }
            SystemBusError::AddressUnmapped { .. } | SystemBusError::UnsupportedAccess { .. } => {
                self.creg.raise_exception(ExceptionCause::BusError, address);
            }
        }
    }

    fn execute_lb(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read8(address) {
            Ok(data) => self.write_gpr(rd, data as i8 as i32 as u32),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_lbu(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read8(address) {
            Ok(data) => self.write_gpr(rd, data as u32),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_lh(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read16(address) {
            Ok(data) => self.write_gpr(rd, data as i16 as i32 as u32),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_lhu(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read16(address) {
            Ok(data) => self.write_gpr(rd, data as u32),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_lw(&mut self, rd: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.read32(address) {
            Ok(data) => self.write_gpr(rd, data),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_sb(&mut self, rs: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.write8(address, self.read_gpr(rs) as u8) {
            Ok(_) => (),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_sh(&mut self, rs: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.write16(address, self.read_gpr(rs) as u16) {
            Ok(_) => (),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    fn execute_sw(&mut self, rs: u8, base: u8, offset: i32) {
        let address = self.read_gpr(base).wrapping_add_signed(offset);
        match self.bus.write32(address, self.read_gpr(rs)) {
            Ok(_) => (),
            Err(error) => self.load_store_bus_error(error, address),
        }
    }

    ///////////////////////////////////////////////////////////////////////////
    // Jump/Call Immediate
    fn execute_jmp(&mut self, offset: i32, condition: bool) {
        if condition {
            self.creg.write_register(
                Creg::PC,
                self.creg
                    .read_register(Creg::PC)
                    .wrapping_add_signed(offset),
            );
        }
    }

    fn execute_call(&mut self, offset: i32) {
        use crate::isa::generated::gpr;

        // Note that PC is updated between fetch and execute, so points to
        // the correct return address already.
        self.write_gpr(gpr::R15 as u8, self.creg.read_register(Creg::PC));
        self.execute_jmp(offset, true);
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
        self.reset_internal();
    }
}

impl Reset for Cpu {
    fn reset(&mut self) {
        self.reset_internal();
    }
}

impl Tick for Cpu {
    fn tick(&mut self) {
        self.tick_with_bus();
        self.check_interrupts();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{ProgramCounter, StatusRegister};
    use crate::isa::generated::gpr;

    #[test]
    fn new_cpu_starts_halted_with_reset_registers() {
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
    fn halted_tick_is_a_noop() {
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
    fn tick_increments_pc() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.reset();
        cpu.tick();

        assert_eq!(cpu.creg.read_register(Creg::PC), 4);
    }

    #[test]
    fn interrupt_sets_vector_and_iret_returns() {
        use crate::isa::generated::{sysop, format::x};
        
        // These values come from SystemBus/Timer
        const TIMER_BASE: u32 = 0xFFFF_0200;
        const TIMER_COUNTER: u32 = TIMER_BASE + 0x00;
        const TIMER_CONTROL: u32 = TIMER_BASE + 0x04;
        const TIMER_COMPARE: u32 = TIMER_BASE + 0x08;

        let mut cpu = Cpu::new(SystemBus::new(1024));
        cpu.reset();
        cpu.creg.sr_mut().set_interrupt_enable(true);
        cpu.bus.write32(TIMER_COUNTER, 246).unwrap();
        cpu.bus.write32(TIMER_CONTROL, 0x0000_0003).unwrap(); // enable | irq_enable
        cpu.bus.write32(TIMER_COMPARE, 0x0000_0100).unwrap(); // 256

        // Iret instruction at timer interrupt handler.
        let iret = sysop::IRET << x::SYSOP_SHIFT;
        cpu.bus.write32(96, iret).unwrap();

        assert!(cpu.creg.read_register(Creg::PC) < 10);
        let sr = cpu.creg.read_register(Creg::SR);
        for _ in 0..10 {
            cpu.tick();
        }
        // Interrupt vector for timer tick is 0x60 (6 * 16 bytes).
        assert_eq!(96, cpu.creg.read_register(Creg::PC));
        assert!(!cpu.creg.sr().interrupt_enable());
        assert_eq!(40, cpu.creg.read_register(Creg::EPC)); // PC would have been at 11th word from starting.
        assert_eq!(sr, cpu.creg.read_register(Creg::ESR));
        assert_ne!(sr, cpu.creg.read_register(Creg::SR));
        assert_eq!(ExceptionCause::TimerInterrupt as u32, cpu.creg.read_register(Creg::ECause));
        assert_eq!(0, cpu.creg.read_register(Creg::EData));

        // Normally, clearing the interrupt is software's job.
        // Here, we're just manually disabling it.
        cpu.bus.write32(TIMER_CONTROL, 0).unwrap();
        cpu.tick();
        
        assert_eq!(40, cpu.creg.read_register(Creg::PC));
        assert!(cpu.creg.sr().interrupt_enable());
        assert_eq!(sr, cpu.creg.read_register(Creg::SR));
    }
}

#[cfg(test)]
mod instruction_unit_tests {
    use super::*;
    use crate::isa::generated::gpr;

    fn cpu() -> Cpu {
        let mut cpu = Cpu::new(SystemBus::new(4096));
        cpu.reset();
        cpu
    }

    fn r(reg: usize) -> u8 {
        reg as u8
    }

    fn set(cpu: &mut Cpu, reg: usize, value: u32) {
        cpu.write_gpr(r(reg), value);
    }

    fn get(cpu: &Cpu, reg: usize) -> u32 {
        cpu.read_gpr(r(reg))
    }

    fn set_pc(cpu: &mut Cpu, value: u32) {
        cpu.creg.write_register(Creg::PC, value);
    }

    fn pc(cpu: &Cpu) -> u32 {
        cpu.creg.read_register(Creg::PC)
    }

    fn assert_flags(cpu: &mut Cpu, ae: bool, z: bool, n: bool, c: bool, v: bool) {
        assert_eq!(cpu.creg.sr().arithmetic_error(), ae, "AE");
        assert_eq!(cpu.creg.sr().zero(), z, "Z");
        assert_eq!(cpu.creg.sr().negative(), n, "N");
        assert_eq!(cpu.creg.sr().carry(), c, "C");
        assert_eq!(cpu.creg.sr().overflow(), v, "V");
    }

    fn assert_exception(cpu: &Cpu, cause: ExceptionCause, edata: u32) {
        assert_eq!(cpu.creg.read_register(Creg::ECause), cause as u32, "ECAUSE");
        assert_eq!(cpu.creg.read_register(Creg::EData), edata, "EDATA");
    }

    ///////////////////////////////////////////////////////////////////////////
    // Control instructions

    #[test]
    fn nop_leaves_visible_state_unchanged() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        set(&mut cpu, gpr::R1, 0xCAFE_BABE);
        cpu.creg.update_sr_flags(false, false, true, true, false);

        cpu.execute(DecodedInstruction::Nop);

        assert_eq!(pc(&cpu), 0x100);
        assert_eq!(get(&cpu, gpr::R1), 0xCAFE_BABE);
        assert_flags(&mut cpu, false, false, true, true, false);
        assert!(!cpu.is_halted());
    }

    #[test]
    fn halt_sets_cpu_halted() {
        let mut cpu = cpu();
        cpu.execute(DecodedInstruction::Halt);
        assert!(cpu.is_halted());
    }

    #[test]
    fn rdpc_reads_current_pc() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x1234_5678);
        cpu.execute(DecodedInstruction::RdPc { rd: r(gpr::R2) });
        assert_eq!(get(&cpu, gpr::R2), 0x1234_5678);
    }

    #[test]
    fn mrs_reads_control_register() {
        let mut cpu = cpu();
        cpu.creg.write_register(Creg::EData, 0xDEAD_BEEF);
        cpu.execute(DecodedInstruction::Mrs {
            creg4: Creg::EData,
            rd: r(gpr::R3),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xDEAD_BEEF);
    }

    #[test]
    fn msr_writes_control_register() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R3, 0xBEEF_CAFE);
        cpu.execute(DecodedInstruction::Msr {
            creg4: Creg::EData,
            rs: r(gpr::R3),
        });
        assert_eq!(cpu.creg.read_register(Creg::EData), 0xBEEF_CAFE);
    }

    #[test]
    fn syscall_raises_system_call_with_zero_edata() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x200);
        cpu.execute(DecodedInstruction::SystemCall);
        assert_exception(&cpu, ExceptionCause::SystemCall, 0);
        assert_eq!(cpu.creg.read_register(Creg::EPC), 0x200);
    }

    #[test]
    fn software_trap_raises_trap_with_imm_edata() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x204);
        cpu.execute(DecodedInstruction::SoftwareTrap { imm: -4 });
        assert_exception(&cpu, ExceptionCause::SoftwareTrap, 0xFFFF_FFFC);
        assert_eq!(cpu.creg.read_register(Creg::EPC), 0x204);
    }

    #[test]
    fn iret_restores_pc_and_sr() {
        let mut cpu = cpu();
        cpu.creg.write_register(Creg::EPC, 0x3456_789A);
        cpu.creg.write_register(Creg::ESR, 0x0000_001F);
        cpu.execute(DecodedInstruction::IRet);
        assert_eq!(cpu.creg.read_register(Creg::PC), 0x3456_789A);
        assert_eq!(cpu.creg.read_register(Creg::SR), 0x0000_001F);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Add/Sub/Cmp, including immediate forms

    #[test]
    fn add_sets_unsigned_carry_and_zero() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_FFFF);
        set(&mut cpu, gpr::R2, 1);
        cpu.execute(DecodedInstruction::Add {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, true, false);
    }

    #[test]
    fn add_sets_signed_overflow() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x7FFF_FFFF);
        set(&mut cpu, gpr::R2, 1);
        cpu.execute(DecodedInstruction::Add {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0000);
        assert_flags(&mut cpu, false, false, true, false, true);
    }

    #[test]
    fn addi_uses_sign_extended_bit_pattern() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 1);
        cpu.execute(DecodedInstruction::Addi {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 0xFFFF_FFFF,
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, true, false);
    }

    #[test]
    fn sub_sets_unsigned_borrow_as_carry_clear() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        set(&mut cpu, gpr::R2, 1);
        cpu.execute(DecodedInstruction::Sub {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xFFFF_FFFF);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn sub_sets_signed_overflow() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        set(&mut cpu, gpr::R2, 1);
        cpu.execute(DecodedInstruction::Sub {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x7FFF_FFFF);
        assert_flags(&mut cpu, false, false, false, true, true);
    }

    #[test]
    fn subi_uses_sign_extended_bit_pattern() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 1);
        cpu.execute(DecodedInstruction::Subi {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 0xFFFF_FFFF,
        });
        assert_eq!(get(&cpu, gpr::R3), 2);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn cmp_sets_subtraction_flags_without_writing_registers() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        set(&mut cpu, gpr::R2, 1);
        set(&mut cpu, gpr::R3, 0xA5A5_A5A5);
        cpu.execute(DecodedInstruction::Cmp {
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xA5A5_A5A5);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn cmp_equal_sets_zero_and_no_borrow() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x1234_5678);
        set(&mut cpu, gpr::R2, 0x1234_5678);
        cpu.execute(DecodedInstruction::Cmp {
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_flags(&mut cpu, false, true, false, true, false);
    }

    #[test]
    fn cmpi_uses_sign_extended_bit_pattern() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        cpu.execute(DecodedInstruction::Cmpi {
            ra: r(gpr::R1),
            imm: 0xFFFF_FFFF,
        });
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Logical instructions

    #[test]
    fn and_sets_zero_when_mask_clears_all_bits() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xF0F0_0000);
        set(&mut cpu, gpr::R2, 0x0000_F0F0);
        cpu.execute(DecodedInstruction::And {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, false, false);
    }

    #[test]
    fn andi_uses_zero_extended_immediate_operand() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_FFFF);
        cpu.execute(DecodedInstruction::Andi {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 0x0000_8000,
        });
        assert_eq!(get(&cpu, gpr::R3), 0x0000_8000);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn or_sets_negative_from_result() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        set(&mut cpu, gpr::R2, 0x0000_0001);
        cpu.execute(DecodedInstruction::Or {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0001);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn xor_toggles_bits() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_0000);
        set(&mut cpu, gpr::R2, 0x00FF_00FF);
        cpu.execute(DecodedInstruction::Xor {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xFF00_00FF);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn not_inverts_all_bits() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_0000);
        cpu.execute(DecodedInstruction::Not {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x0000_FFFF);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn neg_zero_sets_zero_and_no_borrow() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        cpu.execute(DecodedInstruction::Neg {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, true, false);
    }

    #[test]
    fn neg_one_wraps_to_all_ones() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 1);
        cpu.execute(DecodedInstruction::Neg {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xFFFF_FFFF);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn neg_min_i32_sets_overflow() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        cpu.execute(DecodedInstruction::Neg {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0000);
        assert_flags(&mut cpu, false, false, true, false, true);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Shifts

    #[test]
    fn shl_by_zero_preserves_value_and_clears_carry() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        set(&mut cpu, gpr::R2, 0);
        cpu.execute(DecodedInstruction::Shl {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0000);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn shl_sets_carry_from_last_bit_shifted_out() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0001);
        cpu.execute(DecodedInstruction::Shli {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 1,
        });
        assert_eq!(get(&cpu, gpr::R3), 0x0000_0002);
        assert_flags(&mut cpu, false, false, false, true, false);
    }

    #[test]
    fn shl_by_31_sets_carry_from_original_bit_1() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x0000_0002);
        cpu.execute(DecodedInstruction::Shli {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 31,
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, true, false);
    }

    #[test]
    fn shl_invalid_count_sets_ae_and_does_not_write_destination() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 1);
        set(&mut cpu, gpr::R2, 32);
        set(&mut cpu, gpr::R3, 0xDEAD_BEEF);
        cpu.execute(DecodedInstruction::Shl {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xDEAD_BEEF);
        assert_flags(&mut cpu, true, false, false, false, false);
    }

    #[test]
    fn shr_sets_carry_from_last_bit_shifted_out() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 3);
        cpu.execute(DecodedInstruction::Shri {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 1,
        });
        assert_eq!(get(&cpu, gpr::R3), 1);
        assert_flags(&mut cpu, false, false, false, true, false);
    }

    #[test]
    fn shr_by_31_uses_original_bit_30_for_carry() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x4000_0000);
        cpu.execute(DecodedInstruction::Shri {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 31,
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, true, false);
    }

    #[test]
    fn sar_preserves_sign_bit() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        cpu.execute(DecodedInstruction::Sari {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 1,
        });
        assert_eq!(get(&cpu, gpr::R3), 0xC000_0000);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn sar_sets_carry_from_last_bit_shifted_out() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0001);
        cpu.execute(DecodedInstruction::Sari {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 1,
        });
        assert_eq!(get(&cpu, gpr::R3), 0xC000_0000);
        assert_flags(&mut cpu, false, false, true, true, false);
    }

    #[test]
    fn sar_invalid_count_sets_ae_and_does_not_write_destination() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        set(&mut cpu, gpr::R2, 33);
        set(&mut cpu, gpr::R3, 0xDEAD_BEEF);
        cpu.execute(DecodedInstruction::Sar {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xDEAD_BEEF);
        assert_flags(&mut cpu, true, false, false, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Bit immediate instructions

    #[test]
    fn btst_set_bit_clears_zero_and_does_not_write_register() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x80);
        cpu.execute(DecodedInstruction::Btst {
            ra: r(gpr::R1),
            imm: 7,
        });
        assert_eq!(get(&cpu, gpr::R1), 0x80);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn btst_clear_bit_sets_zero() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        cpu.execute(DecodedInstruction::Btst {
            ra: r(gpr::R1),
            imm: 7,
        });
        assert_flags(&mut cpu, false, true, false, false, false);
    }

    #[test]
    fn bset_sets_selected_bit_and_updates_result_flags() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        cpu.execute(DecodedInstruction::Bset {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 31,
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0000);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn bclr_clears_selected_bit() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_FFFF);
        cpu.execute(DecodedInstruction::Bclr {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 31,
        });
        assert_eq!(get(&cpu, gpr::R3), 0x7FFF_FFFF);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn btgl_toggles_selected_bit() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 1);
        cpu.execute(DecodedInstruction::Btgl {
            rd: r(gpr::R3),
            ra: r(gpr::R1),
            imm: 0,
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Multiply and divide

    #[test]
    fn mul_signed_writes_low_and_high_words() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_FFFE);
        set(&mut cpu, gpr::R2, 3);
        cpu.execute(DecodedInstruction::Mul {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xFFFF_FFFA);
        assert_eq!(get(&cpu, gpr::R4), 0xFFFF_FFFF);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn mul_zero_sets_zero_flag() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0);
        set(&mut cpu, gpr::R2, 0x1234_5678);
        cpu.execute(DecodedInstruction::Mul {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_eq!(get(&cpu, gpr::R4), 0);
        assert_flags(&mut cpu, false, true, false, false, false);
    }

    #[test]
    fn mulu_writes_unsigned_high_word_and_clears_negative() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xFFFF_FFFF);
        set(&mut cpu, gpr::R2, 2);
        cpu.execute(DecodedInstruction::Mulu {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xFFFF_FFFE);
        assert_eq!(get(&cpu, gpr::R4), 1);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn div_signed_writes_quotient_and_remainder() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 7);
        set(&mut cpu, gpr::R2, 3);
        cpu.execute(DecodedInstruction::Div {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 2);
        assert_eq!(get(&cpu, gpr::R4), 1);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn div_signed_negative_uses_truncating_semantics() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, (-7i32) as u32);
        set(&mut cpu, gpr::R2, 3);
        cpu.execute(DecodedInstruction::Div {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), (-2i32) as u32);
        assert_eq!(get(&cpu, gpr::R4), (-1i32) as u32);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn div_by_zero_sets_ae_and_does_not_write_destinations() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 7);
        set(&mut cpu, gpr::R2, 0);
        set(&mut cpu, gpr::R3, 0xAAAA_AAAA);
        set(&mut cpu, gpr::R4, 0xBBBB_BBBB);
        cpu.execute(DecodedInstruction::Div {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xAAAA_AAAA);
        assert_eq!(get(&cpu, gpr::R4), 0xBBBB_BBBB);
        assert_flags(&mut cpu, true, false, false, false, false);
    }

    #[test]
    fn div_min_i32_by_minus_one_sets_ae_and_overflow_without_write() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, i32::MIN as u32);
        set(&mut cpu, gpr::R2, (-1i32) as u32);
        set(&mut cpu, gpr::R3, 0xAAAA_AAAA);
        set(&mut cpu, gpr::R4, 0xBBBB_BBBB);
        cpu.execute(DecodedInstruction::Div {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xAAAA_AAAA);
        assert_eq!(get(&cpu, gpr::R4), 0xBBBB_BBBB);
        assert_flags(&mut cpu, true, false, false, false, true);
    }

    #[test]
    fn divu_writes_unsigned_quotient_and_remainder() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 7);
        set(&mut cpu, gpr::R2, 3);
        cpu.execute(DecodedInstruction::Divu {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 2);
        assert_eq!(get(&cpu, gpr::R4), 1);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn divu_clears_negative_even_when_quotient_has_top_bit() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x8000_0000);
        set(&mut cpu, gpr::R2, 1);
        cpu.execute(DecodedInstruction::Divu {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0000);
        assert_eq!(get(&cpu, gpr::R4), 0);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn divu_by_zero_sets_ae_and_does_not_write_destinations() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 7);
        set(&mut cpu, gpr::R2, 0);
        set(&mut cpu, gpr::R3, 0xAAAA_AAAA);
        set(&mut cpu, gpr::R4, 0xBBBB_BBBB);
        cpu.execute(DecodedInstruction::Divu {
            rd0: r(gpr::R3),
            rd1: r(gpr::R4),
            ra: r(gpr::R1),
            rb: r(gpr::R2),
        });
        assert_eq!(get(&cpu, gpr::R3), 0xAAAA_AAAA);
        assert_eq!(get(&cpu, gpr::R4), 0xBBBB_BBBB);
        assert_flags(&mut cpu, true, false, false, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Constant construction

    #[test]
    fn lui_writes_high_half_and_sets_negative_from_result() {
        let mut cpu = cpu();
        cpu.execute(DecodedInstruction::Lui {
            rd: r(gpr::R3),
            imm16: 0x8000,
        });
        assert_eq!(get(&cpu, gpr::R3), 0x8000_0000);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    #[test]
    fn lui_zero_sets_zero() {
        let mut cpu = cpu();
        cpu.execute(DecodedInstruction::Lui {
            rd: r(gpr::R3),
            imm16: 0,
        });
        assert_eq!(get(&cpu, gpr::R3), 0);
        assert_flags(&mut cpu, false, true, false, false, false);
    }

    #[test]
    fn lli_zero_extends_immediate_and_never_sets_negative() {
        let mut cpu = cpu();
        cpu.execute(DecodedInstruction::Lli {
            rd: r(gpr::R3),
            imm16: 0xFFFF,
        });
        assert_eq!(get(&cpu, gpr::R3), 0x0000_FFFF);
        assert_flags(&mut cpu, false, false, false, false, false);
    }

    #[test]
    fn lhi_sets_high_half_and_preserves_low_half_when_high_half_was_clear() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R3, 0x0000_5678);
        cpu.execute(DecodedInstruction::Lhi {
            rd: r(gpr::R3),
            imm16: 0xABCD,
        });
        assert_eq!(get(&cpu, gpr::R3), 0xABCD_5678);
        assert_flags(&mut cpu, false, false, true, false, false);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Load/store

    #[test]
    fn lb_sign_extends_loaded_byte() {
        let mut cpu = cpu();
        cpu.bus.write8(0x100, 0x80).unwrap();
        set(&mut cpu, gpr::R1, 0x100);
        cpu.execute(DecodedInstruction::Lb {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0xFFFF_FF80);
    }

    #[test]
    fn lbu_zero_extends_loaded_byte() {
        let mut cpu = cpu();
        cpu.bus.write8(0x100, 0x80).unwrap();
        set(&mut cpu, gpr::R1, 0x100);
        cpu.execute(DecodedInstruction::Lbu {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0x0000_0080);
    }

    #[test]
    fn lh_sign_extends_loaded_halfword() {
        let mut cpu = cpu();
        cpu.bus.write16(0x100, 0x8001).unwrap();
        set(&mut cpu, gpr::R1, 0x100);
        cpu.execute(DecodedInstruction::Lh {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0xFFFF_8001);
    }

    #[test]
    fn lhu_zero_extends_loaded_halfword() {
        let mut cpu = cpu();
        cpu.bus.write16(0x100, 0x8001).unwrap();
        set(&mut cpu, gpr::R1, 0x100);
        cpu.execute(DecodedInstruction::Lhu {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0x0000_8001);
    }

    #[test]
    fn lw_reads_full_word() {
        let mut cpu = cpu();
        cpu.bus.write32(0x100, 0x1234_5678).unwrap();
        set(&mut cpu, gpr::R1, 0x100);
        cpu.execute(DecodedInstruction::Lw {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0x1234_5678);
    }

    #[test]
    fn load_uses_signed_byte_offset() {
        let mut cpu = cpu();
        cpu.bus.write32(0x100, 0xCAFE_BABE).unwrap();
        set(&mut cpu, gpr::R1, 0x104);
        cpu.execute(DecodedInstruction::Lw {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: -4,
        });
        assert_eq!(get(&cpu, gpr::R2), 0xCAFE_BABE);
    }

    #[test]
    fn sb_writes_low_byte() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x100);
        set(&mut cpu, gpr::R2, 0xAABB_CCDD);
        cpu.execute(DecodedInstruction::Sb {
            rs: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(cpu.bus.read8(0x100).unwrap(), 0xDD);
    }

    #[test]
    fn sh_writes_low_halfword() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x100);
        set(&mut cpu, gpr::R2, 0xAABB_CCDD);
        cpu.execute(DecodedInstruction::Sh {
            rs: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(cpu.bus.read16(0x100).unwrap(), 0xCCDD);
    }

    #[test]
    fn sw_writes_full_word() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x100);
        set(&mut cpu, gpr::R2, 0xAABB_CCDD);
        cpu.execute(DecodedInstruction::Sw {
            rs: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(cpu.bus.read32(0x100).unwrap(), 0xAABB_CCDD);
    }

    #[test]
    fn misaligned_load_raises_misaligned_data_access_and_does_not_write_rd() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x101);
        set(&mut cpu, gpr::R2, 0xDEAD_BEEF);
        cpu.execute(DecodedInstruction::Lh {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0xDEAD_BEEF);
        assert_exception(&cpu, ExceptionCause::MisalignedDataAccess, 0x101);
    }

    #[test]
    fn unmapped_load_raises_bus_error_and_does_not_write_rd() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0x2000);
        set(&mut cpu, gpr::R2, 0xDEAD_BEEF);
        cpu.execute(DecodedInstruction::Lw {
            rd: r(gpr::R2),
            base: r(gpr::R1),
            offset: 0,
        });
        assert_eq!(get(&cpu, gpr::R2), 0xDEAD_BEEF);
        assert_exception(&cpu, ExceptionCause::BusError, 0x2000);
    }

    ///////////////////////////////////////////////////////////////////////////
    // Branches and jumps

    #[test]
    fn jmp_adds_signed_offset_to_current_pc() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.execute(DecodedInstruction::Jmp { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn jmp_accepts_negative_offset() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.execute(DecodedInstruction::Jmp { offset: -4 });
        assert_eq!(pc(&cpu), 0x0FC);
    }

    #[test]
    fn call_saves_current_pc_to_r15_then_jumps() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.execute(DecodedInstruction::Call { offset: 0x20 });
        assert_eq!(get(&cpu, gpr::R15), 0x100);
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn jr_loads_pc_from_target_register() {
        let mut cpu = cpu();
        set(&mut cpu, gpr::R1, 0xCAFE_BABE);
        cpu.execute(DecodedInstruction::Jr { target: r(gpr::R1) });
        assert_eq!(pc(&cpu), 0xCAFE_BABE);
    }

    #[test]
    fn jalr_saves_current_pc_and_loads_target_pc() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        set(&mut cpu, gpr::R1, 0xCAFE_BABE);
        cpu.execute(DecodedInstruction::Jalr {
            rd: r(gpr::R2),
            target: r(gpr::R1),
        });
        assert_eq!(get(&cpu, gpr::R2), 0x100);
        assert_eq!(pc(&cpu), 0xCAFE_BABE);
    }

    #[test]
    fn bf_eq_branches_when_zero_set() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, true, false, false, false);
        cpu.execute(DecodedInstruction::BfEq { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn bf_eq_does_not_branch_when_zero_clear() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, false, false, false, false);
        cpu.execute(DecodedInstruction::BfEq { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x100);
    }

    #[test]
    fn bf_lt_uses_negative_xor_overflow() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, false, true, false, false);
        cpu.execute(DecodedInstruction::BfLt { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn bf_ge_uses_negative_equals_overflow() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, false, true, false, true);
        cpu.execute(DecodedInstruction::BfGe { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn bf_ltu_branches_when_carry_clear() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, false, false, false, false);
        cpu.execute(DecodedInstruction::BfLtu { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn bf_geu_branches_when_carry_set() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, false, false, true, false);
        cpu.execute(DecodedInstruction::BfGeu { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn bf_es_branches_when_arithmetic_error_set() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(true, false, false, false, false);
        cpu.execute(DecodedInstruction::BfEs { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn bf_ec_branches_when_arithmetic_error_clear() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        cpu.creg.update_sr_flags(false, false, false, false, false);
        cpu.execute(DecodedInstruction::BfEc { offset: 0x20 });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn register_beq_branches_on_equal_values() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        set(&mut cpu, gpr::R1, 0x1234_5678);
        set(&mut cpu, gpr::R2, 0x1234_5678);
        cpu.execute(DecodedInstruction::BEq {
            ra: r(gpr::R1),
            rb: r(gpr::R2),
            offset: 0x20,
        });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn register_blt_uses_signed_comparison() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        set(&mut cpu, gpr::R1, 0xFFFF_FFFF);
        set(&mut cpu, gpr::R2, 0);
        cpu.execute(DecodedInstruction::BLt {
            ra: r(gpr::R1),
            rb: r(gpr::R2),
            offset: 0x20,
        });
        assert_eq!(pc(&cpu), 0x120);
    }

    #[test]
    fn register_bltu_uses_unsigned_comparison() {
        let mut cpu = cpu();
        set_pc(&mut cpu, 0x100);
        set(&mut cpu, gpr::R1, 0);
        set(&mut cpu, gpr::R2, 0xFFFF_FFFF);
        cpu.execute(DecodedInstruction::BLtu {
            ra: r(gpr::R1),
            rb: r(gpr::R2),
            offset: 0x20,
        });
        assert_eq!(pc(&cpu), 0x120);
    }
}
