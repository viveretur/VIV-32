/// Defines memory access routines for memory and memory-mapped items.
pub trait MemoryMapping {
    /// Read the byte at the specific offset. Returns None if the
    /// offset is beyond this items memory range.
    fn read8(&mut self, offset: u32) -> Option<u8>;

    /// Writes the byte to the specified offset. If None is returned,
    /// that means the write was beyond this items memory range.
    fn write8(&mut self, offset: u32, value: u8) -> Option<()>;
}
