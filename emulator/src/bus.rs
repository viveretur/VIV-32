#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusError {
    AddressOutOfRange { addr: u32 },
    UnsupportedAccess { addr: u32 },
}

pub trait Bus {
    fn read8(&mut self, addr: u32) -> Result<u8, BusError>;

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError>;

    fn read16(&mut self, addr: u32) -> Result<u16, BusError> {
        let b0 = self.read8(addr)? as u16;
        let b1 = self.read8(addr.wrapping_add(1))? as u16;

        Ok((b0 << 8) | b1)
    }

    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusError> {
        self.write8(addr, (value >> 8) as u8)?;
        self.write8(addr.wrapping_add(1), value as u8)?;

        Ok(())
    }

    fn read32(&mut self, addr: u32) -> Result<u32, BusError> {
        let b0 = self.read8(addr)? as u32;
        let b1 = self.read8(addr.wrapping_add(1))? as u32;
        let b2 = self.read8(addr.wrapping_add(2))? as u32;
        let b3 = self.read8(addr.wrapping_add(3))? as u32;

        Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    }

    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusError> {
        self.write8(addr, (value >> 24) as u8)?;
        self.write8(addr.wrapping_add(1), (value >> 16) as u8)?;
        self.write8(addr.wrapping_add(2), (value >> 8) as u8)?;
        self.write8(addr.wrapping_add(3), value as u8)?;

        Ok(())
    }
}
