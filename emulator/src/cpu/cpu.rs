use super::{GprFile, ProgramCounter, StatusRegister};

use crate::{
    lifecycle::{Init, Reset, Tick},
    platform::{SystemBus, SystemBusError},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CpuState {
    Running,
    Halted,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExceptionCause {
    Reset = 0x00,
    IllegalInstruction = 0x01,
    MisalignedInstructionFetch = 0x02,
    MisalignedDataAccess = 0x03,
    SoftwareTrap = 0x04,
    SystemCall = 0x05,
    TimerInterrupt = 0x06,
    ExternalInterrupt = 0x07,
}

impl ExceptionCause {
    pub const SLOT_SIZE: u32 = 16;
    pub const TABLE_SIZE: u32 = 8 * Self::SLOT_SIZE;

    pub fn code(self) -> u32 {
        self as u32
    }

    pub fn vector_offset(self) -> u32 {
        self.code() * Self::SLOT_SIZE
    }
}

impl TryFrom<u32> for ExceptionCause {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Reset),
            1 => Ok(Self::IllegalInstruction),
            2 => Ok(Self::MisalignedInstructionFetch),
            3 => Ok(Self::MisalignedDataAccess),
            4 => Ok(Self::SoftwareTrap),
            5 => Ok(Self::SystemCall),
            6 => Ok(Self::TimerInterrupt),
            7 => Ok(Self::ExternalInterrupt),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct Cpu {
    gpr: GprFile,
    pc: ProgramCounter,
    sr: StatusRegister,

    epc: u32,
    ecause: ExceptionCause,
    eaddr: u32,
    evbase: u32,

    bus: SystemBus,
    state: CpuState,
}

impl Cpu {
    pub fn new(bus: SystemBus) -> Self {
        Self {
            gpr: GprFile::new(),
            pc: ProgramCounter::new(),
            sr: StatusRegister::new(),

            epc: 0,
            ecause: ExceptionCause::Reset,
            eaddr: 0,
            evbase: 0,

            bus,
            state: CpuState::Halted,
        }
    }

    pub(crate) fn pc(&self) -> u32 {
        self.pc.get()
    }

    fn sr(&self) -> u32 {
        self.sr.get()
    }

    fn read_gpr(&self, index: usize) -> u32 {
        self.gpr.read(index)
    }

    fn write_gpr(&mut self, index: usize, value: u32) {
        self.gpr.write(index, value);
    }

    fn set_pc(&mut self, value: u32) {
        self.pc.set(value);
    }

    fn set_sr(&mut self, value: u32) {
        self.sr.set(value);
    }

    pub fn ecause(&self) -> u32 {
        self.ecause.code()
    }

    fn raise_exception(&mut self, cause: ExceptionCause, addr: u32) {
        self.epc = self.pc.get();
        self.ecause = cause;
        self.eaddr = addr;
        self.pc.set(self.ecause.vector_offset());
    }

    pub fn reset(&mut self) {
        self.gpr.reset();
        self.pc
            .set(self.evbase + ExceptionCause::Reset.vector_offset());
        self.sr.reset();

        self.epc = 0;
        self.ecause = ExceptionCause::Reset;
        self.eaddr = 0;
        self.state = CpuState::Running;

        self.bus.reset();
    }

    pub fn halt(&mut self) {
        self.state = CpuState::Halted;
    }

    fn tick_with_bus(&mut self) {
        if self.is_halted() {
            return;
        }

        let pc = self.pc.get();

        let instruction = match self.bus.read32(pc) {
            Ok(instruction) => instruction,

            Err(SystemBusError::MisalignedAccess { .. }) => {
                self.raise_exception(ExceptionCause::MisalignedInstructionFetch, pc);
                self.bus.tick();
                return;
            }

            Err(
                SystemBusError::AddressUnmapped { .. } | SystemBusError::UnsupportedAccess { .. },
            ) => {
                // The processor cannot handle these.
                self.eaddr = pc;
                self.halt();
                return;
            }
        };

        self.pc.advance_word();

        // Decode/execute later. NOP for now.
        if instruction != 0 {
            self.raise_exception(ExceptionCause::IllegalInstruction, pc);
        }

        self.bus.tick();
    }

    pub fn is_halted(&self) -> bool {
        self.state == CpuState::Halted
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
        self.pc.reset();
        self.sr.reset();
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
    use crate::isa::generated::gpr;

    #[test]
    fn new_cpu_starts_reset() {
        let cpu = Cpu::new(SystemBus::new(1024));

        assert_eq!(cpu.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.sr(), StatusRegister::RESET_VALUE);

        for index in 0..GprFile::COUNT {
            assert_eq!(cpu.read_gpr(index), 0);
        }
    }

    #[test]
    fn reset_clears_cpu_state() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.set_pc(0x1234_5678);
        cpu.set_sr(0x0000_00FF);
        cpu.write_gpr(gpr::R1, 0xAAAA_BBBB);

        cpu.reset();

        assert_eq!(cpu.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.sr(), StatusRegister::RESET_VALUE);
        assert_eq!(cpu.read_gpr(gpr::R1), 0);
    }

    #[test]
    fn init_resets_cpu_state() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.set_pc(0x1234_5678);
        cpu.set_sr(0x0000_00FF);
        cpu.write_gpr(gpr::R1, 0xAAAA_BBBB);

        cpu.init();

        assert_eq!(cpu.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.sr(), StatusRegister::RESET_VALUE);
        assert_eq!(cpu.read_gpr(gpr::R1), 0);
    }

    #[test]
    fn r0_always_reads_as_zero() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.write_gpr(gpr::R0, 0xFFFF_FFFF);

        assert_eq!(cpu.read_gpr(gpr::R0), 0);
    }

    #[test]
    fn nonzero_gprs_round_trip() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.write_gpr(gpr::R1, 0x1234_5678);
        cpu.write_gpr(gpr::R15, 0xCAFE_BABE);

        assert_eq!(cpu.read_gpr(gpr::R1), 0x1234_5678);
        assert_eq!(cpu.read_gpr(gpr::R15), 0xCAFE_BABE);
    }

    #[test]
    fn tick_is_currently_a_noop() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.tick();

        assert_eq!(cpu.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.sr(), StatusRegister::RESET_VALUE);
    }

    #[test]
    fn set_sr_masks_reserved_bits() {
        let mut cpu = Cpu::new(SystemBus::new(1024));

        cpu.set_sr(u32::MAX);

        assert_eq!(cpu.sr(), StatusRegister::VALID_MASK);
    }
}
