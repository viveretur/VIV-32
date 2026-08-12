use viv32::{Machine, MachineConfig, StdoutSerialSink, VecSerialSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = hello_world_image();

    let mut machine = Machine::from_config(MachineConfig {
        ram_size: 64 * 1024,
        ram_image: Some(image),
        ram_image_base: 0,
        serial_sink: Some(Box::new(StdoutSerialSink)),
        serial_source: Some(Box::new(VecSerialSource::default())),
        ..MachineConfig::default()
    })?;

    machine.reset();

    while machine.is_running() {
        machine.tick();
    }

    Ok(())
}

fn hello_world_image() -> Vec<u8> {
    // For now: hand-encoded VIV-32 machine code + null-terminated string.
    todo!("encode hello-world program")
}
