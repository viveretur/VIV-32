mod creg;
mod decode;
mod instruction;
pub mod spec;

pub use creg::Creg;
pub use decode::{DecodeError, decode};
pub use instruction::Instruction;
