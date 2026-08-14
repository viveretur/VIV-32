//! VIV-32 processor model.
//!
//! This module implements the architectural CPU state and instruction execution
//! for the reference emulator. The CPU owns the system bus, fetches instructions
//! from memory, translates architectural exceptions into vector transfers, and
//! advances attached platform devices through the bus tick.
//!
//! The implementation is intentionally separate from the generated ISA constants:
//! generated data defines encodings, while this module defines processor
//! behaviour.
mod cpu;
mod creg_file;
mod exception_cause;
mod gpr;
mod pc;
mod status;

pub use cpu::Cpu;
pub use creg_file::CregFile;
use exception_cause::ExceptionCause;
pub use gpr::GprFile;
pub use pc::ProgramCounter;
pub use status::StatusRegister;
