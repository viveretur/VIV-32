mod cpu;
mod isa;
mod lifecycle;
mod machine;
mod platform;

use cpu::Cpu;
pub use machine::{Machine, MachineConfig};
pub use platform::{
    SerialSink, SerialSource, StdinSerialSource, StdoutSerialSink, SystemBus, SystemBusError,
    VecSerialSink, VecSerialSource,
};
