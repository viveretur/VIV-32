//! Top-level emulator harness.
//!
//! `Machine` assembles the CPU, bus, memory, and devices into a runnable VIV-32
//! instance. Construction loads the host-side machine image; `reset` starts the
//! architectural machine; `tick` advances execution by one CPU tick.
use crate::{
    Cpu, SystemBus, SystemBusError,
    lifecycle::Reset,
    platform::{SerialSink, SerialSource, VecSerialSink, VecSerialSource},
};

pub struct MachineConfig {
    pub ram_size: u32,
    pub ram_image: Option<Vec<u8>>,
    pub ram_image_base: u32,
    pub serial_sink: Option<Box<dyn SerialSink>>,
    pub serial_source: Option<Box<dyn SerialSource>>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            ram_size: 64 * 1024,
            ram_image: None,
            ram_image_base: 0,
            serial_sink: None,
            serial_source: None,
        }
    }
}

pub struct Machine {
    bus: SystemBus,
    cpu: Cpu,
}

impl Machine {
    pub fn new(bus: SystemBus) -> Self {
        Self {
            cpu: Cpu::new(),
            bus,
        }
    }

    pub fn from_config(mut config: MachineConfig) -> Result<Self, SystemBusError> {
        let serial_sink = config
            .serial_sink
            .take()
            .unwrap_or_else(|| Box::new(VecSerialSink::new()));

        let serial_source = config
            .serial_source
            .take()
            .unwrap_or_else(|| Box::new(VecSerialSource::default()));

        let mut bus = SystemBus::with_serial(config.ram_size, serial_sink, serial_source);

        if let Some(image) = config.ram_image.as_ref() {
            bus.load_ram_image(config.ram_image_base, image)?;
        }

        Ok(Self::new(bus))
    }

    pub fn with_ram_size(ram_size: u32) -> Self {
        Self::from_config(MachineConfig {
            ram_size,
            ..MachineConfig::default()
        })
        .expect("default machine configuration should be valid")
    }

    pub fn with_ram_image(ram_size: u32, image: Vec<u8>) -> Result<Self, SystemBusError> {
        Self::from_config(MachineConfig {
            ram_size,
            ram_image: Some(image),
            ram_image_base: 0,
            ..MachineConfig::default()
        })
    }

    pub fn bus(&self) -> &SystemBus {
        &self.bus
    }

    pub fn bus_mut(&mut self) -> &mut SystemBus {
        &mut self.bus
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn is_running(&self) -> bool {
        !self.cpu.is_halted()
    }

    pub fn reset(&mut self) {
        self.bus.reset();
        self.cpu.reset();
    }

    pub fn tick(&mut self) {
        self.cpu.tick_with_bus(&mut self.bus);
    }

    pub fn run_for(&mut self, cycles: usize) {
        for _ in 0..cycles {
            if self.cpu.is_halted() {
                break;
            }

            self.tick();
        }
    }
}
