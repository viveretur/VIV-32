use crate::lifecycle::{Init, Reset, Tick};

#[derive(Debug, Clone)]
pub struct Timer {
    counter: u32,
    prescale_counter: u32,
    control: u32,
}

impl Timer {
    pub const COUNTER_OFFSET: u32 = 0x00;
    pub const CONTROL_OFFSET: u32 = 0x04;

    pub const CONTROL_ENABLE: u32 = 1 << 0;
    pub const CONTROL_IRQ_ENABLE: u32 = 1 << 1;
    pub const CONTROL_IRQ_PENDING: u32 = 1 << 2;
    pub const CONTROL_AUTO_RELOAD: u32 = 1 << 3;

    pub const CONTROL_PRESCALER_SHIFT: u32 = 4;
    pub const CONTROL_PRESCALER_MASK: u32 = 0xF << Self::CONTROL_PRESCALER_SHIFT;

    pub const CONTROL_PERIOD_SHIFT: u32 = 8;
    pub const CONTROL_PERIOD_MASK: u32 = 0xF << Self::CONTROL_PERIOD_SHIFT;

    pub const CONTROL_WRITABLE_MASK: u32 = Self::CONTROL_ENABLE
        | Self::CONTROL_IRQ_ENABLE
        | Self::CONTROL_IRQ_PENDING
        | Self::CONTROL_AUTO_RELOAD
        | Self::CONTROL_PRESCALER_MASK
        | Self::CONTROL_PERIOD_MASK;

    pub fn new() -> Self {
        Self {
            counter: 0,
            prescale_counter: 0,
            control: 0,
        }
    }

    pub fn counter(&self) -> u32 {
        self.counter
    }

    pub fn control(&self) -> u32 {
        self.control
    }

    pub fn enabled(&self) -> bool {
        self.control & Self::CONTROL_ENABLE != 0
    }

    pub fn interrupt_pending(&self) -> bool {
        self.control & Self::CONTROL_IRQ_PENDING != 0
    }

    pub fn interrupt_asserted(&self) -> bool {
        self.control & Self::CONTROL_IRQ_ENABLE != 0 && self.interrupt_pending()
    }

    pub fn read_counter(&self) -> u32 {
        self.counter
    }

    pub fn write_counter(&mut self, value: u32) {
        self.counter = value;
        self.prescale_counter = 0;
    }

    pub fn read_control(&self) -> u32 {
        self.control
    }

    pub fn write_control(&mut self, value: u32) {
        let clear_pending = value & Self::CONTROL_IRQ_PENDING != 0;

        let preserved_pending = self.control & Self::CONTROL_IRQ_PENDING;
        let writable_without_pending = Self::CONTROL_WRITABLE_MASK & !Self::CONTROL_IRQ_PENDING;

        self.control = value & writable_without_pending;

        if !clear_pending {
            self.control |= preserved_pending;
        }
    }

    fn prescaler_select(&self) -> u32 {
        (self.control & Self::CONTROL_PRESCALER_MASK) >> Self::CONTROL_PRESCALER_SHIFT
    }

    fn period_select(&self) -> u32 {
        (self.control & Self::CONTROL_PERIOD_MASK) >> Self::CONTROL_PERIOD_SHIFT
    }

    fn prescaler_divider(&self) -> u32 {
        1 << self.prescaler_select()
    }

    fn period_limit(&self) -> u32 {
        1 << (8 + self.period_select())
    }
}

impl Init for Timer {
    fn init(&mut self) {
        self.reset();
    }
}

impl Reset for Timer {
    fn reset(&mut self) {
        self.counter = 0;
        self.prescale_counter = 0;
        self.control = 0;
    }
}

impl Tick for Timer {
    fn tick(&mut self) {
        if self.control & Self::CONTROL_ENABLE == 0 {
            return;
        }

        self.prescale_counter = self.prescale_counter.wrapping_add(1);

        if self.prescale_counter < self.prescaler_divider() {
            return;
        }

        self.prescale_counter = 0;
        self.counter = self.counter.wrapping_add(1);

        if self.counter >= self.period_limit() {
            self.control |= Self::CONTROL_IRQ_PENDING;

            if self.control & Self::CONTROL_AUTO_RELOAD != 0 {
                self.counter = 0;
            } else {
                self.control &= !Self::CONTROL_ENABLE;
            }
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_timer_starts_clear_and_disabled() {
        let timer = Timer::new();

        assert_eq!(timer.counter(), 0);
        assert_eq!(timer.control(), 0);
        assert!(!timer.enabled());
        assert!(!timer.interrupt_pending());
        assert!(!timer.interrupt_asserted());
    }

    #[test]
    fn disabled_timer_does_not_count() {
        let mut timer = Timer::new();

        timer.tick();

        assert_eq!(timer.counter(), 0);
    }

    #[test]
    fn enabled_timer_counts_ticks() {
        let mut timer = Timer::new();

        timer.write_control(Timer::CONTROL_ENABLE);
        timer.tick();
        timer.tick();

        assert_eq!(timer.counter(), 2);
    }

    #[test]
    fn reset_clears_timer_state() {
        let mut timer = Timer::new();

        timer.write_counter(123);
        timer.write_control(Timer::CONTROL_ENABLE | Timer::CONTROL_IRQ_ENABLE);
        timer.tick();

        timer.reset();

        assert_eq!(timer.counter(), 0);
        assert_eq!(timer.control(), 0);
        assert!(!timer.enabled());
    }

    #[test]
    fn init_resets_timer_state() {
        let mut timer = Timer::new();

        timer.write_counter(123);
        timer.write_control(Timer::CONTROL_ENABLE | Timer::CONTROL_IRQ_ENABLE);
        timer.tick();

        timer.init();

        assert_eq!(timer.counter(), 0);
        assert_eq!(timer.control(), 0);
        assert!(!timer.enabled());
    }

    #[test]
    fn prescaler_divides_ticks() {
        let mut timer = Timer::new();

        let prescaler_divide_by_4 = 2 << Timer::CONTROL_PRESCALER_SHIFT;

        timer.write_control(Timer::CONTROL_ENABLE | prescaler_divide_by_4);

        timer.tick();
        timer.tick();
        timer.tick();

        assert_eq!(timer.counter(), 0);

        timer.tick();

        assert_eq!(timer.counter(), 1);
    }

    #[test]
    fn timer_sets_pending_at_period_limit() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_control(Timer::CONTROL_ENABLE);

        timer.tick();

        assert_eq!(timer.counter(), 256);
        assert!(timer.interrupt_pending());
        assert!(!timer.interrupt_asserted());
    }

    #[test]
    fn irq_enable_asserts_interrupt_when_pending() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_control(Timer::CONTROL_ENABLE | Timer::CONTROL_IRQ_ENABLE);

        timer.tick();

        assert!(timer.interrupt_pending());
        assert!(timer.interrupt_asserted());
    }

    #[test]
    fn one_shot_timer_disables_after_period() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_control(Timer::CONTROL_ENABLE);

        timer.tick();

        assert_eq!(timer.control() & Timer::CONTROL_ENABLE, 0);
    }

    #[test]
    fn auto_reload_timer_resets_counter_and_keeps_guest_enable_set() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_control(Timer::CONTROL_ENABLE | Timer::CONTROL_AUTO_RELOAD);

        timer.tick();

        assert_eq!(timer.counter(), 0);
        assert_ne!(timer.control() & Timer::CONTROL_ENABLE, 0);
        assert!(timer.interrupt_pending());
    }

    #[test]
    fn writing_one_to_pending_bit_clears_pending() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_control(Timer::CONTROL_ENABLE | Timer::CONTROL_IRQ_ENABLE);
        timer.tick();

        assert!(timer.interrupt_pending());

        timer.write_control(Timer::CONTROL_IRQ_PENDING);

        assert!(!timer.interrupt_pending());
        assert!(!timer.interrupt_asserted());
    }

    #[test]
    fn writing_zero_to_pending_bit_preserves_pending() {
        let mut timer = Timer::new();

        timer.write_counter(255);
        timer.write_control(Timer::CONTROL_ENABLE | Timer::CONTROL_IRQ_ENABLE);
        timer.tick();

        assert!(timer.interrupt_pending());

        timer.write_control(Timer::CONTROL_IRQ_ENABLE);

        assert!(timer.interrupt_pending());
    }

    #[test]
    fn write_counter_resets_prescaler_counter() {
        let mut timer = Timer::new();

        let prescaler_divide_by_4 = 2 << Timer::CONTROL_PRESCALER_SHIFT;

        timer.write_control(Timer::CONTROL_ENABLE | prescaler_divide_by_4);
        timer.tick();
        timer.tick();
        timer.tick();

        timer.write_counter(10);
        timer.tick();

        assert_eq!(timer.counter(), 10);
    }
}
