use super::BusDevice;

use crate::Lifecycle;

pub struct MappedDevice {
    base: u32,
    end: u64,
    device: Box<dyn BusDevice>,
}

impl MappedDevice {
    pub fn new(base: u32, device: Box<dyn BusDevice>) -> Self {
        let size = device.size();
        assert!(
            (base as u64 + size as u64) <= 0x1_0000_0000,
            "device mapping overflows address space: base=0x{base:08X}, size=0x{size:08X}"
        );
        Self {
            base,
            end: base as u64 + size as u64,
            device,
        }
    }

    pub fn contains(&self, address: u32) -> bool {
        self.base <= address && (address as u64) < self.end
    }

    pub fn offset(&self, address: u32) -> u32 {
        address - self.base
    }

    pub fn base(&self) -> u32 {
        self.base
    }

    pub fn end_exclusive(&self) -> u64 {
        self.end
    }

    pub fn overlaps(&self, base: u32, size: u32) -> bool {
        let new_start = u64::from(base);
        let new_end = new_start + u64::from(size);

        let existing_start = u64::from(self.base());
        let existing_end = self.end;

        new_start < existing_end && existing_start < new_end
    }
}

impl BusDevice for MappedDevice {
    fn size(&self) -> u32 {
        self.device.size()
    }

    fn read8(&mut self, offset: u32) -> Option<u8> {
        self.device.read8(offset)
    }

    fn write8(&mut self, offset: u32, value: u8) -> Option<()> {
        self.device.write8(offset, value)
    }

    fn interrupt_asserted(&self) -> bool {
        self.device.interrupt_asserted()
    }
}

impl Lifecycle for MappedDevice {
    fn init(&mut self) {
        self.device.init();
    }
    fn reset(&mut self) {
        self.device.reset();
    }

    fn tick(&mut self) {
        self.device.tick();
    }

    fn halt(&mut self) {
        self.device.halt();
    }
}

impl std::fmt::Debug for MappedDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedDevice")
            .field("base", &self.base)
            .field("end", &self.end)
            .field("device", &"<bus device>")
            .finish()
    }
}
