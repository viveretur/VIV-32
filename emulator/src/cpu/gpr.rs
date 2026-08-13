use crate::Lifecycle;
use crate::isa::generated::gpr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GprFile {
    registers: [u32; Self::COUNT],
}

impl GprFile {
    pub const COUNT: usize = gpr::COUNT as usize;

    pub fn new() -> Self {
        Self {
            registers: [0; Self::COUNT],
        }
    }

    pub fn read(&self, index: usize) -> u32 {
        assert!(index < Self::COUNT, "GPR index out of range");

        if index == gpr::ZERO {
            0
        } else {
            self.registers[index]
        }
    }

    pub fn write(&mut self, index: usize, value: u32) {
        assert!(index < Self::COUNT, "GPR index out of range");

        if index != gpr::ZERO {
            self.registers[index] = value;
        }
    }

    pub fn as_raw_slice(&self) -> &[u32; Self::COUNT] {
        &self.registers
    }
}

impl Lifecycle for GprFile {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.registers = [0; Self::COUNT];
    }
}

impl Default for GprFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_gpr_file_starts_clear() {
        let gprf = GprFile::new();

        for index in 0..GprFile::COUNT {
            assert_eq!(gprf.read(index), 0);
        }
    }

    #[test]
    fn r0_always_reads_as_zero() {
        let mut gprf = GprFile::new();

        gprf.write(gpr::R0, 0xFFFF_FFFF);

        assert_eq!(gprf.read(gpr::R0), 0);
    }

    #[test]
    fn nonzero_registers_round_trip() {
        let mut gprf = GprFile::new();

        gprf.write(gpr::R1, 0x1234_5678);
        gprf.write(gpr::R15, 0xCAFE_BABE);

        assert_eq!(gprf.read(gpr::R1), 0x1234_5678);
        assert_eq!(gprf.read(gpr::R15), 0xCAFE_BABE);
    }

    #[test]
    fn generated_register_numbers_match_expected_values() {
        assert_eq!(GprFile::COUNT, 16);

        assert_eq!(gpr::R0, 0);
        assert_eq!(gpr::R1, 1);
        assert_eq!(gpr::R2, 2);
        assert_eq!(gpr::R3, 3);
        assert_eq!(gpr::R4, 4);
        assert_eq!(gpr::R5, 5);
        assert_eq!(gpr::R6, 6);
        assert_eq!(gpr::R7, 7);
        assert_eq!(gpr::R8, 8);
        assert_eq!(gpr::R9, 9);
        assert_eq!(gpr::R10, 10);
        assert_eq!(gpr::R11, 11);
        assert_eq!(gpr::R12, 12);
        assert_eq!(gpr::R13, 13);
        assert_eq!(gpr::R14, 14);
        assert_eq!(gpr::R15, 15);
    }

    #[test]
    fn generated_aliases_match_expected_register_numbers() {
        assert_eq!(gpr::ZERO, gpr::R0);
        assert_eq!(gpr::FP, gpr::R13);
        assert_eq!(gpr::SP, gpr::R14);
        assert_eq!(gpr::LR, gpr::R15);
    }

    #[test]
    fn reset_clears_registers() {
        let mut gprf = GprFile::new();

        gprf.write(gpr::R1, 0x1234_5678);
        gprf.write(gpr::R15, 0xCAFE_BABE);

        gprf.reset();

        for index in 0..GprFile::COUNT {
            assert_eq!(gprf.read(index), 0);
        }
    }

    #[test]
    fn init_clears_registers() {
        let mut gprf = GprFile::new();

        gprf.write(gpr::R1, 0x1234_5678);
        gprf.write(gpr::R15, 0xCAFE_BABE);

        gprf.init();

        for index in 0..GprFile::COUNT {
            assert_eq!(gprf.read(index), 0);
        }
    }
}
