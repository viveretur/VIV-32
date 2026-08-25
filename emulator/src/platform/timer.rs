use super::BusDevice;

use crate::Lifecycle;

#[derive(Debug, Clone)]
pub struct Timer {
    counter: u32,
    prescale_counter: u32,
    control: u32,
    compare: u32,
}

const COUNTER_OFFSET: u32 = 0x00;
const COUNTER_END_OFFSET: u32 = COUNTER_OFFSET + 3;
const CONTROL_OFFSET: u32 = 0x04;
const CONTROL_END_OFFSET: u32 = CONTROL_OFFSET + 3;
const COMPARE_OFFSET: u32 = 0x08;
const COMPARE_END_OFFSET: u32 = COMPARE_OFFSET + 3;

const CONTROL_ENABLE: u32 = 1 << 0;
const CONTROL_IRQ_ENABLE: u32 = 1 << 1;
const CONTROL_IRQ_PENDING: u32 = 1 << 2;
const CONTROL_AUTO_RELOAD: u32 = 1 << 3;

const CONTROL_PRESCALER_SHIFT: u32 = 4;
const CONTROL_PRESCALER_MASK: u32 = 0xF << CONTROL_PRESCALER_SHIFT;

const CONTROL_PERIOD_SHIFT: u32 = 8;
const CONTROL_PERIOD_MASK: u32 = 0xF << CONTROL_PERIOD_SHIFT;

const CONTROL_WRITABLE_MASK: u32 = CONTROL_ENABLE
    | CONTROL_IRQ_ENABLE
    | CONTROL_IRQ_PENDING
    | CONTROL_AUTO_RELOAD
    | CONTROL_PRESCALER_MASK
    | CONTROL_PERIOD_MASK;

impl Timer {
    pub fn new() -> Self {
        Self {
            counter: 0,
            prescale_counter: 0,
            control: 0,
            compare: 0xFFFF_FFFF,
        }
    }

    fn enabled(&self) -> bool {
        self.compare != 0 && self.control & CONTROL_ENABLE != 0
    }

    fn interrupt_pending(&self) -> bool {
        self.control & CONTROL_IRQ_PENDING != 0
    }

    fn interrupt_asserted(&mut self) -> bool {
        self.control & CONTROL_IRQ_ENABLE != 0 && self.interrupt_pending()
    }

    fn read_counter(&self) -> u32 {
        self.counter
    }

    fn write_counter(&mut self, value: u32) {
        self.counter = value;
        self.prescale_counter = 0;
    }

    fn read_control(&self) -> u32 {
        self.control
    }

    fn write_control(&mut self, value: u32) {
        let clear_pending = value & CONTROL_IRQ_PENDING != 0;

        let preserved_pending = self.control & CONTROL_IRQ_PENDING;
        let writable_without_pending = CONTROL_WRITABLE_MASK & !CONTROL_IRQ_PENDING;

        self.control = value & writable_without_pending;

        if !clear_pending {
            self.control |= preserved_pending;
        }
    }

    fn read_compare(&self) -> u32 {
        self.compare
    }

    fn write_compare(&mut self, value: u32) {
        self.compare = value;
    }

    fn prescaler_select(&self) -> u32 {
        (self.control & CONTROL_PRESCALER_MASK) >> CONTROL_PRESCALER_SHIFT
    }

    fn prescaler_divider(&self) -> u32 {
        1 << self.prescaler_select()
    }
}

impl Lifecycle for Timer {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.counter = 0;
        self.prescale_counter = 0;
        self.control = 0;
        self.compare = 0xFFFF_FFFF;
    }

    fn tick(&mut self) {
        if !self.enabled() {
            return;
        }

        self.prescale_counter = self.prescale_counter.wrapping_add(1);

        if self.prescale_counter < self.prescaler_divider() {
            return;
        }

        self.prescale_counter = 0;
        self.counter = self.counter.wrapping_add(1);

        if self.counter >= self.read_compare() {
            self.control |= CONTROL_IRQ_PENDING;

            if self.control & CONTROL_AUTO_RELOAD != 0 {
                self.counter = 0;
            } else {
                self.control &= !CONTROL_ENABLE;
            }
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl BusDevice for Timer {
    fn size(&self) -> u32 {
        12
    }

    fn read8(&mut self, offset: u32) -> Option<u8> {
        match offset {
            COUNTER_OFFSET..=COUNTER_END_OFFSET => {
                let counter = self.read_counter();
                let shift = (3 - (offset - COUNTER_OFFSET)) * 8;
                Some((counter >> shift) as u8)
            }

            CONTROL_OFFSET..=CONTROL_END_OFFSET => {
                let control = self.read_control();
                let shift = (3 - (offset - CONTROL_OFFSET)) * 8;
                Some((control >> shift) as u8)
            }

            COMPARE_OFFSET..=COMPARE_END_OFFSET => {
                let compare = self.read_compare();
                let shift = (3 - (offset - COMPARE_OFFSET)) * 8;
                Some((compare >> shift) as u8)
            }

            _ => None,
        }
    }

    fn write8(&mut self, offset: u32, value: u8) -> Option<()> {
        match offset {
            COUNTER_OFFSET..=COUNTER_END_OFFSET => {
                let shift = (3 - (offset - COUNTER_OFFSET)) * 8;
                let mask = !(0xFFu32 << shift);

                let new_counter = (self.read_counter() & mask) | ((value as u32) << shift);

                self.write_counter(new_counter);
                Some(())
            }

            CONTROL_OFFSET..=CONTROL_END_OFFSET => {
                let shift = (3 - (offset - CONTROL_OFFSET)) * 8;
                let mask = !(0xFFu32 << shift);

                let new_control = (self.read_control() & mask) | ((value as u32) << shift);

                self.write_control(new_control);
                Some(())
            }

            COMPARE_OFFSET..=COMPARE_END_OFFSET => {
                let shift = (3 - (offset - COMPARE_OFFSET)) * 8;
                let mask = !(0xFFu32 << shift);

                let new_compare = (self.read_compare() & mask) | ((value as u32) << shift);

                self.write_compare(new_compare);
                Some(())
            }

            _ => None,
        }
    }

    fn interrupt_asserted(&mut self) -> bool {
        Timer::interrupt_asserted(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_timer_starts_clear_and_disabled() {
        let mut timer = Timer::new();

        assert_eq!(timer.read_counter(), 0);
        assert_eq!(timer.read_control(), 0);
        assert!(!timer.enabled());
        assert!(!timer.interrupt_pending());
        assert!(!timer.interrupt_asserted());
    }

    #[test]
    fn disabled_timer_does_not_count() {
        let mut timer = Timer::new();

        timer.tick();

        assert_eq!(timer.read_counter(), 0);
    }

    #[test]
    fn enabled_timer_counts_ticks() {
        let mut timer = Timer::new();

        timer.write_control(CONTROL_ENABLE);
        timer.tick();
        timer.tick();

        assert_eq!(timer.read_counter(), 2);
    }

    #[test]
    fn reset_clears_timer_state() {
        let mut timer = Timer::new();

        timer.write_counter(123);
        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE);
        timer.tick();

        timer.reset();

        assert_eq!(timer.read_counter(), 0);
        assert_eq!(timer.read_control(), 0);
        assert!(!timer.enabled());
    }

    #[test]
    fn init_resets_timer_state() {
        let mut timer = Timer::new();

        timer.write_counter(123);
        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE);
        timer.tick();

        timer.init();

        assert_eq!(timer.read_counter(), 0);
        assert_eq!(timer.read_control(), 0);
        assert!(!timer.enabled());
    }

    #[test]
    fn prescaler_divides_ticks() {
        let mut timer = Timer::new();

        let prescaler_divide_by_4 = 2 << CONTROL_PRESCALER_SHIFT;

        timer.write_control(CONTROL_ENABLE | prescaler_divide_by_4);

        timer.tick();
        timer.tick();
        timer.tick();

        assert_eq!(timer.read_counter(), 0);

        timer.tick();

        assert_eq!(timer.read_counter(), 1);
    }

    #[test]
    fn timer_sets_pending_at_period_limit() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE);

        timer.tick();

        assert_eq!(timer.read_counter(), 256);
        assert!(timer.interrupt_pending());
        assert!(!timer.interrupt_asserted());
    }

    #[test]
    fn irq_enable_asserts_interrupt_when_pending() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE);

        timer.tick();

        assert!(timer.interrupt_pending());
        assert!(timer.interrupt_asserted());
    }

    #[test]
    fn one_shot_timer_disables_after_period() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE);

        timer.tick();

        assert_eq!(timer.read_control() & CONTROL_ENABLE, 0);
    }

    #[test]
    fn auto_reload_timer_resets_counter_and_keeps_guest_enable_set() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE | CONTROL_AUTO_RELOAD);

        timer.tick();

        assert_eq!(timer.read_counter(), 0);
        assert_ne!(timer.read_control() & CONTROL_ENABLE, 0);
        assert!(timer.interrupt_pending());
    }

    #[test]
    fn writing_one_to_pending_bit_clears_pending() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE);
        timer.tick();

        assert!(timer.interrupt_pending());

        timer.write_control(CONTROL_IRQ_PENDING);

        assert!(!timer.interrupt_pending());
        assert!(!timer.interrupt_asserted());
    }

    #[test]
    fn writing_zero_to_pending_bit_preserves_pending() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE);
        timer.tick();

        assert!(timer.interrupt_pending());

        timer.write_control(CONTROL_IRQ_ENABLE);

        assert!(timer.interrupt_pending());
    }

    #[test]
    fn write_counter_resets_prescaler_counter() {
        let mut timer = Timer::new();

        let prescaler_divide_by_4 = 2 << CONTROL_PRESCALER_SHIFT;

        timer.write_control(CONTROL_ENABLE | prescaler_divide_by_4);
        timer.tick();
        timer.tick();
        timer.tick();

        timer.write_counter(10);
        timer.tick();

        assert_eq!(timer.read_counter(), 10);
    }

    #[test]
    fn read8_exposes_counter_as_big_endian_bytes() {
        let mut timer = Timer::new();

        timer.write_counter(0x1234_5678);

        assert_eq!(timer.read8(COUNTER_OFFSET), Some(0x12));
        assert_eq!(timer.read8(COUNTER_OFFSET + 1), Some(0x34));
        assert_eq!(timer.read8(COUNTER_OFFSET + 2), Some(0x56));
        assert_eq!(timer.read8(COUNTER_OFFSET + 3), Some(0x78));
    }

    #[test]
    fn write8_updates_counter_big_endian_byte_lanes() {
        let mut timer = Timer::new();

        assert_eq!(timer.write8(COUNTER_OFFSET, 0x12), Some(()));
        assert_eq!(timer.write8(COUNTER_OFFSET + 1, 0x34), Some(()));
        assert_eq!(timer.write8(COUNTER_OFFSET + 2, 0x56), Some(()));
        assert_eq!(timer.write8(COUNTER_OFFSET + 3, 0x78), Some(()));

        assert_eq!(timer.read_counter(), 0x1234_5678);
    }

    #[test]
    fn read8_exposes_control_as_big_endian_bytes() {
        let mut timer = Timer::new();

        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE | CONTROL_AUTO_RELOAD);

        assert_eq!(timer.read8(CONTROL_OFFSET), Some(0x00));
        assert_eq!(timer.read8(CONTROL_OFFSET + 1), Some(0x00));
        assert_eq!(timer.read8(CONTROL_OFFSET + 2), Some(0x00));
        assert_eq!(
            timer.read8(CONTROL_OFFSET + 3),
            Some((CONTROL_ENABLE | CONTROL_IRQ_ENABLE | CONTROL_AUTO_RELOAD) as u8)
        );
    }

    #[test]
    fn write8_control_uses_control_register_semantics() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_compare(256);
        timer.write_control(CONTROL_ENABLE | CONTROL_IRQ_ENABLE);
        timer.tick();

        assert!(timer.interrupt_pending());

        assert_eq!(
            timer.write8(CONTROL_OFFSET + 3, CONTROL_IRQ_PENDING as u8),
            Some(())
        );

        assert!(!timer.interrupt_pending());
    }

    #[test]
    fn timer_unknown_offsets_return_none() {
        let mut timer = Timer::new();

        assert_eq!(timer.read8(0x0C), None);
        assert_eq!(timer.write8(0x0C, 0xFF), None);
    }
}
