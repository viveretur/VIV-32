use crate::bus::BusError;
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

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn as_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
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

impl<S> Serial<S>
where
    S: SerialSink,
{
    pub const DATA_OFFSET: u32 = 0x00;
    pub const CONTROL_OFFSET: u32 = 0x04;

    pub const CONTROL_ENABLE: u32 = 1 << 0;
    pub const CONTROL_TX_ENABLE: u32 = 1 << 1;

    pub const CONTROL_BAUD_SHIFT: u32 = 4;
    pub const CONTROL_BAUD_MASK: u32 = 0xF << Self::CONTROL_BAUD_SHIFT;

    pub const CONTROL_DATA_BITS_SHIFT: u32 = 8;
    pub const CONTROL_DATA_BITS_MASK: u32 = 0x3 << Self::CONTROL_DATA_BITS_SHIFT;

    pub const CONTROL_PARITY_SHIFT: u32 = 10;
    pub const CONTROL_PARITY_MASK: u32 = 0x3 << Self::CONTROL_PARITY_SHIFT;

    pub const CONTROL_STOP_BITS_SHIFT: u32 = 12;
    pub const CONTROL_STOP_BITS_MASK: u32 = 0x1 << Self::CONTROL_STOP_BITS_SHIFT;

    pub const CONTROL_WRITABLE_MASK: u32 = Self::CONTROL_ENABLE
        | Self::CONTROL_TX_ENABLE
        | Self::CONTROL_BAUD_MASK
        | Self::CONTROL_DATA_BITS_MASK
        | Self::CONTROL_PARITY_MASK
        | Self::CONTROL_STOP_BITS_MASK;

    pub fn new(sink: S) -> Self {
        Self { sink, control: 0 }
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    pub fn into_sink(self) -> S {
        self.sink
    }

    pub fn control(&self) -> u32 {
        self.control
    }

    pub fn enabled(&self) -> bool {
        self.control & Self::CONTROL_ENABLE != 0
    }

    pub fn tx_enabled(&self) -> bool {
        self.control & Self::CONTROL_TX_ENABLE != 0
    }

    pub fn write_control(&mut self, value: u32) {
        self.control = value & Self::CONTROL_WRITABLE_MASK;
    }

    pub fn read_control(&self) -> u32 {
        self.control
    }

    pub fn write_data(&mut self, byte: u8) {
        if self.enabled() && self.tx_enabled() {
            self.sink.write_byte(byte);
        }
    }

    pub fn read32(&self, offset: u32) -> Result<u32, BusError> {
        match offset {
            Self::CONTROL_OFFSET => Ok(self.read_control()),
            _ => Err(BusError::UnsupportedAccess { addr: offset }),
        }
    }

    pub fn write32(&mut self, offset: u32, value: u32) -> Result<(), BusError> {
        match offset {
            Self::DATA_OFFSET => {
                self.write_data(value as u8);
                Ok(())
            }
            Self::CONTROL_OFFSET => {
                self.write_control(value);
                Ok(())
            }
            _ => Err(BusError::UnsupportedAccess { addr: offset }),
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

#[cfg(test)]
mod tests {
    use super::*;

    type TestSerial = Serial<VecSerialSink>;

    #[test]
    fn vec_serial_sink_collects_bytes() {
        let mut sink = VecSerialSink::new();

        sink.write_byte(b'H');
        sink.write_byte(b'i');
        sink.write_byte(b'\n');

        assert_eq!(sink.bytes(), b"Hi\n");
    }

    #[test]
    fn vec_serial_sink_can_render_lossy_string() {
        let mut sink = VecSerialSink::new();

        sink.write_byte(b'O');
        sink.write_byte(b'K');

        assert_eq!(sink.as_string_lossy(), "OK");
    }

    #[test]
    fn vec_serial_sink_can_be_cleared() {
        let mut sink = VecSerialSink::new();

        sink.write_byte(b'A');
        sink.clear();

        assert_eq!(sink.bytes(), b"");
    }

    #[test]
    fn new_serial_starts_disabled() {
        let serial = TestSerial::new(VecSerialSink::new());

        assert_eq!(serial.control(), 0);
        assert!(!serial.enabled());
        assert!(!serial.tx_enabled());
    }

    #[test]
    fn disabled_serial_does_not_emit_bytes() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_data(b'A');

        assert_eq!(serial.sink().bytes(), b"");
    }

    #[test]
    fn enabled_without_tx_enable_does_not_emit_bytes() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(TestSerial::CONTROL_ENABLE);
        serial.write_data(b'A');

        assert_eq!(serial.sink().bytes(), b"");
    }

    #[test]
    fn tx_enable_without_device_enable_does_not_emit_bytes() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(TestSerial::CONTROL_TX_ENABLE);
        serial.write_data(b'A');

        assert_eq!(serial.sink().bytes(), b"");
    }

    #[test]
    fn enabled_serial_emits_bytes() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(TestSerial::CONTROL_ENABLE | TestSerial::CONTROL_TX_ENABLE);
        serial.write_data(b'A');

        assert_eq!(serial.sink().bytes(), b"A");
    }

    #[test]
    fn reset_clears_control_but_keeps_sink_contents() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(TestSerial::CONTROL_ENABLE | TestSerial::CONTROL_TX_ENABLE);
        serial.write_data(b'A');

        serial.reset();

        assert_eq!(serial.control(), 0);
        assert!(!serial.enabled());
        assert!(!serial.tx_enabled());
        assert_eq!(serial.sink().bytes(), b"A");
    }

    #[test]
    fn init_resets_serial() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(TestSerial::CONTROL_ENABLE | TestSerial::CONTROL_TX_ENABLE);
        serial.write_data(b'A');

        serial.init();

        assert_eq!(serial.control(), 0);
        assert!(!serial.enabled());
        assert!(!serial.tx_enabled());
        assert_eq!(serial.sink().bytes(), b"A");
    }

    #[test]
    fn write32_to_data_emits_low_byte() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(TestSerial::CONTROL_ENABLE | TestSerial::CONTROL_TX_ENABLE);
        serial.write32(TestSerial::DATA_OFFSET, 0x1234_56AB).unwrap();

        assert_eq!(serial.sink().bytes(), b"\xAB");
    }

    #[test]
    fn read32_control_returns_control_register() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        let control = TestSerial::CONTROL_ENABLE
            | TestSerial::CONTROL_TX_ENABLE
            | (3 << TestSerial::CONTROL_BAUD_SHIFT);

        serial.write_control(control);

        assert_eq!(serial.read32(TestSerial::CONTROL_OFFSET).unwrap(), control);
    }

    #[test]
    fn write_control_masks_reserved_bits() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        serial.write_control(u32::MAX);

        assert_eq!(serial.control(), TestSerial::CONTROL_WRITABLE_MASK);
    }

    #[test]
    fn invalid_read_offset_errors() {
        let serial = TestSerial::new(VecSerialSink::new());

        assert_eq!(
            serial.read32(0x08),
            Err(BusError::UnsupportedAccess { addr: 0x08 })
        );
    }

    #[test]
    fn invalid_write_offset_errors() {
        let mut serial = TestSerial::new(VecSerialSink::new());

        assert_eq!(
            serial.write32(0x08, 0),
            Err(BusError::UnsupportedAccess { addr: 0x08 })
        );
    }
}
