use super::MemoryMapping;

use crate::lifecycle::{Init, Reset};

pub trait SerialSink {
    fn write_byte(&mut self, byte: u8);
}

#[derive(Debug, Clone, Default)]
pub struct VecSerialSink {
    bytes: Vec<u8>,
}

impl VecSerialSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SerialSink for VecSerialSink {
    fn write_byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }
}

#[derive(Debug, Clone)]
pub struct Serial<S> {
    sink: S,
    control: u32,
}

const DATA_OFFSET: u32 = 0x00;
const CONTROL_OFFSET: u32 = 0x04;
const CONTROL_END_OFFSET: u32 = 0x07;
const CONTROL_ENABLE: u32 = 1 << 0;
const CONTROL_TX_ENABLE: u32 = 1 << 1;
const CONTROL_BAUD_SHIFT: u32 = 4;
const CONTROL_BAUD_MASK: u32 = 0xF << CONTROL_BAUD_SHIFT;
const CONTROL_DATA_BITS_SHIFT: u32 = 8;
const CONTROL_DATA_BITS_MASK: u32 = 0x3 << CONTROL_DATA_BITS_SHIFT;
const CONTROL_PARITY_SHIFT: u32 = 10;
const CONTROL_PARITY_MASK: u32 = 0x3 << CONTROL_PARITY_SHIFT;
const CONTROL_STOP_BITS_SHIFT: u32 = 12;
const CONTROL_STOP_BITS_MASK: u32 = 0x1 << CONTROL_STOP_BITS_SHIFT;
const CONTROL_WRITABLE_MASK: u32 = CONTROL_ENABLE
    | CONTROL_TX_ENABLE
    | CONTROL_BAUD_MASK
    | CONTROL_DATA_BITS_MASK
    | CONTROL_PARITY_MASK
    | CONTROL_STOP_BITS_MASK;

impl<S> Serial<S>
where
    S: SerialSink,
{
    pub fn new(sink: S) -> Self {
        Self { sink, control: 0 }
    }

    fn enabled(&self) -> bool {
        self.control & CONTROL_ENABLE != 0
    }

    fn tx_enabled(&self) -> bool {
        self.control & CONTROL_TX_ENABLE != 0
    }

    fn write_control(&mut self, value: u32) {
        self.control = value & CONTROL_WRITABLE_MASK;
    }

    fn write_data(&mut self, byte: u8) {
        if self.enabled() && self.tx_enabled() {
            self.sink.write_byte(byte);
        }
    }
}

impl<S> Init for Serial<S>
where
    S: SerialSink,
{
    fn init(&mut self) {
        self.reset();
    }
}

impl<S> Reset for Serial<S>
where
    S: SerialSink,
{
    fn reset(&mut self) {
        self.control = 0;
    }
}

impl<S> Default for Serial<S>
where
    S: SerialSink + Default,
{
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<S: SerialSink> MemoryMapping for Serial<S> {
    fn read8(&mut self, offset: u32) -> Option<u8> {
        match offset {
            CONTROL_OFFSET => Some((self.control >> 24) as u8),
            offset if offset == CONTROL_OFFSET + 1 => Some((self.control >> 16) as u8),
            offset if offset == CONTROL_OFFSET + 2 => Some((self.control >> 8) as u8),
            offset if offset == CONTROL_OFFSET + 3 => Some(self.control as u8),

            _ => None,
        }
    }

    fn write8(&mut self, offset: u32, value: u8) -> Option<()> {
        match offset {
            DATA_OFFSET => {
                self.write_data(value);
                Some(())
            }

            CONTROL_OFFSET..=CONTROL_END_OFFSET => {
                let shift = (3 - (offset - CONTROL_OFFSET)) * 8;
                let mask = !(0xFFu32 << shift);
                let value = (value as u32) << shift;

                self.write_control((self.control & mask) | value);
                Some(())
            }

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestSerial = Serial<VecSerialSink>;

    #[test]
    fn read8_control_exposes_big_endian_bytes() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(0x0000_1303);

        assert_eq!(serial.read8(CONTROL_OFFSET), Some(0x00));
        assert_eq!(serial.read8(CONTROL_OFFSET + 1), Some(0x00));
        assert_eq!(serial.read8(CONTROL_OFFSET + 2), Some(0x13));
        assert_eq!(serial.read8(CONTROL_OFFSET + 3), Some(0x03));
    }

    #[test]
    fn write8_control_updates_big_endian_byte_lane() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        assert_eq!(
            serial.write8(CONTROL_OFFSET + 3, CONTROL_ENABLE as u8),
            Some(())
        );
        assert!(serial.enabled());
    }

    #[test]
    fn read8_data_returns_none() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        assert_eq!(serial.read8(DATA_OFFSET), None);
    }

    #[test]
    fn serial_unknown_offsets_return_none() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        assert_eq!(serial.read8(0x08), None);
        assert_eq!(serial.write8(0x08, 0xFF), None);
    }
}
