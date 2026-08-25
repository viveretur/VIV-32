use crate::Lifecycle;

/// Defines memory access routines for memory and memory-mapped items.
pub trait BusDevice: Lifecycle {
    /// How much memory in bytes this device spans.
    fn size(&self) -> u32;

    /// Read the byte at the specific offset. Returns None if the
    /// offset is beyond this items memory range.
    fn read8(&mut self, offset: u32) -> Option<u8>;

    /// Writes the byte to the specified offset. If None is returned,
    /// that means the write was beyond this items memory range.
    fn write8(&mut self, offset: u32, value: u8) -> Option<()>;

    /// Signals that the device has something requiring attention.
    fn interrupt_asserted(&mut self) -> bool {
        false
    }
}
