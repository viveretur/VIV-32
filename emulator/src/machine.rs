use crate::{Cpu, SystemBus, lifecycle::Tick};

pub struct Machine {
    cpu: Cpu,
}

impl Machine {
    pub fn new(ram_size: u32) -> Self {
        let bus = SystemBus::new(ram_size);
        let cpu = Cpu::new(bus);

        Self { cpu }
    }

    pub fn with_ram_image(ram_size: u32, image: &[u8]) -> Self {
        let bus = SystemBus::with_ram_image(ram_size, image);
        let cpu = Cpu::new(bus);

        Self { cpu }
    }

    pub fn from_file(ram_size: u32, path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let image = std::fs::read(path)?;
        Ok(Self::with_ram_image(ram_size, &image))
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn tick(&mut self) {
        self.cpu.tick();
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn machine_with_ram_image_starts_at_reset_vector() {
        let image = vec![
            0x00, 0x00, 0x00, 0x00, // NOP
        ];

        let machine = Machine::with_ram_image(1024, &image);

        assert_eq!(machine.cpu().pc(), 0);
    }

    #[test]
    fn machine_tick_executes_one_cpu_tick() {
        let image = vec![
            0x00, 0x00, 0x00, 0x00, // NOP
        ];

        let mut machine = Machine::with_ram_image(1024, &image);
        machine.reset();
        machine.tick();

        assert_eq!(machine.cpu().pc(), 4);
    }
}
