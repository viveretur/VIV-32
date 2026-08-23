use std::env;
use viv32::Machine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = env::args().nth(1).expect("usage: emulator <config.toml>");

    let mut machine = Machine::from_toml_file(config)?;

    machine.reset();

    while !machine.halted() {
        machine.tick();
    }

    Ok(())
}
