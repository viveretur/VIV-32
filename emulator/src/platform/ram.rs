use crate::bus::{Bus, BusError};

#[derive(Debug, Clone)]
pub struct Ram {
    base: u32,
    bytes: Vec<u8>,
}

impl Ram {
    pub fn new(base: u32, size: usize) -> Self {
        Self {
            base,
            bytes: vec![0; size],
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn offset(&self, addr: u32) -> Result<usize, BusError> {
        let offset = addr
            .checked_sub(self.base)
            .ok_or(BusError::AddressOutOfRange { addr })?;

        let offset = offset as usize;

        if offset >= self.bytes.len() {
            return Err(BusError::AddressOutOfRange { addr });
        }

        Ok(offset)
    }
}

impl Bus for Ram {
    fn read8(&mut self, addr: u32) -> Result<u8, BusError> {
        let offset = self.offset(addr)?;
        Ok(self.bytes[offset])
    }

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
        let offset = self.offset(addr)?;
        self.bytes[offset] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_byte_round_trip() {
        let mut ram = Ram::new(0, 1024);

        ram.write8(12, 0xAB).unwrap();

        assert_eq!(ram.read8(12).unwrap(), 0xAB);
    }

    #[test]
    fn read_write_word_is_big_endian() {
        let mut ram = Ram::new(0, 1024);

        ram.write32(4, 0x1234_5678).unwrap();

        assert_eq!(ram.read8(4).unwrap(), 0x12);
        assert_eq!(ram.read8(5).unwrap(), 0x34);
        assert_eq!(ram.read8(6).unwrap(), 0x56);
        assert_eq!(ram.read8(7).unwrap(), 0x78);
        assert_eq!(ram.read32(4).unwrap(), 0x1234_5678);
    }

    #[test]
    fn address_below_base_is_out_of_range() {
        let mut ram = Ram::new(0x1000, 1024);

        assert_eq!(
            ram.read8(0x0FFF),
            Err(BusError::AddressOutOfRange { addr: 0x0FFF })
        );
    }

    #[test]
    fn address_past_end_is_out_of_range() {
        let mut ram = Ram::new(0x1000, 1024);

        assert_eq!(
            ram.read8(0x1400),
            Err(BusError::AddressOutOfRange { addr: 0x1400 })
        );
    }
}
