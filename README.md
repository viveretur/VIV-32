# VIV-32

*For illustrative purposes only*

A custom computer architecture project implementing:

- an ISA specification;
- a reference emulator;
- a QEMU target;
- a Verilog/SystemVerilog implementation;
- an assembler/linker toolchain;
- firmware and diagnostics;
- historical programming language experiments.

## Current Status

For the current development plan, see [TODO.md](TODO.md).

## Documentation

- `docs/architecture/`
- `docs/platform/`
- `docs/abi/`
- `docs/toolchain/`

## Major Components

- `spec/` — machine-readable architecture definitions
- `emulator/` — reference emulator
- `hdl/` — hardware implementation
- `toolchain/` — assembler, linker, disassembler
- `firmware/` — boot code, monitor, diagnostics
- `tests/` — conformance tests

## Development

Common commands are defined in `justfile`.

```bash
just --list
```
  
## Licence

This project is experimental and provided as-is. It is not production hardware, security, safety, or compliance tooling.

See [LICENSE](LICENSE).

