//! Top-level emulator harness.
//!
//! `Machine` assembles the CPU, bus, memory, and devices into a runnable VIV-32
//! instance. Construction loads the host-side machine image; `reset` starts the
//! architectural machine; `tick` advances execution by one CPU tick.
use crate::{
    Cpu, Lifecycle, SystemBus,
    platform::{
        Clock, Ram, Serial, SerialSink, SerialSource, StdoutSerialSink, SystemBusError, Timer,
        VecSerialSink, VecSerialSource,
    },
};

#[derive(Debug, serde::Deserialize)]
pub struct MachineTomlConfig {
    pub devices: Vec<DeviceTomlConfig>,
}

#[derive(Debug, serde::Deserialize)]
pub struct DeviceTomlConfig {
    pub name: Option<String>,
    pub kind: DeviceKind,
    pub base: u32,
    pub size: Option<u32>,
    pub irq: Option<usize>,

    pub image: Option<std::path::PathBuf>,
    pub image_base: Option<u32>,

    pub sink: Option<SerialSinkKind>,
    pub source: Option<SerialSourceKind>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Ram,
    Serial,
    Timer,
    Clock,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerialSinkKind {
    Stdout,
    Vec,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerialSourceKind {
    Empty,
    Stdin,
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

    pub fn from_toml_config(config: MachineTomlConfig) -> Result<Self, SystemBusError> {
        let mut bus = SystemBus::new();

        for device_config in config.devices {
            let device_id = match device_config.kind {
                DeviceKind::Ram => {
                    let size = device_config.size.expect("Missing RAM size!");

                    let mut ram = Ram::new(size);

                    if let Some(path) = device_config.image.as_ref() {
                        let image = std::fs::read(path).expect("Error reading file");
                        ram.write_slice(device_config.image_base.unwrap_or(0), &image)?;
                    }

                    bus.map_device(device_config.base, Box::new(ram))
                }

                DeviceKind::Serial => {
                    let sink: Box<dyn SerialSink> = match device_config.sink {
                        Some(SerialSinkKind::Stdout) | None => Box::new(StdoutSerialSink),
                        Some(SerialSinkKind::Vec) => Box::new(VecSerialSink::new()),
                    };

                    let source: Box<dyn SerialSource> = match device_config.source {
                        Some(SerialSourceKind::Empty) | None => {
                            Box::new(VecSerialSource::default())
                        }
                        Some(SerialSourceKind::Stdin) => {
                            let mut bytes = Vec::new();
                            std::io::Read::read_to_end(&mut std::io::stdin(), &mut bytes)
                                .expect("Failed to load file.");
                            Box::new(VecSerialSource::new(bytes))
                        }
                    };

                    let serial = Serial::new(sink, source);
                    bus.map_device(device_config.base, Box::new(serial))
                }

                DeviceKind::Timer => bus.map_device(device_config.base, Box::new(Timer::new())),

                DeviceKind::Clock => bus.map_device(device_config.base, Box::new(Clock::new())),
            };

            if let Some(irq) = device_config.irq {
                bus.register_irq(irq, device_id);
            }
        }

        Ok(Self::new(bus))
    }

    pub fn from_toml_file(path: impl AsRef<std::path::Path>) -> Result<Self, SystemBusError> {
        let text = std::fs::read_to_string(path).expect("failed to read machine TOML config");

        let config: MachineTomlConfig =
            toml::from_str(&text).expect("failed to parse machine TOML config");

        Self::from_toml_config(config)
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

    pub fn halted(&self) -> bool {
        self.cpu.is_halted()
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
