#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusError {
    AddressOutOfRange { addr: u32 },
    UnsupportedAccess { addr: u32 },
    MisalignedAccess {
        addr: u32,
        alignment: u32,
    },
}

pub trait Bus {

    fn check_alignment(addr: u32, alignment: u32) -> Result<(), BusError> {
        if addr & (alignment - 1) != 0 {
            return Err(BusError::MisalignedAccess {
                addr,
                alignment,
            });
        }

        Ok(())
    }
    
    fn read8(&mut self, addr: u32) -> Result<u8, BusError>;

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError>;

    fn read16(&mut self, addr: u32) -> Result<u16, BusError> {
        Self::check_alignment(addr, 2)?;
        
        let b0 = self.read8(addr)? as u16;
        let b1 = self.read8(addr.wrapping_add(1))? as u16;

        Ok((b0 << 8) | b1)
    }

    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusError> {
        Self::check_alignment(addr, 2)?;
        
        self.write8(addr, (value >> 8) as u8)?;
        self.write8(addr.wrapping_add(1), value as u8)?;

        Ok(())
    }

    fn read32(&mut self, addr: u32) -> Result<u32, BusError> {
        Self::check_alignment(addr, 4)?;
        
        let b0 = self.read8(addr)? as u32;
        let b1 = self.read8(addr.wrapping_add(1))? as u32;
        let b2 = self.read8(addr.wrapping_add(2))? as u32;
        let b3 = self.read8(addr.wrapping_add(3))? as u32;

        Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    }

    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusError> {
        Self::check_alignment(addr, 4)?;
        
        self.write8(addr, (value >> 24) as u8)?;
        self.write8(addr.wrapping_add(1), (value >> 16) as u8)?;
        self.write8(addr.wrapping_add(2), (value >> 8) as u8)?;
        self.write8(addr.wrapping_add(3), value as u8)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestBus {
        bytes: [u8; 16],
    }

    impl TestBus {
        fn new() -> Self {
            Self {
                bytes: [
                    0x00, 0x11, 0x22, 0x33,
                    0x44, 0x55, 0x66, 0x77,
                    0x88, 0x99, 0xAA, 0xBB,
                    0xCC, 0xDD, 0xEE, 0xFF,
                ],
            }
        }
    }

    impl Bus for TestBus {
        fn read8(&mut self, addr: u32) -> Result<u8, BusError> {
            self.bytes
                .get(addr as usize)
                .copied()
                .ok_or(BusError::AddressOutOfRange { addr })
        }

        fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
            let slot = self
                .bytes
                .get_mut(addr as usize)
                .ok_or(BusError::AddressOutOfRange { addr })?;

            *slot = value;
            Ok(())
        }
    }

    #[test]
    fn read8_allows_unaligned_addresses() {
        let mut bus = TestBus::new();

        assert_eq!(bus.read8(1), Ok(0x11));
        assert_eq!(bus.read8(2), Ok(0x22));
        assert_eq!(bus.read8(3), Ok(0x33));
    }

    #[test]
    fn read16_rejects_misaligned_address() {
        let mut bus = TestBus::new();

        assert_eq!(
            bus.read16(1),
            Err(BusError::MisalignedAccess {
                addr: 1,
                alignment: 2,
            })
        );
    }

    #[test]
    fn read32_rejects_misaligned_address() {
        let mut bus = TestBus::new();

        assert_eq!(
            bus.read32(2),
            Err(BusError::MisalignedAccess {
                addr: 2,
                alignment: 4,
            })
        );
    }

    #[test]
    fn write16_rejects_misaligned_address() {
        let mut bus = TestBus::new();

        assert_eq!(
            bus.write16(3, 0xABCD),
            Err(BusError::MisalignedAccess {
                addr: 3,
                alignment: 2,
            })
        );
    }

    #[test]
    fn write32_rejects_misaligned_address() {
        let mut bus = TestBus::new();

        assert_eq!(
            bus.write32(1, 0x1234_5678),
            Err(BusError::MisalignedAccess {
                addr: 1,
                alignment: 4,
            })
        );
    }

    #[test]
    fn read32_reads_big_endian_word() {
        let mut bus = TestBus::new();

        assert_eq!(bus.read32(4), Ok(0x4455_6677));
    }
}
