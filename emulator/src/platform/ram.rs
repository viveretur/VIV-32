use super::{BusDevice, SystemBusError};

use crate::Lifecycle;

#[derive(Debug, Clone)]
pub struct Ram {
    bytes: Vec<u8>,
}

impl Ram {
    pub fn new(size: u32) -> Self {
        Self {
            bytes: vec![0; size as usize],
        }
    }

    pub fn from_bytes(size: u32, data: &[u8]) -> Self {
        assert!(
            data.len() <= size as usize,
            "RAM image is larger than RAM size"
        );

        let mut bytes = vec![0; size as usize];
        bytes[..data.len()].copy_from_slice(data);

        Self { bytes }
    }

    pub fn write_slice(&mut self, base_addr: u32, image: &[u8]) -> Result<(), SystemBusError> {
        let start = usize::try_from(base_addr).map_err(|_| SystemBusError::AddressOverflow)?;

        let end = start
            .checked_add(image.len())
            .ok_or(SystemBusError::AddressOverflow)?;

        if end > self.bytes.len() {
            return Err(SystemBusError::AddressUnmapped { addr: end as u32 });
        }

        self.bytes[start..end].copy_from_slice(image);
        Ok(())
    }
}

impl BusDevice for Ram {
    fn size(&self) -> u32 {
        self.bytes.len() as u32
    }

    fn read8(&mut self, offset: u32) -> Option<u8> {
        self.bytes.get(offset as usize).copied()
    }

    fn write8(&mut self, offset: u32, value: u8) -> Option<()> {
        let byte = self.bytes.get_mut(offset as usize)?;
        *byte = value;
        Some(())
    }
}

impl Lifecycle for Ram {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_byte_round_trip() {
        let mut ram = Ram::new(1024);

        ram.write8(12, 0xAB).unwrap();

        assert_eq!(ram.read8(12).unwrap(), 0xAB);
    }

    #[test]
    fn new_ram_reads_as_zero() {
        let mut ram = Ram::new(4);

        assert_eq!(ram.read8(0), Some(0));
        assert_eq!(ram.read8(3), Some(0));
    }

    #[test]
    fn read8_returns_none_outside_ram() {
        let mut ram = Ram::new(4);

        assert_eq!(ram.read8(4), None);
    }

    #[test]
    fn write8_returns_none_outside_ram() {
        let mut ram = Ram::new(4);

        assert_eq!(ram.write8(4, 0xAA), None);
    }

    #[test]
    fn write8_at_last_byte_succeeds() {
        let mut ram = Ram::new(4);

        assert_eq!(ram.write8(3, 0xAA), Some(()));
        assert_eq!(ram.read8(3), Some(0xAA));
    }
}
