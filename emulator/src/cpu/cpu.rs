use crate::cpu::{GprFile, ProgramCounter, StatusRegister};
use crate::lifecycle::{Init, Reset, Tick};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cpu {
    gpr: GprFile,
    pc: ProgramCounter,
    sr: StatusRegister,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            gpr: GprFile::new(),
            pc: ProgramCounter::new(),
            sr: StatusRegister::new(),
        }
    }

    pub fn pc(&self) -> u32 {
        self.pc.get()
    }

    pub fn sr(&self) -> u32 {
        self.sr.get()
    }

    pub fn read_gpr(&self, index: usize) -> u32 {
        self.gpr.read(index)
    }

    pub fn write_gpr(&mut self, index: usize, value: u32) {
        self.gpr.write(index, value);
    }

    pub fn set_pc(&mut self, value: u32) {
        self.pc.set(value);
    }

    pub fn set_sr(&mut self, value: u32) {
        self.sr.set(value);
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
        // Instruction execution will be wired here later.
        //
        // For the intended simple single-cycle model, this can eventually become
        // the place where one CPU cycle occurs. At that point, the machine-level
        // coordinator will likely call something like:
        //
        // cpu.tick_with_bus(&mut bus)
        //
        // or Tick may remain for zero-argument clocked components while Cpu gets
        // an explicit step/tick method that accepts the bus.
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::isa::generated::gpr;
    use super::*;

    #[test]
    fn new_cpu_starts_reset() {
        let cpu = Cpu::new();

        assert_eq!(cpu.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.sr(), StatusRegister::RESET_VALUE);

        for index in 0..GprFile::COUNT {
            assert_eq!(cpu.read_gpr(index), 0);
        }
    }

    #[test]
    fn reset_clears_cpu_state() {
        let mut cpu = Cpu::new();

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
        let mut cpu = Cpu::new();

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
        let mut cpu = Cpu::new();

        cpu.write_gpr(gpr::R0, 0xFFFF_FFFF);

        assert_eq!(cpu.read_gpr(gpr::R0), 0);
    }

    #[test]
    fn nonzero_gprs_round_trip() {
        let mut cpu = Cpu::new();

        cpu.write_gpr(gpr::R1, 0x1234_5678);
        cpu.write_gpr(gpr::R15, 0xCAFE_BABE);

        assert_eq!(cpu.read_gpr(gpr::R1), 0x1234_5678);
        assert_eq!(cpu.read_gpr(gpr::R15), 0xCAFE_BABE);
    }

    #[test]
    fn tick_is_currently_a_noop() {
        let mut cpu = Cpu::new();

        cpu.tick();

        assert_eq!(cpu.pc(), ProgramCounter::RESET_VALUE);
        assert_eq!(cpu.sr(), StatusRegister::RESET_VALUE);
    }

    #[test]
    fn set_sr_masks_reserved_bits() {
        let mut cpu = Cpu::new();

        cpu.set_sr(u32::MAX);

        assert_eq!(cpu.sr(), StatusRegister::VALID_MASK);
    }
}
