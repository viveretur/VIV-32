use super::MemoryMapping;

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
}

impl MemoryMapping for Ram {
    fn read8(&mut self, offset: u32) -> Option<u8> {
        self.bytes
            .get(offset as usize)
            .copied()
    }

    fn write8(&mut self, offset: u32, value: u8) -> Option<()> {
        let byte = self.bytes.get_mut(offset as usize)?;
        *byte = value;
        Some(())
    }
}

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
