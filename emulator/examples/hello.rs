use viv32::Machine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut machine = Machine::from_toml_file("examples/hello.toml");

    machine.reset();

    while !machine.halted() {
        machine.tick();
    }

    Ok(())
}
