use super::{
    CregFile, ExceptionCause, GprFile,
    decode::{DecodedInstruction, decode},
};

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

        let fetch_pc = self.creg.pc();

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
                self.creg.set_eaddr(fetch_pc);
                self.halt();
                return;
            }
        };

        self.creg.advance_pc_word();

        // Decode/execute later. NOP for now.
        match decode(instruction) {
            Ok(decoded) => self.execute(decoded),
            Err(_) => self
                .creg
                .raise_exception(ExceptionCause::IllegalInstruction, fetch_pc),
        }

        self.bus.tick();
    }

    fn execute(&mut self, instruction: DecodedInstruction) {
        match instruction {
            DecodedInstruction::Nop => (),
            DecodedInstruction::Halt => self.halt(),
            DecodedInstruction::SoftwareTrap { imm } => self
                .creg
                .raise_exception(ExceptionCause::SoftwareTrap, imm as u32),
            DecodedInstruction::SystemCall => self
                .creg
                .raise_exception(ExceptionCause::SystemCall, self.creg.pc()),
            DecodedInstruction::IRet => self.creg.iret(),
            DecodedInstruction::EI => self.creg.ei(),
            DecodedInstruction::DI => self.creg.di(),
            DecodedInstruction::RdPc { rd } => self.write_gpr(rd, self.creg.pc()),
            DecodedInstruction::Mrs { creg4, rd } => {
                self.write_gpr(rd, self.creg.read_register(creg4))
            }
            DecodedInstruction::Msr { creg4, rs } => {
                self.creg.write_register(creg4, self.read_gpr(rs))
            }
            DecodedInstruction::Add { rd, ra, rb } => (),
            DecodedInstruction::Sub { rd, ra, rb } => (),
            DecodedInstruction::And { rd, ra, rb } => (),
            DecodedInstruction::Or { rd, ra, rb } => (),
            DecodedInstruction::Xor { rd, ra, rb } => (),
            DecodedInstruction::Not { rd, ra, rb } => (),
            DecodedInstruction::Neg { rd, ra, rb } => (),
            DecodedInstruction::Cmp { rd, ra, rb } => (),
            DecodedInstruction::Addi { rd, ra, sext32 } => (),
            DecodedInstruction::Subi { rd, ra, sext32 } => (),
            DecodedInstruction::Cmpi { rd, ra, sext32 } => (),
            DecodedInstruction::Andi { rd, ra, imm32 } => (),
            DecodedInstruction::Ori { rd, ra, imm32 } => (),
            DecodedInstruction::Xori { rd, ra, imm32 } => (),
            DecodedInstruction::Shl { rd, ra, rb } => (),
            DecodedInstruction::Shr { rd, ra, rb } => (),
            DecodedInstruction::Sar { rd, ra, rb } => (),
            DecodedInstruction::Shli { rd, ra, imm } => (),
            DecodedInstruction::Shri { rd, ra, imm } => (),
            DecodedInstruction::Sari { rd, ra, imm } => (),
            DecodedInstruction::Btst { rd, ra, imm } => (),
            DecodedInstruction::Bset { rd, ra, imm } => (),
            DecodedInstruction::Bclr { rd, ra, imm } => (),
            DecodedInstruction::Btgl { rd, ra, imm } => (),
            DecodedInstruction::Mul { rd0, rd1, ra, rb } => (),
            DecodedInstruction::Mulu { rd0, rd1, ra, rb } => (),
            DecodedInstruction::Div { rd0, rd1, ra, rb } => (),
            DecodedInstruction::Divu { rd0, rd1, ra, rb } => (),
            DecodedInstruction::Lui { rd, imm16 } => (),
            DecodedInstruction::Lli { rd, imm16 } => (),
            DecodedInstruction::Lhi { rd, imm16 } => (),
            DecodedInstruction::Lb { rd, base, offset } => (),
            DecodedInstruction::Lbu { rd, base, offset } => (),
            DecodedInstruction::Lh { rd, base, offset } => (),
            DecodedInstruction::Lhu { rd, base, offset } => (),
            DecodedInstruction::Lw { rd, base, offset } => (),
            DecodedInstruction::Sb { rs, base, offset } => (),
            DecodedInstruction::Sh { rs, base, offset } => (),
            DecodedInstruction::Sw { rs, base, offset } => (),
            DecodedInstruction::BfEq { offset } => (),
            DecodedInstruction::BfNe { offset } => (),
            DecodedInstruction::BfLt { offset } => (),
            DecodedInstruction::BfLe { offset } => (),
            DecodedInstruction::BfGt { offset } => (),
            DecodedInstruction::BfGe { offset } => (),
            DecodedInstruction::BfLtu { offset } => (),
            DecodedInstruction::BfLeu { offset } => (),
            DecodedInstruction::BfGtu { offset } => (),
            DecodedInstruction::BfGeu { offset } => (),
            DecodedInstruction::BfCs { offset } => (),
            DecodedInstruction::BfCc { offset } => (),
            DecodedInstruction::BfVs { offset } => (),
            DecodedInstruction::BfVc { offset } => (),
            DecodedInstruction::BfEs { offset } => (),
            DecodedInstruction::BfEc { offset } => (),
            DecodedInstruction::BEq { ra, rb, offset } => (),
            DecodedInstruction::BNe { ra, rb, offset } => (),
            DecodedInstruction::BLt { ra, rb, offset } => (),
            DecodedInstruction::BLe { ra, rb, offset } => (),
            DecodedInstruction::BGt { ra, rb, offset } => (),
            DecodedInstruction::BGe { ra, rb, offset } => (),
            DecodedInstruction::BLtu { ra, rb, offset } => (),
            DecodedInstruction::BLeu { ra, rb, offset } => (),
            DecodedInstruction::BGtu { ra, rb, offset } => (),
            DecodedInstruction::BGeu { ra, rb, offset } => (),
            DecodedInstruction::Jmp { offset } => (),
            DecodedInstruction::Call { offset } => (),
            DecodedInstruction::Jr { rd, target } => (),
            DecodedInstruction::Jalr { rd, target } => (),

            _ => (), // Unknown instruction Nops for now
        }
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

        assert_eq!(cpu.creg.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.creg.sr(), StatusRegister::RESET_VALUE);

        for index in 0..GprFile::COUNT {
            assert_eq!(cpu.read_gpr(index as u8), 0);
        }
    }

    #[test]
    fn reset_clears_cpu_state() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.creg.set_pc(0x1234_5678);
        cpu.creg.set_sr(0x0000_00FF);
        cpu.write_gpr(gpr::R1 as u8, 0xAAAA_BBBB);

        cpu.reset();

        assert_eq!(cpu.creg.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.creg.sr(), StatusRegister::RESET_VALUE);
        assert_eq!(cpu.read_gpr(gpr::R1 as u8), 0);
    }

    #[test]
    fn init_resets_cpu_state() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.creg.set_pc(0x1234_5678);
        cpu.creg.set_sr(0x0000_00FF);
        cpu.write_gpr(gpr::R1 as u8, 0xAAAA_BBBB);

        cpu.init();

        assert_eq!(cpu.creg.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.creg.sr(), StatusRegister::RESET_VALUE);
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

        assert_eq!(cpu.creg.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.creg.sr(), StatusRegister::RESET_VALUE);
    }

    #[test]
    fn set_sr_masks_reserved_bits() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.creg.set_sr(u32::MAX);

        assert_eq!(cpu.creg.sr(), StatusRegister::VALID_MASK);
    }

    #[test]
    fn tick_incrememts_pc() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.reset();
        cpu.tick();

        assert_eq!(cpu.creg.pc(), 4);
    }
}
