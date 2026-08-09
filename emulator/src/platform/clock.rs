use super::MemoryMapping;

use crate::lifecycle::{Init, Reset, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    cycles: u64,
}

impl Clock {
    pub fn new() -> Self {
        Self { cycles: 0 }
    }

    fn cycles(&self) -> u64 {
        self.cycles
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

impl MemoryMapping for Clock {
    fn read8(&mut self, offset: u32) -> Option<u8> {
        let cycles = self.cycles();

        match offset {
            // cycle_lo, big-endian bytes
            0x00 => Some((cycles >> 24) as u8),
            0x01 => Some((cycles >> 16) as u8),
            0x02 => Some((cycles >> 8) as u8),
            0x03 => Some(cycles as u8),

            // cycle_hi, big-endian bytes
            0x04 => Some((cycles >> 56) as u8),
            0x05 => Some((cycles >> 48) as u8),
            0x06 => Some((cycles >> 40) as u8),
            0x07 => Some((cycles >> 32) as u8),

            _ => None,
        }
    }

    fn write8(&mut self, _offset: u32, _value: u8) -> Option<()> {
        None
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
    fn cycle_counter_wraps() {
        let mut clock = Clock { cycles: u64::MAX };

        clock.tick();

        assert_eq!(clock.cycles(), 0);
    }

    #[test]
    fn read8_exposes_low_cycle_word_as_big_endian_bytes() {
        let mut clock = Clock {
            cycles: 0x0000_0000_1234_5678,
        };

        assert_eq!(clock.read8(0x00), Some(0x12));
        assert_eq!(clock.read8(0x01), Some(0x34));
        assert_eq!(clock.read8(0x02), Some(0x56));
        assert_eq!(clock.read8(0x03), Some(0x78));
    }

    #[test]
    fn read8_exposes_high_cycle_word_as_big_endian_bytes() {
        let mut clock = Clock {
            cycles: 0x1234_5678_0000_0000,
        };

        assert_eq!(clock.read8(0x04), Some(0x12));
        assert_eq!(clock.read8(0x05), Some(0x34));
        assert_eq!(clock.read8(0x06), Some(0x56));
        assert_eq!(clock.read8(0x07), Some(0x78));
    }

    #[test]
    fn read8_unknown_clock_offset_returns_none() {
        let mut clock = Clock::new();

        assert_eq!(clock.read8(0x08), None);
    }

    #[test]
    fn write8_clock_always_returns_none() {
        let mut clock = Clock::new();

        assert_eq!(clock.write8(0x00, 0xFF), None);
        assert_eq!(clock.write8(0x07, 0xFF), None);
    }
}
