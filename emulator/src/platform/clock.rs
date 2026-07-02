use crate::bus::BusError;
use crate::lifecycle::{Init, Reset, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    cycles: u64,
}

impl Clock {
    pub const CYCLE_LO_OFFSET: u32 = 0x00;
    pub const CYCLE_HI_OFFSET: u32 = 0x04;

    pub fn new() -> Self {
        Self { cycles: 0 }
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    pub fn read32(&self, offset: u32) -> Result<u32, BusError> {
        match offset {
            Self::CYCLE_LO_OFFSET => Ok(self.cycles as u32),
            Self::CYCLE_HI_OFFSET => Ok((self.cycles >> 32) as u32),
            _ => Err(BusError::UnsupportedAccess { addr: offset }),
        }
    }

    pub fn write32(&mut self, offset: u32, _value: u32) -> Result<(), BusError> {
        Err(BusError::UnsupportedAccess { addr: offset })
    }
}

impl Init for Clock {
    fn init(&mut self) {
        self.reset();
    }
}

impl Reset for Clock {
    fn reset(&mut self) {
        self.cycles = 0;
    }
}

impl Tick for Clock {
    fn tick(&mut self) {
        self.cycles = self.cycles.wrapping_add(1);
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clock_starts_at_zero() {
        let clock = Clock::new();

        assert_eq!(clock.cycles(), 0);
    }

    #[test]
    fn clock_ticks() {
        let mut clock = Clock::new();

        clock.tick();
        clock.tick();

        assert_eq!(clock.cycles(), 2);
    }

    #[test]
    fn reset_clears_cycles() {
        let mut clock = Clock::new();

        clock.tick();
        clock.tick();
        clock.reset();

        assert_eq!(clock.cycles(), 0);
    }

    #[test]
    fn init_resets_clock() {
        let mut clock = Clock::new();

        clock.tick();
        clock.tick();
        clock.init();

        assert_eq!(clock.cycles(), 0);
    }

    #[test]
    fn read_cycle_low_word() {
        let clock = Clock {
            cycles: 0x1234_5678_9ABC_DEF0,
        };

        assert_eq!(clock.read32(Clock::CYCLE_LO_OFFSET).unwrap(), 0x9ABC_DEF0);
    }

    #[test]
    fn read_cycle_high_word() {
        let clock = Clock {
            cycles: 0x1234_5678_9ABC_DEF0,
        };

        assert_eq!(clock.read32(Clock::CYCLE_HI_OFFSET).unwrap(), 0x1234_5678);
    }

    #[test]
    fn invalid_read_offset_errors() {
        let clock = Clock::new();

        assert_eq!(
            clock.read32(0x08),
            Err(BusError::UnsupportedAccess { addr: 0x08 })
        );
    }

    #[test]
    fn clock_registers_are_read_only() {
        let mut clock = Clock::new();

        assert_eq!(
            clock.write32(Clock::CYCLE_LO_OFFSET, 123),
            Err(BusError::UnsupportedAccess {
                addr: Clock::CYCLE_LO_OFFSET
            })
        );
    }

    #[test]
    fn cycle_counter_wraps() {
        let mut clock = Clock { cycles: u64::MAX };

        clock.tick();

        assert_eq!(clock.cycles(), 0);
    }
}
