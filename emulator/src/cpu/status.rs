use crate::lifecycle::{Init, Reset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusRegister {
    value: u32,
}

impl StatusRegister {
    pub const RESET_VALUE: u32 = 0x0000_0000;

    pub const FLAG_N: u32 = 1 << 0;
    pub const FLAG_Z: u32 = 1 << 1;
    pub const FLAG_C: u32 = 1 << 2;
    pub const FLAG_V: u32 = 1 << 3;
    pub const FLAG_E: u32 = 1 << 4;
    pub const FLAG_IE: u32 = 1 << 5;

    pub const VALID_MASK: u32 = Self::FLAG_N
        | Self::FLAG_Z
        | Self::FLAG_C
        | Self::FLAG_V
        | Self::FLAG_E
        | Self::FLAG_IE;

    pub fn new() -> Self {
        Self {
            value: Self::RESET_VALUE,
        }
    }

    pub fn get(&self) -> u32 {
        self.value
    }

    pub fn set(&mut self, value: u32) {
        self.value = value & Self::VALID_MASK;
    }

    pub fn negative(&self) -> bool {
        self.has(Self::FLAG_N)
    }

    pub fn zero(&self) -> bool {
        self.has(Self::FLAG_Z)
    }

    pub fn carry(&self) -> bool {
        self.has(Self::FLAG_C)
    }

    pub fn overflow(&self) -> bool {
        self.has(Self::FLAG_V)
    }

    pub fn arithmetic_error(&self) -> bool {
        self.has(Self::FLAG_E)
    }

    pub fn interrupt_enable(&self) -> bool {
        self.has(Self::FLAG_IE)
    }

    pub fn set_negative(&mut self, value: bool) {
        self.set_flag(Self::FLAG_N, value);
    }

    pub fn set_zero(&mut self, value: bool) {
        self.set_flag(Self::FLAG_Z, value);
    }

    pub fn set_carry(&mut self, value: bool) {
        self.set_flag(Self::FLAG_C, value);
    }

    pub fn set_overflow(&mut self, value: bool) {
        self.set_flag(Self::FLAG_V, value);
    }

    pub fn set_arithmetic_error(&mut self, value: bool) {
        self.set_flag(Self::FLAG_E, value);
    }

    pub fn set_interrupt_enable(&mut self, value: bool) {
        self.set_flag(Self::FLAG_IE, value);
    }

    pub fn clear_condition_flags(&mut self) {
        self.value &= Self::FLAG_IE;
    }

    fn has(&self, flag: u32) -> bool {
        self.value & flag != 0
    }

    fn set_flag(&mut self, flag: u32, enabled: bool) {
        if enabled {
            self.value |= flag;
        } else {
            self.value &= !flag;
        }

        self.value &= Self::VALID_MASK;
    }
}

impl Init for StatusRegister {
    fn init(&mut self) {
        self.reset();
    }
}

impl Reset for StatusRegister {
    fn reset(&mut self) {
        self.value = Self::RESET_VALUE;
    }
}

impl Default for StatusRegister {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_status_register_starts_at_reset_value() {
        let sr = StatusRegister::new();

        assert_eq!(sr.get(), StatusRegister::RESET_VALUE);
        assert!(!sr.negative());
        assert!(!sr.zero());
        assert!(!sr.carry());
        assert!(!sr.overflow());
        assert!(!sr.arithmetic_error());
        assert!(!sr.interrupt_enable());
    }

    #[test]
    fn set_masks_reserved_bits() {
        let mut sr = StatusRegister::new();

        sr.set(u32::MAX);

        assert_eq!(sr.get(), StatusRegister::VALID_MASK);
    }

    #[test]
    fn individual_flags_can_be_set_and_cleared() {
        let mut sr = StatusRegister::new();

        sr.set_negative(true);
        sr.set_zero(true);
        sr.set_carry(true);
        sr.set_overflow(true);
        sr.set_arithmetic_error(true);
        sr.set_interrupt_enable(true);

        assert!(sr.negative());
        assert!(sr.zero());
        assert!(sr.carry());
        assert!(sr.overflow());
        assert!(sr.arithmetic_error());
        assert!(sr.interrupt_enable());

        sr.set_negative(false);
        sr.set_zero(false);
        sr.set_carry(false);
        sr.set_overflow(false);
        sr.set_arithmetic_error(false);
        sr.set_interrupt_enable(false);

        assert_eq!(sr.get(), 0);
    }

    #[test]
    fn clear_condition_flags_preserves_interrupt_enable() {
        let mut sr = StatusRegister::new();

        sr.set(StatusRegister::VALID_MASK);
        sr.clear_condition_flags();

        assert_eq!(sr.get(), StatusRegister::FLAG_IE);
    }

    #[test]
    fn reset_restores_reset_value() {
        let mut sr = StatusRegister::new();

        sr.set(StatusRegister::VALID_MASK);
        sr.reset();

        assert_eq!(sr.get(), StatusRegister::RESET_VALUE);
    }

    #[test]
    fn init_restores_reset_value() {
        let mut sr = StatusRegister::new();

        sr.set(StatusRegister::VALID_MASK);
        sr.init();

        assert_eq!(sr.get(), StatusRegister::RESET_VALUE);
    }
}
