//! System bus and platform address map.
//!
//! The bus translates CPU-visible physical addresses into RAM and memory-mapped
//! device accesses. Multi-byte accesses are big-endian and alignment-checked here;
//! the CPU maps resulting bus errors into architectural exceptions or halts.
use super::{Clock, MemoryMapping, Ram, Serial, Timer, VecSerialSink};

use crate::lifecycle::{Init, Reset, Tick};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SystemBusError {
    AddressUnmapped { addr: u32 },
    UnsupportedAccess { addr: u32 },
    MisalignedAccess { addr: u32, alignment: u32 },
}

const RAM_BASE: u32 = 0x0000_0000;
const SERIAL_BASE: u32 = 0xFFFF_0000;
const CLOCK_BASE: u32 = 0xFFFF_0100;
const TIMER_BASE: u32 = 0xFFFF_0200;

#[derive(Debug)]
pub struct SystemBus {
    ram: Ram,
    serial: Serial<VecSerialSink>,
    clock: Clock,
    timer: Timer,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PendingInterrupt {
    Timer,
    External { source: u32 },
}

impl SystemBus {
    pub fn new(ram_size: u32) -> Self {
        Self {
            ram: Ram::new(ram_size),
            serial: Serial::new(VecSerialSink::new()),
            clock: Clock::new(),
            timer: Timer::new(),
        }
    }

    pub fn with_ram_image(ram_size: u32, image: &[u8]) -> Self {
        Self {
            ram: Ram::from_bytes(ram_size, image),
            serial: Serial::new(VecSerialSink::new()),
            clock: Clock::new(),
            timer: Timer::new(),
        }
    }

    fn check_alignment(addr: u32, alignment: u32) -> Result<(), SystemBusError> {
        if addr & (alignment - 1) != 0 {
            return Err(SystemBusError::MisalignedAccess { addr, alignment });
        }

        Ok(())
    }

    fn device_offset(addr: u32, base: u32) -> u32 {
        addr.wrapping_sub(base)
    }

    pub fn read8(&mut self, addr: u32) -> Result<u8, SystemBusError> {
        if let Some(value) = self.ram.read8(Self::device_offset(addr, RAM_BASE)) {
            return Ok(value);
        }

        if let Some(value) = self.serial.read8(Self::device_offset(addr, SERIAL_BASE)) {
            return Ok(value);
        }

        if let Some(value) = self.clock.read8(Self::device_offset(addr, CLOCK_BASE)) {
            return Ok(value);
        }

        if let Some(value) = self.timer.read8(Self::device_offset(addr, TIMER_BASE)) {
            return Ok(value);
        }

        Err(SystemBusError::AddressUnmapped { addr })
    }

    pub fn write8(&mut self, addr: u32, value: u8) -> Result<(), SystemBusError> {
        if self
            .ram
            .write8(Self::device_offset(addr, RAM_BASE), value)
            .is_some()
        {
            return Ok(());
        }

        if self
            .serial
            .write8(Self::device_offset(addr, SERIAL_BASE), value)
            .is_some()
        {
            return Ok(());
        }

        if self
            .clock
            .write8(Self::device_offset(addr, CLOCK_BASE), value)
            .is_some()
        {
            return Ok(());
        }

        if self
            .timer
            .write8(Self::device_offset(addr, TIMER_BASE), value)
            .is_some()
        {
            return Ok(());
        }

        Err(SystemBusError::AddressUnmapped { addr })
    }

    pub fn read16(&mut self, addr: u32) -> Result<u16, SystemBusError> {
        Self::check_alignment(addr, 2)?;

        let b0 = self.read8(addr)? as u16;
        let b1 = self.read8(addr.wrapping_add(1))? as u16;

        Ok((b0 << 8) | b1)
    }

    pub fn write16(&mut self, addr: u32, value: u16) -> Result<(), SystemBusError> {
        Self::check_alignment(addr, 2)?;

        self.write8(addr, (value >> 8) as u8)?;
        self.write8(addr.wrapping_add(1), value as u8)?;

        Ok(())
    }

    pub fn read32(&mut self, addr: u32) -> Result<u32, SystemBusError> {
        Self::check_alignment(addr, 4)?;

        let b0 = self.read8(addr)? as u32;
        let b1 = self.read8(addr.wrapping_add(1))? as u32;
        let b2 = self.read8(addr.wrapping_add(2))? as u32;
        let b3 = self.read8(addr.wrapping_add(3))? as u32;

        Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    }

    pub fn write32(&mut self, addr: u32, value: u32) -> Result<(), SystemBusError> {
        Self::check_alignment(addr, 4)?;

        self.write8(addr, (value >> 24) as u8)?;
        self.write8(addr.wrapping_add(1), (value >> 16) as u8)?;
        self.write8(addr.wrapping_add(2), (value >> 8) as u8)?;
        self.write8(addr.wrapping_add(3), value as u8)?;

        Ok(())
    }

    pub fn pending_interrupt(&self) -> Option<PendingInterrupt> {
        if self.timer.interrupt_asserted() {
            return Some(PendingInterrupt::Timer);
        }

        // Later:
        // if self.serial.interrupt_asserted() {
        //     return Some(PendingInterrupt::External { source: SERIAL_IRQ });
        // }

        None
    }
}

impl Reset for SystemBus {
    fn reset(&mut self) {
        self.serial.reset();
        self.clock.reset();
        self.timer.reset();
    }
}

impl Init for SystemBus {
    fn init(&mut self) {
        self.reset();
    }
}

impl Tick for SystemBus {
    fn tick(&mut self) {
        self.clock.tick();
        self.timer.tick();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_byte_roundtrip_routes_through_system_bus() {
        let mut bus = SystemBus::new(1024);

        bus.write8(0x0000_0010, 0xAB).unwrap();

        assert_eq!(bus.read8(0x0000_0010), Ok(0xAB));
    }

    #[test]
    fn ram_halfword_roundtrip_uses_big_endian_bus_helpers() {
        let mut bus = SystemBus::new(1024);

        bus.write16(0x0000_0010, 0x1234).unwrap();

        assert_eq!(bus.read8(0x0000_0010), Ok(0x12));
        assert_eq!(bus.read8(0x0000_0011), Ok(0x34));
        assert_eq!(bus.read16(0x0000_0010), Ok(0x1234));
    }

    #[test]
    fn ram_word_roundtrip_uses_big_endian_bus_helpers() {
        let mut bus = SystemBus::new(1024);

        bus.write32(0x0000_0010, 0x1234_5678).unwrap();

        assert_eq!(bus.read8(0x0000_0010), Ok(0x12));
        assert_eq!(bus.read8(0x0000_0011), Ok(0x34));
        assert_eq!(bus.read8(0x0000_0012), Ok(0x56));
        assert_eq!(bus.read8(0x0000_0013), Ok(0x78));
        assert_eq!(bus.read32(0x0000_0010), Ok(0x1234_5678));
    }

    #[test]
    fn misaligned_halfword_access_fails_through_system_bus() {
        let mut bus = SystemBus::new(1024);

        assert_eq!(
            bus.read16(0x0000_0001),
            Err(SystemBusError::MisalignedAccess {
                addr: 0x0000_0001,
                alignment: 2,
            })
        );

        assert_eq!(
            bus.write16(0x0000_0001, 0x1234),
            Err(SystemBusError::MisalignedAccess {
                addr: 0x0000_0001,
                alignment: 2,
            })
        );
    }

    #[test]
    fn misaligned_word_access_fails_through_system_bus() {
        let mut bus = SystemBus::new(1024);

        assert_eq!(
            bus.read32(0x0000_0002),
            Err(SystemBusError::MisalignedAccess {
                addr: 0x0000_0002,
                alignment: 4,
            })
        );

        assert_eq!(
            bus.write32(0x0000_0002, 0x1234_5678),
            Err(SystemBusError::MisalignedAccess {
                addr: 0x0000_0002,
                alignment: 4,
            })
        );
    }

    #[test]
    fn unmapped_byte_read_fails() {
        let mut bus = SystemBus::new(1024);

        assert_eq!(
            bus.read8(0x0000_1000),
            Err(SystemBusError::AddressUnmapped { addr: 0x0000_1000 })
        );
    }

    #[test]
    fn unmapped_byte_write_fails() {
        let mut bus = SystemBus::new(1024);

        assert_eq!(
            bus.write8(0x0000_1000, 0xAB),
            Err(SystemBusError::AddressUnmapped { addr: 0x0000_1000 })
        );
    }

    #[test]
    fn read32_clock_low_word_uses_big_endian_mapped_bytes() {
        let mut bus = SystemBus::new(1024);

        bus.tick();
        bus.tick();

        assert_eq!(bus.read32(CLOCK_BASE + 0x00), Ok(2));
    }

    #[test]
    fn write32_timer_counter_uses_big_endian_mapped_bytes() {
        let mut bus = SystemBus::new(1024);

        bus.write32(TIMER_BASE + 0x00, 0x1234_5678).unwrap();

        assert_eq!(bus.read32(TIMER_BASE + 0x00), Ok(0x1234_5678));
    }

    #[test]
    fn address_below_serial_base_does_not_map_to_serial_by_wrapping() {
        let mut bus = SystemBus::new(1024);

        assert_eq!(
            bus.read8(SERIAL_BASE - 1),
            Err(SystemBusError::AddressUnmapped {
                addr: SERIAL_BASE - 1
            })
        );
    }

    #[test]
    fn unmapped_address_returns_address_unmapped_after_all_devices_decline() {
        let mut bus = SystemBus::new(1024);

        assert_eq!(
            bus.read8(0x0000_1000),
            Err(SystemBusError::AddressUnmapped { addr: 0x0000_1000 })
        );
    }
}
