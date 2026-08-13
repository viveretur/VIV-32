//! System bus and platform address map.
//!
//! The bus translates CPU-visible physical addresses into RAM and memory-mapped
//! device accesses. Multi-byte accesses are big-endian and alignment-checked here;
//! the CPU maps resulting bus errors into architectural exceptions or halts.
use super::{BusDevice, MappedDevice};

use crate::Lifecycle;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SystemBusError {
    AddressOverflow,
    AddressUnmapped { addr: u32 },
    UnsupportedAccess { addr: u32 },
    MisalignedAccess { addr: u32, alignment: u32 },
}

impl std::fmt::Display for SystemBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemBusError::UnsupportedAccess { addr } => {
                write!(f, "UnsupportedAccess: 0x{addr:08X}")
            }
            SystemBusError::AddressUnmapped { addr } => {
                write!(f, "unmapped bus address: 0x{addr:08X}")
            }
            SystemBusError::MisalignedAccess { addr, alignment } => {
                write!(f, "misaligned bus access: 0x{addr:08X}, [{alignment}]")
            }
            SystemBusError::AddressOverflow => {
                write!(f, "bus address overflow")
            }
        }
    }
}

impl std::error::Error for SystemBusError {}

pub const IRQ_LINES: usize = 16;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DeviceId(usize);

pub struct SystemBus {
    devices: Vec<MappedDevice>,
    irq_table: [Option<DeviceId>; IRQ_LINES],
}

impl SystemBus {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            irq_table: [None; IRQ_LINES],
        }
    }

    pub fn map_device(&mut self, base: u32, device: Box<dyn BusDevice>) -> DeviceId {
        let size = device.size();

        assert!(
            size > 0,
            "cannot map zero-sized device at base=0x{base:08X}"
        );

        let end_exclusive = u64::from(base) + u64::from(size);

        assert!(
            end_exclusive <= 0x1_0000_0000,
            "device mapping overflows address space: base=0x{base:08X}, size=0x{size:08X}"
        );

        for existing in &self.devices {
            assert!(
                !existing.overlaps(base, size),
                "device mapping overlaps existing mapping: new=0x{base:08X}..0x{end_exclusive:08X}, existing=0x{:08X}..0x{:08X}",
                existing.base(),
                existing.end_exclusive(),
            );
        }

        let mapped = MappedDevice::new(base, device);
        let id = DeviceId(self.devices.len());
        self.devices.push(mapped);
        id
    }

    pub fn register_irq(&mut self, irq_line: usize, device_id: DeviceId) {
        assert!(irq_line < IRQ_LINES, "IRQ line out of range: {irq_line}");

        assert!(
            device_id.0 < self.devices.len(),
            "invalid device id: {:?}",
            device_id
        );

        self.irq_table[irq_line] = Some(device_id);
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
        for device in &mut self.devices {
            if device.contains(addr) {
                if let Some(value) = device.read8(device.offset(addr)) {
                    return Ok(value);
                }
            }
        }

        Err(SystemBusError::AddressUnmapped { addr })
    }

    pub fn write8(&mut self, addr: u32, value: u8) -> Result<(), SystemBusError> {
        for device in &mut self.devices {
            if device.contains(addr) {
                if device.write8(device.offset(addr), value).is_some() {
                    return Ok(());
                }
            }
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

    pub fn pending_interrupt(&self) -> Option<u32> {
        self.irq_table.iter().flatten().find_map(|device_id| {
            let device = &self.devices[device_id.0];

            if device.interrupt_asserted() {
                Some(device.base())
            } else {
                None
            }
        })
    }
}

impl std::fmt::Debug for SystemBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemBus")
            .field("devices", &self.devices)
            .field("irq_table", &self.irq_table)
            .finish()
    }
}

impl Default for SystemBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Lifecycle for SystemBus {
    fn reset(&mut self) {
        for device in &mut self.devices {
            device.reset();
        }
    }

    fn init(&mut self) {
        for device in &mut self.devices {
            device.init();
        }
    }

    fn tick(&mut self) {
        for device in &mut self.devices {
            device.tick();
        }
    }

    fn halt(&mut self) {
        for device in &mut self.devices {
            device.halt();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Clock, Ram, Serial, Timer, VecSerialSink, VecSerialSource};

    fn test_bus() -> SystemBus {
        let mut bus = SystemBus::new();
        let ram = Box::new(Ram::new(1024));
        bus.map_device(0, ram);
        bus
    }

    #[test]
    fn ram_byte_roundtrip_routes_through_system_bus() {
        let mut bus = test_bus();

        bus.write8(0x0000_0010, 0xAB).unwrap();

        assert_eq!(bus.read8(0x0000_0010), Ok(0xAB));
    }

    #[test]
    fn ram_halfword_roundtrip_uses_big_endian_bus_helpers() {
        let mut bus = test_bus();

        bus.write16(0x0000_0010, 0x1234).unwrap();

        assert_eq!(bus.read8(0x0000_0010), Ok(0x12));
        assert_eq!(bus.read8(0x0000_0011), Ok(0x34));
        assert_eq!(bus.read16(0x0000_0010), Ok(0x1234));
    }

    #[test]
    fn ram_word_roundtrip_uses_big_endian_bus_helpers() {
        let mut bus = test_bus();

        bus.write32(0x0000_0010, 0x1234_5678).unwrap();

        assert_eq!(bus.read8(0x0000_0010), Ok(0x12));
        assert_eq!(bus.read8(0x0000_0011), Ok(0x34));
        assert_eq!(bus.read8(0x0000_0012), Ok(0x56));
        assert_eq!(bus.read8(0x0000_0013), Ok(0x78));
        assert_eq!(bus.read32(0x0000_0010), Ok(0x1234_5678));
    }

    #[test]
    fn misaligned_halfword_access_fails_through_system_bus() {
        let mut bus = test_bus();

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
        let mut bus = test_bus();

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
        let mut bus = test_bus();

        assert_eq!(
            bus.read8(0x0000_1000),
            Err(SystemBusError::AddressUnmapped { addr: 0x0000_1000 })
        );
    }

    #[test]
    fn unmapped_byte_write_fails() {
        let mut bus = test_bus();

        assert_eq!(
            bus.write8(0x0000_1000, 0xAB),
            Err(SystemBusError::AddressUnmapped { addr: 0x0000_1000 })
        );
    }

    #[test]
    fn read32_clock_low_word_uses_big_endian_mapped_bytes() {
        let mut bus = test_bus();
        let clock = Box::new(Clock::new());
        bus.map_device(0xFFFF_0000, clock);

        bus.tick();
        bus.tick();

        assert_eq!(bus.read8(0xFFFF_0003), Ok(2));
    }

    #[test]
    fn write32_timer_counter_uses_big_endian_mapped_bytes() {
        let mut bus = test_bus();
        let timer = Box::new(Timer::new());
        bus.map_device(0xFFFF_0000, timer);

        bus.write32(0xFFFF_0000, 0x1234_5678).unwrap();

        assert_eq!(bus.read16(0xFFFF_0000), Ok(0x1234));
        assert_eq!(bus.read16(0xFFFF_0002), Ok(0x5678));
    }

    #[test]
    fn address_below_serial_base_does_not_map_to_serial_by_wrapping() {
        let mut bus = test_bus();
        let serial = Box::new(Serial::new(
            VecSerialSink::new(),
            VecSerialSource::new([0xFu8; 12]),
        ));
        bus.map_device(0xFFFF_0000, serial);

        assert_eq!(
            bus.read8(0xFFFE_FFFF),
            Err(SystemBusError::AddressUnmapped { addr: 0xFFFE_FFFF })
        );
    }

    #[test]
    fn unmapped_address_returns_address_unmapped_after_all_devices_decline() {
        let mut bus = test_bus();

        assert_eq!(
            bus.read8(0x0000_1000),
            Err(SystemBusError::AddressUnmapped { addr: 0x0000_1000 })
        );
    }
}
