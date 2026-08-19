use crate::Lifecycle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramCounter {
    value: u32,
}

impl ProgramCounter {
    pub const RESET_VALUE: u32 = 0x0000_0000;

    pub fn new() -> Self {
        Self {
            value: Self::RESET_VALUE,
        }
    }

    pub fn get(&self) -> u32 {
        self.value
    }

    pub fn set(&mut self, value: u32) {
        self.value = value;
    }

    pub fn advance_word(&mut self) {
        self.value = self.value.wrapping_add(4);
    }
}

impl Lifecycle for ProgramCounter {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.value = Self::RESET_VALUE;
    }
}

impl Default for ProgramCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pc_starts_at_reset_value() {
        let pc = ProgramCounter::new();

        assert_eq!(pc.get(), ProgramCounter::RESET_VALUE);
    }

    #[test]
    fn set_updates_pc() {
        let mut pc = ProgramCounter::new();

        pc.set(0x1234_5678);

        assert_eq!(pc.get(), 0x1234_5678);
    }

    #[test]
    fn reset_restores_reset_value() {
        let mut pc = ProgramCounter::new();

        pc.set(0x1234_5678);
        pc.reset();

        assert_eq!(pc.get(), ProgramCounter::RESET_VALUE);
    }

    #[test]
    fn init_restores_reset_value() {
        let mut pc = ProgramCounter::new();

        pc.set(0x1234_5678);
        pc.init();

        assert_eq!(pc.get(), ProgramCounter::RESET_VALUE);
    }

    #[test]
    fn advance_word_adds_four() {
        let mut pc = ProgramCounter::new();

        pc.set(0x1000);
        pc.advance_word();

        assert_eq!(pc.get(), 0x1004);
    }

    #[test]
    fn advance_wraps() {
        let mut pc = ProgramCounter::new();

        pc.set(0xFFFF_FFFC);
        pc.advance_word();

        assert_eq!(pc.get(), 0x0000_0000);
    }
}
