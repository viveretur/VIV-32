use std::collections::VecDeque;
use std::io::{self, Read};

use super::BusDevice;

use crate::Lifecycle;

pub trait SerialSink {
    fn write_byte(&mut self, byte: u8);
    fn ready(&mut self) -> bool;
}

pub trait SerialSource {
    fn read_byte(&mut self) -> Option<u8>;
    fn has_byte(&mut self) -> bool;
}

impl<T: SerialSink + ?Sized> SerialSink for Box<T> {
    fn write_byte(&mut self, byte: u8) {
        (**self).write_byte(byte);
    }

    fn ready(&mut self) -> bool {
        (**self).ready()
    }
}

impl<T: SerialSource + ?Sized> SerialSource for Box<T> {
    fn read_byte(&mut self) -> Option<u8> {
        (**self).read_byte()
    }

    fn has_byte(&mut self) -> bool {
        (**self).has_byte()
    }
}

#[derive(Debug, Clone, Default)]
pub struct VecSerialSink {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct VecSerialSource {
    bytes: VecDeque<u8>,
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

    fn ready(&mut self) -> bool {
        true
    }
}

impl VecSerialSource {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into().into(),
        }
    }
}

impl Default for VecSerialSource {
    fn default() -> Self {
        Self {
            bytes: VecDeque::new(),
        }
    }
}

impl SerialSource for VecSerialSource {
    fn read_byte(&mut self) -> Option<u8> {
        self.bytes.pop_front()
    }

    fn has_byte(&mut self) -> bool {
        !self.bytes.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct StdoutSerialSink {
    ready: u8,
}

#[derive(Debug)]
pub struct StdinSerialSource {
    data: Option<u8>,
}

impl SerialSink for StdoutSerialSink {
    fn write_byte(&mut self, byte: u8) {
        use std::io::{self, Write};

        let _ = io::stdout().write_all(&[byte]);
        let _ = io::stdout().flush();

        self.ready = 0xFF;
    }

    fn ready(&mut self) -> bool {
        self.ready = self.ready.saturating_sub(1);
        self.ready == 0
    }
}

fn set_stdin_nonblocking() -> io::Result<()> {
    unsafe {
        let flags = libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL);
        if flags == -1 {
            return Err(io::Error::last_os_error());
        }

        if libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK) == -1 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

fn try_read_stdin() -> io::Result<Option<u8>> {
    let mut byte = [0u8; 1];

    match io::stdin().read(&mut byte) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(byte[0])),
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(err) => Err(err),
    }
}

impl Default for StdinSerialSource {
    fn default() -> Self {
        set_stdin_nonblocking().expect("Unable to set stdin correctly, aborting.");
        Self { data: None }
    }
}

impl SerialSource for StdinSerialSource {
    fn has_byte(&mut self) -> bool {
        self.data.is_some() || {
            self.data = try_read_stdin().expect("Something went wrong reading data");
            self.data.is_some()
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        let ret = self.data;
        self.data = None;
        ret
    }
}

#[derive(Debug, Clone)]
pub struct Serial<S, T> {
    sink: S,
    source: T,
    control: u32,
}

const DATA_OFFSET: u32 = 0x00;

const CONTROL_OFFSET: u32 = 0x04;
const CONTROL_END_OFFSET: u32 = 0x07;
const CONTROL_ENABLE: u32 = 1 << 0;
const CONTROL_TX_ENABLE: u32 = 1 << 1;
const CONTROL_RX_ENABLE: u32 = 1 << 2;
const CONTROL_TX_INTERRUPTS: u32 = 1 << 3;
const CONTROL_RX_INTERRUPTS: u32 = 1 << 4;
const CONTROL_WRITABLE_MASK: u32 = CONTROL_ENABLE
    | CONTROL_TX_ENABLE
    | CONTROL_RX_ENABLE
    | CONTROL_TX_INTERRUPTS
    | CONTROL_RX_INTERRUPTS;

const STATUS_OFFSET: u32 = 0x08;
const STATUS_RX_READY: u32 = 1 << 0;
const STATUS_TX_READY: u32 = 1 << 1;

impl<S, T> Serial<S, T>
where
    S: SerialSink,
    T: SerialSource,
{
    pub fn new(sink: S, source: T) -> Self {
        Self {
            sink,
            source,
            control: 0,
        }
    }

    fn enabled(&self) -> bool {
        self.control & CONTROL_ENABLE != 0
    }

    fn tx_enabled(&self) -> bool {
        self.control & CONTROL_TX_ENABLE != 0
    }

    fn rx_enabled(&self) -> bool {
        self.control & CONTROL_RX_ENABLE != 0
    }

    fn write_control(&mut self, value: u32) {
        self.control = value & CONTROL_WRITABLE_MASK;
    }

    fn read_data(&mut self) -> Option<u8> {
        let x = if self.enabled() && self.rx_enabled() {
            Some(self.source.read_byte().unwrap_or(0))
        } else {
            Some(0)
        };
        // println!("Attempted to read {:?}", x);
        x
    }

    fn write_data(&mut self, byte: u8) {
        // println!("attempted to write {}", byte);
        if self.enabled() && self.tx_enabled() {
            self.sink.write_byte(byte);
        }
    }

    fn status(&mut self) -> u32 {
        let mut status = 0;

        if self.enabled() {
            if self.tx_enabled() && self.sink.ready() {
                status |= STATUS_TX_READY;
            }

            if self.rx_enabled() && self.source.has_byte() {
                status |= STATUS_RX_READY;
            }
        }

        status
    }
}

impl<S, T> Lifecycle for Serial<S, T>
where
    S: SerialSink,
    T: SerialSource,
{
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.control = 0;
    }
}

impl<S, T> Default for Serial<S, T>
where
    S: SerialSink + Default,
    T: SerialSource + Default,
{
    fn default() -> Self {
        Self::new(S::default(), T::default())
    }
}

impl<S, T> BusDevice for Serial<S, T>
where
    S: SerialSink,
    T: SerialSource,
{
    fn size(&self) -> u32 {
        12
    }

    fn read8(&mut self, offset: u32) -> Option<u8> {
        match offset {
            DATA_OFFSET => self.read_data(),

            CONTROL_OFFSET => Some((self.control >> 24) as u8),
            offset if offset == CONTROL_OFFSET + 1 => Some((self.control >> 16) as u8),
            offset if offset == CONTROL_OFFSET + 2 => Some((self.control >> 8) as u8),
            offset if offset == CONTROL_OFFSET + 3 => Some(self.control as u8),

            STATUS_OFFSET => Some((self.status() >> 24) as u8),
            offset if offset == STATUS_OFFSET + 1 => Some((self.status() >> 16) as u8),
            offset if offset == STATUS_OFFSET + 2 => Some((self.status() >> 8) as u8),
            offset if offset == STATUS_OFFSET + 3 => Some(self.status() as u8),

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

    fn interrupt_asserted(&mut self) -> bool {
        let control = self.control;
        let status = self.status();

        control & CONTROL_RX_INTERRUPTS != 0 && status & STATUS_RX_READY != 0
            || control & CONTROL_TX_INTERRUPTS != 0 && status & STATUS_TX_READY != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestSerial = Serial<VecSerialSink, VecSerialSource>;

    fn test_serial() -> TestSerial {
        TestSerial::new(VecSerialSink::new(), VecSerialSource::default())
    }

    #[test]
    fn read8_control_exposes_big_endian_bytes() {
        let mut serial = test_serial();

        serial.write_control(0x0000_1303);

        assert_eq!(serial.read8(CONTROL_OFFSET), Some(0x00));
        assert_eq!(serial.read8(CONTROL_OFFSET + 1), Some(0x00));
        assert_eq!(serial.read8(CONTROL_OFFSET + 2), Some(0x13));
        assert_eq!(serial.read8(CONTROL_OFFSET + 3), Some(0x03));
    }

    #[test]
    fn write8_control_updates_big_endian_byte_lane() {
        let mut serial = test_serial();

        assert_eq!(
            serial.write8(CONTROL_OFFSET + 3, CONTROL_ENABLE as u8),
            Some(())
        );
        assert!(serial.enabled());
    }

    #[test]
    fn read8_data_returns_zero_when_rx_disabled() {
        let mut serial = test_serial();

        assert_eq!(serial.read8(DATA_OFFSET), Some(0));
    }

    #[test]
    fn serial_unknown_offsets_return_none() {
        let mut serial = test_serial();

        assert_eq!(serial.read8(0x0C), None);
        assert_eq!(serial.write8(0x0C, 0xFF), None);
    }

    #[test]
    fn write8_data_emits_only_when_enabled_and_tx_enabled() {
        let mut serial = test_serial();

        assert_eq!(serial.write8(DATA_OFFSET, b'A'), Some(()));
        assert_eq!(serial.sink.bytes, b"");

        assert_eq!(
            serial.write8(CONTROL_OFFSET + 3, CONTROL_ENABLE as u8),
            Some(())
        );
        assert_eq!(serial.write8(DATA_OFFSET, b'B'), Some(()));
        assert_eq!(serial.sink.bytes, b"");

        assert_eq!(
            serial.write8(
                CONTROL_OFFSET + 3,
                (CONTROL_ENABLE | CONTROL_TX_ENABLE) as u8,
            ),
            Some(())
        );
        assert_eq!(serial.write8(DATA_OFFSET, b'C'), Some(()));
        assert_eq!(serial.sink.bytes, b"C");
    }

    #[test]
    fn read8_data_consumes_source_when_enabled_and_rx_enabled() {
        let mut serial =
            TestSerial::new(VecSerialSink::new(), VecSerialSource::new(b"AB".to_vec()));

        serial.write_control(CONTROL_ENABLE | CONTROL_RX_ENABLE);

        assert_eq!(serial.read8(DATA_OFFSET), Some(b'A'));
        assert_eq!(serial.read8(DATA_OFFSET), Some(b'B'));
        assert_eq!(serial.read8(DATA_OFFSET), Some(0));
    }
}
